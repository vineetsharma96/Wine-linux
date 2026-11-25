use std::sync::Arc;
use tracing::info;

use androidpc_configuration::EnvironmentConfig;
use androidpc_process::DefaultProcessSupervisor;
use androidpc_runtime_api::{
    EnvironmentManager, EnvironmentState, ProcessCommand, ProcessId, ProcessSupervisor, RuntimeError,
};
use crate::proot::ProotRuntimeProvider;

const START_XFCE_SCRIPT: &str = r#"#!/bin/sh
# Android PC Environment — XFCE 4 Desktop Session Launcher
set -e
export DISPLAY="${DISPLAY:-:1}"
export LANG="C.UTF-8"
export LC_ALL="C.UTF-8"
export XDG_CURRENT_DESKTOP="XFCE"
export XDG_SESSION_TYPE="x11"
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_CACHE_HOME="$HOME/.cache"
export PULSE_SERVER="127.0.0.1:4713"

rm -f /tmp/.X1-lock /tmp/.X11-unix/X1 || true
if command -v dbus-launch >/dev/null 2>&1; then
    eval "$(dbus-launch --sh-syntax --exit-with-session)"
fi
if command -v xfce4-session >/dev/null 2>&1; then
    exec xfce4-session
elif command -v startxfce4 >/dev/null 2>&1; then
    exec startxfce4
else
    exec xterm
fi
"#;

pub struct ProotEnvironmentManager {
    config: EnvironmentConfig,
    state: EnvironmentState,
    provider: Arc<ProotRuntimeProvider>,
    artifacts_dir: Option<std::path::PathBuf>,
    supervisor: DefaultProcessSupervisor,
    active_process_id: Option<ProcessId>,
}

impl ProotEnvironmentManager {
    pub fn new(config: EnvironmentConfig, provider: Arc<ProotRuntimeProvider>) -> Self {
        Self {
            config,
            state: EnvironmentState::Uninitialized,
            provider,
            artifacts_dir: None,
            supervisor: DefaultProcessSupervisor::new(),
            active_process_id: None,
        }
    }

    /// Reconstructs an environment manager from a previously provisioned rootfs on disk.
    pub fn from_existing(config: EnvironmentConfig, provider: Arc<ProotRuntimeProvider>) -> Self {
        let state = if config.rootfs_directory.join("etc").join("resolv.conf").exists() {
            EnvironmentState::Stopped
        } else {
            EnvironmentState::Uninitialized
        };
        Self {
            config,
            state,
            provider,
            artifacts_dir: None,
            supervisor: DefaultProcessSupervisor::new(),
            active_process_id: None,
        }
    }

    pub fn with_artifacts_dir(mut self, artifacts_dir: impl Into<std::path::PathBuf>) -> Self {
        self.artifacts_dir = Some(artifacts_dir.into());
        self
    }

    pub fn config(&self) -> &EnvironmentConfig {
        &self.config
    }

    pub fn set_last_started(&mut self, timestamp: Option<u64>) {
        self.config.last_started_at_epoch_ms = timestamp;
    }

    pub fn set_box64_config(&mut self, box64: androidpc_configuration::Box64EnvironmentConfig) {
        self.config.box64 = box64;
    }

    pub fn setup_networking_and_userspace(&self) -> Result<(), RuntimeError> {
        let rootfs = self.config.rootfs_directory.canonicalize()?;
        let create_guest_dir = |relative: &str| -> Result<std::path::PathBuf, RuntimeError> {
            let mut dir = rootfs.clone();
            for part in std::path::Path::new(relative).components() {
                dir.push(part);
                match std::fs::symlink_metadata(&dir) {
                    Ok(metadata) if metadata.is_dir() => {}
                    Ok(_) => return Err(RuntimeError::InitializationFailed(format!(
                        "Unsafe directory in guest setup: {relative}"
                    ))),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => std::fs::create_dir(&dir)?,
                    Err(e) => return Err(e.into()),
                }
            }
            Ok(dir)
        };
        let write_guest_file = |dir: &std::path::Path, name: &str, contents: &str| -> Result<(), RuntimeError> {
            let path = dir.join(name);
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_symlink() => std::fs::remove_file(&path)?,
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
            std::fs::write(path, contents)?;
            Ok(())
        };
        let etc_dir = create_guest_dir("etc")?;

        write_guest_file(
            &etc_dir,
            "resolv.conf",
            "nameserver 8.8.8.8\nnameserver 1.1.1.1\n",
        )?;
        write_guest_file(
            &etc_dir,
            "hosts",
            "127.0.0.1 localhost android-pc\n::1 localhost\n",
        )?;
        write_guest_file(&etc_dir, "hostname", "android-pc\n")?;
        create_guest_dir("tmp")?;

        // Ensure emulated /dev/shm and PRoot temp directory exist
        create_guest_dir("dev/shm")?;
        create_guest_dir("var/tmp")?;

        create_guest_dir(&format!("home/{}", self.config.default_username))?;

        // Provision startxfce.sh helper inside guest userspace
        let local_bin = create_guest_dir("usr/local/bin")?;
        write_guest_file(&local_bin, "startxfce.sh", START_XFCE_SCRIPT)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(local_bin.join("startxfce.sh"), std::fs::Permissions::from_mode(0o755))?;
        }

        info!("Network and userspace configured for rootfs at {:?}", rootfs);
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.supervisor.list_processes().iter().any(|process| process.running)
    }

    pub fn active_pid(&self) -> Option<ProcessId> {
        self.active_process_id
    }

    pub fn build_guest_env(&self, extra_env: &[(String, String)]) -> Vec<(String, String)> {
        let proot_tmp = self.config.rootfs_directory.join("tmp");
        let proot_dir = self
            .provider
            .proot_binary()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("/data/data/com.androidpc.environment/files/artifacts"));
        let lib_dir = proot_dir.join("lib");
        let loader_path = if proot_dir.join("libproot_loader.so").exists() {
            proot_dir.join("libproot_loader.so")
        } else {
            proot_dir.join("loader")
        };
        let loader32_path = if proot_dir.join("libproot_loader32.so").exists() {
            proot_dir.join("libproot_loader32.so")
        } else {
            proot_dir.join("loader32")
        };

        let mut ld_paths = vec![lib_dir.display().to_string(), proot_dir.display().to_string()];
        if let Some(ref art_dir) = self.artifacts_dir {
            let art_lib = art_dir.join("lib");
            if art_lib != lib_dir {
                ld_paths.push(art_lib.display().to_string());
            }
        }

        let mut env = vec![
            ("TERM".to_string(), "xterm-256color".to_string()),
            ("HOME".to_string(), format!("/home/{}", self.config.default_username)),
            ("USER".to_string(), self.config.default_username.clone()),
            ("DISPLAY".to_string(), format!(":{}", self.config.display.display_number)),
            ("PULSE_SERVER".to_string(), "127.0.0.1:4713".to_string()),
            ("PROOT_TMP_DIR".to_string(), proot_tmp.display().to_string()),
            ("PROOT_NO_SECCOMP".to_string(), "1".to_string()),
            ("PROOT_LOADER".to_string(), loader_path.display().to_string()),
            ("PROOT_LOADER_32".to_string(), loader32_path.display().to_string()),
            ("LD_LIBRARY_PATH".to_string(), ld_paths.join(":")),
            (
                "PATH".to_string(),
                "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
            ),
        ];
        for (k, v) in extra_env {
            env.push((k.clone(), v.clone()));
        }
        env
    }

    pub fn execute_guest_command(&mut self, command: &[String]) -> Result<ProcessId, RuntimeError> {
        self.execute_guest_command_with_env(command, &[])
    }

    pub fn verify_guest(&self, distribution: &str) -> Result<String, RuntimeError> {
        self.verify_guest_identity(distribution, false)
    }

    pub fn verify_and_provision_guest(&self, distribution: &str, expected_version: &str) -> Result<String, RuntimeError> {
        let ubuntu = distribution == "ubuntu";
        let version = self.verify_guest_identity(distribution, ubuntu)?;
        if version != expected_version {
            return Err(RuntimeError::InitializationFailed(
                format!("Guest release {version} differs from expected release {expected_version}"),
            ));
        }
        if ubuntu {
            crate::apt_sources::configure_ports(&self.config.rootfs_directory)?;
        }
        Ok(version)
    }

    fn verify_guest_identity(&self, distribution: &str, require_codename: bool) -> Result<String, RuntimeError> {
        let command = [
            "/bin/sh".to_string(),
            "-ec".to_string(),
            "dpkg --print-architecture; cat /etc/os-release; apt --version; /usr/bin/env true".to_string(),
        ];
        let output = std::process::Command::new(self.provider.proot_binary())
            .args(self.provider.build_proot_command(&self.config, &command))
            .env_clear()
            .envs(self.build_guest_env(&[]))
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RuntimeError::InitializationFailed(format!(
                "Guest health check failed ({}): {}", output.status,
                stderr.chars().take(750).collect::<String>()
            )));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let os_field = |key: &str| {
            stdout.lines().filter_map(|line| line.split_once('='))
                .find(|(name, _)| *name == key)
                .map(|(_, value)| value.trim_matches('"'))
        };
        if stdout.lines().next() != Some("arm64") || os_field("ID") != Some(distribution) {
            return Err(RuntimeError::InitializationFailed("Unexpected guest architecture or distribution".into()));
        }
        if require_codename && !os_field("VERSION_CODENAME").is_some_and(|codename| {
            !codename.is_empty() && codename.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        }) {
            return Err(RuntimeError::InitializationFailed("Ubuntu release has no valid VERSION_CODENAME".into()));
        }
        os_field("VERSION_ID").filter(|version| !version.is_empty()).map(str::to_owned)
            .ok_or_else(|| RuntimeError::InitializationFailed("Guest release is missing VERSION_ID".into()))
    }

    pub fn repair_guest(&mut self) -> Result<(), RuntimeError> {
        self.state = EnvironmentState::Recovering;
        let result = (|| {
            self.setup_networking_and_userspace()?;
            self.verify_and_provision_guest(&self.config.distribution, &self.config.version)?;
            Ok(())
        })();
        self.state = if result.is_ok() { EnvironmentState::Stopped } else { EnvironmentState::Failed };
        result
    }

    pub fn execute_guest_command_with_env(
        &mut self,
        command: &[String],
        extra_env: &[(String, String)],
    ) -> Result<ProcessId, RuntimeError> {
        let proot_args = self.provider.build_proot_command(&self.config, command);
        let proot_bin = self.provider.proot_binary().to_string_lossy().to_string();

        let proc_cmd = ProcessCommand {
            program: proot_bin,
            args: proot_args,
            env: self.build_guest_env(extra_env),
            cwd: None,
            uid: Some(0),
            gid: Some(0),
        };

        let pid = self.supervisor.spawn_process(&proc_cmd)?;
        self.active_process_id = Some(pid);
        self.state = EnvironmentState::Running;
        Ok(pid)
    }
}

impl EnvironmentManager for ProotEnvironmentManager {
    fn environment_id(&self) -> &str {
        &self.config.id
    }

    fn state(&self) -> EnvironmentState {
        if self.state == EnvironmentState::Running && !self.is_running() {
            return EnvironmentState::Stopped;
        }
        self.state
    }

    fn initialize(&mut self) -> Result<(), RuntimeError> {
        info!("Initializing rootless environment '{}'", self.config.id);
        std::fs::create_dir_all(&self.config.rootfs_directory)?;
        self.setup_networking_and_userspace()?;
        self.state = EnvironmentState::Stopped;
        Ok(())
    }

    fn start(&mut self) -> Result<(), RuntimeError> {
        if self.is_running() {
            return Ok(());
        }
        info!("Starting environment '{}' (launching default shell)", self.config.id);
        self.execute_guest_command(&[])?;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), RuntimeError> {
        info!("Stopping environment '{}'", self.config.id);
        self.supervisor.terminate_all()?;
        self.active_process_id = None;
        self.state = EnvironmentState::Stopped;
        Ok(())
    }

    fn supervisor(&self) -> &dyn ProcessSupervisor {
        &self.supervisor
    }
}
