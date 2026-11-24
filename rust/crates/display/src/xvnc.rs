use std::path::PathBuf;
use tracing::info;

use androidpc_process::DefaultProcessSupervisor;
use androidpc_runtime_api::{
    DisplayBackend, DisplayConfig, DisplayEndpoint, DisplayType, ProcessCommand,
    ProcessId, ProcessSupervisor, RuntimeError,
};

pub struct XvncDisplayBackend {
    xvnc_binary_path: PathBuf,
    supervisor: DefaultProcessSupervisor,
    active_process_id: Option<ProcessId>,
    current_endpoint: Option<DisplayEndpoint>,
}

impl XvncDisplayBackend {
    pub fn new(xvnc_binary_path: impl Into<PathBuf>) -> Self {
        Self {
            xvnc_binary_path: xvnc_binary_path.into(),
            supervisor: DefaultProcessSupervisor::new(),
            active_process_id: None,
            current_endpoint: None,
        }
    }

    pub fn build_xvnc_args(config: &DisplayConfig) -> Vec<String> {
        vec![
            format!(":{}", config.display_number),
            "-geometry".to_string(),
            format!("{}x{}", config.width, config.height),
            "-depth".to_string(),
            "24".to_string(),
            "-dpi".to_string(),
            config.dpi.to_string(),
            "-rfbport".to_string(),
            config.port.to_string(),
            "-SecurityTypes".to_string(),
            "None".to_string(),
            "-listen".to_string(),
            config.listen_address.clone(),
            "-ac".to_string(), // Disable access control for local rootless loopback
            "-pn".to_string(), // Do not exit on client disconnect
        ]
    }
}

impl DisplayBackend for XvncDisplayBackend {
    fn backend_type(&self) -> DisplayType {
        DisplayType::Xvnc
    }

    fn is_running(&self) -> bool {
        if let Some(pid) = self.active_process_id {
            self.supervisor.is_alive(pid)
        } else {
            false
        }
    }

    fn start(&mut self, config: &DisplayConfig) -> Result<DisplayEndpoint, RuntimeError> {
        info!(
            "Starting Xvnc display :{} on {}:{} ({}x{}, {}dpi)",
            config.display_number, config.listen_address, config.port, config.width, config.height, config.dpi
        );

        let args = Self::build_xvnc_args(config);
        let cmd = ProcessCommand {
            program: self.xvnc_binary_path.to_string_lossy().to_string(),
            args,
            env: vec![("DISPLAY".to_string(), format!(":{}", config.display_number))],
            cwd: None,
            uid: None,
            gid: None,
        };

        let pid = self.supervisor.spawn_process(&cmd)?;
        self.active_process_id = Some(pid);

        let endpoint = DisplayEndpoint {
            display_type: DisplayType::Xvnc,
            address: config.listen_address.clone(),
            port: config.port,
            auth_required: false,
        };

        self.current_endpoint = Some(endpoint.clone());
        info!("Xvnc display server online with PID {}", pid);
        Ok(endpoint)
    }

    fn stop(&mut self) -> Result<(), RuntimeError> {
        if let Some(pid) = self.active_process_id.take() {
            info!("Stopping Xvnc display server (PID {})", pid);
            self.supervisor.send_signal(pid, androidpc_runtime_api::Signal::Term)?;
        }
        self.current_endpoint = None;
        Ok(())
    }

    fn endpoint(&self) -> Option<DisplayEndpoint> {
        self.current_endpoint.clone()
    }
}
