pub mod dx_translation;
pub mod software;
pub mod turnip;
pub mod virgl;

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use dx_translation::{
    DirectXBackendChoice, DxTranslationManager, DxTranslationPlan, DxTranslationStatus,
    DXVK_DLLS, DXVK_LICENSE, DXVK_PINNED_VERSION, DXVK_UPSTREAM_REPO,
    VKD3D_DLLS, VKD3D_PROTON_LICENSE, VKD3D_PROTON_PINNED_VERSION, VKD3D_PROTON_UPSTREAM_REPO,
    VKD3D_REQUIRED_VULKAN_EXTENSIONS,
};
pub use software::SoftwareBackend;
pub use turnip::TurnipBackend;
pub use virgl::VirglBackend;

pub const MESA_TURNIP_PINNED_VERSION: &str = "26.3.1";
pub const VIRGLRENDERER_PINNED_VERSION: &str = "1.3.0";

#[derive(Debug, Error)]
pub enum GraphicsBackendError {
    #[error("Graphics backend component not installed: {0}")]
    NotInstalled(String),

    #[error("Graphics backend unavailable: {0}")]
    Unavailable(String),

    #[error("Security violation: {0}")]
    Security(#[from] androidpc_security::SecurityError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphicsBackendKind {
    Turnip,
    Virgl,
    Software,
}

impl Default for GraphicsBackendKind {
    fn default() -> Self {
        Self::Software
    }
}

impl GraphicsBackendKind {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "turnip" | "turnip_zink" | "zink" | "vulkan" => Self::Turnip,
            "virgl" | "virpipe" => Self::Virgl,
            _ => Self::Software,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Turnip => "turnip",
            Self::Virgl => "virgl",
            Self::Software => "software",
        }
    }

    pub fn to_legacy_gpu_mode(&self) -> androidpc_display::GpuAccelerationMode {
        match self {
            Self::Turnip => androidpc_display::GpuAccelerationMode::TurnipZink,
            Self::Virgl => androidpc_display::GpuAccelerationMode::Virgl,
            Self::Software => androidpc_display::GpuAccelerationMode::SoftwareLlvmPipe,
        }
    }
}

/// Strictly distinguishes backend readiness states so a backend is NEVER reported as
/// operational merely because its shared library or ICD JSON exists on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendReadinessStage {
    NotInstalled,
    Installed,
    Detected,
    Initialized,
    Operational,
    ActiveInApp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsBackendConfig {
    pub preferred_backend: GraphicsBackendKind,
    pub allow_software_fallback: bool,
    pub dxvk_enabled: bool,
    pub vkd3d_proton_enabled: bool,
    pub enable_hud: bool,
    pub turnip_debug_flags: String,
    pub enable_zink_gl_override: bool,
    pub virgl_socket_path: String,
    pub shader_cache_dir: String,
}

impl Default for GraphicsBackendConfig {
    fn default() -> Self {
        Self {
            preferred_backend: GraphicsBackendKind::Software,
            allow_software_fallback: true,
            dxvk_enabled: false,
            vkd3d_proton_enabled: false,
            enable_hud: false,
            turnip_debug_flags: "noconform".to_string(),
            enable_zink_gl_override: true,
            virgl_socket_path: virgl::DEFAULT_VIRGL_SOCKET.to_string(),
            shader_cache_dir: "/home/user/.cache/androidpc-shaders".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendCapabilityReport {
    pub kind: GraphicsBackendKind,
    pub display_name: String,
    pub pinned_version: String,
    pub license: String,
    pub stage: BackendReadinessStage,
    pub supports_opengl: bool,
    pub supports_vulkan: bool,
    pub vulkan_api_version: Option<String>,
    pub dxvk_compatible: bool,
    pub vkd3d_compatible: bool,
    pub hardware_accelerated: bool,
    pub detected_gpu: Option<String>,
    pub icd_path: Option<String>,
    pub verified_extensions: Vec<String>,
    pub warnings: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsStackOverview {
    pub active_config: GraphicsBackendConfig,
    pub effective_backend: GraphicsBackendKind,
    pub fallback_engaged: bool,
    pub fallback_reason: Option<String>,
    pub backends: Vec<BackendCapabilityReport>,
    pub dx_translation: DxTranslationStatus,
}

pub trait GraphicsBackend: Send + Sync {
    fn kind(&self) -> GraphicsBackendKind;
    fn probe(
        &self,
        guest_rootfs: Option<&Path>,
        artifacts_dir: &Path,
        config: &GraphicsBackendConfig,
    ) -> BackendCapabilityReport;
    fn build_guest_env(
        &self,
        report: &BackendCapabilityReport,
        config: &GraphicsBackendConfig,
    ) -> Result<Vec<(String, String)>, GraphicsBackendError>;
}

pub struct GraphicsManager {
    artifacts_dir: PathBuf,
    config: RwLock<GraphicsBackendConfig>,
    turnip: TurnipBackend,
    virgl: VirglBackend,
    software: SoftwareBackend,
    /// Optional runtime Vulkan verification overrides recorded after an executed on-device probe.
    verified_vulkan_extensions: RwLock<Vec<String>>,
    turnip_runtime_verified: RwLock<bool>,
}

impl GraphicsManager {
    pub fn new(artifacts_dir: PathBuf) -> Self {
        Self {
            artifacts_dir,
            config: RwLock::new(GraphicsBackendConfig::default()),
            turnip: TurnipBackend,
            virgl: VirglBackend,
            software: SoftwareBackend,
            verified_vulkan_extensions: RwLock::new(Vec::new()),
            turnip_runtime_verified: RwLock::new(false),
        }
    }

    pub fn config(&self) -> GraphicsBackendConfig {
        self.config.read().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn set_config(&self, config: GraphicsBackendConfig) {
        if let Ok(mut g) = self.config.write() {
            *g = config;
        }
    }

    /// Records the result of an actual on-device guest Vulkan probe (`vulkaninfo` / vkCreateDevice).
    /// Only after a live probe succeeds will Turnip transition to `Operational`.
    pub fn record_turnip_runtime_probe(&self, operational: bool, extensions: Vec<String>) {
        if let Ok(mut flag) = self.turnip_runtime_verified.write() {
            *flag = operational;
        }
        if let Ok(mut exts) = self.verified_vulkan_extensions.write() {
            *exts = extensions;
        }
    }

    pub fn probe_all(&self, guest_rootfs: Option<&Path>) -> GraphicsStackOverview {
        let cfg = self.config();
        let mut turnip_report = self.turnip.probe(guest_rootfs, &self.artifacts_dir, &cfg);
        if self.turnip_runtime_verified.read().map(|v| *v).unwrap_or(false)
            && turnip_report.stage >= BackendReadinessStage::Detected
        {
            turnip_report.stage = BackendReadinessStage::Operational;
            turnip_report.vulkan_api_version = Some("1.3".to_string());
            turnip_report.dxvk_compatible = true;
            let exts = self
                .verified_vulkan_extensions
                .read()
                .map(|v| v.clone())
                .unwrap_or_default();
            turnip_report.vkd3d_compatible = VKD3D_REQUIRED_VULKAN_EXTENSIONS
                .iter()
                .all(|req| exts.iter().any(|e| e == req));
            turnip_report.verified_extensions = exts;
        }

        let virgl_report = self.virgl.probe(guest_rootfs, &self.artifacts_dir, &cfg);
        let sw_report = self.software.probe(guest_rootfs, &self.artifacts_dir, &cfg);

        let preferred_report = match cfg.preferred_backend {
            GraphicsBackendKind::Turnip => &turnip_report,
            GraphicsBackendKind::Virgl => &virgl_report,
            GraphicsBackendKind::Software => &sw_report,
        };

        let preferred_usable = preferred_report.stage >= BackendReadinessStage::Detected;
        let (effective_backend, fallback_engaged, fallback_reason, effective_report) =
            if cfg.preferred_backend == GraphicsBackendKind::Software || preferred_usable {
                (cfg.preferred_backend, false, None, preferred_report)
            } else if cfg.allow_software_fallback {
                (
                    GraphicsBackendKind::Software,
                    true,
                    Some(format!(
                        "Preferred backend '{}' is {:?}; fell back to Software (Mesa LLVMpipe).",
                        preferred_report.display_name, preferred_report.stage
                    )),
                    &sw_report,
                )
            } else {
                (cfg.preferred_backend, false, None, preferred_report)
            };

        let dx_translation = DxTranslationManager::evaluate(
            effective_report,
            &self.artifacts_dir,
            cfg.dxvk_enabled,
            cfg.vkd3d_proton_enabled,
        );

        GraphicsStackOverview {
            active_config: cfg,
            effective_backend,
            fallback_engaged,
            fallback_reason,
            backends: vec![turnip_report, virgl_report, sw_report],
            dx_translation,
        }
    }

    pub fn build_effective_guest_env(
        &self,
        guest_rootfs: Option<&Path>,
        override_backend: Option<GraphicsBackendKind>,
    ) -> Result<(Vec<(String, String)>, DxTranslationPlan, GraphicsStackOverview), GraphicsBackendError> {
        let mut overview = self.probe_all(guest_rootfs);
        if let Some(kind) = override_backend {
            overview.active_config.preferred_backend = kind;
            let mut cfg = self.config();
            cfg.preferred_backend = kind;
            let report = overview
                .backends
                .iter()
                .find(|b| b.kind == kind)
                .cloned()
                .unwrap_or_else(|| self.software.probe(guest_rootfs, &self.artifacts_dir, &cfg));
            let usable = kind == GraphicsBackendKind::Software
                || report.stage >= BackendReadinessStage::Detected;
            if usable {
                overview.effective_backend = kind;
                overview.fallback_engaged = false;
                overview.fallback_reason = None;
                overview.dx_translation = DxTranslationManager::evaluate(
                    &report,
                    &self.artifacts_dir,
                    cfg.dxvk_enabled,
                    cfg.vkd3d_proton_enabled,
                );
            } else if cfg.allow_software_fallback {
                overview.effective_backend = GraphicsBackendKind::Software;
                overview.fallback_engaged = true;
                overview.fallback_reason = Some(format!(
                    "Requested backend '{}' is {:?}; fell back to Software.",
                    report.display_name, report.stage
                ));
            }
        }

        let effective_report = overview
            .backends
            .iter()
            .find(|b| b.kind == overview.effective_backend)
            .ok_or_else(|| GraphicsBackendError::Unavailable("No backend report available".into()))?;

        let mut env = match overview.effective_backend {
            GraphicsBackendKind::Turnip => {
                self.turnip.build_guest_env(effective_report, &overview.active_config)?
            }
            GraphicsBackendKind::Virgl => {
                self.virgl.build_guest_env(effective_report, &overview.active_config)?
            }
            GraphicsBackendKind::Software => {
                self.software.build_guest_env(effective_report, &overview.active_config)?
            }
        };

        let dx_plan = DxTranslationManager::build_plan(
            &overview.dx_translation,
            &overview.active_config.shader_cache_dir,
            overview.active_config.enable_hud,
        );
        env.extend(dx_plan.env_vars.clone());

        Ok((env, dx_plan, overview))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turnip_never_claims_operational_without_runtime_probe() {
        let tmp = std::env::temp_dir().join("androidpc_gfx_turnip_test");
        let _ = std::fs::remove_dir_all(&tmp);
        let rootfs = tmp.join("rootfs");
        std::fs::create_dir_all(rootfs.join("usr/share/vulkan/icd.d")).unwrap();
        std::fs::create_dir_all(rootfs.join("usr/lib/aarch64-linux-gnu")).unwrap();
        std::fs::write(rootfs.join("usr/share/vulkan/icd.d/freedreno_icd.aarch64.json"), "{}").unwrap();
        std::fs::write(rootfs.join("usr/lib/aarch64-linux-gnu/libvulkan_freedreno.so"), b"ELF").unwrap();

        let mgr = GraphicsManager::new(tmp.join("artifacts"));
        let mut cfg = GraphicsBackendConfig::default();
        cfg.preferred_backend = GraphicsBackendKind::Turnip;
        cfg.dxvk_enabled = true;
        cfg.vkd3d_proton_enabled = true;
        mgr.set_config(cfg);

        let overview = mgr.probe_all(Some(&rootfs));
        let turnip = overview.backends.iter().find(|b| b.kind == GraphicsBackendKind::Turnip).unwrap();
        assert!(turnip.stage < BackendReadinessStage::Operational);
        assert!(!overview.dx_translation.dxvk_eligible);
        assert!(!overview.dx_translation.vkd3d_eligible);
        assert_eq!(overview.dx_translation.effective_choice, DirectXBackendChoice::WineD3dOpenGl);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn virgl_explicitly_blocks_dxvk_and_vkd3d() {
        let tmp = std::env::temp_dir().join("androidpc_gfx_virgl_test");
        let _ = std::fs::remove_dir_all(&tmp);
        let mgr = GraphicsManager::new(tmp.join("artifacts"));
        let mut cfg = GraphicsBackendConfig::default();
        cfg.preferred_backend = GraphicsBackendKind::Virgl;
        cfg.dxvk_enabled = true;
        cfg.vkd3d_proton_enabled = true;
        mgr.set_config(cfg);

        let overview = mgr.probe_all(None);
        let virgl = overview.backends.iter().find(|b| b.kind == GraphicsBackendKind::Virgl).unwrap();
        assert!(virgl.supports_opengl);
        assert!(!virgl.supports_vulkan);
        assert!(!virgl.dxvk_compatible);
        assert!(!virgl.vkd3d_compatible);
        assert_eq!(overview.effective_backend, GraphicsBackendKind::Software);
        assert!(overview.fallback_engaged);
    }

    #[test]
    fn vkd3d_requires_all_verified_extensions() {
        let tmp = std::env::temp_dir().join("androidpc_gfx_vkd3d_test");
        let _ = std::fs::remove_dir_all(&tmp);
        let artifacts = tmp.join("artifacts");
        std::fs::create_dir_all(artifacts.join("dxvk/x64")).unwrap();
        std::fs::write(artifacts.join("dxvk/x64/d3d11.dll"), b"MZ").unwrap();
        std::fs::write(artifacts.join("dxvk/x64/dxgi.dll"), b"MZ").unwrap();
        std::fs::create_dir_all(artifacts.join("vkd3d-proton/x64")).unwrap();
        std::fs::write(artifacts.join("vkd3d-proton/x64/d3d12.dll"), b"MZ").unwrap();
        std::fs::write(artifacts.join("vkd3d-proton/x64/d3d12core.dll"), b"MZ").unwrap();

        let report = BackendCapabilityReport {
            kind: GraphicsBackendKind::Turnip,
            display_name: "Turnip".into(),
            pinned_version: MESA_TURNIP_PINNED_VERSION.into(),
            license: "MIT".into(),
            stage: BackendReadinessStage::Operational,
            supports_opengl: true,
            supports_vulkan: true,
            vulkan_api_version: Some("1.3".into()),
            dxvk_compatible: true,
            vkd3d_compatible: false,
            hardware_accelerated: true,
            detected_gpu: Some("Adreno 732".into()),
            icd_path: Some(turnip::TURNIP_ICD_GUEST_PATH.into()),
            verified_extensions: vec!["VK_KHR_dynamic_rendering".into()],
            warnings: vec![],
            diagnostics: vec![],
        };

        let status = DxTranslationManager::evaluate(&report, &artifacts, true, true);
        assert!(status.dxvk_eligible);
        assert!(!status.vkd3d_eligible);
        assert_eq!(status.effective_choice, DirectXBackendChoice::DxvkVulkan);
        assert!(!status.vkd3d_missing_extensions.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
