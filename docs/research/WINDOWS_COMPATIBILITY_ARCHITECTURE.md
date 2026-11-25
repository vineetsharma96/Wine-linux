# Windows Compatibility End-to-End Architecture (Phase 4)

*Last updated: 2026-11-25*
*Target Platform: ARM64 Android (Xiaomi Pad 7) — Rootless PRoot Ubuntu/Debian + Box64 + Wine + Graphics Backends*

## 1. Layered System Architecture

Android PC Environment preserves a strict architectural separation: **Rust** owns all execution orchestration, configuration, binary inspection, prefix isolation, graphics probing, dependency resolution, process supervision, and diagnostics; **Kotlin + Jetpack Compose** owns Android UI, Activity lifecycle, permissions, and notifications; **JNI** (`androidpc-android-bridge`) bridges the two with panic-safe JSON contracts.

```text
+-------------------------------------------------------------------------+
| Android UI Layer (Kotlin + Jetpack Compose — Star Plus Design)          |
| DashboardScreen | WindowsCompatibilityScreen | CompatibilityScreen      |
| DisplayViewportScreen | TerminalScreen | SettingsScreen                 |
+-------------------------------------------------------------------------+
| Kotlin Bridge (NativeBridge.kt + WindowsCompatibilityBridge.kt)         |
+-------------------------------------------------------------------------+
| JNI Boundary (rust/crates/android-bridge: lib.rs + windows_jni.rs)      |
+-------------------------------------------------------------------------+
| Rust Engine Core (rust/crates/core: lib.rs + windows_compat.rs)         |
|  +-- androidpc-linux-runtime (PRoot Ubuntu 26.04.1 / Debian 13.8)       |
|  +-- androidpc-box64         (ELF inspect, Box64 v0.4.4 routing)        |
|  +-- androidpc-wine          (Wine 11.0 WoW64, PE/MSI, Prefixes, Procs) |
|  +-- androidpc-graphics-backend (Turnip 26.3.1, VirGL 1.3.0, Software,  |
|  |                               DXVK 2.7.1, VKD3D-Proton 3.1 gating)   |
|  +-- androidpc-dependency-manager (DLL scan, catalog, conflict check)   |
|  +-- androidpc-windows-apps  (Profiles, .desktop shortcuts, app logs)   |
|  +-- androidpc-display       (Xvnc + RFB 003.008 + ANativeWindow)       |
|  +-- androidpc-audio         (Loopback TCP PCM sink 127.0.0.1:4713)     |
|  +-- androidpc-security      (SHA-256, Ed25519, Zip-Slip validation)    |
+-------------------------------------------------------------------------+
| Rootless Android Sandbox (u0_aXXX, PRoot 5.1.107.102, /dev/kgsl-3d0)    |
+-------------------------------------------------------------------------+
```

## 2. Windows Application Execution Flow

When a user launches a registered Windows application (`EngineCore::launch_windows_app`):

1. **Profile & Prefix Resolution**: `WindowsAppLibrary` loads the `WindowsAppProfile`; `WinePrefixManager` verifies the isolated `WINEPREFIX` directory exists and loads `prefix.json`.
2. **Binary Format Inspection**: `inspect_windows_binary` checks DOS `MZ` / PE32 (`i386`) / PE32+ (`x86_64`) / OLE2 (`.msi`) headers if the path is accessible on the host filesystem, rejecting malformed or unsupported architectures (e.g., Windows ARM64-only binaries on x86-64 Wine).
3. **Graphics & DX Translation Resolution**:
   - `GraphicsManager::resolve` probes the requested backend (`Turnip`, `VirGL`, or `Software`) and verifies its live readiness stage (`NotInstalled` through `Operational`).
   - If a hardware backend is not `Operational`, `GraphicsManager` automatically falls back to `Software` (`llvmpipe`) with a structured reason.
   - `DxTranslationManager::evaluate` checks whether DXVK (`v2.7.1`) or VKD3D-Proton (`v3.1`) is staged AND supported by the effective backend (including all 6 required Vulkan extensions for VKD3D). Otherwise it selects `WineD3D` OpenGL.
4. **Environment Assembly**:
   - Merges Box64 per-environment + per-app Dynarec/logging settings (rejecting any Android Bionic library paths or host `LD_PRELOAD`).
   - Merges isolated `WINEPREFIX`, `WINEARCH=win64`, `WINEDLLOVERRIDES`, `DISPLAY=:1`, `PULSE_SERVER=tcp:127.0.0.1:4713`, graphics ICD/Gallium env vars, and per-app custom environment variables.
5. **Scoped Command Construction & Tracking**:
   - Builds the PRoot guest invocation: `box64 <wine_bin> [explorer /desktop=App,<W>x<H>] [msiexec /i] <target> <args...>`.
   - Registers a `TrackedWindowsProcess` bound to `(app_id, prefix_id, environment_id)` with `/proc/<pid>/status` VmRSS monitoring when running, and per-app bounded log buffering (`max_lines = 400`).
6. **Clean Scoped Termination**:
   - Stopping an application issues `wineserver -k` (or `-k9` on force stop) **strictly scoped to that app's isolated `WINEPREFIX`**, preserving other prefixes and the underlying Ubuntu/Debian Linux environment and XFCE desktop.

## 3. Existing Systems Preserved Without Regression

- **Ubuntu 26.04.1 LTS & Debian 13.8 ARM64 PRoot environments** remain completely unchanged in lifecycle, APT management, PTY terminal, and backups.
- **Stage 3.2 `ANativeWindow` Display Engine + Xvnc/XFCE** continues to serve as the X11 display host for Windows applications (`DISPLAY=:1`), with existing Canvas fallback and optional Termux:X11 launch intact.
- **Zero Root Access**: Every component runs unprivileged inside the Android app sandbox.
- **No Proprietary Bundling**: Microsoft VC++ / `.NET` / DirectX redistributables are never bundled; the Dependency Manager uses official user-supplied installers or verified open-source artifacts (`wine-mono`, `DXVK`, `VKD3D-Proton`).
