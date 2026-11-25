# Mesa Turnip Vulkan Research — Xiaomi Pad 7 / Adreno 732 (Phase 4.3)

*Last updated: 2026-11-25*
*Target hardware: Xiaomi Pad 7 (`2410CRP4CI`, Qualcomm Snapdragon 7+ Gen 3, Adreno 732 GPU, Android 16, ARM64)*

## 1. Hardware & Driver Context

- **Target SoC / GPU**: Qualcomm Snapdragon 7+ Gen 3 (`SM7675`) featuring **Adreno 732** (`a7xx` / Gen7 architecture).
- **Kernel Interface**: Qualcomm KGSL character device `/dev/kgsl-3d0` exposed by the Android vendor kernel to unprivileged graphics applications.
- **Open-Source Driver**: **Mesa Freedreno Turnip** (`libvulkan_freedreno.so`), part of upstream Mesa (`https://gitlab.freedesktop.org/mesa/mesa`).
- **Pinned Version**: Mesa **`26.3.1`** (MIT License, released November 2026).

## 2. Why Turnip Is Selected Over Stock Android Adreno Vulkan

1. **Linux Guest ABI Compatibility**: Android's vendor Vulkan driver (`/vendor/lib64/hw/vulkan.adreno.so`) is linked against Android Bionic (`libc.so`, `liblog.so`, `libvndksupport.so`) and uses Android-specific WSI (`VK_KHR_android_surface`). Guest Ubuntu/Debian ARM64 glibc applications (and Box64 + Wine + DXVK/VKD3D) require a glibc-compatible Vulkan ICD (`libvulkan_freedreno.so`) with X11/headless WSI (`VK_KHR_xlib_surface`, `VK_KHR_xcb_surface`, `VK_EXT_headless_surface`) and KGSL backend enabled (`-Dfreedreno-kmds=kgsl`).
2. **Vulkan 1.3+ Extensions Required by DXVK & VKD3D-Proton**: Turnip exposes modern desktop Vulkan 1.3+ extensions (`VK_KHR_dynamic_rendering`, `VK_EXT_descriptor_indexing`, `VK_KHR_buffer_device_address`, `VK_KHR_timeline_semaphore`, `VK_EXT_extended_dynamic_state`, `VK_EXT_robustness2`, `VK_EXT_transform_feedback`) that stock mobile Qualcomm drivers frequently omit or ship with desktop-incompatible quirks.
3. **OpenGL via Zink**: When Turnip Vulkan is `Operational`, Mesa **Zink** (`MESA_LOADER_DRIVER_OVERRIDE=zink`, `GALLIUM_DRIVER=zink`) translates OpenGL calls from WineD3D or Linux desktop apps onto Turnip Vulkan without needing a separate Adreno OpenGL blob.

## 3. Honest Readiness Stages (Implemented in `rust/crates/graphics-backend/src/turnip.rs`)

Android PC Environment strictly prohibits declaring Turnip "working" simply because `libvulkan_freedreno.so` exists on disk. `TurnipBackend` enforces 6 explicit stages:

| Stage | Meaning | Verification Criteria |
|---|---|---|
| `not_installed` | ICD library or `freedreno_icd.aarch64.json` missing | File absence check in `<artifacts>/graphics/turnip/` and guest `/usr/share/vulkan/icd.d/` |
| `installed` | ICD `.so` and JSON manifest staged on disk | Verified file presence only (hardware NOT confirmed) |
| `detected` | `/dev/kgsl-3d0` device node accessible without root | Non-root `open(O_RDWR)` / metadata check on `/dev/kgsl-3d0` |
| `initialized` | Vulkan loader enumerates a Turnip physical device | Guest `vulkaninfo` / Vulkan probe reports `Turnip Adreno` + Vulkan API version |
| `operational` | Live Vulkan device creation & extension query succeeded | Confirmed Vulkan 1.3+ and verified extension set from live probe |
| `active_in_app` | Actively bound by a running Windows app session | Live session tracking in `EngineCore` |

## 4. Non-Root Android & PRoot Constraints

- **SELinux & KGSL Access**: On stock non-rooted Android 16 (Xiaomi HyperOS / Pad 7), regular app UIDs (`untrusted_app`) typically retain `read`/`write`/`ioctl` access to `/dev/kgsl-3d0` because Android's own HWUI/GLES/Vulkan stack uses it. However, PRoot's syscall tracing (`ptrace`) and bind-mount setup must bind `/dev/kgsl-3d0` into the guest `/dev` cleanly.
- **No `SYS_ADMIN` or Vendor Partition Modification**: Turnip runs strictly in userspace inside the app's private artifact directory and guest rootfs.
- **Zink & Xvnc Interaction**: Because our display server (`Xvnc` / TigerVNC inside PRoot) renders into an in-memory X11 framebuffer without kernel DRM/KMS, Turnip must be compiled with `-Dfreedreno-kmds=kgsl` and `x11` / `xcb` WSI using software present / `MIT-SHM` fallback (`TU_DEBUG=noconform` optional, configurable via UI).

## 5. Confirmed Facts vs. Unverified Hardware Unknowns

### Confirmed
- Mesa `26.3.1` (MIT) is the pinned upstream release with Adreno `a7xx` Turnip support.
- `TurnipBackend` in `rust/crates/graphics-backend/src/turnip.rs` detects ICD presence, `/dev/kgsl-3d0`, configures `VK_ICD_FILENAMES`, `VK_DRIVER_FILES`, `TU_DEBUG`, and optional Zink overrides, and cleanly falls back to Software (`llvmpipe`) whenever Turnip is not `Operational`.

### Unknowns / Pending Physical Xiaomi Pad 7 Validation (`NOT RUN`)
- Exact Adreno 732 (`a732` / `a7xx`) device ID recognition in Mesa `26.3.1` on Xiaomi Pad 7's specific Android 16 KGSL kernel ioctl version.
- Whether `VK_KHR_xcb_surface` presentation into TigerVNC (`Xvnc`) over `libandroid-shmem` or pure X11 PutImage exhibits tearing, synchronization stalls, or requires `VK_EXT_swapchain_maintenance1` workarounds.
- Whether all 6 VKD3D-Proton `v3.1` required Vulkan extensions pass live `vulkaninfo` enumeration on Adreno 732 without GPU hangs or shader compiler (`ir3`) miscompilations.
- Thermal throttling behavior under sustained Turnip + Box64 + Wine load on Snapdragon 7+ Gen 3.
