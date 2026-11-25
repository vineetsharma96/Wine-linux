use std::path::Path;

use crate::{
    BackendCapabilityReport, BackendReadinessStage, GraphicsBackend, GraphicsBackendConfig,
    GraphicsBackendError, GraphicsBackendKind, MESA_TURNIP_PINNED_VERSION,
};

pub const TURNIP_ICD_GUEST_PATH: &str = "/usr/share/vulkan/icd.d/freedreno_icd.aarch64.json";
pub const TURNIP_ICD_ALT_GUEST_PATH: &str = "/usr/local/share/vulkan/icd.d/freedreno_icd.aarch64.json";
pub const KGSL_DEVICE_PATH: &str = "/dev/kgsl-3d0";

/// Mesa Turnip Vulkan driver backend targeting Qualcomm Adreno GPUs (e.g. Xiaomi Pad 7 Adreno 732).
///
/// Strictly separates `Installed`, `Detected`, `Initialized`, `Operational`, and `ActiveInApp`.
/// Never marks Turnip operational merely because the ICD JSON or `.so` file exists on disk.
#[derive(Debug, Default)]
pub struct TurnipBackend;

impl TurnipBackend {
    fn detect_kgsl_access() -> (bool, bool) {
        let path = Path::new(KGSL_DEVICE_PATH);
        let exists = path.exists();
        if !exists {
            return (false, false);
        }
        let accessible = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .is_ok();
        (true, accessible)
    }

    fn detect_adreno_hint() -> Option<String> {
        let sysfs_candidates = [
            "/sys/class/kgsl/kgsl-3d0/gpu_model",
            "/sys/devices/platform/soc/3d00000.qcom,kgsl-3d0/kgsl/kgsl-3d0/gpu_model",
        ];
        for candidate in sysfs_candidates {
            if let Ok(raw) = std::fs::read_to_string(candidate) {
                let trimmed = raw.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
        None
    }

    fn find_guest_icd(guest_rootfs: Option<&Path>, artifacts_dir: &Path) -> Option<String> {
        if let Some(rootfs) = guest_rootfs {
            for rel in [
                "usr/share/vulkan/icd.d/freedreno_icd.aarch64.json",
                "usr/local/share/vulkan/icd.d/freedreno_icd.aarch64.json",
                "usr/share/vulkan/icd.d/turnip_icd.aarch64.json",
            ] {
                if rootfs.join(rel).is_file() {
                    return Some(format!("/{rel}"));
                }
            }
        }
        let staged = artifacts_dir.join("graphics").join("turnip").join("freedreno_icd.aarch64.json");
        if staged.is_file() {
            return Some(staged.display().to_string());
        }
        None
    }

    fn has_guest_turnip_library(guest_rootfs: Option<&Path>, artifacts_dir: &Path) -> bool {
        if let Some(rootfs) = guest_rootfs {
            let candidates = [
                "usr/lib/aarch64-linux-gnu/libvulkan_freedreno.so",
                "usr/local/lib/aarch64-linux-gnu/libvulkan_freedreno.so",
                "usr/lib/libvulkan_freedreno.so",
            ];
            if candidates.iter().any(|rel| rootfs.join(rel).is_file()) {
                return true;
            }
        }
        artifacts_dir
            .join("graphics")
            .join("turnip")
            .join("libvulkan_freedreno.so")
            .is_file()
    }
}

impl GraphicsBackend for TurnipBackend {
    fn kind(&self) -> GraphicsBackendKind {
        GraphicsBackendKind::Turnip
    }

    fn probe(
        &self,
        guest_rootfs: Option<&Path>,
        artifacts_dir: &Path,
        _config: &GraphicsBackendConfig,
    ) -> BackendCapabilityReport {
        let icd_path = Self::find_guest_icd(guest_rootfs, artifacts_dir);
        let lib_present = Self::has_guest_turnip_library(guest_rootfs, artifacts_dir);
        let installed = icd_path.is_some() && lib_present;

        let (kgsl_exists, kgsl_accessible) = Self::detect_kgsl_access();
        let gpu_model = Self::detect_adreno_hint();

        let mut warnings = Vec::new();
        let mut diagnostics = Vec::new();

        diagnostics.push(format!(
            "Target reference GPU: Qualcomm Adreno 732 (Snapdragon 7+ Gen 3, Xiaomi Pad 7); detected sysfs model: {}",
            gpu_model.as_deref().unwrap_or("unavailable in current sandbox")
        ));
        diagnostics.push(format!(
            "KGSL node {}: present={}, rw_accessible={}",
            KGSL_DEVICE_PATH, kgsl_exists, kgsl_accessible
        ));
        diagnostics.push(format!(
            "Guest Turnip artifacts: icd={}, libvulkan_freedreno.so={}",
            icd_path.as_deref().unwrap_or("missing"),
            lib_present
        ));

        let stage = if !installed {
            warnings.push(
                "Mesa Turnip guest ICD and libvulkan_freedreno.so are not staged inside the ARM64 Linux guest."
                    .to_string(),
            );
            BackendReadinessStage::NotInstalled
        } else if !kgsl_exists {
            warnings.push(
                "/dev/kgsl-3d0 not present on host; hardware Adreno KGSL Vulkan path unavailable."
                    .to_string(),
            );
            BackendReadinessStage::Installed
        } else if !kgsl_accessible {
            warnings.push(
                "/dev/kgsl-3d0 exists but is blocked by Android SELinux/app permissions (rootless boundary)."
                    .to_string(),
            );
            BackendReadinessStage::Detected
        } else {
            // Operational status requires an executed Vulkan instance/device creation probe
            // inside the guest (e.g., vulkaninfo / vkcube-headless). Until verified live,
            // report `Detected` rather than overclaiming `Operational`.
            warnings.push(
                "Turnip artifacts and /dev/kgsl-3d0 are detected; run guest Vulkan capability probe to verify Operational status."
                    .to_string(),
            );
            BackendReadinessStage::Detected
        };

        // Never claim Vulkan 1.3+ / DXVK / VKD3D readiness before runtime probe succeeds.
        let vulkan_ready = matches!(
            stage,
            BackendReadinessStage::Operational | BackendReadinessStage::ActiveInApp
        );

        BackendCapabilityReport {
            kind: GraphicsBackendKind::Turnip,
            display_name: "Mesa Turnip (Adreno KGSL Vulkan)".to_string(),
            pinned_version: MESA_TURNIP_PINNED_VERSION.to_string(),
            license: "MIT".to_string(),
            stage,
            supports_opengl: true,  // via Mesa Zink over Turnip Vulkan
            supports_vulkan: true,
            vulkan_api_version: if vulkan_ready {
                Some("1.3".to_string())
            } else {
                None
            },
            dxvk_compatible: vulkan_ready,
            vkd3d_compatible: false, // Requires live Vulkan extension probe (dynamic_rendering, descriptor_indexing, bda, timeline_semaphore)
            hardware_accelerated: true,
            detected_gpu: gpu_model,
            icd_path,
            verified_extensions: Vec::new(),
            warnings,
            diagnostics,
        }
    }

    fn build_guest_env(
        &self,
        report: &BackendCapabilityReport,
        config: &GraphicsBackendConfig,
    ) -> Result<Vec<(String, String)>, GraphicsBackendError> {
        let icd = report
            .icd_path
            .clone()
            .unwrap_or_else(|| TURNIP_ICD_GUEST_PATH.to_string());

        let mut env = vec![
            ("VK_ICD_FILENAMES".to_string(), icd.clone()),
            ("VK_DRIVER_FILES".to_string(), icd),
            ("MESA_LOADER_DRIVER_OVERRIDE".to_string(), "zink".to_string()),
            ("GALLIUM_DRIVER".to_string(), "zink".to_string()),
            ("MESA_VK_WSI_PRESENT_MODE".to_string(), "mailbox".to_string()),
            // Preserve guest glibc loader isolation; never reference Android /system/lib64.
            ("TU_DEBUG".to_string(), config.turnip_debug_flags.clone()),
        ];

        if config.enable_zink_gl_override {
            env.push(("MESA_GL_VERSION_OVERRIDE".to_string(), "4.6".to_string()));
            env.push(("MESA_GLSL_VERSION_OVERRIDE".to_string(), "460".to_string()));
        }

        Ok(env)
    }
}
