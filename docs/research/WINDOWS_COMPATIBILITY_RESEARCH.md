# Windows Compatibility Research (Phase 4 Preparation)

## Public Architecture Observations (Winlator / Wine / Box64)
- **CPU Translation**: `Box64` translates `x86_64` Linux/Wine calls to `aarch64` natively in userspace; `Hangover` / `Wine` WoW64 handles 32-bit/64-bit Windows PE execution.
- **Graphics Translation**: Direct3D 9/10/11 -> `DXVK` -> Vulkan (`Turnip` on Adreno, `VirGL`/`Zink` fallback); Direct3D 12 -> `VKD3D-Proton` -> Vulkan.
- **Display Coupling**: High-FPS Wine/DXVK requires shared-memory (`shmPath` / `AHardwareBuffer`) X11 presentation (`Lorie` / `SurfaceView`) rather than uncompressed VNC `Raw` polling.
- **Existing Repo Scaffolding**: `rust/crates/wine` and `android/app/src/main/assets/box64/` hold initial Phase 4 scaffolding to be audited before Phase 4 implementation.
