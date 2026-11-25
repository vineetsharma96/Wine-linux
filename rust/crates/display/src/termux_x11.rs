use std::path::PathBuf;
use androidpc_runtime_api::{DisplayBackend, DisplayConfig, DisplayEndpoint, DisplayType, RuntimeError};

pub struct TermuxX11Backend {
    socket_path: PathBuf,
    is_active: bool,
}

impl Default for TermuxX11Backend {
    fn default() -> Self {
        Self::new()
    }
}

impl TermuxX11Backend {
    pub fn new() -> Self {
        Self {
            socket_path: PathBuf::from("/data/data/com.termux/files/usr/tmp/.X11-unix/X0"),
            is_active: false,
        }
    }

    pub fn with_socket_path(path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: path.into(),
            is_active: false,
        }
    }

    pub fn is_socket_present(&self) -> bool {
        self.socket_path.exists()
    }
}

impl DisplayBackend for TermuxX11Backend {
    fn backend_type(&self) -> DisplayType {
        DisplayType::TermuxX11
    }

    fn is_running(&self) -> bool {
        self.is_active && self.is_socket_present()
    }

    fn start(&mut self, config: &DisplayConfig) -> Result<DisplayEndpoint, RuntimeError> {
        self.is_active = true;
        Ok(DisplayEndpoint {
            display_type: DisplayType::TermuxX11,
            address: self.socket_path.to_string_lossy().to_string(),
            port: config.port,
            auth_required: false,
        })
    }

    fn stop(&mut self) -> Result<(), RuntimeError> {
        self.is_active = false;
        Ok(())
    }

    fn endpoint(&self) -> Option<DisplayEndpoint> {
        if self.is_active {
            Some(DisplayEndpoint {
                display_type: DisplayType::TermuxX11,
                address: self.socket_path.to_string_lossy().to_string(),
                port: 0,
                auth_required: false,
            })
        } else {
            None
        }
    }
}
