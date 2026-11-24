# Changelog

All notable changes to the **Android PC Environment** project will be documented in this file.

## [0.6.0-alpha.0] - 2026-11-24
### Phase 5: Windows Application Compatibility (Wine & Box64 Subsystem)
- **Wine Crate (`androidpc-wine`)**: Created dedicated crate implementing `WineConfig`, `Box64Config`, `WineArch` (Win32, Win64, WoW64), `WineSessionState`, and `WineStatusInfo`.
- **Box64 Dynarec Acceleration Integration**: Added environment variable resolution for Box64 dynamic recompilation (`BOX64_DYNAREC=1`, `BOX64_FASTNAN=1`, `BOX64_FASTROUND=1`, `BOX64_BOGUS_AVX=1`, `BOX64_PREFER_WRAPPED=1`, `BOX64_SAFEFLAGS=1`, `BOX64_EMULATED_LIBS=libpulse.so.0:libvulkan.so.1`).
- **Wine Command Launcher (`WineCommandBuilder`)**: Implemented command generation for `wineboot -u` (prefix initialization), Windows PE binary launcher (`box64 wine <exe_path> <args...>` or native wine), `wineserver -k` process termination, `winecfg`, and `cmd.exe`.
- **Bidirectional Path Translation**: Implemented automatic path mapping between Windows paths (`C:\...` and `Z:\...`) and guest Unix rootfs paths (`/home/user/.wine/drive_c/...`).
- **Wine Process Supervisor (`WineProcessSupervisor`)**: Implemented session lifecycle management, active PID tracking, and prefix initialization verification.
- **Engine Core & JNI Bridge Integration**: Added `init_wine_prefix`, `build_wine_launch_command`, `terminate_wine_session`, and `get_wine_status` to `EngineCore` and exposed them via panic-safe JNI exports.
- **Android Dashboard UI**: Integrated "Windows Compatibility (Wine + Box64)" card with real-time subsystem status, architecture indicator, and prefix initialization controls.
- **Hardware Proof-of-Concept**: Verified all 11/11 hardware test suites on physical Xiaomi Redmi (Android 16 API 36 / Linux 6.1 GKI `aarch64`), validating wineboot generation, Box64 dynarec flags, path translation, and prefix lifecycle.

## [0.5.0-alpha.0] - 2026-11-24
### Phase 4: Full Desktop Session & Graphics Loop
- **RFB Streaming Client (`androidpc-display`)**: Implemented `RfbClient` supporting full RFC 6143 003.008 handshake negotiation, `SetEncodings` (Raw), and in-memory 32-bit RGBA pixel blitter (`Vec<u8>`).
- **Engine Core Graphics Engine**: Integrated `connect_display_client`, `get_framebuffer_slice`, `send_display_pointer`, `send_display_key`, and clean disconnection into `EngineCore`.
- **High-Performance JNI Pixel Transport**: Added direct memory region copy via `set_byte_array_region` for frame buffer transfers into Android Java byte arrays.
- **Android Viewport Live Rasterizer (`DisplayViewportScreen`)**: Created active 30 FPS coroutine loop copying decoded VNC frames into `android.graphics.Bitmap` and rendering via Compose `drawImage` with touch-to-mouse scaling.
- **Physical Device Proof-of-Concept**: Verified 10/10 hardware test suites on physical Xiaomi Redmi (Android 16 API 36 / Linux 6.1 GKI `aarch64`) including 3.68 MB decoded framebuffer slice verification.

## [0.4.0-alpha.0] - 2026-11-24
### Phase 3: Rootfs Artifact Pipeline & Guest Provisioning
- **Artifact Catalog (`androidpc-downloader`)**: Added verified catalog descriptors for Ubuntu 24.04 (Noble), Debian 12 (Bookworm), and PRoot ARM64 with precomputed SHA-256 hashes, file sizes, and upstream mirrors.
- **Rootfs Environment Lifecycle (`androidpc-linux-runtime`)**: Implemented automated rootfs provisioning including `/etc/resolv.conf` (DNS servers 8.8.8.8 and 1.1.1.1), `/etc/hosts`, `/etc/hostname`, world-writable `/tmp`, and home directory creation.
- **PRoot Command Hardening**: Added `--kill-on-exit`, `--link2symlink` (hardlink translation for Android storage), and `/dev/shm` emulation.
- **Engine Core Orchestration**: Added environment registry (`HashMap<String, ProotEnvironmentManager>`), dynamic status tracking, and guest command dispatching.
- **Interactive Android UI**: Built guided deployment flow in `CreateEnvironmentScreen.kt` with live stepper progress indicator, manifest verification card, and direct launch transition.
- **Physical Device Proof-of-Concept**: Verified environment directory provisioning, DNS/resolv.conf generation, artifact catalog lookup, and clean teardown on physical Android 16 hardware (`6e5b28dd`).

## [0.3.0-alpha.0] - 2026-11-24
### Phase 2: Display & Desktop Integration
- **Display Subsystem (`androidpc-display`)**: Implemented `XvncDisplayBackend` for rootless TigerVNC execution with loopback access control. Implemented RFC 6143 Remote Framebuffer (RFB 003.008) protocol packet serialization for Pointer events, Key events, and Framebuffer update requests. Added `TermuxX11Backend` for optional accelerated X11 server socket.
- **Terminal Subsystem (`androidpc-terminal`)**: Implemented Unix PTY allocator on Android Bionic (`posix_openpt`, `grantpt`, `unlockpt`) with slave node resolution (`ptsname_r`), non-blocking I/O, and window resize ioctl (`TIOCSWINSZ`).
- **Android Viewport UI (`DisplayViewportScreen`)**: Created Jetpack Compose hardware viewport with touch-to-mouse translation (tap-to-click, long-press right-click, drag) and floating special-keys toolbar (Esc, Tab, Ctrl, Alt, Super, Enter, arrows) with RFB packet construction.
- **JNI Native Bridge**: Added 10 new JNI bridge functions for display server lifecycle, PTY terminal creation/read/write/resize/close, and RFB packet generation.
- **Physical Device Proof-of-Concept**: Verified RFB packet encoding, Unix PTY allocation (`/dev/pts/0` with master fd 3), resize ioctl, and Xvnc CLI generator on connected Xiaomi Redmi (Android 16 API 36 / Linux 6.1 GKI `aarch64`) hardware.

## [0.2.0-alpha.0] - 2026-11-24
### Phase 1: Rootless Linux Runtime Implementation
- **Downloader Subsystem (`androidpc-downloader`)**: Added streaming SHA-256 cryptographic verification and atomic `.part` download manager.
- **Filesystem Security Subsystem (`androidpc-filesystem`)**: Implemented strict Zip-Slip and path-traversal prevention for rootfs unpacking. Added safe `tar.gz` and `tar` extractor.
- **Process Supervisor Subsystem (`androidpc-process`)**: Implemented subprocess runner with signal dispatching (`SIGINT`, `SIGTERM`, `SIGHUP`) and child process tracking.
- **Rootless Virtualization (`androidpc-linux-runtime`)**: Implemented PRoot command generator for guest user-namespace mounting (`-r`, `-0`, `-b /dev`, `-b /proc`, `-b /sys`).
- **Physical Device Proof-of-Concept**: Successfully deployed and executed test suite on connected Android 16 (`aarch64`) hardware via ADB, verifying SHA-256 computation, path traversal rejection, archive unpacking, and process supervision on real silicon.

## [0.1.0-alpha.0] - 2026-11-24
### Phase 0: Rust-First Project Foundation
- Initialized Cargo workspace with `core`, `runtime-api`, `configuration`, `logging`, and `android-bridge`.
- Implemented panic-safe JNI bridge with `catch_unwind`.
- Scaffolded Jetpack Compose modern dark interface.
- Verified cross-compilation with `cargo-ndk` targeting `aarch64-linux-android`.
