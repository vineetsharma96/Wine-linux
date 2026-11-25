use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::RwLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};

use androidpc_configuration::Box64EnvironmentConfig;
use androidpc_linux_runtime::{ProotEnvironmentManager, ProotRuntimeProvider};

use crate::elf::{inspect_guest_binary, ElfArchitecture, ElfEndianness, ElfInspection, ElfParseError};
use crate::env::{build_box64_env_vars, render_box64_rcfile};
use crate::resolver::{resolve_guest_libraries, LibraryResolutionReport};
use crate::{BOX64_LICENSE, BOX64_PINNED_VERSION, BOX64_UPSTREAM_REPO};

const MAX_EXECUTION_HISTORY: usize = 64;
const GUEST_BOX64_CANDIDATES: &[&str] = &[
    "/usr/local/bin/box64",
    "/usr/bin/box64",
    "/opt/androidpc/box64/box64",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Box64ErrorKind {
    None,
    BinaryNotFound,
    InvalidElf,
    UnsupportedArchitecture,
    BionicAbiContamination,
    Box64NotInstalled,
    Box64Disabled,
    MissingSharedLibrary,
    CrashedSignal,
    OutOfMemory,
    Interrupted,
    EnvironmentShutdown,
    ExecutionFailed,
}

#[derive(Debug, Error)]
pub enum CompatibilityError {
    #[error("Guest binary not found: {0}")]
    BinaryNotFound(String),

    #[error("Invalid ELF executable '{path}': {reason}")]
    InvalidElf { path: String, reason: String },

    #[error("Unsupported executable architecture '{arch}' (e_machine={e_machine}): {reason}")]
    UnsupportedArchitecture {
        arch: String,
        e_machine: u16,
        reason: String,
    },

    #[error("Android Bionic binary rejected in Linux guest: '{path}' uses host Android linker/libc")]
    BionicAbiContamination { path: String },

    #[error("Box64 is disabled for this environment. Enable Box64 in Compatibility settings to execute x86-64 applications.")]
    Box64Disabled,

    #[error("Box64 binary is not installed in the Linux guest or app artifacts directory.")]
    Box64NotInstalled,

    #[error("Missing required x86-64 shared libraries: {0}")]
    MissingSharedLibraries(String),

    #[error("Process execution failed: {0}")]
    ExecutionFailed(String),
}

impl CompatibilityError {
    pub fn kind(&self) -> Box64ErrorKind {
        match self {
            Self::BinaryNotFound(_) => Box64ErrorKind::BinaryNotFound,
            Self::InvalidElf { .. } => Box64ErrorKind::InvalidElf,
            Self::UnsupportedArchitecture { .. } => Box64ErrorKind::UnsupportedArchitecture,
            Self::BionicAbiContamination { .. } => Box64ErrorKind::BionicAbiContamination,
            Self::Box64Disabled => Box64ErrorKind::Box64Disabled,
            Self::Box64NotInstalled => Box64ErrorKind::Box64NotInstalled,
            Self::MissingSharedLibraries(_) => Box64ErrorKind::MissingSharedLibrary,
            Self::ExecutionFailed(_) => Box64ErrorKind::ExecutionFailed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Box64RouteDecision {
    NativeArm64,
    Box64X86_64,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Box64InstallationStatus {
    pub installed: bool,
    pub binary_path: Option<String>,
    pub detected_version: Option<String>,
    pub pinned_version: String,
    pub license: String,
    pub upstream_repo: String,
    pub host_arch: String,
    pub dynarec_supported_on_host: bool,
    pub abi_isolation_verified: bool,
    pub status_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Box64ExecutionPlan {
    pub route: Box64RouteDecision,
    pub guest_command: Vec<String>,
    pub extra_env: Vec<(String, String)>,
    pub inspection: ElfInspection,
    pub libraries: LibraryResolutionReport,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Box64ExecutionOutcome {
    pub environment_id: String,
    pub binary_path: String,
    pub route: Box64RouteDecision,
    pub architecture: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub terminating_signal: Option<i32>,
    pub error_kind: Box64ErrorKind,
    pub error_explanation: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub missing_libraries: Vec<String>,
    pub duration_ms: u64,
    pub timestamp_epoch_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Box64ExecutionRecord {
    pub timestamp_epoch_ms: u64,
    pub environment_id: String,
    pub binary_path: String,
    pub route: Box64RouteDecision,
    pub architecture: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub terminating_signal: Option<i32>,
    pub error_kind: Box64ErrorKind,
    pub summary: String,
    pub duration_ms: u64,
}

pub struct Box64CompatibilityLayer {
    artifacts_dir: PathBuf,
    history: RwLock<VecDeque<Box64ExecutionRecord>>,
}

impl Box64CompatibilityLayer {
    pub fn new(artifacts_dir: impl Into<PathBuf>) -> Self {
        Self {
            artifacts_dir: artifacts_dir.into(),
            history: RwLock::new(VecDeque::with_capacity(MAX_EXECUTION_HISTORY)),
        }
    }

    pub fn artifacts_box64_binary(&self) -> PathBuf {
        self.artifacts_dir.join("box64").join("box64")
    }

    /// Probes whether a Linux ARM64 Box64 binary is installed in the guest rootfs or staged artifacts.
    pub fn probe_installation(&self, guest_rootfs: Option<&Path>) -> Box64InstallationStatus {
        let host_arch = std::env::consts::ARCH.to_string();
        let dynarec_supported_on_host = host_arch == "aarch64";

        let mut binary_path: Option<PathBuf> = None;
        let mut guest_display_path: Option<String> = None;

        if let Some(rootfs) = guest_rootfs {
            for candidate in GUEST_BOX64_CANDIDATES {
                let host_candidate = rootfs.join(candidate.trim_start_matches('/'));
                if host_candidate.is_file() {
                    binary_path = Some(host_candidate);
                    guest_display_path = Some((*candidate).to_string());
                    break;
                }
            }
        }

        if binary_path.is_none() {
            let staged = self.artifacts_box64_binary();
            if staged.is_file() {
                guest_display_path = Some(staged.display().to_string());
                binary_path = Some(staged);
            }
        }

        let mut abi_isolation_verified = true;
        let mut detected_version: Option<String> = None;

        if let Some(ref bin) = binary_path {
            if let Ok(bytes) = std::fs::read(bin) {
                if let Ok(elf) = crate::elf::inspect_elf_bytes(&bin.to_string_lossy(), &bytes) {
                    abi_isolation_verified = !elf.uses_android_bionic;
                }
            }
            if cfg!(target_arch = "aarch64") {
                if let Ok(output) = Command::new(bin).arg("--version").output() {
                    if output.status.success() {
                        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !text.is_empty() {
                            detected_version = Some(text);
                        }
                    }
                }
            }
        }

        let installed = binary_path.is_some();
        let status_summary = if installed && abi_isolation_verified {
            format!(
                "Box64 {} ready (guest Linux glibc ABI isolated)",
                detected_version.as_deref().unwrap_or(BOX64_PINNED_VERSION)
            )
        } else if installed {
            "Box64 binary found, but Android Bionic linkage warning detected — rebuild with aarch64-linux-gnu toolchain".to_string()
        } else {
            format!(
                "Box64 v{BOX64_PINNED_VERSION} not yet staged in guest rootfs (/usr/local/bin/box64) or artifacts/box64/box64"
            )
        };

        Box64InstallationStatus {
            installed,
            binary_path: guest_display_path,
            detected_version,
            pinned_version: BOX64_PINNED_VERSION.to_string(),
            license: BOX64_LICENSE.to_string(),
            upstream_repo: BOX64_UPSTREAM_REPO.to_string(),
            host_arch,
            dynarec_supported_on_host,
            abi_isolation_verified,
            status_summary,
        }
    }

    /// Stages the per-environment `box64.rc` and, if present in artifacts, installs Box64 inside the guest rootfs.
    pub fn ensure_guest_box64_staged(
        &self,
        rootfs: &Path,
        config: &Box64EnvironmentConfig,
    ) -> Result<(Option<String>, String), std::io::Error> {
        let guest_rc_rel = "etc/androidpc-box64.rc";
        let host_rc = rootfs.join(guest_rc_rel);
        if let Some(parent) = host_rc.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&host_rc, render_box64_rcfile(config))?;

        for candidate in GUEST_BOX64_CANDIDATES {
            let host_path = rootfs.join(candidate.trim_start_matches('/'));
            if host_path.is_file() {
                return Ok((Some((*candidate).to_string()), format!("/{guest_rc_rel}")));
            }
        }

        let staged_artifact = self.artifacts_box64_binary();
        if staged_artifact.is_file() {
            let guest_bin = rootfs.join("usr/local/bin/box64");
            if let Some(parent) = guest_bin.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&staged_artifact, &guest_bin)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&guest_bin, std::fs::Permissions::from_mode(0o755));
            }
            return Ok((
                Some("/usr/local/bin/box64".to_string()),
                format!("/{guest_rc_rel}"),
            ));
        }

        Ok((None, format!("/{guest_rc_rel}")))
    }

    /// Inspects a guest binary, validates ABI isolation and shared libraries, and builds the execution plan.
    pub fn plan_execution(
        &self,
        rootfs: &Path,
        guest_binary: &str,
        args: &[String],
        config: &Box64EnvironmentConfig,
    ) -> Result<Box64ExecutionPlan, CompatibilityError> {
        let inspection = inspect_guest_binary(rootfs, guest_binary).map_err(|err| match err {
            ElfParseError::NotFound(p) => CompatibilityError::BinaryNotFound(p),
            ElfParseError::InvalidElf(reason) => CompatibilityError::InvalidElf {
                path: guest_binary.to_string(),
                reason,
            },
            ElfParseError::Io { path, source } => {
                CompatibilityError::ExecutionFailed(format!("Failed reading '{path}': {source}"))
            }
        })?;

        if inspection.endianness != ElfEndianness::Little {
            return Err(CompatibilityError::UnsupportedArchitecture {
                arch: inspection.architecture.display_name().to_string(),
                e_machine: inspection.e_machine,
                reason: "Big-endian ELF executables are not supported on ARM64 Linux guests".into(),
            });
        }

        if !inspection.is_runnable_elf() {
            return Err(CompatibilityError::InvalidElf {
                path: guest_binary.to_string(),
                reason: format!("ELF object type {:?} is not executable", inspection.object_type),
            });
        }

        if inspection.uses_android_bionic {
            return Err(CompatibilityError::BionicAbiContamination {
                path: guest_binary.to_string(),
            });
        }

        let libraries = resolve_guest_libraries(rootfs, &inspection, &config.library_search_paths);

        match inspection.architecture {
            ElfArchitecture::Aarch64 => {
                let mut cmd = vec![guest_binary.to_string()];
                cmd.extend(args.iter().cloned());
                Ok(Box64ExecutionPlan {
                    route: Box64RouteDecision::NativeArm64,
                    guest_command: cmd,
                    extra_env: Vec::new(),
                    inspection,
                    libraries,
                    explanation: "Native ARM64 ELF detected — routing directly to PRoot Linux runtime without Box64 overhead.".into(),
                })
            }
            ElfArchitecture::X86_64 => {
                if !config.enabled {
                    return Err(CompatibilityError::Box64Disabled);
                }
                if libraries.has_missing_dependencies() {
                    return Err(CompatibilityError::MissingSharedLibraries(
                        libraries.missing_libs.join(", "),
                    ));
                }
                let (box64_guest_path, rc_guest_path) = self
                    .ensure_guest_box64_staged(rootfs, config)
                    .map_err(|e| CompatibilityError::ExecutionFailed(e.to_string()))?;

                let Some(box64_path) = box64_guest_path else {
                    return Err(CompatibilityError::Box64NotInstalled);
                };

                let mut cmd = vec![box64_path, guest_binary.to_string()];
                cmd.extend(args.iter().cloned());
                let extra_env = build_box64_env_vars(config, Some(guest_binary), Some(&rc_guest_path));

                Ok(Box64ExecutionPlan {
                    route: Box64RouteDecision::Box64X86_64,
                    guest_command: cmd,
                    extra_env,
                    inspection,
                    libraries,
                    explanation: format!(
                        "x86-64 Linux ELF detected — routing through Box64 ({}) with conservative {} mode.",
                        config.binary_version,
                        if config.dynarec_enabled { "ARM64 Dynarec" } else { "Interpreter fallback" }
                    ),
                })
            }
            other => Err(CompatibilityError::UnsupportedArchitecture {
                arch: other.display_name().to_string(),
                e_machine: inspection.e_machine,
                reason: format!(
                    "Only native ARM64 (aarch64) and translated x86-64 (via Box64) Linux ELF binaries are supported. Detected {}.",
                    other.display_name()
                ),
            }),
        }
    }

    /// Executes a guest binary through the existing PRoot runtime provider with architecture-aware routing,
    /// capturing stdout/stderr, exit codes, signals, and structured failure diagnostics.
    pub fn execute_and_capture(
        &self,
        env_mgr: &ProotEnvironmentManager,
        provider: &ProotRuntimeProvider,
        guest_binary: &str,
        args: &[String],
        extra_env: &[(String, String)],
    ) -> Box64ExecutionOutcome {
        let started = Instant::now();
        let env_id = env_mgr.config().id.clone();
        let rootfs = &env_mgr.config().rootfs_directory;
        let box64_cfg = &env_mgr.config().box64;

        let plan = match self.plan_execution(rootfs, guest_binary, args, box64_cfg) {
            Ok(plan) => plan,
            Err(err) => {
                let kind = err.kind();
                let missing = match &err {
                    CompatibilityError::MissingSharedLibraries(list) => list
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    _ => Vec::new(),
                };
                let outcome = Box64ExecutionOutcome {
                    environment_id: env_id.clone(),
                    binary_path: guest_binary.to_string(),
                    route: Box64RouteDecision::Unsupported,
                    architecture: "unknown".to_string(),
                    success: false,
                    exit_code: None,
                    terminating_signal: None,
                    error_kind: kind,
                    error_explanation: Some(err.to_string()),
                    stdout: String::new(),
                    stderr: err.to_string(),
                    missing_libraries: missing,
                    duration_ms: started.elapsed().as_millis() as u64,
                    timestamp_epoch_ms: now_epoch_ms(),
                };
                self.record_outcome(&outcome);
                return outcome;
            }
        };

        let mut merged_env = extra_env.to_vec();
        merged_env.extend(plan.extra_env.clone());

        let proot_args = provider.build_proot_command(env_mgr.config(), &plan.guest_command);
        let output = Command::new(provider.proot_binary())
            .args(&proot_args)
            .env_clear()
            .envs(env_mgr.build_guest_env(&merged_env))
            .output();

        let duration_ms = started.elapsed().as_millis() as u64;
        let arch_label = plan.inspection.architecture.display_name().to_string();

        let outcome = match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let exit_code = out.status.code();
                #[cfg(unix)]
                let terminating_signal = {
                    use std::os::unix::process::ExitStatusExt;
                    out.status.signal()
                };
                #[cfg(not(unix))]
                let terminating_signal = None;

                let (error_kind, error_explanation) =
                    classify_process_result(out.status.success(), exit_code, terminating_signal, &stderr);

                if out.status.success() {
                    info!(
                        "Compatible execution succeeded: env={} bin={} route={:?} duration={}ms",
                        env_id, guest_binary, plan.route, duration_ms
                    );
                } else {
                    warn!(
                        "Compatible execution failed: env={} bin={} route={:?} kind={:?} exit={:?} sig={:?}",
                        env_id, guest_binary, plan.route, error_kind, exit_code, terminating_signal
                    );
                }

                Box64ExecutionOutcome {
                    environment_id: env_id,
                    binary_path: guest_binary.to_string(),
                    route: plan.route,
                    architecture: arch_label,
                    success: out.status.success(),
                    exit_code,
                    terminating_signal,
                    error_kind,
                    error_explanation,
                    stdout,
                    stderr,
                    missing_libraries: plan.libraries.missing_libs,
                    duration_ms,
                    timestamp_epoch_ms: now_epoch_ms(),
                }
            }
            Err(io_err) => Box64ExecutionOutcome {
                environment_id: env_id,
                binary_path: guest_binary.to_string(),
                route: plan.route,
                architecture: arch_label,
                success: false,
                exit_code: None,
                terminating_signal: None,
                error_kind: Box64ErrorKind::ExecutionFailed,
                error_explanation: Some(format!("Failed to launch PRoot subprocess: {io_err}")),
                stdout: String::new(),
                stderr: io_err.to_string(),
                missing_libraries: plan.libraries.missing_libs,
                duration_ms,
                timestamp_epoch_ms: now_epoch_ms(),
            },
        };

        self.record_outcome(&outcome);
        outcome
    }

    pub fn record_outcome(&self, outcome: &Box64ExecutionOutcome) {
        let summary = if outcome.success {
            format!("Exited 0 via {:?} ({} ms)", outcome.route, outcome.duration_ms)
        } else {
            outcome
                .error_explanation
                .clone()
                .unwrap_or_else(|| format!("Failed ({:?})", outcome.error_kind))
        };

        androidpc_logging::push_log(
            if outcome.success { "INFO" } else { "WARN" },
            "androidpc::box64",
            &format!("{} [{}]: {}", outcome.binary_path, outcome.architecture, summary),
        );

        let record = Box64ExecutionRecord {
            timestamp_epoch_ms: outcome.timestamp_epoch_ms,
            environment_id: outcome.environment_id.clone(),
            binary_path: outcome.binary_path.clone(),
            route: outcome.route,
            architecture: outcome.architecture.clone(),
            success: outcome.success,
            exit_code: outcome.exit_code,
            terminating_signal: outcome.terminating_signal,
            error_kind: outcome.error_kind,
            summary,
            duration_ms: outcome.duration_ms,
        };

        if let Ok(mut guard) = self.history.write() {
            if guard.len() >= MAX_EXECUTION_HISTORY {
                guard.pop_front();
            }
            guard.push_back(record);
        }
    }

    pub fn execution_history(&self, limit: usize) -> Vec<Box64ExecutionRecord> {
        let max_items = if limit == 0 { 25 } else { limit.min(MAX_EXECUTION_HISTORY) };
        match self.history.read() {
            Ok(guard) => guard.iter().rev().take(max_items).cloned().collect(),
            Err(_) => Vec::new(),
        }
    }
}

pub fn classify_process_result(
    success: bool,
    exit_code: Option<i32>,
    signal: Option<i32>,
    stderr: &str,
) -> (Box64ErrorKind, Option<String>) {
    if success {
        return (Box64ErrorKind::None, None);
    }

    let lower = stderr.to_ascii_lowercase();
    if lower.contains("error while loading shared libraries")
        || lower.contains("cannot open shared object file")
        || lower.contains("error loading needed lib")
    {
        return (
            Box64ErrorKind::MissingSharedLibrary,
            Some("Required x86-64 shared library could not be resolved by Box64 inside the guest rootfs.".into()),
        );
    }

    if lower.contains("cannot allocate memory") || lower.contains("out of memory") || signal == Some(9) && exit_code.is_none() {
        if signal == Some(9) {
            return (
                Box64ErrorKind::OutOfMemory,
                Some("Process received SIGKILL (9) — terminated due to out-of-memory or environment shutdown.".into()),
            );
        }
        return (
            Box64ErrorKind::OutOfMemory,
            Some("Process failed due to insufficient available memory.".into()),
        );
    }

    if matches!(signal, Some(2) | Some(15)) || matches!(exit_code, Some(130) | Some(143)) {
        return (
            Box64ErrorKind::Interrupted,
            Some("Process was interrupted (SIGINT/SIGTERM) before completion.".into()),
        );
    }

    if let Some(sig) = signal {
        return (
            Box64ErrorKind::CrashedSignal,
            Some(format!(
                "Application terminated abnormally with signal {sig} (e.g., SIGSEGV=11, SIGILL=4, SIGABRT=6)."
            )),
        );
    }

    if let Some(code) = exit_code {
        if code > 128 {
            let sig = code - 128;
            return (
                Box64ErrorKind::CrashedSignal,
                Some(format!("Application exited with signal status {code} (signal {sig}).")),
            );
        }
        return (
            Box64ErrorKind::ExecutionFailed,
            Some(format!("Application exited with non-zero exit code {code}.")),
        );
    }

    (
        Box64ErrorKind::ExecutionFailed,
        Some("Application failed without a clear exit code.".into()),
    )
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
