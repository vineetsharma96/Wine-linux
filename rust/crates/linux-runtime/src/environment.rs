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

    pub fn setup_networking_and_userspace(&self) -> Result<(), RuntimeError> {
        let rootfs = &self.config.rootfs_directory;
        let etc_dir = rootfs.join("etc");
        std::fs::create_dir_all(&etc_dir)?;

        std::fs::write(
            etc_dir.join("resolv.conf"),
            "nameserver 8.8.8.8\nnameserver 1.1.1.1\n",
        )?;
        std::fs::write(
            etc_dir.join("hosts"),
            "127.0.0.1 localhost android-pc\n::1 localhost\n",
        )?;
        std::fs::write(etc_dir.join("hostname"), "android-pc\n")?;

        let tmp_dir = rootfs.join("tmp");
        std::fs::create_dir_all(&tmp_dir)?;

        // Ensure emulated /dev/shm and PRoot temp directory exist
        std::fs::create_dir_all(rootfs.join("dev").join("shm"))?;
        std::fs::create_dir_all(rootfs.join("var").join("tmp"))?;

        let home_user_dir = rootfs.join("home").join(&self.config.default_username);
        std::fs::create_dir_all(&home_user_dir)?;

        // Provision startxfce.sh helper inside guest userspace
        let local_bin = rootfs.join("usr").join("local").join("bin");
        std::fs::create_dir_all(&local_bin)?;
        let xfce_script = local_bin.join("startxfce.sh");
        std::fs::write(&xfce_script, START_XFCE_SCRIPT)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&xfce_script, std::fs::Permissions::from_mode(0o755));
        }

        info!("Network and userspace configured for rootfs at {:?}", rootfs);
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        if let Some(pid) = self.active_process_id {
            self.supervisor.is_alive(pid)
        } else {
            false
        }
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
        info!("Starting environment '{}' (launching default shell)", self.config.id);
        self.execute_guest_command(&[])?;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), RuntimeError> {
        info!("Stopping environment '{}'", self.config.id);
        if let Some(pid) = self.active_process_id.take() {
            let _ = self.supervisor.send_signal(pid, androidpc_runtime_api::Signal::Term);
        }
        self.state = EnvironmentState::Stopped;
        Ok(())
    }

    fn supervisor(&self) -> &dyn ProcessSupervisor {
        &self.supervisor
    }
}
