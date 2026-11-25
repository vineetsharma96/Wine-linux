use std::fs;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use androidpc_display::{packages_for_profile, DesktopProfileKind};
use androidpc_runtime_api::{DisplayEndpoint, DisplayType};
use serde::Serialize;
use tracing::{info, warn};

use crate::{packages::PackageResult, CoreError, EngineCore};

pub struct DesktopSession {
    id: String,
    pub env_id: String,
    pub display_number: u32,
    pub port: u16,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub started_at: Instant,
    child: Arc<Mutex<Child>>,
    shutdown: Option<ChildStdin>,
    control: PathBuf,
}

impl DesktopSession {
    fn child(&self) -> MutexGuard<'_, Child> {
        self.child.lock().unwrap_or_else(|error| error.into_inner())
    }

    pub fn is_alive(&mut self) -> bool {
        match self.child().try_wait() {
            Ok(None) => true,
            _ => false,
        }
    }

    fn wait_for_exit(child: &mut Child, timeout: Duration) -> std::io::Result<bool> {
        let deadline = Instant::now() + timeout;
        loop {
            if child.try_wait()?.is_some() {
                return Ok(true);
            }
            if Instant::now() >= deadline {
                return Ok(false);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn terminate(&mut self) -> std::io::Result<()> {
        // EOF wakes the guest supervisor, including when Android kills the app process.
        self.shutdown.take();
        let mut child = self.child();
        if Self::wait_for_exit(&mut child, Duration::from_secs(4))? {
            return Ok(());
        }
        // Termux PRoot handles SIGQUIT by killing its tracees, including those that
        // entered their own process group. The owned Child has not been reaped.
        #[cfg(unix)]
        unsafe {
            libc::kill(child.id() as i32, libc::SIGQUIT);
        }
        if Self::wait_for_exit(&mut child, Duration::from_millis(750))? {
            return Ok(());
        }
        #[cfg(unix)]
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        child.kill()?;
        if Self::wait_for_exit(&mut child, Duration::from_millis(750))? {
            Ok(())
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "Desktop supervisor did not exit"))
        }
    }

    fn reap_on_exit(&self) {
        let child = Arc::downgrade(&self.child);
        let reaper = std::thread::Builder::new().name("androidpc-desktop-reaper".into()).spawn(move || {
            while let Some(child) = child.upgrade() {
                let status = child.lock().unwrap_or_else(|error| error.into_inner()).try_wait();
                if !matches!(status, Ok(None)) {
                    break;
                }
                drop(child);
                std::thread::sleep(Duration::from_millis(250));
            }
        });
        if let Err(error) = reaper {
            warn!(session_id = %self.id, %error, "Could not start desktop reaper");
        }
    }

    fn endpoint(&self) -> DesktopEndpoint {
        DesktopEndpoint {
            session_id: self.id.clone(),
            endpoint: DisplayEndpoint {
                display_type: DisplayType::Xvnc,
                address: "127.0.0.1".to_string(),
                port: self.port,
                auth_required: false,
            },
        }
    }
}

impl Drop for DesktopSession {
    fn drop(&mut self) {
        match self.terminate() {
            Ok(()) => { let _ = fs::remove_dir_all(&self.control); }
            Err(error) => warn!(session_id = %self.id, %error, "Desktop cleanup did not complete"),
        }
    }
}

#[derive(Serialize)]
pub struct DesktopEndpoint {
    session_id: String,
    #[serde(flatten)]
    endpoint: DisplayEndpoint,
}

#[derive(Clone, Debug, Serialize)]
pub struct DesktopExit {
    session_id: String,
    env_id: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DesktopStatus {
    pub active: bool,
    pub env_id: Option<String>,
    pub display_number: u32,
    pub port: u16,
    pub width: u32,
    pub height: u32,
    pub xvnc_installed: bool,
    pub xfce_installed: bool,
    pub uptime_secs: u64,
    pub message: String,
    pub session_id: Option<String>,
    pub process_group: Option<u32>,
    pub xvnc_identity: Option<String>,
    pub xfce_identity: Option<String>,
    pub last_exit: Option<DesktopExit>,
}

pub fn check_guest_desktop_binaries(rootfs: &Path) -> (bool, bool) {
    let xvnc = rootfs.join("usr/bin/Xvnc").exists()
        || rootfs.join("usr/bin/Xtigervnc").exists();
    let xfce = rootfs.join("usr/bin/xfce4-session").exists()
        || rootfs.join("usr/bin/startxfce4").exists();
    (xvnc, xfce)
}

pub fn build_desktop_launcher_script(
    display_num: u32,
    width: u32,
    height: u32,
    dpi: u32,
    port: u16,
) -> String {
    format!(
        r#"#!/bin/sh
set -e
export DISPLAY=":{display_num}"
export HOME="/root"
export USER="root"
export LANG="C.UTF-8"
export LC_ALL="C.UTF-8"
export XDG_CURRENT_DESKTOP="XFCE"
export XDG_SESSION_TYPE="x11"
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_CACHE_HOME="$HOME/.cache"
export XDG_RUNTIME_DIR="$ANDROIDPC_DESKTOP_CONTROL/runtime"
export GLYCIN_DISABLE_SANDBOX="i-know-the-risks"
export PULSE_SERVER="127.0.0.1:4713"

mkdir -p /tmp/.X11-unix /usr/local/lib/androidpc-bin "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_CACHE_HOME" "$XDG_RUNTIME_DIR"
chmod 1777 /tmp/.X11-unix 2>/dev/null || true
chmod 700 "$XDG_RUNTIME_DIR" 2>/dev/null || true

# Preserve stdin explicitly because a POSIX asynchronous command defaults to /dev/null.
exec 3<&0
(IFS= read -r _ <&3 || true; : >"$ANDROIDPC_DESKTOP_CONTROL/stop") &
exec 3<&-

# Linux /proc start time (field 22) anchors the live child identities.
process_identity() {{
    IFS= read -r stat <"/proc/$1/stat" || return 1
    stat=${{stat##*) }}
    set -- $stat
    [ "$1" != Z ] && printf '%s\n' "${{20}}"
}}

# Ubuntu 26.04's glycin image loader invokes bubblewrap (bwrap) which requires
# kernel user namespaces unavailable inside rootless PRoot. Forward glycin
# loader invocations directly within the existing PRoot/SELinux sandbox.
cat <<'EOF' >/usr/local/lib/androidpc-bin/bwrap
#!/bin/sh
while [ "$#" -gt 0 ]; do
  case "$1" in
    /usr/libexec/glycin-loaders/*) exec "$@" ;;
    *) shift ;;
  esac
done
exec /usr/bin/bwrap "$@"
EOF
chmod 755 /usr/local/lib/androidpc-bin/bwrap
export PATH="/usr/local/lib/androidpc-bin:$PATH"

XVNC_BIN="/usr/bin/Xvnc"
if [ ! -x "$XVNC_BIN" ] && [ -x /usr/bin/Xtigervnc ]; then
    XVNC_BIN="/usr/bin/Xtigervnc"
fi

"$XVNC_BIN" ":{display_num}" \
    -geometry "{width}x{height}" \
    -depth 24 \
    -dpi "{dpi}" \
    -rfbport "{port}" \
    -SecurityTypes None \
    -interface 127.0.0.1 \
    -ac -pn >"$ANDROIDPC_DESKTOP_CONTROL/xvnc.log" 2>&1 &
XVNC_PID=$!
xvnc_start=$(process_identity "$XVNC_PID") || exit 1
printf '%s %s\n' "$XVNC_PID" "$xvnc_start" >"$ANDROIDPC_DESKTOP_CONTROL/xvnc"

cleanup() {{
    if [ "$(process_identity "$XVNC_PID" 2>/dev/null)" = "$xvnc_start" ]; then
        kill -TERM "$XVNC_PID" 2>/dev/null || true
        i=0
        while [ "$i" -lt 10 ] && [ "$(process_identity "$XVNC_PID" 2>/dev/null)" = "$xvnc_start" ]; do
            sleep 0.1
            i=$((i + 1))
        done
    fi
}}
trap cleanup EXIT
trap 'exit 1' INT TERM

i=0
while [ "$i" -lt 40 ]; do
    [ ! -f "$ANDROIDPC_DESKTOP_CONTROL/stop" ] || exit 0
    if [ -S "/tmp/.X11-unix/X{display_num}" ]; then
        break
    fi
    if ! kill -0 "$XVNC_PID" 2>/dev/null; then
        echo "Xvnc exited prematurely" >&2
        exit 1
    fi
    sleep 0.1
    i=$((i + 1))
done
[ -S "/tmp/.X11-unix/X{display_num}" ] || exit 1

if command -v dbus-run-session >/dev/null 2>&1 && command -v xfce4-session >/dev/null 2>&1; then
    dbus-run-session -- /bin/sh -c '
        xfce4-session &
        desktop=$!
        stat=$(cat "/proc/$desktop/stat") || exit 1
        stat=${{stat##*) }}
        set -- $stat
        printf "%s %s\n" "$desktop" "${{20}}" >"$ANDROIDPC_DESKTOP_CONTROL/xfce"
        while kill -0 "$desktop" 2>/dev/null; do
            if [ -f "$ANDROIDPC_DESKTOP_CONTROL/stop" ]; then
                xfce4-session-logout --logout --fast &
                break
            fi
            sleep 0.15
        done
        wait "$desktop"
    ' >"$ANDROIDPC_DESKTOP_CONTROL/xfce.log" 2>&1 &
    desktop_job=$!
elif command -v startxfce4 >/dev/null 2>&1; then
    startxfce4 >"$ANDROIDPC_DESKTOP_CONTROL/xfce.log" 2>&1 &
    desktop_job=$!
    desktop_start=$(process_identity "$desktop_job") || exit 1
    printf '%s %s\n' "$desktop_job" "$desktop_start" >"$ANDROIDPC_DESKTOP_CONTROL/xfce"
elif command -v xterm >/dev/null 2>&1; then
    xterm >"$ANDROIDPC_DESKTOP_CONTROL/xfce.log" 2>&1 &
    desktop_job=$!
fi
touch "$ANDROIDPC_DESKTOP_CONTROL/ready"
while [ "$(process_identity "$XVNC_PID" 2>/dev/null)" = "$xvnc_start" ]; do
    if [ -f "$ANDROIDPC_DESKTOP_CONTROL/stop" ]; then
        # Give XFCE's session bus time to complete the logout before Xvnc exits.
        i=0
        while [ -n "$desktop_job" ] && kill -0 "$desktop_job" 2>/dev/null && [ "$i" -lt 18 ]; do
            sleep 0.1
            i=$((i + 1))
        done
        exit 0
    fi
    if [ -n "$desktop_job" ] && ! kill -0 "$desktop_job" 2>/dev/null; then
        echo "Desktop process exited" >"$ANDROIDPC_DESKTOP_CONTROL/reason"
        exit 0
    fi
    sleep 0.15
done
echo "Xvnc exited" >"$ANDROIDPC_DESKTOP_CONTROL/reason"
"#
    )
}

impl EngineCore {
    pub(crate) fn has_live_desktop(&self, env_id: &str) -> Result<bool, CoreError> {
        let mut desktop = self.desktop_session.write()
            .map_err(|_| CoreError::SubsystemFailure("Desktop lock poisoned".into()))?;
        Ok(desktop.as_mut().is_some_and(|session| session.env_id == env_id && session.is_alive()))
    }

    fn resolve_target_env_id(&self, env_id: &str) -> Result<String, CoreError> {
        let environments = self
            .environments
            .read()
            .map_err(|_| CoreError::SubsystemFailure("Environment lock poisoned".into()))?;
        if !env_id.trim().is_empty() {
            if environments.contains_key(env_id) {
                return Ok(env_id.to_string());
            }
            return Err(CoreError::SubsystemFailure("Environment not installed".into()));
        }
        if let Some(ready_id) = environments.iter().find_map(|(id, mgr)| {
            let (xvnc, _) = check_guest_desktop_binaries(&mgr.config().rootfs_directory);
            if xvnc { Some(id.clone()) } else { None }
        }) {
            return Ok(ready_id);
        }
        environments
            .keys()
            .next()
            .cloned()
            .ok_or_else(|| CoreError::SubsystemFailure("No Linux environment installed".into()))
    }

    pub fn install_desktop_profile(
        &self,
        env_id: &str,
        profile: DesktopProfileKind,
    ) -> Result<PackageResult, CoreError> {
        let target_id = self.resolve_target_env_id(env_id)?;
        let distro_id = {
            let environments = self
                .environments
                .read()
                .map_err(|_| CoreError::SubsystemFailure("Environment lock poisoned".into()))?;
            let mgr = environments
                .get(&target_id)
                .ok_or_else(|| CoreError::SubsystemFailure("Environment not installed".into()))?;
            mgr.config().distribution.clone()
        };
        let packages = packages_for_profile(profile, &distro_id).join(" ");
        self.run_package_operation(&target_id, "install-minimal", &packages)
    }

    pub fn desktop_status(&self, env_id: &str) -> Result<DesktopStatus, CoreError> {
        let resolved_id = self.resolve_target_env_id(env_id).ok();
        let (xvnc_installed, xfce_installed) = if let Some(ref id) = resolved_id {
            let environments = self
                .environments
                .read()
                .map_err(|_| CoreError::SubsystemFailure("Environment lock poisoned".into()))?;
            if let Some(mgr) = environments.get(id) {
                check_guest_desktop_binaries(&mgr.config().rootfs_directory)
            } else {
                (false, false)
            }
        } else {
            (false, false)
        };

        let mut guard = self
            .desktop_session
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Desktop lock poisoned".into()))?;
        if let Some(session) = guard.as_mut() {
            if session.is_alive() {
                return Ok(DesktopStatus {
                    active: true,
                    env_id: Some(session.env_id.clone()),
                    display_number: session.display_number,
                    port: session.port,
                    width: session.width,
                    height: session.height,
                    xvnc_installed,
                    xfce_installed,
                    uptime_secs: session.started_at.elapsed().as_secs(),
                    session_id: Some(session.id.clone()),
                    process_group: Some(session.child().id()),
                    xvnc_identity: fs::read_to_string(session.control.join("xvnc")).ok().map(|s| s.trim().to_owned()),
                    xfce_identity: fs::read_to_string(session.control.join("xfce")).ok().map(|s| s.trim().to_owned()),
                    last_exit: self.desktop_exit.read().ok().and_then(|exit| exit.clone()),
                    message: format!(
                        "Active Xvnc :{} on 127.0.0.1:{} ({}x{})",
                        session.display_number, session.port, session.width, session.height
                    ),
                });
            }
            let reason = fs::read_to_string(session.control.join("reason")).unwrap_or_else(|_| "Desktop supervisor exited".into());
            self.record_desktop_exit(session, reason.trim());
            *guard = None;
        }

        let message = if !xvnc_installed {
            "Xvnc not installed in selected environment. Install Minimal Desktop profile."
        } else if !xfce_installed {
            "Xvnc installed; XFCE session not installed."
        } else {
            "Desktop ready (stopped)."
        };

        Ok(DesktopStatus {
            active: false,
            env_id: resolved_id,
            display_number: 1,
            port: 5901,
            width: 1280,
            height: 720,
            xvnc_installed,
            xfce_installed,
            uptime_secs: 0,
            message: message.to_string(),
            session_id: None,
            process_group: None,
            xvnc_identity: None,
            xfce_identity: None,
            last_exit: self.desktop_exit.read().ok().and_then(|exit| exit.clone()),
        })
    }

    pub fn start_desktop_session(
        &self,
        env_id: &str,
        display_num: u32,
        width: u32,
        height: u32,
        dpi: u32,
        port: u16,
    ) -> Result<DesktopEndpoint, CoreError> {
        let target_id = self.resolve_target_env_id(env_id)?;
        // Stop Linux acquires these locks in the same order. It cannot miss an
        // in-flight launch while the environment and desktop locks are held.
        let environments = self.environments.read()
            .map_err(|_| CoreError::SubsystemFailure("Environment lock poisoned".into()))?;
        let mgr = environments.get(&target_id)
            .ok_or_else(|| CoreError::SubsystemFailure("Environment not installed".into()))?;
        self.ensure_no_package_operation(&target_id)?;
        let mut guard = self.desktop_session.write()
            .map_err(|_| CoreError::SubsystemFailure("Desktop lock poisoned".into()))?;
        if let Some(session) = guard.as_mut() {
            if session.is_alive() {
                return if session.env_id == target_id && session.port == port && session.display_number == display_num {
                    Ok(session.endpoint())
                } else {
                    Err(CoreError::SubsystemFailure("Exit the existing desktop session before opening another".into()))
                };
            }
            self.record_desktop_exit(session, "Desktop supervisor exited");
            *guard = None;
        }
        let config = mgr.config().clone();
        let provider = self.resolve_proot_provider();

        let label = config.display_name.as_deref().unwrap_or(&config.id);
        let (xvnc_ok, xfce_ok) = check_guest_desktop_binaries(&config.rootfs_directory);
        if !xvnc_ok {
            return Err(CoreError::SubsystemFailure(format!(
                "Xvnc is not installed in '{label}'. Install Minimal Desktop first."
            )));
        }
        if !xfce_ok {
            warn!("XFCE session binary not found in '{label}'; starting Xvnc with fallback");
        }

        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        TcpListener::bind(addr).map_err(|error| CoreError::SubsystemFailure(
            format!("Desktop port {port} is already in use or unavailable: {error}")
        ))?;
        let script = build_desktop_launcher_script(display_num, width, height, dpi, port);

        let short_tmp = self.config.base_data_dir.join("dtmp");
        fs::create_dir_all(&short_tmp)?;
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let id = format!("s{elapsed:x}-{:x}", SEQUENCE.fetch_add(1, Ordering::Relaxed));
        let control = short_tmp.join(&id);
        fs::create_dir(&control)?;
        let mut launch_config = config.clone();
        launch_config.custom_mounts.push((short_tmp.clone(), Path::new("/tmp").to_path_buf()));

        let guest_cmd = vec![
            "/bin/sh".to_string(),
            "-c".to_string(),
            script,
        ];
        let args = provider.build_proot_command(&launch_config, &guest_cmd);
        let mut command = Command::new(provider.proot_binary());
        command
            .args(args)
            .env_clear()
            .env(
                "PATH",
                "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
            )
            .env("HOME", "/root")
            .env("USER", "root")
            .env("TERM", "xterm-256color")
            .env("PROOT_TMP_DIR", &short_tmp)
            .env("ANDROIDPC_DESKTOP_CONTROL", format!("/tmp/{id}"))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if let Some(native_dir) = self.config.native_lib_dir.as_ref() {
            let loader = native_dir.join("libproot_loader.so");
            if loader.is_file() {
                command.env("PROOT_LOADER", &loader);
            }
            let art_lib = self.config.runtime_artifacts_dir.join("lib");
            command.env(
                "LD_LIBRARY_PATH",
                format!("{}:{}", native_dir.display(), art_lib.display()),
            );
        }

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                let _ = fs::remove_dir(&control);
                return Err(CoreError::SubsystemFailure(format!("Failed to launch PRoot desktop session: {error}")));
            }
        };
        let mut session = DesktopSession {
            id, env_id: target_id, display_number: display_num, port, width, height, dpi,
            started_at: Instant::now(), shutdown: child.stdin.take(), child: Arc::new(Mutex::new(child)), control,
        };
        let deadline = Instant::now() + Duration::from_secs(8);
        let mut ready = false;
        while Instant::now() < deadline {
            if let Some(status) = session.child().try_wait()? {
                self.record_desktop_exit(&session, "Desktop startup failed");
                let xvnc_log = fs::read_to_string(session.control.join("xvnc.log"))
                    .unwrap_or_default();
                return Err(CoreError::SubsystemFailure(format!(
                    "Desktop server exited early ({status}): {}",
                    xvnc_log.lines().last().unwrap_or("no log")
                )));
            }
            if session.control.join("ready").is_file()
                && TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok() {
                ready = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(150));
        }

        if !ready {
            self.record_desktop_exit(&session, "Desktop startup timed out");
            if let Err(error) = session.terminate() {
                *guard = Some(session);
                return Err(error.into());
            }
            return Err(CoreError::SubsystemFailure(format!(
                "Timed out waiting for Xvnc on 127.0.0.1:{port}"
            )));
        }

        info!(
            "Desktop session online for '{label}' on 127.0.0.1:{} ({}x{})",
            port, width, height
        );
        let endpoint = session.endpoint();
        session.reap_on_exit();
        *guard = Some(session);
        Ok(endpoint)
    }

    fn record_desktop_exit(&self, session: &DesktopSession, reason: &str) {
        if let Ok(mut last) = self.desktop_exit.write() {
            *last = Some(DesktopExit {
                session_id: session.id.clone(), env_id: session.env_id.clone(), reason: reason.to_owned(),
            });
        }
        info!(session_id = %session.id, env_id = %session.env_id, %reason, "Desktop session exited");
    }

    pub fn stop_desktop_session(&self, session_id: Option<&str>) -> Result<(), CoreError> {
        self.stop_matching_desktop(|session| session_id.is_none_or(|id| session.id == id), "Desktop exit requested")
    }

    pub(crate) fn stop_desktop_for_environment(&self, env_id: &str, reason: &str) -> Result<(), CoreError> {
        self.stop_matching_desktop(|session| session.env_id == env_id, reason)
    }

    fn stop_matching_desktop(&self, matches: impl FnOnce(&DesktopSession) -> bool, reason: &str) -> Result<(), CoreError> {
        let mut guard = self.desktop_session.write()
            .map_err(|_| CoreError::SubsystemFailure("Desktop lock poisoned".into()))?;
        if let Some(session) = guard.as_mut().filter(|session| matches(session)) {
            if let Ok(mut client) = self.rfb_client.write() {
                let address = SocketAddr::from(([127, 0, 0, 1], session.port));
                if client.as_ref().and_then(|client| client.peer_addr()) == Some(address) {
                    if let Some(mut client) = client.take() {
                        client.disconnect();
                    }
                }
            }
            session.terminate()?;
            self.record_desktop_exit(session, reason);
            *guard = None;
        }
        Ok(())
    }

    pub fn resize_desktop_session(&self, width: u32, height: u32) -> Result<(u32, u32), CoreError> {
        let w = width.clamp(320, 3840);
        let h = height.clamp(240, 2160);
        let (env_id, display_num) = {
            let mut guard = self
                .desktop_session
                .write()
                .map_err(|_| CoreError::SubsystemFailure("Desktop lock poisoned".into()))?;
            let Some(session) = guard.as_mut() else {
                return Err(CoreError::SubsystemFailure("Desktop session not active".into()));
            };
            if !session.is_alive() {
                return Err(CoreError::SubsystemFailure("Desktop session exited".into()));
            }
            session.width = w;
            session.height = h;
            (session.env_id.clone(), session.display_number)
        };

        let (config, provider) = {
            let environments = self
                .environments
                .read()
                .map_err(|_| CoreError::SubsystemFailure("Environment lock poisoned".into()))?;
            let mgr = environments
                .get(&env_id)
                .ok_or_else(|| CoreError::SubsystemFailure("Environment not installed".into()))?;
            (mgr.config().clone(), self.resolve_proot_provider())
        };

        let short_tmp = self.config.base_data_dir.join("dtmp");
        let mut launch_config = config.clone();
        launch_config.custom_mounts.push((short_tmp.clone(), Path::new("/tmp").to_path_buf()));

        let resize_cmd = vec![
            "/bin/sh".to_string(),
            "-c".to_string(),
            format!("DISPLAY=:{display_num} xrandr --fb {w}x{h} 2>/dev/null || DISPLAY=:{display_num} xrandr --output VNC-0 --mode {w}x{h} 2>/dev/null || true"),
        ];
        let args = provider.build_proot_command(&launch_config, &resize_cmd);
        let mut command = Command::new(provider.proot_binary());
        command
            .args(args)
            .env_clear()
            .env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
            .env("HOME", "/root")
            .env("PROOT_TMP_DIR", &short_tmp)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(native_dir) = self.config.native_lib_dir.as_ref() {
            let loader = native_dir.join("libproot_loader.so");
            if loader.is_file() {
                command.env("PROOT_LOADER", &loader);
            }
            let art_lib = self.config.runtime_artifacts_dir.join("lib");
            command.env(
                "LD_LIBRARY_PATH",
                format!("{}:{}", native_dir.display(), art_lib.display()),
            );
        }
        let _ = command.status();
        Ok((w, h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_script_targets_loopback_and_supervises_children() {
        let script = build_desktop_launcher_script(1, 1280, 720, 160, 5901);
        assert!(script.contains("-interface 127.0.0.1"));
        assert!(script.contains("-rfbport \"5901\""));
        assert!(script.contains("xfce4-session"));
        assert!(script.contains("trap cleanup EXIT"));
        assert!(script.contains("xfce4-session-logout --logout --fast"));
        assert!(!script.contains("exec dbus-launch"));
    }

    #[test]
    fn guest_binary_check_detects_xvnc_and_xfce() {
        let base = std::env::temp_dir().join(format!("androidpc-desk-{}", std::process::id()));
        let bin = base.join("usr/bin");
        fs::create_dir_all(&bin).unwrap();
        assert_eq!(check_guest_desktop_binaries(&base), (false, false));
        fs::write(bin.join("Xvnc"), b"").unwrap();
        fs::write(bin.join("xfce4-session"), b"").unwrap();
        assert_eq!(check_guest_desktop_binaries(&base), (true, true));
        fs::remove_dir_all(&base).unwrap();
    }
}
