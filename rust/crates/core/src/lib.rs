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

use androidpc_audio::{AudioError, AudioStatusInfo, AudioTcpSink};
use androidpc_configuration::{EngineConfig, EnvironmentConfig};
use androidpc_display::XvncDisplayBackend;
use androidpc_downloader::{ArtifactMetadata, DefaultArtifactProvider, DownloadManager};
use androidpc_filesystem::{ArchiveExtractor, StorageManager};
use androidpc_linux_runtime::{ProotEnvironmentManager, ProotRuntimeProvider};
use androidpc_runtime_api::{
    Architecture, ArtifactProvider, DisplayBackend, DisplayConfig, DisplayEndpoint,
    EngineStatusInfo, EnvironmentManager, EnvironmentState, HealthReport, LinuxDistribution,
    ProcessCommand, ProcessId, RuntimeProvider, SystemDiagnostics, TerminalSession,
};
use androidpc_terminal::DefaultTerminalSession;
use androidpc_wine::{
    WineArch, WineCommandBuilder, WineConfig, WineProcessSupervisor, WineStatusInfo,
};
use androidpc_vm::{KvmCapabilities, QemuSupervisor, VmArch, VmError, VmStatusInfo};

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

    #[error("Audio error: {0}")]
    Audio(#[from] AudioError),

    #[error("Engine already initialized")]
    AlreadyInitialized,

    #[error("Engine not initialized")]
    NotInitialized,

    #[error("Subsystem failure: {0}")]
    SubsystemFailure(String),

    #[error("Wine error: {0}")]
    Wine(#[from] androidpc_wine::WineError),

    #[error("VM error: {0}")]
    Vm(#[from] VmError),

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
    pub audio_sink: RwLock<AudioTcpSink>,
    pub wine_supervisor: Arc<WineProcessSupervisor>,
    pub vm_supervisor: QemuSupervisor,
    pub gpu_mode: RwLock<androidpc_display::GpuAccelerationMode>,
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
        let proot_name = ProotRuntimeProvider::binary_name_for_arch(Architecture::host());
        let proot_bin = config.runtime_artifacts_dir.join(proot_name);
        let proot_provider = Box::new(ProotRuntimeProvider::new(proot_bin));
        let vm_supervisor = QemuSupervisor::new(
            &config.base_data_dir,
            &config.runtime_artifacts_dir.join("qemu"),
        );

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
            audio_sink: RwLock::new(AudioTcpSink::default()),
            wine_supervisor: Arc::new(WineProcessSupervisor::new()),
            vm_supervisor,
            gpu_mode: RwLock::new(androidpc_display::GpuAccelerationMode::default()),
        };

        core.register_default_distributions();
        core.reload_persisted_environments();
        info!("Engine initialization complete. Rootless Runtime, Display, Audio, Environment & Wine Engine online.");
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

    fn resolve_proot_provider(&self) -> Arc<ProotRuntimeProvider> {
        if let Some(ref native_dir) = self.config.native_lib_dir {
            let native_proot = native_dir.join("libproot.so");
            if native_proot.is_file() {
                return Arc::new(ProotRuntimeProvider::new(native_proot));
            }
        }
        let proot_name = ProotRuntimeProvider::binary_name_for_arch(Architecture::host());
        let proot_bin = self.config.runtime_artifacts_dir.join(proot_name);
        Arc::new(ProotRuntimeProvider::new(proot_bin))
    }

    fn reload_persisted_environments(&self) {
        let jsons = self.storage.list_saved_config_jsons();
        if jsons.is_empty() {
            return;
        }
        let provider = self.resolve_proot_provider();
        if let Ok(mut guard) = self.environments.write() {
            for raw in jsons {
                if let Ok(env_cfg) = serde_json::from_str::<EnvironmentConfig>(&raw) {
                    let id = env_cfg.id.clone();
                    let mgr = ProotEnvironmentManager::from_existing(env_cfg, provider.clone())
                        .with_artifacts_dir(&self.config.runtime_artifacts_dir);
                    guard.insert(id, mgr);
                }
            }
        }
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
        let active_procs = self
            .environments
            .read()
            .map(|envs| envs.values().filter(|m| m.is_running()).count())
            .unwrap_or(0)
            + usize::from(self.is_display_running());
        diagnostics::collect_system_diagnostics(&self.config, &self.run_health_check(), active_procs)
    }

    pub fn create_environment_config(&self, id: &str, distro: &str) -> EnvironmentConfig {
        EnvironmentConfig::new_default(id, distro, &self.config.base_data_dir)
    }

    pub fn verify_archive(&self, path: &Path, expected_sha256: &str) -> Result<bool, CoreError> {
        Ok(self.artifacts.verify_artifact(path, expected_sha256)?)
    }

    pub fn verify_and_install_artifact(
        &self,
        part_path: &Path,
        dest_filename: &str,
        expected_sha256: &str,
    ) -> Result<std::path::PathBuf, CoreError> {
        let dest = self.config.runtime_artifacts_dir.join(dest_filename);
        let final_path = DownloadManager::verify_and_commit_part(part_path, &dest, expected_sha256)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&final_path, std::fs::Permissions::from_mode(0o755));
        }
        Ok(final_path)
    }

    pub fn extract_rootfs(&self, archive: &Path, dest: &Path) -> Result<u64, CoreError> {
        let lower = archive.to_string_lossy().to_ascii_lowercase();
        let entries = if lower.ends_with(".gz") || lower.ends_with(".tgz") {
            ArchiveExtractor::extract_tar_gz(archive, dest)?
        } else if lower.ends_with(".xz") || lower.ends_with(".txz") {
            ArchiveExtractor::extract_tar_xz(archive, dest)?
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

    pub fn list_artifacts_for_host(&self) -> Vec<ArtifactMetadata> {
        self.artifacts.list_for_architecture(Architecture::host())
    }

    pub fn start_display_server(&self, config: &DisplayConfig) -> Result<DisplayEndpoint, CoreError> {
        let xvnc_bin = self.config.runtime_artifacts_dir.join("tigervnc-arm64").join("Xvnc");
        let mut backend = XvncDisplayBackend::new(xvnc_bin);
        let endpoint = backend.start(config).map_err(CoreError::Runtime)?;
        let mut guard = self
            .display_backend
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        *guard = Some(backend);
        info!("Display server registered in core on port {}", endpoint.port);
        Ok(endpoint)
    }

    pub fn stop_display_server(&self) -> Result<(), CoreError> {
        let mut guard = self
            .display_backend
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
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
        self.create_terminal_session_for_env(id, None, cols, rows)
    }

    pub fn create_terminal_session_for_env(
        &self,
        id: &str,
        env_id: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> Result<i32, CoreError> {
        let mut session = DefaultTerminalSession::new(id, cols, rows).map_err(CoreError::Runtime)?;

        // Determine whether to attach a PRoot guest shell or host Android shell
        let mut attached = false;
        if let Some(eid) = env_id {
            if let Ok(envs) = self.environments.read() {
                if let Some(mgr) = envs.get(eid) {
                    let rootfs = &mgr.config().rootfs_directory;
                    let proot_bin = self.resolve_proot_provider();
                    let guest_sh = if rootfs.join("bin/bash").exists() {
                        Some(vec!["/bin/bash".to_string(), "-l".to_string()])
                    } else if rootfs.join("bin/sh").exists() {
                        Some(vec!["/bin/sh".to_string()])
                    } else {
                        None
                    };
                    if let Some(sh_cmd) = guest_sh {
                        if proot_bin.proot_binary().exists() {
                            let proot_args = proot_bin.build_proot_command(mgr.config(), &sh_cmd);
                            let gpu_env = self.get_gpu_acceleration_mode().build_guest_env();
                            let proot_env = mgr.build_guest_env(&gpu_env);
                            let prog = proot_bin.proot_binary().to_string_lossy().to_string();
                            let _ = session.spawn_shell(&prog, &proot_args, &proot_env, None);
                            attached = true;
                        }
                    }
                }
            }
        }

        if !attached {
            let host_sh = if std::path::Path::new("/system/bin/sh").exists() {
                "/system/bin/sh"
            } else {
                "/bin/sh"
            };
            let home_str = self.config.base_data_dir.to_string_lossy().to_string();
            let env = vec![
                ("TERM".to_string(), "xterm-256color".to_string()),
                ("HOME".to_string(), home_str.clone()),
            ];
            let _ = session.spawn_shell(host_sh, &["-i".to_string()], &env, Some(&home_str));
        }

        let fd = session.master_fd();
        let mut guard = self
            .terminal_sessions
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        guard.insert(id.to_string(), session);
        Ok(fd)
    }

    pub fn write_terminal(&self, id: &str, data: &[u8]) -> Result<usize, CoreError> {
        let mut guard = self
            .terminal_sessions
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(session) = guard.get_mut(id) {
            session.write_input(data).map_err(CoreError::Runtime)
        } else {
            Err(CoreError::Runtime(
                androidpc_runtime_api::RuntimeError::TerminalError(format!("Session not found: {id}")),
            ))
        }
    }

    pub fn read_terminal(&self, id: &str, buf: &mut [u8]) -> Result<usize, CoreError> {
        let mut guard = self
            .terminal_sessions
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(session) = guard.get_mut(id) {
            session.read_output(buf).map_err(CoreError::Runtime)
        } else {
            Err(CoreError::Runtime(
                androidpc_runtime_api::RuntimeError::TerminalError(format!("Session not found: {id}")),
            ))
        }
    }

    pub fn resize_terminal(&self, id: &str, cols: u16, rows: u16) -> Result<(), CoreError> {
        let mut guard = self
            .terminal_sessions
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(session) = guard.get_mut(id) {
            session.resize(cols, rows).map_err(CoreError::Runtime)
        } else {
            Err(CoreError::Runtime(
                androidpc_runtime_api::RuntimeError::TerminalError(format!("Session not found: {id}")),
            ))
        }
    }

    pub fn close_terminal(&self, id: &str) -> Result<(), CoreError> {
        let mut guard = self
            .terminal_sessions
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(mut session) = guard.remove(id) {
            session.close().map_err(CoreError::Runtime)?;
        }
        Ok(())
    }

    pub fn provision_environment(&self, id: &str, distro: &str) -> Result<EnvironmentConfig, CoreError> {
        self.provision_environment_with_memory(id, distro, 2048)
    }

    pub fn provision_environment_with_memory(
        &self,
        id: &str,
        distro: &str,
        memory_limit_mb: u32,
    ) -> Result<EnvironmentConfig, CoreError> {
        let mut env_config = self.create_environment_config(id, distro);
        env_config.memory_limit_mb = memory_limit_mb;
        env_config.validate()?;

        self.storage.ensure_environment_directories(id)?;
        let cfg_json = serde_json::to_string_pretty(&env_config)?;
        self.storage.save_environment_config(id, &cfg_json)?;

        let provider = self.resolve_proot_provider();
        let mut manager = ProotEnvironmentManager::new(env_config.clone(), provider)
            .with_artifacts_dir(&self.config.runtime_artifacts_dir);
        manager.initialize().map_err(CoreError::Runtime)?;

        let mut guard = self
            .environments
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        guard.insert(id.to_string(), manager);
        info!("Environment '{id}' provisioned successfully ({distro})");
        Ok(env_config)
    }

    pub fn start_environment(&self, id: &str) -> Result<(), CoreError> {
        let mut guard = self
            .environments
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(manager) = guard.get_mut(id) {
            manager.start().map_err(CoreError::Runtime)
        } else {
            Err(CoreError::Runtime(
                androidpc_runtime_api::RuntimeError::Internal(format!("Environment not found: {id}")),
            ))
        }
    }

    pub fn execute_environment_command(&self, id: &str, cmd: &[String]) -> Result<ProcessId, CoreError> {
        let mut guard = self
            .environments
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(manager) = guard.get_mut(id) {
            manager.execute_guest_command(cmd).map_err(CoreError::Runtime)
        } else {
            Err(CoreError::Runtime(
                androidpc_runtime_api::RuntimeError::Internal(format!("Environment not found: {id}")),
            ))
        }
    }

    pub fn stop_environment(&self, id: &str) -> Result<(), CoreError> {
        let mut guard = self
            .environments
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(manager) = guard.get_mut(id) {
            manager.stop().map_err(CoreError::Runtime)?;
        }
        Ok(())
    }

    pub fn delete_environment(&self, id: &str) -> Result<(), CoreError> {
        let mut guard = self
            .environments
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(mut manager) = guard.remove(id) {
            let _ = manager.stop();
        }
        self.storage.remove_environment(id)?;
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

    pub fn get_artifact_metadata(&self, distro: &str) -> Option<ArtifactMetadata> {
        self.artifacts
            .find_by_distribution(distro, Architecture::host())
            .cloned()
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
        let mut guard = self
            .rfb_client
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        *guard = Some(client);
        Ok(dims)
    }

    pub fn get_framebuffer_slice(&self, out: &mut [u8]) -> Result<usize, CoreError> {
        let mut guard = self
            .rfb_client
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(client) = guard.as_mut() {
            if client.is_connected {
                let _ = client.poll_updates();
                let _ = client.request_frame(true);
            }
            let len = out.len().min(client.framebuffer.len());
            out[..len].copy_from_slice(&client.framebuffer[..len]);
            Ok(len)
        } else {
            Err(CoreError::Runtime(
                androidpc_runtime_api::RuntimeError::Internal("RFB client not connected".into()),
            ))
        }
    }

    pub fn send_display_pointer(&self, button_mask: u8, x: u16, y: u16) -> Result<(), CoreError> {
        let mut guard = self
            .rfb_client
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(client) = guard.as_mut() {
            client
                .send_pointer(button_mask, x, y)
                .map_err(|e| CoreError::Runtime(androidpc_runtime_api::RuntimeError::Internal(e.to_string())))?;
        }
        Ok(())
    }

    pub fn send_display_key(&self, down_flag: u8, key: u32) -> Result<(), CoreError> {
        let mut guard = self
            .rfb_client
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(client) = guard.as_mut() {
            client
                .send_key(down_flag, key)
                .map_err(|e| CoreError::Runtime(androidpc_runtime_api::RuntimeError::Internal(e.to_string())))?;
        }
        Ok(())
    }

    pub fn disconnect_display_client(&self) -> Result<(), CoreError> {
        let mut guard = self
            .rfb_client
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(mut client) = guard.take() {
            client.disconnect();
        }
        Ok(())
    }

    pub fn start_audio_sink(&self, port: u16) -> Result<u16, CoreError> {
        let mut guard = self
            .audio_sink
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if port != guard.status().port && !guard.status().running {
            *guard = AudioTcpSink::new("127.0.0.1", port, 44100, 2);
        }
        Ok(guard.start()?)
    }

    pub fn stop_audio_sink(&self) -> Result<(), CoreError> {
        let mut guard = self
            .audio_sink
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        guard.stop();
        Ok(())
    }

    pub fn read_audio_pcm(&self, out: &mut [u8]) -> Result<usize, CoreError> {
        let guard = self
            .audio_sink
            .read()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        Ok(guard.read_samples(out))
    }

    pub fn get_audio_status(&self) -> Result<AudioStatusInfo, CoreError> {
        let guard = self
            .audio_sink
            .read()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        Ok(guard.status())
    }

    pub fn init_wine_prefix(
        &self,
        env_id: &str,
        prefix_path: &str,
        arch: WineArch,
    ) -> Result<ProcessCommand, CoreError> {
        let prefix = if prefix_path.is_empty() {
            std::path::PathBuf::from("/home/user/.wine")
        } else {
            std::path::PathBuf::from(prefix_path)
        };

        let mut wine_cfg = WineConfig::default();
        wine_cfg.prefix_path = prefix.clone();
        wine_cfg.arch = arch;

        self.wine_supervisor.register_session(env_id, wine_cfg.clone())?;
        let cmd = WineCommandBuilder::build_wineboot_command(&wine_cfg);

        // Prepare prefix directory on rootfs if environment exists and attempt guest execution
        let mut guard = self
            .environments
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(env_mgr) = guard.get_mut(env_id) {
            let clean_rel = prefix.strip_prefix("/").unwrap_or(&prefix);
            let host_prefix = env_mgr.config().rootfs_directory.join(clean_rel);
            let _ = std::fs::create_dir_all(host_prefix.join("drive_c").join("windows").join("system32"));

            let mut guest_argv = vec![cmd.program.clone()];
            guest_argv.extend(cmd.args.clone());
            if let Ok(pid) = env_mgr.execute_guest_command_with_env(&guest_argv, &cmd.env) {
                let _ = self.wine_supervisor.add_wine_pid(env_id, pid);
            }
        }

        Ok(cmd)
    }

    pub fn build_wine_launch_command(
        &self,
        env_id: &str,
        exe_path: &str,
        args: &[String],
    ) -> Result<ProcessCommand, CoreError> {
        let config = self
            .wine_supervisor
            .get_session_config(env_id)?
            .unwrap_or_default();
        let cmd = WineCommandBuilder::build_launch_command(&config, exe_path, args);
        Ok(cmd)
    }

    /// Builds and dispatches a Windows application launch command inside the PRoot guest if provisioned.
    /// Returns `(ProcessCommand, Option<ProcessId>)`.
    pub fn launch_wine_app(
        &self,
        env_id: &str,
        exe_path: &str,
        args: &[String],
    ) -> Result<(ProcessCommand, Option<ProcessId>), CoreError> {
        let cmd = self.build_wine_launch_command(env_id, exe_path, args)?;
        let mut guest_argv = vec![cmd.program.clone()];
        guest_argv.extend(cmd.args.clone());

        let mut spawned_pid = None;
        let mut guard = self
            .environments
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(env_mgr) = guard.get_mut(env_id) {
            if let Ok(pid) = env_mgr.execute_guest_command_with_env(&guest_argv, &cmd.env) {
                let _ = self.wine_supervisor.add_wine_pid(env_id, pid);
                spawned_pid = Some(pid);
            }
        }
        Ok((cmd, spawned_pid))
    }

    pub fn terminate_wine_session(&self, env_id: &str) -> Result<ProcessCommand, CoreError> {
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
                self.config.base_data_dir.join("environments").join(env_id).join("rootfs")
            }
        };

        let status = self.wine_supervisor.get_status(env_id, &rootfs)?;
        Ok(status)
    }

    pub fn probe_kvm_support(&self) -> KvmCapabilities {
        KvmCapabilities::probe()
    }

    pub fn create_vm(
        &self,
        id: &str,
        arch: VmArch,
        memory_mb: u32,
        cpus: u8,
        disk_size_mb: u32,
    ) -> Result<VmStatusInfo, CoreError> {
        Ok(self.vm_supervisor.create_vm(id, arch, memory_mb, cpus, disk_size_mb)?)
    }

    pub fn start_vm(&self, id: &str) -> Result<VmStatusInfo, CoreError> {
        Ok(self.vm_supervisor.start_vm(id)?)
    }

    pub fn stop_vm(&self, id: &str) -> Result<(), CoreError> {
        Ok(self.vm_supervisor.stop_vm(id)?)
    }

    pub fn delete_vm(&self, id: &str) -> Result<(), CoreError> {
        Ok(self.vm_supervisor.delete_vm(id)?)
    }

    pub fn list_vms(&self) -> Vec<VmStatusInfo> {
        self.vm_supervisor.list_vms()
    }

    pub fn set_gpu_acceleration_mode(&self, mode: androidpc_display::GpuAccelerationMode) {
        if let Ok(mut guard) = self.gpu_mode.write() {
            *guard = mode;
            info!("GPU acceleration mode set to {:?}", mode);
        }
    }

    pub fn get_gpu_acceleration_mode(&self) -> androidpc_display::GpuAccelerationMode {
        self.gpu_mode
            .read()
            .map(|g| *g)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use androidpc_wine::WineSessionState;

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
    fn test_environment_persistence_and_stop_preserves_entry() {
        let tmp_base = std::env::temp_dir().join("androidpc_core_persist_test");
        let _ = std::fs::remove_dir_all(&tmp_base);
        let cfg = EngineConfig {
            base_data_dir: tmp_base.clone(),
            runtime_artifacts_dir: tmp_base.join("artifacts"),
            temporary_dir: tmp_base.join("tmp"),
            ..EngineConfig::default()
        };

        {
            let core = EngineCore::new(cfg.clone()).unwrap();
            core.provision_environment("env-ubuntu", "ubuntu").unwrap();
            assert_eq!(core.get_environment_state("env-ubuntu"), Some(EnvironmentState::Stopped));
            core.stop_environment("env-ubuntu").unwrap();
            assert_eq!(core.get_environment_state("env-ubuntu"), Some(EnvironmentState::Stopped));
        }

        // Reload in a fresh EngineCore instance
        let core2 = EngineCore::new(cfg).unwrap();
        let envs = core2.list_environments();
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].id, "env-ubuntu");

        core2.delete_environment("env-ubuntu").unwrap();
        assert!(core2.list_environments().is_empty());
        let _ = std::fs::remove_dir_all(&tmp_base);
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

        let (launch_cmd, _) = core
            .launch_wine_app("test-env", "notepad.exe", &[])
            .expect("build launch command");
        assert_eq!(launch_cmd.program, "box64");
        assert_eq!(launch_cmd.args, vec!["wine", "notepad.exe"]);
    }

    #[test]
    fn test_vm_lifecycle() {
        let tmp_base = std::env::temp_dir().join("androidpc_core_vm_test");
        let _ = std::fs::remove_dir_all(&tmp_base);
        let cfg = EngineConfig {
            base_data_dir: tmp_base.clone(),
            runtime_artifacts_dir: tmp_base.join("artifacts"),
            temporary_dir: tmp_base.join("tmp"),
            ..EngineConfig::default()
        };
        let core = EngineCore::new(cfg).unwrap();
        let vm = core.create_vm("vm-alpine", VmArch::Aarch64, 512, 2, 512).unwrap();
        assert_eq!(vm.id, "vm-alpine");
        assert_eq!(core.list_vms().len(), 1);
        core.delete_vm("vm-alpine").unwrap();
        assert!(core.list_vms().is_empty());
        let _ = std::fs::remove_dir_all(&tmp_base);
    }
}
