use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::{BackendCapabilityReport, BackendReadinessStage, GraphicsBackendError};

pub const DXVK_PINNED_VERSION: &str = "2.7.1";
pub const DXVK_LICENSE: &str = "zlib";
pub const DXVK_UPSTREAM_REPO: &str = "https://github.com/doitsujin/dxvk";

pub const VKD3D_PROTON_PINNED_VERSION: &str = "3.1";
pub const VKD3D_PROTON_LICENSE: &str = "LGPL-2.1";
pub const VKD3D_PROTON_UPSTREAM_REPO: &str = "https://github.com/HansKristian-Work/vkd3d-proton";

pub const DXVK_DLLS: &[&str] = &["d3d8", "d3d9", "d3d10core", "d3d11", "dxgi"];
pub const VKD3D_DLLS: &[&str] = &["d3d12", "d3d12core"];

/// Minimum Vulkan extensions required by VKD3D-Proton v3.1 for Direct3D 12 translation.
pub const VKD3D_REQUIRED_VULKAN_EXTENSIONS: &[&str] = &[
    "VK_KHR_dynamic_rendering",
    "VK_EXT_descriptor_indexing",
    "VK_KHR_buffer_device_address",
    "VK_KHR_timeline_semaphore",
    "VK_EXT_robustness2",
    "VK_EXT_extended_dynamic_state",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectXBackendChoice {
    WineD3dOpenGl,
    DxvkVulkan,
    DxvkAndVkd3dProton,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxTranslationStatus {
    pub dxvk_pinned_version: String,
    pub dxvk_license: String,
    pub dxvk_staged: bool,
    pub dxvk_eligible: bool,
    pub dxvk_fallback_reason: Option<String>,
    pub vkd3d_pinned_version: String,
    pub vkd3d_license: String,
    pub vkd3d_staged: bool,
    pub vkd3d_eligible: bool,
    pub vkd3d_missing_extensions: Vec<String>,
    pub vkd3d_fallback_reason: Option<String>,
    pub effective_choice: DirectXBackendChoice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxTranslationPlan {
    pub effective_choice: DirectXBackendChoice,
    pub wine_dll_overrides: Vec<(String, String)>,
    pub env_vars: Vec<(String, String)>,
    pub warnings: Vec<String>,
}

pub struct DxTranslationManager;

impl DxTranslationManager {
    pub fn is_dxvk_staged(artifacts_dir: &Path) -> bool {
        let x64_dir = artifacts_dir.join("dxvk").join("x64");
        x64_dir.join("d3d11.dll").is_file() && x64_dir.join("dxgi.dll").is_file()
    }

    pub fn is_vkd3d_staged(artifacts_dir: &Path) -> bool {
        let x64_dir = artifacts_dir.join("vkd3d-proton").join("x64");
        x64_dir.join("d3d12.dll").is_file() && x64_dir.join("d3d12core.dll").is_file()
    }

    /// Evaluates DXVK (D3D8/9/10/11) and VKD3D-Proton (D3D12) eligibility against the
    /// probed graphics backend capabilities. Never forces Vulkan translation when the
    /// selected backend lacks operational Vulkan support or required extensions.
    pub fn evaluate(
        backend_report: &BackendCapabilityReport,
        artifacts_dir: &Path,
        want_dxvk: bool,
        want_vkd3d: bool,
    ) -> DxTranslationStatus {
        let dxvk_staged = Self::is_dxvk_staged(artifacts_dir);
        let vkd3d_staged = Self::is_vkd3d_staged(artifacts_dir);

        let backend_operational = matches!(
            backend_report.stage,
            BackendReadinessStage::Operational | BackendReadinessStage::ActiveInApp
        );

        let dxvk_fallback_reason = if !want_dxvk {
            Some("DXVK disabled in configuration; using WineD3D OpenGL.".to_string())
        } else if !backend_report.supports_vulkan {
            Some(format!(
                "{} does not provide Vulkan; falling back to WineD3D OpenGL.",
                backend_report.display_name
            ))
        } else if !backend_operational || !backend_report.dxvk_compatible {
            Some(format!(
                "Vulkan backend {} is in stage {:?} (not yet verified Operational); falling back to WineD3D OpenGL.",
                backend_report.display_name, backend_report.stage
            ))
        } else if !dxvk_staged {
            Some("DXVK v2.7.1 DLLs are not staged in artifacts/dxvk/x64; falling back to WineD3D OpenGL.".to_string())
        } else {
            None
        };
        let dxvk_eligible = dxvk_fallback_reason.is_none();

        let missing_exts: Vec<String> = VKD3D_REQUIRED_VULKAN_EXTENSIONS
            .iter()
            .filter(|req| !backend_report.verified_extensions.iter().any(|e| e == *req))
            .map(|s| (*s).to_string())
            .collect();

        let vkd3d_fallback_reason = if !want_vkd3d {
            Some("VKD3D-Proton (Direct3D 12) disabled in configuration.".to_string())
        } else if !dxvk_eligible {
            Some("VKD3D-Proton requires an operational Vulkan backend (DXVK prerequisites not met).".to_string())
        } else if !vkd3d_staged {
            Some("VKD3D-Proton v3.1 DLLs are not staged in artifacts/vkd3d-proton/x64.".to_string())
        } else if !missing_exts.is_empty() || !backend_report.vkd3d_compatible {
            Some(format!(
                "Direct3D 12 via VKD3D-Proton requires verified Vulkan extensions (missing/unverified: {}).",
                missing_exts.join(", ")
            ))
        } else {
            None
        };
        let vkd3d_eligible = vkd3d_fallback_reason.is_none();

        let effective_choice = if dxvk_eligible && vkd3d_eligible {
            DirectXBackendChoice::DxvkAndVkd3dProton
        } else if dxvk_eligible {
            DirectXBackendChoice::DxvkVulkan
        } else {
            DirectXBackendChoice::WineD3dOpenGl
        };

        DxTranslationStatus {
            dxvk_pinned_version: DXVK_PINNED_VERSION.to_string(),
            dxvk_license: DXVK_LICENSE.to_string(),
            dxvk_staged,
            dxvk_eligible,
            dxvk_fallback_reason,
            vkd3d_pinned_version: VKD3D_PROTON_PINNED_VERSION.to_string(),
            vkd3d_license: VKD3D_PROTON_LICENSE.to_string(),
            vkd3d_staged,
            vkd3d_eligible,
            vkd3d_missing_extensions: missing_exts,
            vkd3d_fallback_reason,
            effective_choice,
        }
    }

    /// Builds environment variables and `WINEDLLOVERRIDES` entries matching the verified
    /// DirectX translation choice, keeping fallback safe and explicit.
    pub fn build_plan(
        status: &DxTranslationStatus,
        shader_cache_dir: &str,
        enable_hud: bool,
    ) -> DxTranslationPlan {
        let mut overrides = Vec::new();
        let mut env_vars = Vec::new();
        let mut warnings = Vec::new();

        match status.effective_choice {
            DirectXBackendChoice::WineD3dOpenGl => {
                if let Some(ref reason) = status.dxvk_fallback_reason {
                    warnings.push(reason.clone());
                }
                for dll in DXVK_DLLS {
                    overrides.push(((*dll).to_string(), "b".to_string()));
                }
            }
            DirectXBackendChoice::DxvkVulkan => {
                for dll in DXVK_DLLS {
                    overrides.push(((*dll).to_string(), "n,b".to_string()));
                }
                env_vars.push(("DXVK_LOG_LEVEL".to_string(), "warn".to_string()));
                env_vars.push(("DXVK_STATE_CACHE_PATH".to_string(), shader_cache_dir.to_string()));
                env_vars.push((
                    "DXVK_HUD".to_string(),
                    if enable_hud { "fps,devinfo,api".to_string() } else { "0".to_string() },
                ));
                if let Some(ref reason) = status.vkd3d_fallback_reason {
                    warnings.push(reason.clone());
                }
            }
            DirectXBackendChoice::DxvkAndVkd3dProton => {
                for dll in DXVK_DLLS {
                    overrides.push(((*dll).to_string(), "n,b".to_string()));
                }
                for dll in VKD3D_DLLS {
                    overrides.push(((*dll).to_string(), "n,b".to_string()));
                }
                env_vars.push(("DXVK_LOG_LEVEL".to_string(), "warn".to_string()));
                env_vars.push(("DXVK_STATE_CACHE_PATH".to_string(), shader_cache_dir.to_string()));
                env_vars.push((
                    "DXVK_HUD".to_string(),
                    if enable_hud { "fps,devinfo,api".to_string() } else { "0".to_string() },
                ));
                env_vars.push(("VKD3D_DEBUG".to_string(), "warn".to_string()));
                env_vars.push(("VKD3D_SHADER_CACHE_PATH".to_string(), shader_cache_dir.to_string()));
            }
        }

        DxTranslationPlan {
            effective_choice: status.effective_choice,
            wine_dll_overrides: overrides,
            env_vars,
            warnings,
        }
    }

    /// Installs staged DXVK x64/x32 DLLs into a Wine prefix's `drive_c/windows/system32`
    /// (and `syswow64` when WoW64 x32 DLLs are staged) using safe path validation.
    pub fn stage_dxvk_into_prefix(
        artifacts_dir: &Path,
        prefix_host_dir: &Path,
    ) -> Result<Vec<String>, GraphicsBackendError> {
        let src_x64 = artifacts_dir.join("dxvk").join("x64");
        if !Self::is_dxvk_staged(artifacts_dir) {
            return Err(GraphicsBackendError::NotInstalled(
                "DXVK v2.7.1 x64 DLLs are not staged in artifacts/dxvk/x64".to_string(),
            ));
        }

        let sys32 = androidpc_security::resolve_safe_path(
            prefix_host_dir,
            Path::new("drive_c/windows/system32"),
        )?;
        std::fs::create_dir_all(&sys32)?;

        let mut copied = Vec::new();
        for dll in DXVK_DLLS {
            let filename = format!("{dll}.dll");
            let src = src_x64.join(&filename);
            if src.is_file() {
                std::fs::copy(&src, sys32.join(&filename))?;
                copied.push(format!("system32/{filename}"));
            }
        }

        let src_x32 = artifacts_dir.join("dxvk").join("x32");
        if src_x32.is_dir() {
            let syswow64 = androidpc_security::resolve_safe_path(
                prefix_host_dir,
                Path::new("drive_c/windows/syswow64"),
            )?;
            std::fs::create_dir_all(&syswow64)?;
            for dll in DXVK_DLLS {
                let filename = format!("{dll}.dll");
                let src = src_x32.join(&filename);
                if src.is_file() {
                    std::fs::copy(&src, syswow64.join(&filename))?;
                    copied.push(format!("syswow64/{filename}"));
                }
            }
        }

        Ok(copied)
    }
}
