use std::path::Path;

use crate::{
    BackendCapabilityReport, BackendReadinessStage, GraphicsBackend, GraphicsBackendConfig,
    GraphicsBackendError, GraphicsBackendKind, MESA_TURNIP_PINNED_VERSION,
};

pub const LAVAPIPE_ICD_GUEST_PATH: &str = "/usr/share/vulkan/icd.d/lvp_icd.aarch64.json";

/// Software rendering fallback (`llvmpipe` for OpenGL, optional `lavapipe` for CPU Vulkan).
/// Ensures the Linux desktop and basic Wine applications remain usable when hardware GPU
/// acceleration (Turnip or VirGL) is unavailable or blocked.
#[derive(Debug, Default)]
pub struct SoftwareBackend;

impl SoftwareBackend {
    fn has_guest_llvmpipe(guest_rootfs: Option<&Path>) -> bool {
        let Some(rootfs) = guest_rootfs else {
            return false;
        };
        let candidates = [
            "usr/lib/aarch64-linux-gnu/dri/swrast_dri.so",
            "usr/lib/aarch64-linux-gnu/dri/libgallium_dri.so",
            "usr/lib/aarch64-linux-gnu/libGL.so.1",
        ];
        candidates.iter().any(|rel| rootfs.join(rel).is_file())
    }

    fn find_guest_lavapipe_icd(guest_rootfs: Option<&Path>) -> Option<String> {
        let rootfs = guest_rootfs?;
        for rel in [
            "usr/share/vulkan/icd.d/lvp_icd.aarch64.json",
            "usr/share/vulkan/icd.d/lvp_icd.json",
        ] {
            if rootfs.join(rel).is_file() {
                return Some(format!("/{rel}"));
            }
        }
        None
    }
}

impl GraphicsBackend for SoftwareBackend {
    fn kind(&self) -> GraphicsBackendKind {
        GraphicsBackendKind::Software
    }

    fn probe(
        &self,
        guest_rootfs: Option<&Path>,
        _artifacts_dir: &Path,
        _config: &GraphicsBackendConfig,
    ) -> BackendCapabilityReport {
        let has_llvmpipe = Self::has_guest_llvmpipe(guest_rootfs);
        let lavapipe_icd = Self::find_guest_lavapipe_icd(guest_rootfs);

        let mut warnings = vec![
            "Software rendering runs entirely on ARM64 CPU cores; suitable for UI/2D/diagnostics, not 3D gaming."
                .to_string(),
        ];
        let diagnostics = vec![
            format!("Guest Mesa llvmpipe/swrast detected: {has_llvmpipe}"),
            format!(
                "Guest Mesa lavapipe CPU Vulkan ICD: {}",
                lavapipe_icd.as_deref().unwrap_or("not installed")
            ),
        ];

        let stage = if has_llvmpipe {
            BackendReadinessStage::Detected
        } else {
            warnings.push(
                "Guest Mesa llvmpipe packages (libgl1-mesa-dri) not yet detected in selected rootfs."
                    .to_string(),
            );
            BackendReadinessStage::NotInstalled
        };

        BackendCapabilityReport {
            kind: GraphicsBackendKind::Software,
            display_name: "Software Fallback (Mesa LLVMpipe / Lavapipe)".to_string(),
            pinned_version: MESA_TURNIP_PINNED_VERSION.to_string(),
            license: "MIT".to_string(),
            stage,
            supports_opengl: true,
            supports_vulkan: lavapipe_icd.is_some(),
            vulkan_api_version: lavapipe_icd.as_ref().map(|_| "1.3 (CPU software)".to_string()),
            dxvk_compatible: false, // Do not force DXVK on CPU software rendering by default
            vkd3d_compatible: false,
            hardware_accelerated: false,
            detected_gpu: Some("ARM64 CPU (Mesa llvmpipe)".to_string()),
            icd_path: lavapipe_icd,
            verified_extensions: Vec::new(),
            warnings,
            diagnostics,
        }
    }

    fn build_guest_env(
        &self,
        report: &BackendCapabilityReport,
        _config: &GraphicsBackendConfig,
    ) -> Result<Vec<(String, String)>, GraphicsBackendError> {
        let mut env = vec![
            ("LIBGL_ALWAYS_SOFTWARE".to_string(), "1".to_string()),
            ("GALLIUM_DRIVER".to_string(), "llvmpipe".to_string()),
        ];
        if let Some(ref lvp) = report.icd_path {
            env.push(("VK_ICD_FILENAMES".to_string(), lvp.clone()));
            env.push(("VK_DRIVER_FILES".to_string(), lvp.clone()));
        }
        Ok(env)
    }
}
