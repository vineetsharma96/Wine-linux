# Development Roadmap — Android PC Environment

## Phase 0: Rust-First Project Foundation (Completed)
- [x] Development environment audit & toolchain configuration.
- [x] Rust workspace initialization (`core`, `runtime-api`, `configuration`, `logging`, `android-bridge`).
- [x] Runtime abstraction traits (`RuntimeProvider`, `LinuxDistribution`, `EnvironmentManager`, `ProcessSupervisor`, etc.).
- [x] Panic-safe JNI bridge with `catch_unwind` and structured JSON serialization.
- [x] Jetpack Compose modern dark interface (Dashboard, Health Check, Diagnostics, Onboarding, Splash).
- [x] ARM64 cross-compilation pipeline with `cargo-ndk` + NDK r26b.
- [x] Hardware verification on physical Android 16 / Linux 6.1 GKI device (`aarch64`).

## Phase 1: Rootless Linux Runtime Implementation (Core Complete & Gate Verified)
- [x] PRoot integration and Android-safe `jniLibs` + `libtalloc.so.2` packaging (`docs/PROOT_COMPATIBILITY.md`).
- [x] Download and SHA-256 verification of Ubuntu 24.04.5 LTS and Debian 12 Bookworm ARM64 root filesystems.
- [x] Safe archive extraction (`.tar`, `.tar.gz`, `.tar.xz`) with path traversal protection and Android hardlink fallback.
- [x] Process supervisor with signal propagation (`SIGINT`, `SIGTERM`, `SIGHUP`).
- [x] PTY creation and interactive shell attachment (`DefaultTerminalSession::spawn_shell` + `clearenv()`).

## Phase 2: Display & Desktop Environment Integration (Core Complete)
- [x] TigerVNC (`Xvnc`) server lifecycle management with `LD_LIBRARY_PATH` resolution.
- [x] Custom integrated Android VNC client / RFB 003.008 renderer with touch-to-mouse & keyboard input.
- [x] XFCE 4 desktop session launcher (`startxfce.sh`) and viewport auto-launch hook.
- [x] Optional Termux:X11 socket backend (`TermuxX11Backend`).

## Phase 3: Hardware Acceleration & Audio
- [x] VirGL / Turnip / Zink hardware acceleration environment layer (`GpuAccelerationMode` + `SettingsScreen.kt`).
- [x] Audio TCP sink (`127.0.0.1:4713`) with Android `AudioTrack` bridge (`AudioBridgePlayer`).
- [x] Android input handling (touch-to-mouse, drag, virtual keyboard Ctrl/Alt modifiers).

## Phase 4: Windows Application Compatibility (Core Complete)
- [x] Box64 userspace emulation (`default.box64rc` + Dynarec env builder) for x86_64 Windows binaries on ARM64.
- [x] Wine prefix lifecycle (`WineCommandBuilder`, `WineProcessSupervisor`, `DashboardScreen` controls).
- [x] GPU acceleration (`LLVMpipe` / `VirGL` / `Turnip+Zink`) env injection into Wine & Box64.

## Phase 5: Virtual Machine Virtualization (In Progress — Core & UI Implemented)
- [x] QEMU crate (`androidpc-vm`) with `aarch64` (`virt`) & `x86_64` (`q35`) command builder, VirtIO disk/network, VNC & QMP integration.
- [x] `/dev/kvm` capability probe with automatic rootless TCG fallback.
- [x] `VirtualMachinesScreen.kt` UI & `DisplayViewportScreen` VNC port routing.
- [ ] Standalone `qemu-system-aarch64` / `qemu-system-x86_64` binary packaging.
