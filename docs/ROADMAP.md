# Development Roadmap — Android PC Environment

## Phase 0: Rust-First Project Foundation (Completed)
- [x] Development environment audit & toolchain configuration.
- [x] Rust workspace initialization (`core`, `runtime-api`, `configuration`, `logging`, `android-bridge`).
- [x] Runtime abstraction traits (`RuntimeProvider`, `LinuxDistribution`, `EnvironmentManager`, `ProcessSupervisor`, etc.).
- [x] Panic-safe JNI bridge with `catch_unwind` and structured JSON serialization.
- [x] Jetpack Compose modern dark interface (Dashboard, Health Check, Diagnostics, Onboarding, Splash).
- [x] ARM64 cross-compilation pipeline with `cargo-ndk` + NDK r26b.
- [x] Hardware verification on physical Android 16 / Linux 6.1 GKI device (`aarch64`).

## Phase 1: Rootless Linux Runtime Implementation (Next Milestone)
- [ ] PRoot integration and customized Android-safe binary packaging.
- [ ] Automated download and SHA-256 verification of Ubuntu 24.04 and Debian 12 ARM64 root filesystems.
- [ ] Safe archive extraction with path traversal protection (zip-slip defense).
- [ ] Process supervisor with signal propagation (SIGINT, SIGTERM, SIGHUP).
- [ ] PTY creation and pseudo-terminal allocation inside the Android sandbox.

## Phase 2: Display & Desktop Environment Integration
- [ ] TigerVNC (Xvnc) server lifecycle management inside guest userspace.
- [ ] Custom integrated Android VNC client / RFB protocol renderer with touch-to-mouse translation.
- [ ] XFCE 4 desktop session configuration and auto-startup scripts.
- [ ] Optional Termux:X11 display bridge for low-latency hardware acceleration.

## Phase 3: Hardware Acceleration & Audio
- [ ] VirGL / Turnip / Zink hardware acceleration layer.
- [ ] PulseAudio server integration with OpenSL ES / AAudio bridge for low-latency sound.
- [ ] Android input handling (multi-touch, physical keyboard shortcuts, mouse scroll wheel).

## Phase 4: Windows Application Compatibility (Future)
- [ ] Box64 userspace emulation for x86_64 Windows binaries on ARM64.
- [ ] Wine / Proton integration inside the rootless Linux guest.
- [ ] DXVK / VKD3D Direct3D-to-Vulkan translation.

## Phase 5: Virtual Machine Virtualization (In Progress — Core & UI Implemented)
- [x] QEMU crate (`androidpc-vm`) with `aarch64` (`virt`) & `x86_64` (`q35`) command builder, VirtIO disk/network, VNC & QMP integration.
- [x] `/dev/kvm` capability probe with automatic rootless TCG fallback.
- [x] `VirtualMachinesScreen.kt` UI & `DisplayViewportScreen` VNC port routing.
- [ ] Standalone `qemu-system-aarch64` / `qemu-system-x86_64` binary packaging.
