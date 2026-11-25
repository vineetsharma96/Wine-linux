# Graphics Backends & DirectX Translation (Phase 4.3 & 4.4)

*Last updated: 2026-11-25*

All graphics backend probing, readiness verification, fallback selection, and DXVK/VKD3D-Proton gating are implemented in **`rust/crates/graphics-backend` (`androidpc-graphics-backend`)**.

## 1. Backend Capability Matrix

| Backend | Pinned Component | OpenGL | Vulkan | DXVK (D3D8–11) | VKD3D-Proton (D3D12) | Hardware Target |
|---|---|---|---|---|---|---|
| **Turnip** | Mesa `26.3.1` (MIT) | Yes (via Zink) | Yes (1.3+ when `Operational`) | Yes (when `Operational`) | Only if all 6 Vulkan ext. verified | Qualcomm Adreno (`a6xx`/`a7xx`, e.g., Xiaomi Pad 7 Adreno 732 via `/dev/kgsl-3d0`) |
| **VirGL** | `virglrenderer` `1.3.0` (MIT) | Yes (`virpipe` → EGL/GLES) | **No** | **No** | **No** | Generic Android GPUs (Mali, Xclipse, Adreno, PowerVR) |
| **Software** | Mesa `llvmpipe` (`26.3.1`, MIT) | Yes (CPU) | Optional (`lavapipe`, slow) | **No** (defaults to WineD3D) | **No** | Universal fallback on all ARM64 devices |

## 2. Six-Stage Honest Readiness Model

Every backend reports an exact `BackendReadinessStage`:
1. `not_installed`: Driver binaries / ICD JSON missing from artifact directory and guest rootfs.
2. `installed`: Driver files present on disk (NOT yet verified on hardware).
3. `detected`: Required device node (`/dev/kgsl-3d0` for Turnip) or IPC socket (`VTEST_SOCKET_NAME` for VirGL) detected.
4. `initialized`: Driver loader enumerated physical GPU / GL renderer string.
5. `operational`: Live runtime probe confirmed rendering/Vulkan API version and extensions.
6. `active_in_app`: Currently bound by an active Windows application session.

**Invariant**: A backend is eligible for hardware selection **only at `Operational` or `ActiveInApp`**. Simply placing `libvulkan_freedreno.so` on disk reaches `Installed` (or `Detected` if `/dev/kgsl-3d0` exists) and triggers automatic fallback to `Software` (`llvmpipe`) until a live Vulkan probe confirms `Operational`.

## 3. DirectX Translation Gating (`DxTranslationManager`)

- **DXVK `v2.7.1` (zlib)**: Enabled only when:
  1. User enables DXVK in graphics/prefix config,
  2. `x64/` and `x32/` DLLs (`d3d8`, `d3d9`, `d3d10core`, `d3d11`, `dxgi`) are staged, AND
  3. Active backend is `Operational` with `supports_vulkan == true` and `dxvk_compatible == true`.
  - Otherwise falls back automatically to **`WineD3D` OpenGL** with a recorded reason.
- **VKD3D-Proton `v3.1` (LGPL-2.1)**: Enabled only when:
  1. User enables VKD3D-Proton,
  2. `d3d12.dll` and `d3d12core.dll` are staged,
  3. Active backend is `Operational` with `vkd3d_compatible == true`, AND
  4. Live Vulkan probe verified **all 6 mandatory extensions**:
     - `VK_KHR_dynamic_rendering`
     - `VK_EXT_descriptor_indexing`
     - `VK_KHR_buffer_device_address`
     - `VK_KHR_timeline_semaphore`
     - `VK_EXT_robustness2`
     - `VK_EXT_extended_dynamic_state`
  - Missing extensions are reported explicitly in both Rust diagnostics and the **Graphics & DX** Android UI tab.

## 4. Runtime Manifests

- `/runtime/graphics/manifest.json` — Mesa Turnip `26.3.1`, `virglrenderer 1.3.0`, and `llvmpipe` metadata.
- `/runtime/dxvk/manifest.json` — DXVK `v2.7.1` DLL list and staging targets.
- `/runtime/vkd3d/manifest.json` — VKD3D-Proton `v3.1` DLL list and mandatory Vulkan extensions.
