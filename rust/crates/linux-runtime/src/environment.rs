use std::sync::Arc;
use tracing::info;

use androidpc_configuration::EnvironmentConfig;
use androidpc_process::DefaultProcessSupervisor;
use androidpc_runtime_api::{
    EnvironmentManager, EnvironmentState, ProcessCommand, ProcessId, ProcessSupervisor, RuntimeError,
};
use crate::proot::ProotRuntimeProvider;

pub struct ProotEnvironmentManager {
    config: EnvironmentConfig,
    state: EnvironmentState,
    provider: Arc<ProotRuntimeProvider>,
    supervisor: DefaultProcessSupervisor,
    active_process_id: Option<ProcessId>,
}

impl ProotEnvironmentManager {
    pub fn new(config: EnvironmentConfig, provider: Arc<ProotRuntimeProvider>) -> Self {
        Self {
            config,
            state: EnvironmentState::Uninitialized,
            provider,
            supervisor: DefaultProcessSupervisor::new(),
            active_process_id: None,
        }
    }

    pub fn config(&self) -> &EnvironmentConfig {
        &self.config
    }

    pub fn setup_networking_and_userspace(&self) -> Result<(), RuntimeError> {
        let rootfs = &self.config.rootfs_directory;
        let etc_dir = rootfs.join("etc");
        let _ = std::fs::create_dir_all(&etc_dir);

        let resolv_conf = etc_dir.join("resolv.conf");
        let _ = std::fs::write(&resolv_conf, "nameserver 8.8.8.8\nnameserver 1.1.1.1\n");

        let hosts_file = etc_dir.join("hosts");
        let _ = std::fs::write(&hosts_file, "127.0.0.1 localhost android-pc\n::1 localhost\n");

        let hostname_file = etc_dir.join("hostname");
        let _ = std::fs::write(&hostname_file, "android-pc\n");

        let tmp_dir = rootfs.join("tmp");
        let _ = std::fs::create_dir_all(&tmp_dir);

        let home_user_dir = rootfs.join("home").join(&self.config.default_username);
        let _ = std::fs::create_dir_all(&home_user_dir);

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

    pub fn execute_guest_command(&mut self, command: &[String]) -> Result<ProcessId, RuntimeError> {
        let proot_args = self.provider.build_proot_command(&self.config, command);
        let proot_bin = self.provider.proot_binary().to_string_lossy().to_string();

        let proc_cmd = ProcessCommand {
            program: proot_bin,
            args: proot_args,
            env: vec![
                ("TERM".to_string(), "xterm-256color".to_string()),
                ("HOME".to_string(), format!("/home/{}", self.config.default_username)),
                ("USER".to_string(), self.config.default_username.clone()),
                ("PATH".to_string(), "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string()),
            ],
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
