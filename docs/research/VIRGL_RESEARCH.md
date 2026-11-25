# VirGL Renderer Research — OpenGL-Only IPC Backend (Phase 4.3)

*Last updated: 2026-11-25*
*Pinned component: `virglrenderer` `v1.3.0` (MIT License) + Mesa `virpipe` guest driver*

## 1. Architecture & Role in Android PC Environment

**VirGL** provides an architecture-neutral, vendor-independent **OpenGL** acceleration path for Android devices where Qualcomm Turnip is unavailable (e.g., ARM Mali, MediaTek Immortalis, Samsung Xclipse, PowerVR, or locked-down devices without `/dev/kgsl-3d0`).

### Execution Topology
```text
Windows App (OpenGL / WineD3D OpenGL)
  → Wine 11.0 (Box64)
  → Guest Mesa Gallium `virpipe` driver (GALLIUM_DRIVER=virpipe)
  → Unix Domain Socket (`VTEST_SOCKET_NAME=/tmp/.virgl_test`)
  → Host `virgl_test_server_android` (Android NDK EGL / GLES 3.x)
  → Android Host GPU (Adreno / Mali / Xclipse / PowerVR)
```

## 2. Critical Technical Limitation (Enforced in Code)

**VirGL (`virglrenderer 1.3.0` via `vtest`/`virpipe` over Android EGL/GLES) is strictly OpenGL-only.**

- **Supports**: OpenGL / WineD3D (Direct3D 1–11 translated by Wine to OpenGL).
- **Does NOT Support**:
  - Vulkan ICD (`supports_vulkan = false`).
  - **DXVK** (`dxvk_compatible = false`) — DXVK requires Vulkan 1.3+.
  - **VKD3D-Proton** (`vkd3d_compatible = false`) — VKD3D-Proton requires Vulkan 1.3+ and advanced desktop Vulkan extensions.

*Note on Venus (`virglrenderer` Vulkan)*: While upstream `virglrenderer` includes experimental Venus Vulkan serialization for KVM/virtio-gpu VMs, Venus requires a host Vulkan 1.3 driver with desktop extensions + virtio-gpu/cross-domain transport and is **not** available over `virpipe`/`vtest` EGL/GLES sockets on non-rooted Android. Therefore, `VirglBackend` in `rust/crates/graphics-backend/src/virgl.rs` explicitly sets `supports_vulkan = false`, `dxvk_compatible = false`, and `vkd3d_compatible = false`, and `DxTranslationManager` automatically forces `WineD3D` OpenGL whenever VirGL is selected.

## 3. Confirmed Facts & Implementation (`rust/crates/graphics-backend/src/virgl.rs`)

- **Official Source**: `https://gitlab.freedesktop.org/virgl/virglrenderer`
- **Pinned Version**: `1.3.0` (MIT License)
- **Readiness Probing**:
  - `not_installed`: `virgl_test_server_android` binary missing.
  - `installed`: Server binary staged, but Unix socket (`VTEST_SOCKET_NAME`) is not listening.
  - `detected` / `initialized` / `operational`: Socket present and accepts a verified `vtest` connection handshake; guest Mesa `virpipe` GL renderer string confirmed.
- **Environment Exported When Active**:
  - `GALLIUM_DRIVER=virpipe`
  - `MESA_GL_VERSION_OVERRIDE=4.3COMPAT`
  - `VTEST_SOCKET_NAME=<short_socket_path>` (respecting `< 108` byte Unix `sun_path` limit via our `<base_data_dir>/dtmp` bind).
- **Fallback Guarantee**: If `virgl_test_server_android` fails to bind, crashes, or refuses connection, `GraphicsManager` falls back automatically to `Software` (`llvmpipe`) with an explicit diagnostic reason.

## 4. Unknowns / Pending Device Validation (`NOT RUN`)

- Frame pacing and socket IPC serialization overhead between PRoot guest `virpipe` and host `virgl_test_server_android` on Android 16 (Xiaomi Pad 7).
- Compatibility of `MESA_GL_VERSION_OVERRIDE=4.3COMPAT` with WineD3D shader translation on Android GLES 3.2 host contexts (certain desktop GL geometry/tessellation features may degrade or fail depending on host GLES driver extensions).
- Process cleanup behavior of `virgl_test_server_android` across Android Activity backgrounding and app force-stop.
