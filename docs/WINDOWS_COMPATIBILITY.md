# Windows Compatibility Overview (Phase 4)

*Last updated: 2026-11-25*

Android PC Environment transforms its rootless ARM64 Ubuntu/Debian PRoot Linux environments into a modular Windows application compatibility platform using **Box64**, **Wine 11.0 (pure-64-bit WoW64)**, **Mesa Turnip / VirGL / Software graphics backends**, **DXVK / VKD3D-Proton (strictly gated)**, a **Windows Dependency Manager**, and a **Windows Application Manager**.

## 1. Component Stack & Pinned Versions

| Subsystem | Pinned Component | Version | License | Rust Crate / Module |
|---|---|---|---|---|
| x86-64 User-Space Translation | Box64 | `v0.4.4` | MIT | `rust/crates/box64` |
| Windows API Translation | Wine (x86-64 pure-64-bit WoW64) | `11.0` | LGPL-2.1-or-later | `rust/crates/wine` |
| Adreno Vulkan Driver | Mesa Freedreno Turnip | `26.3.1` | MIT | `rust/crates/graphics-backend/src/turnip.rs` |
| OpenGL IPC Driver | `virglrenderer` (`virpipe`/`vtest`) | `1.3.0` | MIT | `rust/crates/graphics-backend/src/virgl.rs` |
| Software Fallback | Mesa `llvmpipe` / `lavapipe` | `26.3.1` | MIT | `rust/crates/graphics-backend/src/software.rs` |
| D3D 8/9/10/11 → Vulkan | DXVK | `v2.7.1` | zlib | `rust/crates/graphics-backend/src/dx_translation.rs` |
| D3D 12 → Vulkan | VKD3D-Proton | `v3.1` | LGPL-2.1 | `rust/crates/graphics-backend/src/dx_translation.rs` |
| DLL Diagnostics & Catalog | Dependency Manager | In-tree | Project | `rust/crates/dependency-manager` |
| App Library & Shortcuts | Windows Application Manager | In-tree | Project | `rust/crates/windows-apps` |
| Core Orchestration | EngineCore Phase 4 | In-tree | Project | `rust/crates/core/src/windows_compat.rs` |
| JNI Bridge | Panic-safe JSON JNI exports | In-tree | Project | `rust/crates/android-bridge/src/windows_jni.rs` |

## 2. Module Documentation Index

- [Box64 Integration](BOX64_INTEGRATION.md)
- [Wine Integration](WINE_INTEGRATION.md)
- [Wine Prefix Management](WINE_PREFIX_MANAGEMENT.md)
- [Graphics Backends & DXVK/VKD3D Gating](GRAPHICS_BACKENDS.md)
- [Windows Application Manager](WINDOWS_APPLICATION_MANAGER.md)
- [Dependency Management](DEPENDENCY_MANAGEMENT.md)
- [Windows Process Lifecycle](PROCESS_LIFECYCLE.md)
- [Independent Phase 4 Test Plan](testing/PHASE_4_TEST_PLAN.md)

## 3. Android UI Entry Points

- **Dashboard (`DashboardScreen.kt`)**: Displays a unified **Windows Compatibility Engine** summary card showing Box64 status/version, Wine status/version, Prefix count, Installed Windows apps count, active Graphics backend & verified capabilities, active Windows processes, and any runtime notices.
- **Navigation Drawer (`MainActivity.kt`)**: Provides direct access to **Windows Apps**, **Wine & Prefixes**, **Graphics & DXVK**, and **Box64 Compatibility** alongside existing Containers, Desktop (Xvnc), Terminal, VMs, Artifacts, Backups, Health, Diagnostics, and Settings.
- **Windows Compatibility Screen (`WindowsCompatibilityScreen.kt`)**: Four tabs (`Apps`, `Wine & Prefixes`, `Graphics & DX`, `Deps & Diag`) backed by `WindowsCompatibilityBridge.kt` → `NativeBridge.kt` → Rust JNI.

## 4. Verification Status Notice

Phase 4 implementation is complete across the Rust engine, runtime manifests, build/benchmark scripts, Kotlin/Compose UI, and documentation. **Android APK compilation passed via `compile_applet`**, while `cargo test --workspace`, `cargo-ndk` ARM64 rebuild of `libandroidpc_engine.so`, cross-compilation of guest runtime artifacts (`scripts/build_phase4_stack.sh`), and physical **Xiaomi Pad 7** hardware validation are **`NOT RUN`** and handed off to the independent testing agent via [docs/testing/PHASE_4_TEST_PLAN.md](testing/PHASE_4_TEST_PLAN.md).
