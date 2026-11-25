pub mod diagnostics;
pub mod backups;
pub mod desktop;
pub mod distributions;
pub mod environment_metrics;
pub mod health;
pub mod packages;

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex, RwLock};
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

    #[error("Runtime file operation failed: {0}")]
    Io(#[from] std::io::Error),

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
    pub desktop_session: RwLock<Option<desktop::DesktopSession>>,
    desktop_exit: RwLock<Option<desktop::DesktopExit>>,
    pub terminal_sessions: RwLock<HashMap<String, DefaultTerminalSession>>,
    pub environments: RwLock<HashMap<String, ProotEnvironmentManager>>,
    environment_install: Mutex<()>,
    install_progress: Mutex<Option<InstallProgress>>,
    package_jobs: Mutex<HashMap<String, Arc<packages::PackageJob>>>,
    pub rfb_client: RwLock<Option<androidpc_display::RfbClient>>,
    pub audio_sink: RwLock<AudioTcpSink>,
    pub wine_supervisor: Arc<WineProcessSupervisor>,
    pub vm_supervisor: QemuSupervisor,
    pub gpu_mode: RwLock<androidpc_display::GpuAccelerationMode>,
}

struct InstallProgress {
    id: String,
    state: EnvironmentState,
    cancelled: bool,
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
        #[cfg(target_os = "android")]
        storage.clean_staging_on_startup();
        let proot_name = ProotRuntimeProvider::binary_name_for_arch(Architecture::host());
        let proot_bin = config.runtime_artifacts_dir.join(proot_name);
        let proot_provider = Box::new(ProotRuntimeProvider::new(proot_bin));
        let vm_supervisor = QemuSupervisor::new(
            &config.base_data_dir,
            &config.runtime_artifacts_dir.join("qemu"),
        );
        let artifacts = if let Some(key) = config.trusted_rootfs_key_hex.as_deref() {
            let catalog = config.runtime_artifacts_dir.join("rootfs/catalog.json");
            let signature = config.runtime_artifacts_dir.join("rootfs/catalog.sig");
            if catalog.metadata()?.len() > 64 * 1024 || signature.metadata()?.len() != 64 {
                return Err(CoreError::SubsystemFailure("Invalid rootfs manifest or signature length".into()));
            }
            DefaultArtifactProvider::from_signed_rootfs(
                &std::fs::read(catalog)?,
                &std::fs::read(signature)?,
                key,
                config.app_version_code,
            )?
        } else if cfg!(target_os = "android") && !config.debug_mode {
            return Err(CoreError::SubsystemFailure("Release build requires a trusted rootfs catalog".into()));
        } else {
            DefaultArtifactProvider::new()
        };

        let mut core = Self {
            config,
            start_time: Instant::now(),
            init_epoch_ms: AtomicU64::new(now_epoch_ms),
            storage,
            artifacts,
            providers: RwLock::new(vec![proot_provider]),
            distributions: RwLock::new(Vec::new()),
            display_backend: RwLock::new(None),
            desktop_session: RwLock::new(None),
            desktop_exit: RwLock::new(None),
            terminal_sessions: RwLock::new(HashMap::new()),
            environments: RwLock::new(HashMap::new()),
            environment_install: Mutex::new(()),
            install_progress: Mutex::new(None),
            package_jobs: Mutex::new(HashMap::new()),
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
            for (directory, raw) in jsons {
                if let Ok(env_cfg) = serde_json::from_str::<EnvironmentConfig>(&raw) {
                    let id = env_cfg.id.clone();
                    let Ok(scoped) = self.storage.distribution_root(&env_cfg.distribution) else { continue };
                    if env_cfg.validate().is_err()
                        || (directory != scoped.join(&id) && directory != self.storage.environment_root(&id))
                        || env_cfg.rootfs_directory != directory.join("rootfs")
                        || !env_cfg.rootfs_directory.symlink_metadata().is_ok_and(|meta| meta.is_dir())
                        || guard.contains_key(&id)
                    {
                        continue;
                    }
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
        let (active, last_check) = self.environments.read().map(|envs| {
            let active = envs.values().flat_map(|env| env.supervisor().list_processes())
                .filter(|process| process.running).count();
            let last_check = envs.values().filter_map(|env| env.config().verified_at_epoch_ms).max();
            (active, last_check)
        }).unwrap_or_default();
        let terminals = self.terminal_sessions.write().map(|mut sessions| {
            sessions.values_mut().filter_map(|session| session.exit_status().is_none().then_some(())).count()
        }).unwrap_or(0);
        let mut result = diagnostics::collect_system_diagnostics(
            &self.config, &self.run_health_check(), active + terminals + usize::from(self.is_display_running()),
        );
        result.rootless_ptrace_supported = result.proot_ready && last_check.is_some();
        result.last_guest_check_epoch_ms = last_check;
        result
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
        self.extract_rootfs_as(archive, dest, &archive.to_string_lossy())
    }

    fn extract_rootfs_as(&self, archive: &Path, dest: &Path, filename: &str) -> Result<u64, CoreError> {
        let lower = filename.to_ascii_lowercase();
        let entries = if lower.ends_with(".gz") || lower.ends_with(".tgz") {
            ArchiveExtractor::extract_tar_gz(archive, dest)?
        } else if lower.ends_with(".xz") || lower.ends_with(".txz") {
            ArchiveExtractor::extract_tar_xz(archive, dest)?
        } else {
            ArchiveExtractor::extract_tar(archive, dest)?
        };
        Ok(entries)
    }

    pub fn install_environment(
        &self,
        id: &str,
        distro: &str,
        name: &str,
        memory_limit_mb: u32,
        archive: &Path,
    ) -> Result<EnvironmentConfig, CoreError> {
        if Architecture::host() != Architecture::Aarch64 || !matches!(distro, "ubuntu" | "debian") {
            return Err(CoreError::SubsystemFailure("Select Ubuntu or Debian on an ARM64 device".into()));
        }
        EnvironmentConfig::validate_new_id(id)?;
        let _install = self.environment_install.try_lock()
            .map_err(|_| CoreError::SubsystemFailure("An environment install is already active".into()))?;
        let meta = self.get_artifact_metadata(distro)
            .ok_or_else(|| CoreError::SubsystemFailure("No matching rootfs artifact".into()))?;
        let mut config = self.create_environment_config(id, distro);
        config.display_name = Some(name.trim().to_owned());
        config.memory_limit_mb = memory_limit_mb;
        config.validate()?;
        let destination = self.storage.distribution_root(distro)?.join(id);
        config.rootfs_directory = destination.join("rootfs");
        if self.environments.read().map_err(|_| CoreError::SubsystemFailure("Environment lock poisoned".into()))?.contains_key(id)
            || ["ubuntu", "debian"].into_iter().filter_map(|distro| self.storage.distribution_root(distro).ok())
                .any(|root| root.join(id).symlink_metadata().is_ok())
            || self.storage.environment_root(id).symlink_metadata().is_ok()
        {
            return Err(CoreError::SubsystemFailure("Environment already exists".into()));
        }
        let staging = destination.with_file_name(format!(".install-{id}"));
        match std::fs::remove_dir_all(&staging) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let staging_rootfs = staging.join("rootfs");
        let provider = self.resolve_proot_provider();
        *self.install_progress.lock().map_err(|_| CoreError::SubsystemFailure("Install progress lock poisoned".into()))? =
            Some(InstallProgress { id: id.to_string(), state: EnvironmentState::Verifying, cancelled: false });
        let result = (|| {
            if archive.metadata()?.len() != meta.size_bytes || !self.verify_archive(archive, &meta.sha256)? {
                return Err(CoreError::SubsystemFailure("Rootfs archive size or SHA-256 does not match the catalog".into()));
            }
            self.install_checkpoint(EnvironmentState::Extracting)?;
            std::fs::create_dir_all(&staging_rootfs)?;
            self.extract_rootfs_as(archive, &staging_rootfs, &meta.filename)?;
            self.install_checkpoint(EnvironmentState::Initializing)?;
            let mut staged = config.clone();
            staged.rootfs_directory = staging_rootfs;
            let mut manager = ProotEnvironmentManager::new(staged, provider.clone())
                .with_artifacts_dir(&self.config.runtime_artifacts_dir);
            manager.initialize()?;
            let expected_release = self.artifacts.expected_release(distro)
                .ok_or_else(|| CoreError::SubsystemFailure("Missing guest release in artifact catalog".into()))?;
            config.version = manager.verify_and_provision_guest(distro, expected_release)?;
            config.verified_at_epoch_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .ok().map(|time| time.as_millis() as u64);
            config.runtime_version = diagnostics::probe_proot_version(&self.config);

            self.storage.save_config_at(&staging, &serde_json::to_vec_pretty(&config)?)?;
            let mut progress = self.install_progress.lock()
                .map_err(|_| CoreError::SubsystemFailure("Install progress lock poisoned".into()))?;
            let active = progress.as_mut().ok_or_else(|| CoreError::SubsystemFailure("Missing install progress".into()))?;
            if active.cancelled {
                return Err(CoreError::SubsystemFailure("Installation cancelled".into()));
            }
            let mut environments = self.environments.write()
                .map_err(|_| CoreError::SubsystemFailure("Environment lock poisoned".into()))?;
            if environments.contains_key(id) || destination.symlink_metadata().is_ok() {
                return Err(CoreError::SubsystemFailure("Environment already exists".into()));
            }
            std::fs::rename(&staging, &destination)?;
            environments.insert(id.to_string(), ProotEnvironmentManager::from_existing(config.clone(), provider)
                .with_artifacts_dir(&self.config.runtime_artifacts_dir));
            active.state = EnvironmentState::Ready;
            Ok(config)
        })();
        if result.is_err() {
            if let Ok(mut progress) = self.install_progress.lock() {
                if let Some(progress) = progress.as_mut() {
                    progress.state = EnvironmentState::Failed;
                }
            }
            if let Err(error) = std::fs::remove_dir_all(&staging) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!("Could not clean installation staging directory: {error}");
                }
            }
        }
        result
    }

    fn install_checkpoint(&self, state: EnvironmentState) -> Result<(), CoreError> {
        let mut progress = self.install_progress.lock()
            .map_err(|_| CoreError::SubsystemFailure("Install progress lock poisoned".into()))?;
        let active = progress.as_mut().ok_or_else(|| CoreError::SubsystemFailure("Missing install progress".into()))?;
        if active.cancelled {
            return Err(CoreError::SubsystemFailure("Installation cancelled".into()));
        }
        active.state = state;
        Ok(())
    }

    pub fn cancel_installation(&self, id: &str) -> bool {
        let Ok(mut progress) = self.install_progress.lock() else { return false };
        if let Some(active) = progress.as_mut() {
            if active.id == id && matches!(active.state, EnvironmentState::Verifying | EnvironmentState::Extracting | EnvironmentState::Initializing) {
                active.cancelled = true;
                return true;
            }
        }
        false
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
        self.create_terminal_session_for_env_with_shell(id, env_id, cols, rows, None)
    }

    pub fn create_terminal_session_for_env_with_shell(
        &self,
        id: &str,
        env_id: Option<&str>,
        cols: u16,
        rows: u16,
        preferred_shell: Option<&str>,
    ) -> Result<i32, CoreError> {
        let mut session = DefaultTerminalSession::new(id, cols, rows).map_err(CoreError::Runtime)?;
        let mut environments = self.environments.write()
            .map_err(|_| CoreError::SubsystemFailure("Environment lock poisoned".into()))?;
        let mut sessions = self.terminal_sessions.write()
            .map_err(|_| CoreError::SubsystemFailure("Terminal lock poisoned".into()))?;
        if sessions.contains_key(id) || sessions.len() >= self.config.max_concurrent_processes {
            return Err(CoreError::SubsystemFailure("Terminal already exists or session limit reached".into()));
        }
        if let Some(env_id) = env_id {
            self.ensure_no_package_operation(env_id)?;
            let manager = environments.get_mut(env_id)
                .ok_or_else(|| CoreError::SubsystemFailure(format!("Environment not installed: {env_id}")))?;
            let rootfs = &manager.config().rootfs_directory;
            let prefer_sh = matches!(preferred_shell, Some("/bin/sh"));
            let shell = if prefer_sh && rootfs.join("bin/sh").exists() {
                vec!["/bin/sh".to_string(), "-i".to_string()]
            } else if rootfs.join("bin/bash").exists() {
                vec!["/bin/bash".to_string(), "-l".to_string()]
            } else if rootfs.join("bin/sh").exists() {
                vec!["/bin/sh".to_string(), "-i".to_string()]
            } else {
                return Err(CoreError::SubsystemFailure("The installed rootfs has no Linux shell".into()));
            };
            let provider = self.resolve_proot_provider();
            let args = provider.build_proot_command(manager.config(), &shell);
            let env = manager.build_guest_env(&self.get_gpu_acceleration_mode().build_guest_env());
            session.spawn_shell(&provider.proot_binary().to_string_lossy(), &args, &env, None)?;
            session.associate_environment(env_id);
            self.record_start(manager)?;
        } else {
            let host_sh = if std::path::Path::new("/system/bin/sh").exists() {
                "/system/bin/sh"
            } else {
                "/bin/sh"
            };
            let home = self.config.base_data_dir.to_string_lossy().to_string();
            let env = vec![
                ("TERM".to_string(), "xterm-256color".to_string()),
                ("HOME".to_string(), home.clone()),
            ];
            session.spawn_shell(host_sh, &["-i".to_string()], &env, Some(&home))?;
        }

        let fd = session.master_fd();
        sessions.insert(id.to_string(), session);
        Ok(fd)
    }

    pub fn terminal_exit_status(&self, id: &str) -> Result<Option<i32>, CoreError> {
        let mut sessions = self.terminal_sessions.write()
            .map_err(|_| CoreError::SubsystemFailure("Terminal lock poisoned".into()))?;
        let session = sessions.get_mut(id)
            .ok_or_else(|| CoreError::SubsystemFailure("Terminal is closed".into()))?;
        Ok(session.exit_status())
    }

    fn close_environment_terminals(&self, id: &str) -> Result<(), CoreError> {
        let mut sessions = self.terminal_sessions.write()
            .map_err(|_| CoreError::SubsystemFailure("Terminal lock poisoned".into()))?;
        sessions.retain(|_, session| !session.belongs_to_environment(id));
        Ok(())
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

        let mut guard = self.environments.write()
            .map_err(|_| CoreError::SubsystemFailure("Environment lock poisoned".into()))?;
        if guard.contains_key(id) || self.storage.environment_root(id).symlink_metadata().is_ok() {
            return Err(CoreError::SubsystemFailure("Environment already exists".into()));
        }
        self.storage.ensure_environment_directories(id)?;
        let cfg_json = serde_json::to_string_pretty(&env_config)?;
        self.storage.save_environment_config(id, &cfg_json)?;

        let provider = self.resolve_proot_provider();
        let mut manager = ProotEnvironmentManager::new(env_config.clone(), provider)
            .with_artifacts_dir(&self.config.runtime_artifacts_dir);
        manager.initialize().map_err(CoreError::Runtime)?;

        guard.insert(id.to_string(), manager);
        info!("Environment '{id}' provisioned successfully ({distro})");
        Ok(env_config)
    }

    pub fn start_environment(&self, id: &str) -> Result<(), CoreError> {
        let mut guard = self
            .environments
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        self.ensure_no_package_operation(id)?;
        if let Some(manager) = guard.get_mut(id) {
            if !manager.is_running() && !self.has_live_terminal(id)? {
                manager.start()?;
                if let Err(error) = self.record_start(manager) {
                    let _ = manager.stop();
                    return Err(error);
                }
            }
            Ok(())
        } else {
            Err(CoreError::Runtime(
                androidpc_runtime_api::RuntimeError::Internal(format!("Environment not found: {id}")),
            ))
        }
    }

    fn has_live_terminal(&self, id: &str) -> Result<bool, CoreError> {
        let mut sessions = self.terminal_sessions.write()
            .map_err(|_| CoreError::SubsystemFailure("Terminal lock poisoned".into()))?;
        Ok(sessions.values_mut().any(|session| session.belongs_to_environment(id) && session.exit_status().is_none()))
    }

    fn record_start(&self, manager: &mut ProotEnvironmentManager) -> Result<(), CoreError> {
        let previous = manager.config().last_started_at_epoch_ms;
        manager.set_last_started(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .ok().map(|duration| duration.as_millis() as u64));
        let config = manager.config();
        let root = config.rootfs_directory.parent().ok_or_else(|| CoreError::SubsystemFailure("Missing environment directory".into()))?;
        let result = self.storage.save_config_at(root, &serde_json::to_vec_pretty(config)?);
        if let Err(error) = result {
            manager.set_last_started(previous);
            return Err(error.into());
        }
        Ok(())
    }

    pub fn execute_environment_command(&self, id: &str, cmd: &[String]) -> Result<ProcessId, CoreError> {
        let gpu_env = self.get_gpu_acceleration_mode().build_guest_env();
        let mut guard = self
            .environments
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        self.ensure_no_package_operation(id)?;
        if let Some(manager) = guard.get_mut(id) {
            manager
                .execute_guest_command_with_env(cmd, &gpu_env)
                .map_err(CoreError::Runtime)
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
        self.ensure_no_package_operation(id)?;
        self.stop_desktop_for_environment(id, "Linux environment stopped")?;
        self.close_environment_terminals(id)?;
        if let Some(manager) = guard.get_mut(id) {
            manager.stop().map_err(CoreError::Runtime)?;
        }
        Ok(())
    }

    pub fn recover_environment(&self, id: &str, restart: bool) -> Result<(), CoreError> {
        let mut environments = self.environments.write()
            .map_err(|_| CoreError::SubsystemFailure("Environment lock poisoned".into()))?;
        self.ensure_no_package_operation(id)?;
        let manager = environments.get_mut(id)
            .ok_or_else(|| CoreError::SubsystemFailure("Environment not installed".into()))?;
        self.stop_desktop_for_environment(id, "Linux environment recovery")?;
        self.close_environment_terminals(id)?;
        manager.stop()?;
        if restart {
            manager.start()?;
            if let Err(error) = self.record_start(manager) {
                let _ = manager.stop();
                return Err(error);
            }
        } else {
            manager.repair_guest()?;
        }
        Ok(())
    }

    pub fn delete_environment(&self, id: &str) -> Result<(), CoreError> {
        EnvironmentConfig::validate_id(id)?;
        let mut guard = self
            .environments
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        self.ensure_no_package_operation(id)?;
        let manager = guard.get(id)
            .ok_or_else(|| CoreError::SubsystemFailure("Environment not installed".into()))?;
        if manager.is_running() || self.has_live_terminal(id)? || self.has_live_desktop(id)? {
            return Err(CoreError::SubsystemFailure("Stop the environment before removing it".into()));
        }
        self.close_environment_terminals(id)?;
        let path = manager.config().rootfs_directory.parent()
            .ok_or_else(|| CoreError::SubsystemFailure("Missing environment directory".into()))?;
        let removed = path.with_file_name(format!(".deleted-{id}"));
        let moved = match std::fs::rename(&path, &removed) {
            Ok(()) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(e.into()),
        };
        guard.remove(id);
        drop(guard);
        if moved {
            std::fs::remove_dir_all(removed)?;
        }
        Ok(())
    }

    pub fn get_environment_state(&self, id: &str) -> Option<EnvironmentState> {
        if let Some(state) = self.package_state(id) {
            return Some(state);
        }
        let state = self.environments
            .read()
            .ok()
            .and_then(|guard| guard.get(id).map(|m| m.state()));
        let Some(state) = state else {
            return self.install_progress.lock().ok()?.as_ref()
                .filter(|progress| progress.id == id && progress.state != EnvironmentState::Ready)
                .map(|progress| progress.state);
        };
        if self.has_live_terminal(id).ok()? || self.has_live_desktop(id).ok()? {
            Some(EnvironmentState::Running)
        } else {
            Some(state)
        }
    }

    pub fn list_environments(&self) -> Vec<EnvironmentConfig> {
        match self.environments.read() {
            Ok(guard) => guard.values().map(|m| m.config().clone()).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn environment_storage_bytes(&self, id: &str) -> Result<u64, CoreError> {
        let environments = self.environments.read()
            .map_err(|_| CoreError::SubsystemFailure("Environment lock poisoned".into()))?;
        let manager = environments.get(id)
            .ok_or_else(|| CoreError::SubsystemFailure("Environment not installed".into()))?;
        let root = manager.config().rootfs_directory.parent()
            .ok_or_else(|| CoreError::SubsystemFailure("Missing environment directory".into()))?;
        Ok(self.storage.environment_storage_bytes(root)?)
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

    /// Polls RFB updates and copies the framebuffer into `out` ONLY when `generation != last_gen`.
    /// Returns `0` when unchanged, or the new `generation` when updated.
    pub fn get_framebuffer_if_dirty(&self, out: &mut [u8], last_gen: u64) -> Result<u64, CoreError> {
        let mut guard = self
            .rfb_client
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(client) = guard.as_mut() {
            if client.is_connected {
                let _ = client.poll_updates();
                let _ = client.request_frame(true);
            }
            if client.generation == last_gen {
                return Ok(0);
            }
            let len = client.framebuffer.len();
            if out.len() < len {
                return Ok(0);
            }
            out[..len].copy_from_slice(&client.framebuffer);
            Ok(client.generation.max(1))
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

    pub fn has_live_display_connection(&self) -> bool {
        self.rfb_client.read().ok().is_some_and(|client| {
            client.as_ref().is_some_and(|client| client.is_connected)
        })
    }

    pub fn send_display_clipboard(&self, text: &str) -> Result<(), CoreError> {
        let mut guard = self
            .rfb_client
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        if let Some(client) = guard.as_mut() {
            client
                .send_clipboard(text)
                .map_err(|e| CoreError::Runtime(androidpc_runtime_api::RuntimeError::Internal(e.to_string())))?;
        }
        Ok(())
    }

    pub fn take_display_clipboard(&self) -> Result<Option<String>, CoreError> {
        let mut guard = self
            .rfb_client
            .write()
            .map_err(|_| CoreError::SubsystemFailure("Lock poisoned".into()))?;
        Ok(guard.as_mut().and_then(|c| c.take_server_clipboard()))
    }

    pub fn get_display_dimensions(&self) -> (u16, u16) {
        match self.rfb_client.read() {
            Ok(guard) => guard.as_ref().map(|c| (c.width, c.height)).unwrap_or((0, 0)),
            Err(_) => (0, 0),
        }
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
        self.ensure_no_package_operation(env_id)?;
        if let Some(env_mgr) = guard.get_mut(env_id) {
            let clean_rel = prefix.strip_prefix("/").unwrap_or(&prefix);
            let host_prefix = env_mgr.config().rootfs_directory.join(clean_rel);
            let _ = std::fs::create_dir_all(host_prefix.join("drive_c").join("windows").join("system32"));

            let mut guest_argv = vec![cmd.program.clone()];
            guest_argv.extend(cmd.args.clone());
            let mut merged_env = cmd.env.clone();
            merged_env.extend(self.get_gpu_acceleration_mode().build_guest_env());
            let rcfile = self.config.runtime_artifacts_dir.join("box64").join("default.box64rc");
            if rcfile.exists() {
                merged_env.push(("BOX64_RCFILE".to_string(), rcfile.display().to_string()));
            }
            if let Ok(pid) = env_mgr.execute_guest_command_with_env(&guest_argv, &merged_env) {
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
        self.ensure_no_package_operation(env_id)?;
        if let Some(env_mgr) = guard.get_mut(env_id) {
            let mut merged_env = cmd.env.clone();
            merged_env.extend(self.get_gpu_acceleration_mode().build_guest_env());
            let rcfile = self.config.runtime_artifacts_dir.join("box64").join("default.box64rc");
            if rcfile.exists() {
                merged_env.push(("BOX64_RCFILE".to_string(), rcfile.display().to_string()));
            }
            if let Ok(pid) = env_mgr.execute_guest_command_with_env(&guest_argv, &merged_env) {
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
        assert!(!diag.rootless_ptrace_supported);
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
    fn registry_reopens_scoped_and_legacy_guests_without_collisions() {
        let base = std::env::temp_dir().join(format!("androidpc-registry-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let cfg = EngineConfig { base_data_dir: base.clone(), ..EngineConfig::default() };
        let core = EngineCore::new(cfg.clone()).unwrap();
        core.provision_environment("legacy", "debian").unwrap();
        let fixtures = [
            ("f15d206e-e6c9-4f2a-87bc-5a3a68a1b117", "ubuntu"),
            ("f15d206e-e6c9-4f2a-87bc-5a3a68a1b118", "ubuntu"),
            ("f15d206e-e6c9-4f2a-87bc-5a3a68a1b119", "debian"),
        ];
        assert!(EnvironmentConfig::validate_new_id("legacy").is_err());
        for (id, distribution) in fixtures {
            EnvironmentConfig::validate_new_id(id).unwrap();
            let directory = core.storage.distribution_root(distribution).unwrap().join(id);
            let mut config = core.create_environment_config(id, distribution);
            config.rootfs_directory = directory.join("rootfs");
            config.display_name = Some("Same display name".to_owned());
            std::fs::create_dir_all(&config.rootfs_directory).unwrap();
            core.storage.save_config_at(&directory, &serde_json::to_vec(&config).unwrap()).unwrap();
        }
        let duplicate = core.storage.distribution_root("ubuntu").unwrap().join("wrong-location");
        std::fs::create_dir_all(duplicate.join("rootfs")).unwrap();
        let config = core.create_environment_config(fixtures[0].0, "ubuntu");
        core.storage.save_config_at(&duplicate, &serde_json::to_vec(&config).unwrap()).unwrap();
        drop(core);
        let core = EngineCore::new(cfg.clone()).unwrap();
        assert_eq!(core.list_environments().len(), 4);
        let id = fixtures[0].0;
        {
            let mut registry = core.environments.write().unwrap();
            core.record_start(registry.get_mut(id).unwrap()).unwrap();
        }
        drop(core);
        let core = EngineCore::new(cfg).unwrap();
        assert!(core.list_environments().iter().find(|env| env.id == id).unwrap().last_started_at_epoch_ms.is_some());
        core.delete_environment(id).unwrap();
        assert_eq!(core.list_environments().len(), 3);
        assert!(core.storage.distribution_root("ubuntu").unwrap().join(fixtures[1].0).exists());
        std::fs::remove_dir_all(base).unwrap();
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
