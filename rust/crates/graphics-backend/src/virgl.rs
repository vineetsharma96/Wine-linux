use std::path::Path;

use crate::{
    BackendCapabilityReport, BackendReadinessStage, GraphicsBackend, GraphicsBackendConfig,
    GraphicsBackendError, GraphicsBackendKind, VIRGLRENDERER_PINNED_VERSION,
};

pub const DEFAULT_VIRGL_SOCKET: &str = "/tmp/.virgl_test";

/// VirGL (virglrenderer + Mesa virpipe) OpenGL-over-IPC backend.
///
/// Architectural reality:
/// - VirGL transports Gallium OpenGL commands from the Linux guest (`GALLIUM_DRIVER=virpipe`)
///   to a host `virgl_test_server` backed by Android EGL/GLES.
/// - VirGL does NOT provide a native Vulkan 1.3+ driver inside PRoot and therefore CANNOT
///   replace Turnip for DXVK or VKD3D-Proton. It only supports WineD3D OpenGL acceleration.
#[derive(Debug, Default)]
pub struct VirglBackend;

impl VirglBackend {
    fn has_guest_virpipe(guest_rootfs: Option<&Path>) -> bool {
        let Some(rootfs) = guest_rootfs else {
            return false;
        };
        let candidates = [
            "usr/lib/aarch64-linux-gnu/dri/virtio_gpu_dri.so",
            "usr/lib/aarch64-linux-gnu/dri/libgallium_dri.so",
            "usr/lib/aarch64-linux-gnu/libGL.so.1",
        ];
        candidates.iter().any(|rel| rootfs.join(rel).is_file())
    }

    fn find_host_server(artifacts_dir: &Path) -> Option<String> {
        let candidates = [
            artifacts_dir.join("graphics").join("virgl").join("virgl_test_server_android"),
            artifacts_dir.join("graphics").join("virgl").join("virgl_test_server"),
        ];
        candidates
            .into_iter()
            .find(|p| p.is_file())
            .map(|p| p.display().to_string())
    }

    fn socket_reachable(guest_rootfs: Option<&Path>, socket_path: &str) -> bool {
        if Path::new(socket_path).exists() {
            return true;
        }
        if let Some(rootfs) = guest_rootfs {
            let trimmed = socket_path.trim_start_matches('/');
            if rootfs.join(trimmed).exists() {
                return true;
            }
        }
        false
    }
}

impl GraphicsBackend for VirglBackend {
    fn kind(&self) -> GraphicsBackendKind {
        GraphicsBackendKind::Virgl
    }

    fn probe(
        &self,
        guest_rootfs: Option<&Path>,
        artifacts_dir: &Path,
        config: &GraphicsBackendConfig,
    ) -> BackendCapabilityReport {
        let guest_virpipe = Self::has_guest_virpipe(guest_rootfs);
        let host_server = Self::find_host_server(artifacts_dir);
        let socket_path = if config.virgl_socket_path.trim().is_empty() {
            DEFAULT_VIRGL_SOCKET
        } else {
            config.virgl_socket_path.as_str()
        };
        let socket_live = Self::socket_reachable(guest_rootfs, socket_path);

        let mut warnings = vec![
            "VirGL provides OpenGL (Gallium virpipe) acceleration only; it does NOT support Vulkan, DXVK, or VKD3D-Proton.".to_string(),
        ];
        let diagnostics = vec![
            format!("Host virgl_test_server binary: {}", host_server.as_deref().unwrap_or("not staged")),
            format!("Guest Mesa virpipe/Gallium driver present: {guest_virpipe}"),
            format!("VirGL IPC socket ({socket_path}): reachable={socket_live}"),
        ];

        let stage = if host_server.is_none() && !guest_virpipe {
            warnings.push(
                "Neither host virgl_test_server nor guest Mesa virpipe drivers were found.".to_string(),
            );
            BackendReadinessStage::NotInstalled
        } else if host_server.is_none() || !guest_virpipe {
            warnings.push(
                "Partial VirGL stack detected; both host virgl_test_server and guest Mesa virpipe are required."
                    .to_string(),
            );
            BackendReadinessStage::Installed
        } else if socket_live {
            BackendReadinessStage::Initialized
        } else {
            warnings.push(
                "VirGL binaries detected, but host virgl_test_server socket is not currently listening."
                    .to_string(),
            );
            BackendReadinessStage::Detected
        };

        BackendCapabilityReport {
            kind: GraphicsBackendKind::Virgl,
            display_name: "VirGL (OpenGL-over-IPC via virglrenderer)".to_string(),
            pinned_version: VIRGLRENDERER_PINNED_VERSION.to_string(),
            license: "MIT".to_string(),
            stage,
            supports_opengl: true,
            supports_vulkan: false,
            vulkan_api_version: None,
            dxvk_compatible: false,
            vkd3d_compatible: false,
            hardware_accelerated: true,
            detected_gpu: Some("Host EGL/GLES via virglrenderer".to_string()),
            icd_path: host_server,
            verified_extensions: Vec::new(),
            warnings,
            diagnostics,
        }
    }

    fn build_guest_env(
        &self,
        _report: &BackendCapabilityReport,
        config: &GraphicsBackendConfig,
    ) -> Result<Vec<(String, String)>, GraphicsBackendError> {
        let socket = if config.virgl_socket_path.trim().is_empty() {
            DEFAULT_VIRGL_SOCKET.to_string()
        } else {
            config.virgl_socket_path.clone()
        };

        Ok(vec![
            ("GALLIUM_DRIVER".to_string(), "virpipe".to_string()),
            ("VTEST_SOCK".to_string(), socket),
            ("MESA_GL_VERSION_OVERRIDE".to_string(), "4.3".to_string()),
            ("MESA_GLSL_VERSION_OVERRIDE".to_string(), "430".to_string()),
        ])
    }
}
