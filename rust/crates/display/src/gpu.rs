use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuAccelerationMode {
    SoftwareLlvmPipe,
    Virgl,
    TurnipZink,
}

impl Default for GpuAccelerationMode {
    fn default() -> Self {
        Self::SoftwareLlvmPipe
    }
}

impl GpuAccelerationMode {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "virgl" => Self::Virgl,
            "turnip_zink" | "turnip" | "zink" => Self::TurnipZink,
            _ => Self::SoftwareLlvmPipe,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SoftwareLlvmPipe => "software",
            Self::Virgl => "virgl",
            Self::TurnipZink => "turnip_zink",
        }
    }

    /// Generates Mesa / Vulkan environment variables for rootless Linux & Wine guests.
    pub fn build_guest_env(&self) -> Vec<(String, String)> {
        match self {
            Self::SoftwareLlvmPipe => vec![
                ("LIBGL_ALWAYS_SOFTWARE".to_string(), "1".to_string()),
                ("GALLIUM_DRIVER".to_string(), "llvmpipe".to_string()),
            ],
            Self::Virgl => vec![
                ("GALLIUM_DRIVER".to_string(), "virpipe".to_string()),
                ("MESA_GL_VERSION_OVERRIDE".to_string(), "4.3".to_string()),
                ("MESA_GLSL_VERSION_OVERRIDE".to_string(), "430".to_string()),
            ],
            Self::TurnipZink => vec![
                ("MESA_LOADER_DRIVER_OVERRIDE".to_string(), "zink".to_string()),
                ("GALLIUM_DRIVER".to_string(), "zink".to_string()),
                ("TU_DEBUG".to_string(), "noconform".to_string()),
                (
                    "VK_ICD_FILENAMES".to_string(),
                    "/usr/share/vulkan/icd.d/freedreno_icd.aarch64.json".to_string(),
                ),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_acceleration_env_generation() {
        let sw = GpuAccelerationMode::parse("software").build_guest_env();
        assert!(sw.iter().any(|(k, v)| k == "GALLIUM_DRIVER" && v == "llvmpipe"));

        let virgl = GpuAccelerationMode::parse("virgl").build_guest_env();
        assert!(virgl.iter().any(|(k, v)| k == "GALLIUM_DRIVER" && v == "virpipe"));

        let zink = GpuAccelerationMode::parse("turnip_zink").build_guest_env();
        assert!(zink.iter().any(|(k, v)| k == "MESA_LOADER_DRIVER_OVERRIDE" && v == "zink"));
        assert!(zink.iter().any(|(k, _)| k == "VK_ICD_FILENAMES"));
    }
}
