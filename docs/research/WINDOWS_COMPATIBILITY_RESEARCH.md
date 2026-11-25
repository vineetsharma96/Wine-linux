# Windows Compatibility Research (Phase 4 Preparation)

## Public Architecture Observations (Winlator / Wine / Box64)
- **CPU Translation**: `Box64` translates `x86_64` Linux/Wine calls to `aarch64` natively in userspace; `Hangover` / `Wine` WoW64 handles 32-bit/64-bit Windows PE execution.
- **Graphics Translation**: Direct3D 9/10/11 -> `DXVK` -> Vulkan (`Turnip` on Adreno, `VirGL`/`Zink` fallback); Direct3D 12 -> `VKD3D-Proton` -> Vulkan.
- **Display Coupling**: High-FPS Wine/DXVK requires shared-memory (`shmPath` / `AHardwareBuffer`) X11 presentation (`Lorie` / `SurfaceView`) rather than uncompressed VNC `Raw` polling.
- **Phase 4.1 Foundation (Box64 Linux x86-64 Compatibility)**: Implemented `rust/crates/box64` (`androidpc-box64`) pinning Box64 `v0.4.4` (MIT), pure-Rust ELF inspection & routing, per-environment `Box64EnvironmentConfig` with conservative defaults, reproducible `aarch64-linux-gnu` glibc cross-compilation (`scripts/build_box64_arm64.sh`), and `app/src/main/assets/box64/default.box64rc`. See [BOX64_RESEARCH.md](BOX64_RESEARCH.md), [ARM64_X86_COMPATIBILITY.md](ARM64_X86_COMPATIBILITY.md), [BOX64_ANDROID_BUILD.md](BOX64_ANDROID_BUILD.md), and [BOX64_PERFORMANCE.md](BOX64_PERFORMANCE.md).
- **Phase 4.2 Gate (Wine)**: Existing `rust/crates/wine` scaffolding remains untouched. Wine integration MUST NOT begin until Phase 4.1 Box64 Categories A–D acceptance tests and Xiaomi Pad 7 benchmarks pass on physical ARM64 hardware.

