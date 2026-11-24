pub mod diagnostics;
pub mod distributions;
pub mod health;

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use thiserror::Error;
use tracing::info;

use androidpc_configuration::{EngineConfig, EnvironmentConfig};
use androidpc_display::XvncDisplayBackend;
use androidpc_downloader::DefaultArtifactProvider;
use androidpc_filesystem::{ArchiveExtractor, StorageManager};
use androidpc_linux_runtime::{ProotEnvironmentManager, ProotRuntimeProvider};
use androidpc_runtime_api::{
    Architecture, ArtifactProvider, DisplayBackend, DisplayConfig, DisplayEndpoint,
    EngineStatusInfo, EnvironmentManager, EnvironmentState, HealthReport, LinuxDistribution,
    ProcessId, RuntimeProvider, SystemDiagnostics, TerminalSession,
};
use androidpc_terminal::DefaultTerminalSession;
use androidpc_wine::{
    WineArch, WineCommandBuilder, WineConfig, WineProcessSupervisor, WineSessionState, WineStatusInfo,
};

pub const ENGINE_VERSION: &str = "0.1.0-alpha.0";
pub const ENGINE_NAME: &str = "Android PC Environment Engine";

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Configuration error: {0}")]
    Config(#[from] androidpc_configuration::ConfigError),

    #[error("Logging error: {0}")]
    Log(#[from] androidpc_logging::LogError),

    #[error("Filesystem error: {0}")]
    Filesystem(#[from] androidpc_filesystem::FilesystemError),

    #[error("Downloader error: {0}")]
    Downloader(#[from] androidpc_downloader::DownloaderError),

    #[error("Runtime error: {0}")]
    Runtime(#[from] androidpc_runtime_api::RuntimeError),

    #[error("Engine already initialized")]
    AlreadyInitialized,

    #[error("Engine not initialized")]
    NotInitialized,

    #[error("Subsystem failure: {0}")]
    SubsystemFailure(String),

    #[error("Wine error: {0}")]
    Wine(#[from] androidpc_wine::WineError),

    #[error("Serialization failure: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub struct EngineCore {
    config: EngineConfig,
    start_time: Instant,
    pub init_epoch_ms: AtomicU64,
    pub storage: StorageManager,
    pub artifacts: DefaultArtifactProvider,
    pub providers: RwLock<Vec<Box<dyn RuntimeProvider>>>,
    pub distributions: RwLock<Vec<Box<dyn LinuxDistribution>>>,
    pub display_backend: RwLock<Option<XvncDisplayBackend>>,
    pub terminal_sessions: RwLock<HashMap<String, DefaultTerminalSession>>,
    pub environments: RwLock<HashMap<String, ProotEnvironmentManager>>,
    pub rfb_client: RwLock<Option<androidpc_display::RfbClient>>,
    pub wine_supervisor: Arc<WineProcessSupervisor>,
}

impl EngineCore {
    pub fn new(config: EngineConfig) -> Result<Self, CoreError> {
        config.validate()?;
        let _ = androidpc_logging::init_logging(&config.log_level);
        info!("Initializing Android PC Environment Engine v{}", ENGINE_VERSION);

        let now_epoch_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let storage = StorageManager::new(&config.base_data_dir);
        let proot_bin = config.runtime_artifacts_dir.join("proot-arm64");
        let proot_provider = Box::new(ProotRuntimeProvider::new(proot_bin));

        let mut core = Self {
            config,
            start_time: Instant::now(),
            init_epoch_ms: AtomicU64::new(now_epoch_ms),
            storage,
            artifacts: DefaultArtifactProvider::new(),
            providers: RwLock::new(vec![proot_provider]),
            distributions: RwLock::new(Vec::new()),
            display_backend: RwLock::new(None),
            terminal_sessions: RwLock::new(HashMap::new()),
            environments: RwLock::new(HashMap::new()),
            rfb_client: RwLock::new(None),
            wine_supervisor: Arc::new(WineProcessSupervisor::new()),
        };

        core.register_default_distributions();
        info!("Engine initialization complete. Phase 1, 2, 3 & 4 Rootless Runtime, Display, Environment & Graphics Engine online.");
        androidpc_logging::push_log(
            "INFO",
            "androidpc::core",
            &format!("Engine initialized with host architecture {:?}", Architecture::host()),
        );

        Ok(core)
    }

    pub fn version() -> &'static str {
        ENGINE_VERSION
    }

    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    pub fn status(&self) -> EngineStatusInfo {
        let uptime = self.start_time.elapsed().as_secs();
        let env_count = self.environments.read().map(|e| e.len()).unwrap_or(0);
        EngineStatusInfo {
            state: "Ready".to_string(),
            version: ENGINE_VERSION.to_string(),
            build_profile: if cfg!(debug_assertions) { "debug" } else { "release" }.to_string(),
            host_arch: Architecture::host(),
            active_environments: env_count,
            uptime_seconds: uptime,
        }
    }

    pub fn run_health_check(&self) -> HealthReport {
        health::execute_health_check(&self.config)
    }

    pub fn collect_diagnostics(&self) -> SystemDiagnostics {
        diagnostics::collect_system_diagnostics(&self.config, &self.run_health_check())
    }

    pub fn create_environment_config(&self, id: &str, distro: &str) -> EnvironmentConfig {
        EnvironmentConfig::new_default(id, distro, &self.config.base_data_dir)
    }

    pub fn verify_archive(&self, path: &Path, expected_sha256: &str) -> Result<bool, CoreError> {
        Ok(self.artifacts.verify_artifact(path, expected_sha256)?)
    }

    pub fn extract_rootfs(&self, archive: &Path, dest: &Path) -> Result<u64, CoreError> {
        let entries = if archive.to_string_lossy().ends_with(".gz") {
            ArchiveExtractor::extract_tar_gz(archive, dest)?
        } else {
            ArchiveExtractor::extract_tar(archive, dest)?
        };
        Ok(entries)
    }

    fn register_default_distributions(&mut self) {
        if let Ok(mut dists) = self.distributions.write() {
            dists.push(Box::new(distributions::UbuntuDistribution::new()));
            dists.push(Box::new(distributions::DebianDistribution::new()));
        }
    }

    pub fn list_distribution_ids(&self) -> Vec<String> {
        match self.distributions.read() {
            Ok(dists) => dists.iter().map(|d| d.id().to_string()).collect(),
            Err(_) => vec!["ubuntu".to_string(), "debian".to_string()],
        }
    }

    pub fn start_display_server(&self, config: &DisplayConfig) -> Result<DisplayEndpoint, CoreError> {
        let xvnc_bin = self.config.runtime_artifacts_dir.join("tigervnc-arm64").join("Xvnc");
        let mut backend = XvncDisplayBackend::new(xvnc_bin);
        let endpoint = backend.start(config).map_err(CoreError::Runtime)?;
        let mut guard = self.display_backend.write().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        *guard = Some(backend);
        info!("Display server registered in core on port {}", endpoint.port);
        Ok(endpoint)
    }

    pub fn stop_display_server(&self) -> Result<(), CoreError> {
        let mut guard = self.display_backend.write().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(mut backend) = guard.take() {
            backend.stop().map_err(CoreError::Runtime)?;
        }
        Ok(())
    }

    pub fn is_display_running(&self) -> bool {
        match self.display_backend.read() {
            Ok(guard) => guard.as_ref().map(|b| b.is_running()).unwrap_or(false),
            Err(_) => false,
        }
    }

    pub fn create_terminal_session(&self, id: &str, cols: u16, rows: u16) -> Result<i32, CoreError> {
        let session = DefaultTerminalSession::new(id, cols, rows).map_err(CoreError::Runtime)?;
        let fd = session.master_fd();
        let mut guard = self.terminal_sessions.write().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        guard.insert(id.to_string(), session);
        Ok(fd)
    }

    pub fn write_terminal(&self, id: &str, data: &[u8]) -> Result<usize, CoreError> {
        let mut guard = self.terminal_sessions.write().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(session) = guard.get_mut(id) {
            session.write_input(data).map_err(CoreError::Runtime)
        } else {
            Err(CoreError::Runtime(androidpc_runtime_api::RuntimeError::TerminalError(format!("Session not found: {id}"))))
        }
    }

    pub fn read_terminal(&self, id: &str, buf: &mut [u8]) -> Result<usize, CoreError> {
        let mut guard = self.terminal_sessions.write().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(session) = guard.get_mut(id) {
            session.read_output(buf).map_err(CoreError::Runtime)
        } else {
            Err(CoreError::Runtime(androidpc_runtime_api::RuntimeError::TerminalError(format!("Session not found: {id}"))))
        }
    }

    pub fn resize_terminal(&self, id: &str, cols: u16, rows: u16) -> Result<(), CoreError> {
        let mut guard = self.terminal_sessions.write().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(session) = guard.get_mut(id) {
            session.resize(cols, rows).map_err(CoreError::Runtime)
        } else {
            Err(CoreError::Runtime(androidpc_runtime_api::RuntimeError::TerminalError(format!("Session not found: {id}"))))
        }
    }

    pub fn close_terminal(&self, id: &str) -> Result<(), CoreError> {
        let mut guard = self.terminal_sessions.write().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(mut session) = guard.remove(id) {
            session.close().map_err(CoreError::Runtime)?;
        }
        Ok(())
    }

    pub fn provision_environment(&self, id: &str, distro: &str) -> Result<EnvironmentConfig, CoreError> {
        let env_config = self.create_environment_config(id, distro);
        env_config.validate()?;

        self.storage.ensure_environment_directories(id)?;

        let proot_bin = self.config.runtime_artifacts_dir.join("proot-arm64");
        let provider = Arc::new(ProotRuntimeProvider::new(proot_bin));

        let mut manager = ProotEnvironmentManager::new(env_config.clone(), provider);
        manager.initialize().map_err(CoreError::Runtime)?;

        let mut guard = self.environments.write().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        guard.insert(id.to_string(), manager);
        info!("Environment '{id}' provisioned successfully ({distro})");
        Ok(env_config)
    }

    pub fn execute_environment_command(&self, id: &str, cmd: &[String]) -> Result<ProcessId, CoreError> {
        let mut guard = self.environments.write().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(manager) = guard.get_mut(id) {
            manager.execute_guest_command(cmd).map_err(CoreError::Runtime)
        } else {
            Err(CoreError::Runtime(androidpc_runtime_api::RuntimeError::Internal(format!("Environment not found: {id}"))))
        }
    }

    pub fn stop_environment(&self, id: &str) -> Result<(), CoreError> {
        let mut guard = self.environments.write().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(mut manager) = guard.remove(id) {
            manager.stop().map_err(CoreError::Runtime)?;
        }
        Ok(())
    }

    pub fn get_environment_state(&self, id: &str) -> Option<EnvironmentState> {
        self.environments
            .read()
            .ok()
            .and_then(|guard| guard.get(id).map(|m| m.state()))
    }

    pub fn list_environments(&self) -> Vec<EnvironmentConfig> {
        match self.environments.read() {
            Ok(guard) => guard.values().map(|m| m.config().clone()).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn get_artifact_metadata(&self, distro: &str) -> Option<androidpc_downloader::ArtifactMetadata> {
        self.artifacts.find_by_distribution(distro, Architecture::host()).cloned()
    }

    pub fn connect_display_client(&self, address: &str, port: u16) -> Result<(u16, u16), CoreError> {
        let client = match androidpc_display::RfbClient::connect(address, port) {
            Ok(c) => c,
            Err(e) => {
                androidpc_logging::push_log(
                    "WARN",
                    "androidpc::core",
                    &format!("Failed to connect to live RFB server ({e}). Falling back to offline client."),
                );
                androidpc_display::RfbClient::new_offline(1280, 720)
            }
        };

        let dims = (client.width, client.height);
        let mut guard = self.rfb_client.write().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        *guard = Some(client);
        Ok(dims)
    }

    pub fn get_framebuffer_slice(&self, out: &mut [u8]) -> Result<usize, CoreError> {
        let guard = self.rfb_client.read().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(client) = guard.as_ref() {
            let len = out.len().min(client.framebuffer.len());
            out[..len].copy_from_slice(&client.framebuffer[..len]);
            Ok(len)
        } else {
            Err(CoreError::Runtime(androidpc_runtime_api::RuntimeError::Internal("RFB client not connected".into())))
        }
    }

    pub fn send_display_pointer(&self, button_mask: u8, x: u16, y: u16) -> Result<(), CoreError> {
        let mut guard = self.rfb_client.write().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(client) = guard.as_mut() {
            client.send_pointer(button_mask, x, y).map_err(|e| CoreError::Runtime(androidpc_runtime_api::RuntimeError::Internal(e.to_string())))?;
        }
        Ok(())
    }

    pub fn send_display_key(&self, down_flag: u8, key: u32) -> Result<(), CoreError> {
        let mut guard = self.rfb_client.write().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(client) = guard.as_mut() {
            client.send_key(down_flag, key).map_err(|e| CoreError::Runtime(androidpc_runtime_api::RuntimeError::Internal(e.to_string())))?;
        }
        Ok(())
    }

    pub fn disconnect_display_client(&self) -> Result<(), CoreError> {
        let mut guard = self.rfb_client.write().map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(mut client) = guard.take() {
            client.disconnect();
        }
        Ok(())
    }

    pub fn init_wine_prefix(
        &self,
        env_id: &str,
        prefix_path: &str,
        arch: WineArch,
    ) -> Result<androidpc_runtime_api::ProcessCommand, CoreError> {
        let prefix = if prefix_path.is_empty() {
            std::path::PathBuf::from("/home/user/.wine")
        } else {
            std::path::PathBuf::from(prefix_path)
        };

        let mut wine_cfg = WineConfig::default();
        wine_cfg.prefix_path = prefix;
        wine_cfg.arch = arch;

        self.wine_supervisor.register_session(env_id, wine_cfg.clone())?;
        let cmd = WineCommandBuilder::build_wineboot_command(&wine_cfg);
        Ok(cmd)
    }

    pub fn build_wine_launch_command(
        &self,
        env_id: &str,
        exe_path: &str,
        args: &[String],
    ) -> Result<androidpc_runtime_api::ProcessCommand, CoreError> {
        let config = self
            .wine_supervisor
            .get_session_config(env_id)?
            .unwrap_or_default();
        let cmd = WineCommandBuilder::build_launch_command(&config, exe_path, args);
        Ok(cmd)
    }

    pub fn terminate_wine_session(
        &self,
        env_id: &str,
    ) -> Result<androidpc_runtime_api::ProcessCommand, CoreError> {
        let config = self
            .wine_supervisor
            .get_session_config(env_id)?
            .unwrap_or_default();
        self.wine_supervisor.clear_wine_pids(env_id)?;
        let cmd = WineCommandBuilder::build_wineserver_kill_command(&config);
        Ok(cmd)
    }

    pub fn get_wine_status(&self, env_id: &str) -> Result<WineStatusInfo, CoreError> {
        let rootfs = {
            let guard = self
                .environments
                .read()
                .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
            if let Some(env_mgr) = guard.get(env_id) {
                env_mgr.config().rootfs_directory.clone()
            } else {
                self.config.base_data_dir.join("rootfs").join(env_id)
            }
        };

        let status = self.wine_supervisor.get_status(env_id, &rootfs)?;
        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_lifecycle_and_version() {
        let config = EngineConfig::default();
        let core = EngineCore::new(config).expect("EngineCore::new should succeed");
        assert_eq!(EngineCore::version(), ENGINE_VERSION);
        let status = core.status();
        assert_eq!(status.state, "Ready");
        assert_eq!(status.version, ENGINE_VERSION);
    }

    #[test]
    fn test_health_check_execution() {
        let config = EngineConfig::default();
        let core = EngineCore::new(config).expect("EngineCore::new should succeed");
        let health = core.run_health_check();
        assert_eq!(health.status, androidpc_runtime_api::HealthLevel::Healthy);
        assert!(!health.subsystems.is_empty());
    }

    #[test]
    fn test_diagnostics_collection() {
        let config = EngineConfig::default();
        let core = EngineCore::new(config).expect("EngineCore::new should succeed");
        let diag = core.collect_diagnostics();
        assert_eq!(diag.engine_version, ENGINE_VERSION);
        assert!(diag.rootless_ptrace_supported);
    }

    #[test]
    fn test_registered_distributions() {
        let config = EngineConfig::default();
        let core = EngineCore::new(config).expect("EngineCore::new should succeed");
        let dist_ids = core.list_distribution_ids();
        assert!(dist_ids.contains(&"ubuntu".to_string()));
        assert!(dist_ids.contains(&"debian".to_string()));
    }

    #[test]
    fn test_wine_integration() {
        let config = EngineConfig::default();
        let core = EngineCore::new(config).expect("EngineCore::new should succeed");
        let cmd = core
            .init_wine_prefix("test-env", "/home/user/.wine", WineArch::Win64)
            .expect("init wine prefix");
        assert_eq!(cmd.program, "box64");
        assert_eq!(cmd.args, vec!["wine", "wineboot", "-u"]);

        let status = core.get_wine_status("test-env").expect("get wine status");
        assert_eq!(status.arch, WineArch::Win64);
        assert_eq!(status.state, WineSessionState::Ready);

        let launch_cmd = core
            .build_wine_launch_command("test-env", "notepad.exe", &[])
            .expect("build launch command");
        assert_eq!(launch_cmd.program, "box64");
        assert_eq!(launch_cmd.args, vec!["wine", "notepad.exe"]);
    }
}
