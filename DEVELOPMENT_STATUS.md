# Development Status — Android PC Environment

*Last updated: 2026-11-25*
*Current work: **Phase 5 — Full Virtual Machine Engine (QEMU `11.1.3` + `libslirp 4.9.7` + EDK2 `edk2-stable202608`) implemented** across `rust/crates/vm`, `rust/crates/core/src/vm_engine.rs`, `rust/crates/android-bridge/src/vm_jni.rs`, `rust/crates/downloader`, runtime manifests, `scripts/build_qemu_arm64.sh`, `VmEngineBridge.kt`, `VirtualMachinesScreen.kt` + `VmDialogsAndTabs.kt` (`compile_applet` PASS), Phase 5 architecture/research/security docs, and independent testing handoff (`docs/testing/PHASE_5_TEST_PLAN.md`). Host `cargo test --workspace`, `cargo-ndk` rebuild of `libandroidpc_engine.so`, QEMU/EDK2 binary staging, local XERJ re-index, and all on-device VM boot/lifecycle/regression tests (T1–T16) are **`NOT RUN`** and handed off to the independent testing agent.*

## Phase 5 — Full Virtual Machine Engine (Implementation Complete; Device & Rust Testing `NOT RUN`)

- **Rust VM Engine (`rust/crates/vm`)**: Modularized into 10 files (`config.rs`, `error.rs`, `storage.rs`, `network.rs`, `display_audio.rs`, `monitor.rs`, `snapshot.rs`, `images.rs`, `diagnostics.rs`, `qemu_engine.rs`) while preserving all legacy Phase 1 APIs. Implements pinned **QEMU `11.1.3`**, **EDK2 `edk2-stable202608`** (AAVMF + OVMF), and **`libslirp 4.9.7`** profiles for ARM64 (`virt,gic-version=3`, `cortex-a72`) and x86-64 (`q35`, `max`, explicitly TCG-only on ARM64 hosts).
- **Honest Acceleration & Platform Probing**: `KvmCapabilities::probe()` checks `/dev/kvm` read/write accessibility without root; `AvfCapabilities::report()` honestly states `MANAGE_VIRTUAL_MACHINE` is `@SystemApi`-restricted and unavailable to ordinary third-party apps; multi-threaded TCG (`-accel tcg,thread=multi,tb-size=128`) is the primary rootless execution backend.
- **Storage, Snapshots, Networking, Display & Audio**: `VmStorageManager` creates specification-compliant QCOW2 v3 (pure Rust or `qemu-img`) and sparse RAW disks atomically, blocks unsafe shrinking, verifies SHA-256 on import/export, and cleans interrupted `.part` staging files. `VmSnapshotManager` supports QMP/`qemu-img` internal QCOW2 snapshots plus SHA-256 verified offline snapshots. `VmNetworkBuilder` enforces rootless SLIRP NAT/Isolated/Disabled modes with loopback-only `hostfwd` (`port >= 1024`). `VmDisplayAudioBuilder` connects localhost VNC (`5910+`) + `virtio-gpu-pci`/`virtio-vga` + `usb-tablet`/`usb-kbd` to the existing Stage 3.2 `ANativeWindow` viewer without colliding with XFCE `:1` (`5901`), and keeps modular audio `Disabled` by default (`PulseLoopback` / `WavDebug` optional).
- **Core, Downloader, JNI & Android Compose UI**: Added `rust/crates/core/src/vm_engine.rs`, `"QEMU Virtual Machines"` check in `health.rs`, pinned QEMU/EDK2/libslirp entries in `downloader/src/artifact.rs`, and 10 panic-safe JSON JNI exports in `rust/crates/android-bridge/src/vm_jni.rs`. Added `VmEngineBridge.kt` (graceful `UnsatisfiedLinkError` fallback without fabricating running VM state), `VirtualMachinesScreen.kt` (4 tabs: `Machines`, `Storage & Snaps`, `Images`, `Diagnostics`; full lifecycle controls Start/Pause/Resume/Restart/Graceful Shutdown/Force Stop/Clone/Display/Delete-with-confirmation), and `VmDialogsAndTabs.kt` (separating ordinary VM creation controls from collapsible Advanced Hardware settings). **Android APK build verified via `compile_applet` (PASS).**
- **Documentation & Independent Testing Handoff**: Added `docs/research/{QEMU_RESEARCH,ANDROID_VIRTUALIZATION_RESEARCH}.md`, `docs/{VIRTUALIZATION_ARCHITECTURE,QEMU_INTEGRATION,VM_MANAGEMENT,VM_STORAGE,VM_NETWORKING,VM_DISPLAY,VM_AUDIO,VM_SECURITY}.md`, updated `RESEARCH_INDEX.md`, `TECHNICAL_DECISIONS.md`, `VERSION_INVENTORY.md`, `XERJ_INTEGRATION.md`, and created the 16-area testing handoff at [docs/testing/PHASE_5_TEST_PLAN.md](docs/testing/PHASE_5_TEST_PLAN.md).
- **Explicitly `NOT RUN` (Handed Off to Independent Testing Agent)**: `cargo test --workspace`, `cargo-ndk` ARM64 rebuild of `app/src/main/jniLibs/arm64-v8a/libandroidpc_engine.so`, QEMU `11.1.3` + EDK2 firmware binary staging (`scripts/build_qemu_arm64.sh`), local Windows XERJ index refresh, and all T1–T16 physical device VM boot, display, networking, audio, snapshot, and regression tests.

## Phase 4 (4.2–4.10) — Complete Windows Compatibility Engine (Implementation Complete; Device Testing `NOT RUN`)

- **Wine 11.0 WoW64 Engine (`rust/crates/wine`)**: Preserved all legacy APIs and added pure-Rust PE32/PE32+/MSI binary inspection (`registry.rs`), `WineRegistryManager` (`WINEDLLOVERRIDES` + `reg add`), isolated `WinePrefixManager` (`prefix.rs` — create/configure/clone/SHA-256 backup/Zip-Slip-safe restore/repair/delete), `WineRuntimeManager` (`manager.rs` — pinned Wine `11.0` LGPL-2.1-or-later pure-64-bit WoW64 via Box64, virtual desktop launch, scoped `wineserver -k`/`-k9`), and `TrackedWindowsProcess` with `/proc/<pid>/status` VmRSS reading.
- **Modular Graphics Backend & DirectX Gating (`rust/crates/graphics-backend`)**: Implemented `GraphicsBackend` trait and 6-stage honest readiness model (`NotInstalled` → `ActiveInApp`) across **Mesa Turnip `26.3.1`** (MIT, Adreno `/dev/kgsl-3d0` + Zink), **VirGL `1.3.0`** (MIT, strictly OpenGL-only `virpipe`/`vtest`), and **Software (`llvmpipe`)** automatic fallback. Added `DxTranslationManager` gating **DXVK `v2.7.1`** (zlib) on live `Operational` Vulkan 1.3+ and **VKD3D-Proton `v3.1`** (LGPL-2.1) on all 6 mandatory Vulkan extensions, with automatic `WineD3D` OpenGL fallback.
- **Windows Dependency Manager (`rust/crates/dependency-manager`)**: Implemented missing DLL scanner (`system32`/`syswow64` + Wine log parsing), conflict detection (`wine-mono` vs `dotnet-desktop-8`), and auditable catalog separating SHA-256 verified open-source artifacts from user-supplied official installers for proprietary Microsoft runtimes (zero bundled proprietary binaries, zero `curl | sh`).
- **Windows Application Manager (`rust/crates/windows-apps`)**: Implemented per-app `WindowsAppProfile` CRUD, sanitized XFCE `.desktop` shortcut generation, and bounded credential-redacted per-app log buffers.
- **Core Orchestration, Downloader, JNI & Android Compose UI**: Wired all Phase 4 subsystems into `rust/crates/core/src/windows_compat.rs`, `health.rs` (`"Windows Compatibility & Graphics"`), `downloader/src/artifact.rs`, and 14 panic-safe JNI exports in `rust/crates/android-bridge/src/windows_jni.rs`. Added `WindowsCompatibilityBridge.kt` (with graceful `UnsatisfiedLinkError` fallback until `libandroidpc_engine.so` is rebuilt), `WindowsCompatibilityScreen.kt` (4 tabs: `Apps`, `Wine & Prefixes`, `Graphics & DX`, `Deps & Diag`), upgraded `DashboardScreen.kt` summary card, and updated `MainActivity.kt` drawer & routing. **Android build verified via `compile_applet` (PASS).**
- **Manifests, Scripts, Research & Independent Testing Handoff**: Added `/runtime/{wine,graphics,dxvk,vkd3d}/manifest.json`, `/scripts/build_phase4_stack.sh`, `/scripts/benchmark_phase4_device.sh`, 6 research documents (`WINE`, `TURNIP`, `VIRGL`, `DXVK`, `VKD3D`, `WINDOWS_COMPATIBILITY_ARCHITECTURE`), 8 operational docs (`WINDOWS_COMPATIBILITY`, `BOX64_INTEGRATION`, `WINE_INTEGRATION`, `WINE_PREFIX_MANAGEMENT`, `GRAPHICS_BACKENDS`, `WINDOWS_APPLICATION_MANAGER`, `DEPENDENCY_MANAGEMENT`, `PROCESS_LIFECYCLE`), updated `SECURITY.md`, `TECHNICAL_DECISIONS.md`, `VERSION_INVENTORY.md`, `XERJ_INTEGRATION.md`, and created the complete Sections A–L handoff at [docs/testing/PHASE_4_TEST_PLAN.md](docs/testing/PHASE_4_TEST_PLAN.md).
- **Explicitly `NOT RUN` (Handed Off to Independent Testing Agent)**: `cargo test --workspace`, `cargo-ndk` ARM64 rebuild of `app/src/main/jniLibs/arm64-v8a/libandroidpc_engine.so`, runtime artifact staging (`Box64`, `Wine 11.0`, `Turnip 26.3.1`, `virglrenderer 1.3.0`, `DXVK 2.7.1`, `VKD3D-Proton 3.1`), local Windows XERJ index refresh, and all physical Xiaomi Pad 7 hardware/graphics/app/regression tests (Sections A–L).

## Phase 4.1 — Box64 Integration & Star Plus UI Redesign

- **Star Plus UI Redesign**: Migrated Android Gradle project to repository root (`app/`, Gradle 9.3.1, AGP 9.1.1, Kotlin 2.2.21) and redesigned the Compose UI to match the Star Plus dark obsidian/starlight visual system (`Color.kt`, `Type.kt` with bundled Space Grotesk / Inter / JetBrains Mono fonts, `Theme.kt`, `SplashScreen.kt`, `ModalNavigationDrawer` in `MainActivity.kt`, container-centric `DashboardScreen.kt`, and dedicated `CompatibilityScreen.kt`) while preserving all working Ubuntu/Debian PRoot, Terminal, XFCE/Xvnc Display, Backups, VMs, and Artifacts screens without adding unrelated features.
- **Rust Box64 Compatibility Layer (`rust/crates/box64`)**: Implemented pure-Rust ELF32/ELF64 inspection (`elf.rs`), Android Bionic ABI isolation checks, `DT_NEEDED` guest library resolution (`resolver.rs`), per-environment `Box64EnvironmentConfig` with conservative defaults (`BOX64_DYNAREC=1`, `SAFEFLAGS=1`, `STRONGMEM=1`, `BIGBLOCK=1`, `BOX64_LOG=1`) and per-app overrides (`env.rs`), explicit ARM64-vs-x86-64 execution routing and bounded execution history (`router.rs`), and Categories A–D compatibility test suite (`suite.rs`). Integrated into `androidpc-configuration`, `androidpc-linux-runtime`, `androidpc-core` (including `"Box64 x86-64 Compatibility"` health check), `androidpc-downloader`, and `androidpc-android-bridge` (6 new JNI exports with `UnsatisfiedLinkError` fallback in `NativeBridge.kt` until `libandroidpc_engine.so` is rebuilt).
- **Reproducible Build & Test Harness**: Added `scripts/build_box64_arm64.sh` (cross-compiles pinned Box64 `v0.4.4` MIT for `aarch64-linux-gnu` glibc with `-DARM64=1 -DARM_DYNAREC=ON -DBAD_SIGNAL=ON -DBOX32=OFF -DCMAKE_BUILD_TYPE=RelWithDebInfo` and verifies non-Bionic ELF linkage), `scripts/build_box64_test_binaries.sh` (self-authored ARM64 & x86-64 `compat_probe` test binaries), and `app/src/main/assets/box64/default.box64rc`.
- **Documentation**: Added [BOX64_RESEARCH.md](docs/research/BOX64_RESEARCH.md), [ARM64_X86_COMPATIBILITY.md](docs/research/ARM64_X86_COMPATIBILITY.md), [BOX64_ANDROID_BUILD.md](docs/research/BOX64_ANDROID_BUILD.md), and [BOX64_PERFORMANCE.md](docs/research/BOX64_PERFORMANCE.md), and updated [VERSION_INVENTORY.md](docs/VERSION_INVENTORY.md), [TECHNICAL_DECISIONS.md](docs/TECHNICAL_DECISIONS.md), [WINDOWS_COMPATIBILITY_RESEARCH.md](docs/research/WINDOWS_COMPATIBILITY_RESEARCH.md), and [XERJ_INTEGRATION.md](docs/XERJ_INTEGRATION.md).
- **Pending Verification (No Unexecuted Claims)**: `cargo test --workspace`, `scripts/build_box64_arm64.sh`, `cargo-ndk` rebuild of `app/src/main/jniLibs/arm64-v8a/libandroidpc_engine.so`, local Windows XERJ index refresh, and physical Xiaomi Pad 7 Categories A–D acceptance tests & benchmarks have **not** yet been executed in this cloud container. Wine integration (Phase 4.2) remains gated until Box64 passes on hardware.

## Stage 3.2 renderer

The Rust presenter now copies dirty RFB generations directly into ANativeWindow. The Canvas presenter reuses the same connection and provides explicit selection and automatic fallback after a native surface failure. One incremental framebuffer request stays outstanding until the server sends an update. The final short paired trial presented 29.02 FPS on the direct path, 25.31 on the staged baseline and 24.63 on Canvas. An independent shorter comparison showed similar throughput for the baseline and direct prototype. The extended native run completed 120 seconds at 25.42 FPS. [Benchmark protocol and evidence](docs/research/DISPLAY_BENCHMARKS.md)

The tablet delivered Android touch, mouse and key actions to an X11 input probe. Home and resume recreated its BLAST layer with the same desktop process identities. The viewport resized from 1920×1080 to 1280×720 and returned to the dashboard through Exit Desktop. [Architecture decision](docs/research/DISPLAY_ARCHITECTURE_DECISION.md#stage-32-decision)

All 57 Rust workspace tests passed. The Android ARM64 native target and both APKs built. Android accepted the instrumentation APK and all four on-device tests passed, covering native image delivery, surface recreation, resize, desktop cleanup while a PTY remains live, fresh signed-catalog Ubuntu and Debian provisioning, and engine diagnostics.

Stage 3.2 implementation files include the [native presenter](rust/crates/android-bridge/src/native_display.rs), [RFB client](rust/crates/display/src/client.rs), [core framebuffer entry point](rust/crates/core/src/lib.rs), [viewport](android/app/src/main/java/com/androidpc/environment/ui/screens/DisplayViewportScreen.kt), [Kotlin bridge](android/app/src/main/java/com/androidpc/environment/bridge/NativeBridge.kt), [Android integration tests](android/app/src/androidTest/java/com/androidpc/environment/NativeEngineInstrumentationTest.kt), and [benchmark harness](scripts/measure_display.ps1).

## Stage 3.1 verification

The verified Ubuntu Base 26.04.1 ARM64 archive contained the x86 archive and security URLs and omitted a working default HTTPS trust path. Fresh provision and Repair migrate the known Ubuntu endpoints to HTTPS Ports based on the guest package architecture and release, keep source backups, and configure verified TLS for Ports. On Android 16, the fresh ARM64 Ubuntu guest recovered from a refused test proxy, fetched all four pockets with `apt-get update`, reported the Ports origin in `apt-cache policy`, and installed `hello 2.10-5build1`. Debian 13.8 passed `apt-get update` and its source file remained byte-identical to the original archive.

On the tablet, Disconnect left the same Xvnc and XFCE identities alive and reconnect reused the session. Exit Desktop removed the desktop group while the independent guest runtime survived. Android backgrounding preserved the session. Ending Xvnc made the viewport report disconnection. Freezing the owned PRoot tracer exercised bounded termination and left the independent Linux runtime alive. Stop Linux removed both the environment runtime and its desktop. Force-stopping the app cleared the observed Xvnc and XFCE processes, and a subsequent desktop launch and repeated Exit presses returned to the dashboard. The separate instrumentation APK built, but Android returned `INSTALL_FAILED_USER_RESTRICTED` when asked to install it. These hardware observations came from the ordinary app and ADB process inspection. [Stage 3.1 decisions and cleanup limits](docs/TECHNICAL_DECISIONS.md#stage-31)

All 56 Rust workspace tests passed on Windows. The Android ARM64 native build, debug app APK, and instrumentation APK compiled. The non-Android build no longer warned about the Android-only `Mutex` import.

The existing Ubuntu repair test reapplied the original x86 endpoints, ran Repair twice, and compared the resulting source and original backup bytes. Repair rejected both malformed and missing source data. Restoring the working file returned the guest to a repairable state.

An occupied loopback port produced a startup error. In a separate test, the owned PRoot tracer was frozen during startup and Exit Desktop was pressed while the viewport waited. The viewport returned to the dashboard and ADB found no remaining process in that startup group.

### Stage 3.1 change map

- Guest migration and install integration. [APT source parser and TLS bootstrap](rust/crates/linux-runtime/src/apt_sources.rs) · [Guest verification and repair](rust/crates/linux-runtime/src/environment.rs) · [Runtime module](rust/crates/linux-runtime/src/lib.rs)
- Desktop ownership and workspace integration. [Desktop supervisor](rust/crates/core/src/desktop.rs) · [Environment and display engine](rust/crates/core/src/lib.rs) · [Backup guard](rust/crates/core/src/backups.rs) · [Package operation guard](rust/crates/core/src/packages.rs) · [RFB connection lifecycle](rust/crates/display/src/client.rs)
- Android route, controls, and JNI. [Main activity](android/app/src/main/java/com/androidpc/environment/MainActivity.kt) · [Dashboard](android/app/src/main/java/com/androidpc/environment/ui/screens/DashboardScreen.kt) · [Viewport](android/app/src/main/java/com/androidpc/environment/ui/screens/DisplayViewportScreen.kt) · [Kotlin bridge](android/app/src/main/java/com/androidpc/environment/bridge/NativeBridge.kt) · [Endpoint model](android/app/src/main/java/com/androidpc/environment/bridge/Models.kt) · [Rust JNI bridge](rust/crates/android-bridge/src/lib.rs) · [Native presenter](rust/crates/android-bridge/src/native_display.rs)
- Rebuilt the app's ARM64 native library and added opt-in acceptance coverage. [Instrumentation source](android/app/src/androidTest/java/com/androidpc/environment/NativeEngineInstrumentationTest.kt) · [Version inventory](docs/VERSION_INVENTORY.md)

## Stage 3 verification (`ANativeWindow` Display Engine)

- **Architecture Decision (ADR-007)**: Selected **Architecture D (`ANativeWindow` native surface renderer)** in Rust (`rust/crates/android-bridge/src/native_display.rs`) with a dedicated `androidpc-anativewindow` presenter thread polling `RfbClient` and blitting directly into `ANativeWindow_Buffer` via `libandroid.so`. Rejected embedding `libXlorie` due to **GPL-3.0** license incompatibility, and kept `Termux:X11` as an optional external launch path.
- **Hardware Benchmark (`6e5b28dd`, Xiaomi Pad 7, Android 16 ARM64)**: At `1920x1080` (`2.25x` the `1280x720` baseline), measured `22.2%` App CPU (`-51.0%` vs `45.3%` baseline), `257.1 MiB` App RSS (`-15.3%`), `14` UI-thread `gfxinfo` frames with `0` missed Vsync (`-96.6%`), `171` `Xvnc` updates (`718` rects, `+137.5%`), and `23 fps` HUD during the 5-second drag workload. Dynamic resize (`1280x720`) and `IME` input were verified. Stage 3.1 addresses the desktop cleanup issue observed in this benchmark.

## Phase 2 verification

The Android 16 ARM64 debug app created **Phase Two Ubuntu** with a UUID and stored it under the Ubuntu registry directory. The user-facing Start action ran it, the PTY returned `aarch64`, and a file written in the guest was present after Stop, Restart and app process relaunch. Existing Ubuntu and Debian records remained visible. In Details, the stopped guest's `dpkg-query` ran with exit code 0 and reported `apt 3.2.0`, `base-files 14ubuntu6.2` and `bash 5.3-2ubuntu1`.

The same device saved an Ubuntu backup, restored it into a new UUID, and cloned the original into a third UUID. Both new guests passed the Rust guest boot/release check. The original, restored guest and clone kept the marker in three distinct file inodes. All 45 Rust tests and the ARM64 native, debug APK and test APK builds passed. RustSec reported no vulnerabilities in 108 locked crates. Outstanding coverage includes APT mutations and interruption on-device, low-storage and cancellation recovery, resource measurements under load, and the full preference set. See [environment management](docs/ENVIRONMENT_MANAGEMENT.md) and [backup and restore](docs/BACKUP_AND_RESTORE.md).

## Phase 1 evidence

The implementation includes later-phase code. The older milestone reports below describe the tests available at the time, including command builders and standalone probes. They do not establish Windows application launch, VM boot, or current clean-checkout packaging. See [Phase 1 testing](docs/TESTING.md) and the [version inventory](docs/VERSION_INVENTORY.md) for the current acceptance criteria and versions.

On 2026-11-25, the Android 16 ARM64 debug app used a test-signed catalog to download and install Ubuntu Base 26.04.1 and Debian 13.8. The Compose terminal ran `uname -m`, `cat /etc/os-release`, and `apt --version` inside both. Ubuntu returned `apt 3.2.0 (arm64)`, Debian returned `apt 3.0.4 (arm64)`, and both reported exit status 7 for `exit 7`. Both environments appeared after an APK update. Debian repair, restart and stop were checked from the dashboard. Earlier instrumentation verified a 96×34 resize and rejection of corrupt and duplicate installs with Ubuntu 24.04.5 and Debian 12. The newer instrumentation APK was built, but the phone declined its installation. The two latest distro results came from direct interaction with the installed app.

The updated dashboard reported both guests' file sizes, and the device terminal reported a layout-driven size of 180×44. The current Rust workspace passed 41 tests, RustSec found no vulnerabilities in the locked set, and the ARM64 NDK build and both debug APK builds passed. Production release signing requires a project key. Device checks for Android versions beyond 16, process-death recovery, interrupted HTTPS downloads, and later phases remain open.

---

## 1. Completed Milestones

| Milestone | Component | Status | Hardware Evidence |
|---|---|---|---|
| **Phase 0: Project Foundation** | Rust workspace, JNI bridge, Compose UI, NDK r26b setup | **DONE** | Tested & verified on hardware |
| **Downloader Crate** | Streaming SHA-256 integrity verification, atomic write `.part` | **DONE** | NIST vector & real-time hash verified |
| **Filesystem Crate** | Zip-Slip / Path Traversal security defense, archive unpacker | **DONE** | Blocked malicious traversal `../../system/bin/sh` on Android |
| **Process Supervisor Crate** | Process spawning, signal dispatch, process table tracking | **DONE** | Successfully spawned and supervised PID 2188 / 6205 / 9069 / 10189 on device |
| **Display Crate (`androidpc-display`)** | Xvnc backend, RFC 6143 RFB 003.008 protocol, Termux:X11 backend | **DONE** | Validated packet serialization & arg builder |
| **Terminal Crate (`androidpc-terminal`)** | Bionic Unix PTY (`posix_openpt`, `grantpt`, `unlockpt`, `TIOCSWINSZ`) | **DONE** | Validated `/dev/pts/0` allocation & resize on Android 16 |
| **RFB Streaming Engine** | Client stream receiver, 32-bit RGBA pixel blitter, input forwarder | **DONE** | Handshake & 3.68 MB frame slice verified on device |
| **Android Viewport UI** | Canvas touch-to-mouse translation & live Bitmap rasterizer loop | **DONE** | Integrated with `NativeBridge` & RFB 30 FPS sync |
| **Rootfs Provisioning Pipeline** | Manifest metadata, `/etc/resolv.conf`, `/etc/hosts`, user directories | **DONE** | Automated DNS & hosts provisioning verified on hardware |
| **Environment Management UI** | Stepper progress indicator with verified manifest details | **DONE** | Integrated in `CreateEnvironmentScreen.kt` |
| **Wine & Box64 Compatibility Crate (`androidpc-wine`)** | Box64 Dynarec env builder, WineArch, WineConfig, WineProcessSupervisor | **DONE** | Validated Box64 Dynarec flags & prefix lifecycle |
| **Wine Launcher & Path Translation** | `WineCommandBuilder` (wineboot, launch PE binary, wineserver -k, C:\ <-> Unix) | **DONE** | Validated bidirectional path mapping & command formatting |
| **Android Wine Dashboard Card** | Live `WineStatusData`, `Init C: Drive`, `Run .exe` modal dialog, `Kill` controls | **DONE** | Integrated in `DashboardScreen.kt` & `MainActivity.kt` |
| **Audio TCP Sink (`androidpc-audio`)** | `AudioTcpSink` (`127.0.0.1:4713`) + `AudioBridgePlayer` (`AudioTrack`) | **DONE** | Verified loopback PCM streaming on device (`6e5b28dd`) |
| **Live RFB Decoder & PTY Terminal UI** | `RfbClient::poll_updates` (Raw/CopyRect/DesktopSize), `TerminalScreen.kt`, `LinuxRuntimeService.kt` | **DONE** | Verified RFB live decoding & Bionic PTY on device |
| **JNI Bridge Extension** | Panic-safe `catch_unwind` JNI exports across all 13 Rust crates | **DONE** | Cross-compiled in `libandroidpc_engine.so` & APK built |
| **QEMU / KVM VM Crate (`androidpc-vm`)** | `QemuCommandBuilder` (aarch64/x86_64), `/dev/kvm` probe, `QemuSupervisor`, `VirtualMachinesScreen.kt` | **DONE** | Verified 3/3 VM unit tests & core VM lifecycle test |
| **PRoot & Download Hardening** | `LD_LIBRARY_PATH` `libtalloc.so.2` resolution + enforced SHA-256 verification before rootfs extraction | **DONE** | Wired into `ProotEnvironmentManager` & `CreateEnvironmentScreen.kt` |
| **Interactive Shell PTY & CI** | Non-blocking `poll()` PTY, `spawn_shell` (`forkpty` + PRoot guest shell), `ArtifactsScreen.kt`, `.github/workflows/ci.yml` | **DONE** | Verified workspace tests, ARM64 build, and CI pipeline |
| **Security, Networking & Instrumentation** | Dedicated `androidpc-security` & `androidpc-networking` crates, bundled `Xvnc`/`startxfce.sh` assets, `androidTest` suite | **DONE** | Verified workspace unit tests & Android instrumentation build |
| **Ubuntu 24.04 & Debian 12 Rootfs Pipeline** | Real Ubuntu 24.04.5 (`a91d5a93...`) + Debian 12 Bookworm `.tar.xz` (`3a841a79...`, `lzma-rs` + hardlink fallback) | **DONE** | Verified Ubuntu 24.04.5 LTS (`apt 2.8.3 (arm64)`, `/usr/bin/bash`) inside `u0_a768` sandbox on `uke` |
| **Phase 3: Hardware Graphics Acceleration** | `GpuAccelerationMode` (`LLVMpipe`, `VirGL`, `Turnip+Zink`), `Xvnc` `LD_LIBRARY_PATH` fix, `SettingsScreen` GPU selector | **DONE** | Verified GPU env builder tests & JNI integration |
| **Hardware Proof-of-Concept** | Executed Phase 1–6 test suites on physical Android 16 device (`6e5b28dd`) | **DONE** | **100% Passed (13/13) on real ARM64 hardware; 29/29 unit tests** |

---

## 2. Test Execution Logs

### Physical Hardware Execution (Xiaomi Redmi 2410CRP4CI `uke` / Android 16 / Linux 6.1 GKI / `aarch64`)
```text
============================================================
 ANDROID PC ENVIRONMENT — HARDWARE PHASE 1 RUNTIME TEST     
============================================================
Host Architecture: aarch64
Target OS:         android
2026-11-24T13:28:03.497877Z  INFO androidpc_core: Initializing Android PC Environment Engine v0.1.0-alpha.0
2026-11-24T13:28:03.498220Z  INFO androidpc_core: Engine initialization complete. Phase 1, 2, 3 & 4 Rootless Runtime, Display, Environment & Graphics Engine online.
[1/5] Engine Core v0.1.0-alpha.0 online.
[2/5] Testing SHA-256 cryptographic stream hashing...
      Calculated Hash: b37b23d65c321a5e1ff801910a5c706238ac8c14ed33c676fd81bf1f1e613ca4
      SHA-256 verification validated successfully.
[3/5] Testing Zip-Slip / Path Traversal defense...
      Malicious traversal '../../system/bin/sh' successfully BLOCKED.
[4/5] Testing archive unpacker with traversal protection...
2026-11-24T13:28:03.500079Z  INFO androidpc_filesystem::extractor: Successfully extracted 1 entries to /data/local/tmp/androidpc_sandbox/rootfs
      Safely unpacked 1 entries into target rootfs sandbox.
      Verified extracted content: "Ubuntu 24.04\n\0\0"
[5/5] Testing Process Supervisor & PRoot command builder...
      PRoot Generated Arguments: ["-r", "/data/local/tmp/androidpc_sandbox/files/environments/default-ubuntu/rootfs", "-0", "-b", "/dev", "-b", "/proc", "-b", "/sys", "-w", "/home/user", "/bin/uname", "-m"]
2026-11-24T13:28:03.503860Z  INFO androidpc_process::supervisor: Spawned process '/system/bin/sh' with PID 14258
      Process supervisor active: spawned child PID 14258
[6/8] Testing RFC 6143 RFB display protocol serialization...
      RFB pointer, keyboard, and frame-update packets correctly formatted.
[7/8] Testing Unix PTY subsystem (posix_openpt, grantpt, unlockpt)...
2026-11-24T13:28:03.504730Z  INFO androidpc_terminal::pty: Unix PTY session allocated successfully (master_fd: 3)
      Allocated PTY session id: term-device-0
      Slave PTY node: /dev/pts/0 (master fd: 3)
      PTY resize to 120x40 succeeded via TIOCSWINSZ.
[8/8] Testing Xvnc display backend command line builder...
      Generated Xvnc arguments: [":1", "-geometry", "1920x1080", "-depth", "24", "-dpi", "160", "-rfbport", "5901", "-SecurityTypes", "None", "-listen", "127.0.0.1", "-ac", "-pn"]
[9/9] Testing rootless environment provisioning & network setup...
2026-11-24T13:28:03.505655Z  INFO androidpc_linux_runtime::environment: Initializing rootless environment 'test-device-ubuntu'
2026-11-24T13:28:03.506343Z  INFO androidpc_linux_runtime::environment: Network and userspace configured for rootfs at "/data/local/tmp/androidpc_sandbox/files/environments/test-device-ubuntu/rootfs"
2026-11-24T13:28:03.506452Z  INFO androidpc_core: Environment 'test-device-ubuntu' provisioned successfully (ubuntu)
      Environment provisioned: ID='test-device-ubuntu', Distro='ubuntu'
      Environment state validated: Stopped
      DNS configuration (/etc/resolv.conf) verified: nameserver 8.8.8.8
nameserver 1.1.1.1
      Catalog lookup verified: ubuntu-base-24.04.1-base-arm64.tar.gz (31457280 bytes)
      Test sandbox cleaned up successfully.
[10/10] Testing RFB client stream receiver & pixel blitter...
2026-11-24T13:28:03.507597Z  INFO androidpc_display::client: Connecting RFB client to 127.0.0.1:5901
      RFB client display stream initialized: 1280x720
      Framebuffer slice decoded: 3686400 bytes (32-bit RGBA).
      Pointer & Key events dispatched through display client stream.
      Display stream cleanly disconnected.
[11/11] Testing Wine & Box64 Windows compatibility subsystem...
2026-11-24T13:28:03.523300Z  INFO androidpc_wine::process: Registered Wine session for environment 'test-wine-env'
      Wineboot initialization command & Box64 dynarec flags validated.
      Wine prefix status validated: Ready
      Windows binary launch command generated: ["wine", "C:\\Games\\Doom.exe", "-fullscreen"]
      Wineserver termination command validated.
      Windows/Unix path bidirection translation verified (C:\ <-> drive_c).
============================================================
 ALL PHASE 1, 2, 3, 4 & 5 HARDWARE TESTS PASSED: 100% OPERATIONAL
============================================================
```
```

### Workspace Unit Tests (`cargo test --workspace`)
```text
running 16 tests across 11 crates
test tests::test_environment_config_validation ... ok
test tests::test_engine_config_json_roundtrip ... ok
test tests::test_engine_config_default_and_validation ... ok
test tests::test_engine_lifecycle_and_version ... ok
test tests::test_registered_distributions ... ok
test tests::test_health_check_execution ... ok
test tests::test_diagnostics_collection ... ok
test rfb::tests::test_key_event_serialization ... ok
test rfb::tests::test_pointer_event_serialization ... ok
test rfb::tests::test_rfb_handshake_constants ... ok
test verifier::tests::test_sha256_known_vectors ... ok
test security::tests::test_path_traversal_detection ... ok
test tests::test_log_buffer_push_and_retrieval ... ok
test tests::test_architecture_properties ... ok
test tests::test_health_report_serialization ... ok
test pty::tests::test_terminal_session_lifecycle ... ok
test result: ok. 16 passed; 0 failed; 0 warnings
```

---

## 2b. Phase 2 Verification Summary (2026-11-25, Device `6e5b28dd` Android 16 ARM64)
- **Host Tests & Native Build**:
  - `cargo test --workspace`: **45 passed, 0 failed** across all workspace crates (including `core`, `backups`, `packages`, `environment_metrics`, `security`, `configuration`, `terminal`, `linux-runtime`, `android-bridge`).
  - `cargo ndk -t arm64-v8a -o ../android/app/src/main/jniLibs build --release` and `gradle assembleDebug` succeeded.
- **Milestone 1 (APT Lifecycle on Disposable Clone `Phase Two Ubuntu copy`, `d8457abc-5031-4015-be92-f68c69268506`)**:
  - Verified `Refresh index` (`apt-get update`, `Exit 0`, `Fetched 30.6 MB in 5s`), `Search` (`apt-cache search hello`, `Exit 0`), `Show details` (`apt-cache show hello`, `Exit 0`, `hello 2.10-5build1`), `Install packages` (`hello:arm64`, `Exit 0`), `Remove packages` (`hello:arm64`), `Configure pending packages` (`dpkg --configure -a`, `Exit 0`), and explicit `AlertDialog` confirmation before `Upgrade packages`.
  - Note: tapping `Cancel operation` during `nano` install returned `False` because the fast local install completed before the UI tap; `PackageJob` cancellation was directly verified via `Create backup` -> `Cancel operation`.
- **Milestone 2 (Resume, Cancel, Low-Storage & Kill Recovery)**:
  - Verified mid-download `am force-stop` preserved `cache/ubuntu-base-26.04.1-base-arm64.tar.gz.part` (`327680` bytes), kept all existing environments intact on reopen, and resumed via HTTP `Range` into `f37c8650-4da8-4044-91c6-42b9da84af73`.
  - Verified `Create backup` -> `Cancel operation` (`Subsystem failure: Operation cancelled`), leaving `files/backups` with only the original backup.
  - Low-storage checks: `RootfsDownloader.kt` checks `StatFs` available bytes against `expectedSizeBytes + reserveBytes`; `backups::tests::backup_round_trip_is_independent_and_corruption_blocks_restore` verifies SHA-256 corruption rejection and startup staging cleanup (`.backup-*`).
- **Milestone 3 (Preferences, Resource Metrics & Single-Release Catalog)**:
  - Persisted `default_env_id` and `default_shell` (`/bin/bash` / `/bin/sh`) in `SharedPreferences("androidpc_prefs", MODE_PRIVATE)` across `am force-stop` and reopen, wired to `SettingsScreen` and `TerminalScreen`.
  - Verified `EnvironmentDetailsScreen` resource readings on device:
    - Running: `Measured: 1 supervised PRoot & PTY (0 terminals); guest child processes excluded`, `Resident memory: 3 MiB`, `CPU time: 40 ms`, `Active runtime: 29s since launch`.
    - Stopped: `Measured: 0 supervised PRoot & PTY (0 terminals); guest child processes excluded`, `Resident memory: Unavailable`, `CPU time: Unavailable`, `Active runtime: Unavailable (stopped)`.
  - Verified `CreateEnvironmentScreen` single-release labels (`26.04.1 LTS • Single catalog release`, `13.8 • Single catalog release`, `Release: Ubuntu (single signed release in catalog)`).
- **Milestone 4 (Clean Packaging, Signing & Test Coverage)**:
  - Documented clean-checkout build and production signing inputs (`ANDROIDPC_ROOTFS_PUBLIC_KEY_HEX`, `catalog.json`/`catalog.sig`, release APK keystore) in [docs/SECURITY.md](docs/SECURITY.md). Current device APK is a debug build using a test-signed rootfs catalog.
  - Coverage note: the device previously declined installing a separate `androidTest` instrumentation APK, so on-device verification used direct app UI/ADB automation against `com.androidpc.environment.debug` alongside host `cargo test --workspace` unit/integration tests.

---

## 2c. Phase 3 Verification Summary (2026-11-25, Device `6e5b28dd` Android 16 ARM64)
- **Desktop Runtime & Vertical Slice**:
  - Installed `Minimal Desktop` profile (`xfce4-session` 4.20.4-1, `xfwm4` 4.20.0-1build1, `xfce4-panel` 4.20.7-1, `xfdesktop4` 4.20.1-1ubuntu2, `xfce4-terminal` 1.1.5-1, `dbus-x11` 1.16.2-2ubuntu4, `librsvg2-common` 2.61.3+dfsg-3ubuntu0.1, `tigervnc-standalone-server` 1.15.0+dfsg-2build1) in Ubuntu 26.04.1 (`d8457abc...`).
  - Added Rust display session supervisor ([desktop.rs](rust/crates/core/src/desktop.rs)) and declarative profiles ([profiles.rs](rust/crates/display/src/profiles.rs)), binding `<base_data_dir>/dtmp:/tmp` (keeping Unix socket paths `< 108` bytes) and adding a PRoot-compatible `glycin` `bwrap` shim.
  - Verified live XFCE 4.20 desktop rendering in Android `SurfaceView` (`Live XFCE [1280x720 RFB 003.008]`, 11 FPS measured), dock click launching `xfce4-terminal` (`root@localhost:/home/user#`), `BackHandler` disconnect confirmation, and clean `stop_desktop_session()` shutdown.
- **Protocol, Input, Clipboard, Termux:X11 & Audio**:
  - Hardened `RfbClient` ([client.rs](rust/crates/display/src/client.rs)) with dimension/size bounds (`<= 8192x8192`, `checked_mul`, `ServerCutText <= 1 MiB`), `ClientCutText`/`ServerCutText` clipboard sync, and unit tests for handshake, security rejection, oversized framebuffer rejection, clipboard round-trip, and `DesktopSize` resize (all workspace Rust tests passing in `task-2991`).
  - Added Direct Touch / Trackpad / Relative Pointer modes, soft `IME` + physical keyboard + modifier toolbar (`Ctrl`, `Alt`, `Shift`, `Super`, `F1..F11`), `xrandr` dynamic resize, `AudioBridgePlayer` mute toggle, and `Termux:X11` `<queries>` visibility + launch (`DashboardScreen.kt`).
  - Verified on `6e5b28dd`: `Backend B: Launch Termux:X11` launched `com.termux.x11/com.termux.x11.MainActivity`; `IME` typed `echo PHASE3_OK` into `xfce4-terminal`; `Touch` toggled to `Trackpad`; `1280x720` resized dynamically to `1600x900` (`Resized [1600x900] • 10fps`); Debian 13.8 (`env-debian`) installed `Minimal Desktop` via `Install XFCE` (`Exit 0`) and rendered live XFCE 4.20 (`Live XFCE [1280x720 RFB 003.008] • 11fps`).
  - **Remaining note & Performance Investigation**: Audio PCM forwarding (`AudioBridgePlayer` + `AudioTcpSink`) is wired and mute-toggled in UI; live guest PulseAudio audio-sink playback remains to be exercised with an audio-producing guest app. Display performance investigation and Stage 2 optimizations (`DirectByteBuffer` zero-alloc JNI, dirty-skip generation counter, 32bpp `rgb888` fast path, decoupled Compose HUD) on Xiaomi Pad 7 (`2410CRP4CI`, 120 Hz, Android 16) reduced app CPU from `45.3%` to `36.1%`, RSS from `303.7 MiB` to `251.2 MiB`, UI-thread `gfxinfo` frames from `416` (`15` missed Vsync) to `20` (`0` missed Vsync), and increased Xvnc updates from `72` to `101` in the 5-second drag benchmark. See [docs/PERFORMANCE_BASELINE.md](docs/PERFORMANCE_BASELINE.md) and [docs/DISPLAY_BENCHMARKS.md](docs/DISPLAY_BENCHMARKS.md).
- **Phase 3.5 (XERJ & Reverse-Engineering Infrastructure)**: Configured `.xerjignore` (excluding `android/app/src/main/assets/` and binary/secret paths), `.mcp.json` (`xerj.exe mcp` on `http://127.0.0.1:9200`), and created the permanent knowledge base under [docs/research/RESEARCH_INDEX.md](docs/research/RESEARCH_INDEX.md) and [docs/XERJ_INTEGRATION.md](docs/XERJ_INTEGRATION.md).

---

## 3. Next Implementation Milestone: Phase 5 (Production Hardening & Packaging)
1. **Packaging Standalone Runtime Artifacts**:
   - Compiling static `proot-arm64` and packaging into app assets / artifact cache.
   - Bundling Xvnc / TigerVNC binary with library dependencies.
2. **Audio & PulseAudio Integration**:
   - Setting up local PulseAudio TCP sink (`127.0.0.1:4713`) for Android AudioTrack forwarding.
3. **Multi-Architecture Support**:
   - Expanding target validation to x86_64 and ARMv7 architectures.
