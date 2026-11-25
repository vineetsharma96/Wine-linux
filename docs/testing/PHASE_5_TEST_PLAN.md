# Phase 5 Independent Testing Handoff Plan (`PHASE_5_TEST_PLAN.md`)

*Created: 2026-11-25*
*Target subsystem: Phase 5 — Full Virtual Machine Engine (`rust/crates/vm`, `rust/crates/core/src/vm_engine.rs`, `rust/crates/android-bridge/src/vm_jni.rs`, `VmEngineBridge.kt`, `VirtualMachinesScreen.kt`, `VmDialogsAndTabs.kt`)*
*Target hardware: Stock non-rooted ARM64 Android device (e.g., Xiaomi Pad 7 / Snapdragon 7+ Gen 3 / Android 16)*

> **HONESTY POLICY**: Every test case below that was not executed in this cloud build environment is explicitly marked **`NOT RUN`**. Do not mark any test `PASS` without real execution evidence (command output, exit code, `/proc` telemetry, or device logs).

---

## Pre-Test Staging Prerequisites (For Testing Agent)
1. **Rust workspace tests**: Run `cargo test --workspace` on a Rust `1.100.0+` host.
2. **ARM64 JNI rebuild**: Rebuild `app/src/main/jniLibs/arm64-v8a/libandroidpc_engine.so` via `cargo ndk -t arm64-v8a -o ../app/src/main/jniLibs build --release`.
3. **QEMU `11.1.3` + EDK2 staging**: Build or stage ARM64 `qemu-system-aarch64`, `qemu-system-x86_64`, `qemu-img` (`scripts/build_qemu_arm64.sh`), plus verified `AAVMF_CODE.fd` and `OVMF_CODE.fd` (`edk2-stable202608`) into app runtime artifacts.
4. **Official test ISOs**: Download verified Alpine Linux `3.21` ARM64 (`alpine-virt-*-aarch64.iso`) and x86-64 (`alpine-virt-*-x86_64.iso`) plus SHA-256 checksums from `https://alpinelinux.org/downloads/`.

---

## Test Matrix (All 16 Required Validation Areas)

| ID | Validation Domain | Verification Procedure & Acceptance Criteria | Current Status |
|---|---|---|---|
| **T1** | **Rust Engine & Build Verification** | Run `cargo test --workspace` (all 4 new `androidpc-vm` unit tests + existing workspace tests); rebuild `libandroidpc_engine.so` with `cargo-ndk`; build debug APK (`gradle :app:assembleDebug`). *(Note: `compile_applet` Android Kotlin/Gradle build PASSED on 2026-11-25; Rust host tests & `.so` rebuild remain `NOT RUN`.)* | **`NOT RUN`** (Rust / NDK) · **PASS** (Kotlin/Compose APK compile) |
| **T2** | **QEMU Installation & Binary Verification** | Stage `qemu-system-aarch64`, `qemu-system-x86_64`, `qemu-img` (`11.1.3`), and `AAVMF_CODE.fd`/`OVMF_CODE.fd`. Verify `VmEngineBridge.getCapabilities()` and `HealthStatusScreen` (`"QEMU Virtual Machines"`) accurately report `STAGED` vs `PENDING` without false positives. | **`NOT RUN`** |
| **T3** | **KVM / Acceleration Detection** | Verify `KvmCapabilities::probe()` on stock non-rooted Xiaomi Pad 7 reports `accessible = false`, `accelerator = "tcg"`, and `AvfCapabilities::report()` reports `ordinary_app_accessible = false` (`MANAGE_VIRTUAL_MACHINE @SystemApi`). Verify `QemuCommandBuilder` never passes `-accel kvm` for `x86_64` guests. | **`NOT RUN`** |
| **T4** | **VM Creation & Configuration Persistence** | Create both an ARM64 (`virt`, QCOW2, AAVMF) and an x86-64 (`q35`, RAW, OVMF) VM from `VirtualMachinesScreen`. Force-stop and relaunch app; verify `vm_profile.json`, disk files, non-colliding VNC (`5910+`) and QMP (`4444+`) ports persist cleanly. | **`NOT RUN`** |
| **T5** | **Virtual Disk Creation, Resize & Integrity** | Verify pure-Rust QCOW2 v3 header creation (`QFI\xfb`, v3, 64 KiB clusters) is accepted by `qemu-img check` and `qemu-system-aarch64`. Attach secondary disk, grow-resize RAW & QCOW2, verify shrink attempt is rejected, verify SHA-256 import rejects tampered image and cleans `.part` staging files. | **`NOT RUN`** |
| **T6** | **ARM64 Guest Boot Verification** | Boot official Alpine Linux `3.21` ARM64 ISO under `qemu-system-aarch64` (`-machine virt,gic-version=3 -cpu cortex-a72 -accel tcg,thread=multi`). Verify UEFI firmware boots kernel to login prompt and measure actual boot duration (do not estimate). | **`NOT RUN`** |
| **T7** | **x86-64 Guest Boot Verification** | Boot official Alpine Linux `3.21` x86-64 ISO under `qemu-system-x86_64` (`-machine q35 -cpu max -accel tcg,thread=multi`). Verify UI displays TCG emulation warning, guest boots cleanly, and record real boot time and CPU load. | **`NOT RUN`** |
| **T8** | **Display Rendering & Input Verification** | Tap **Display** on a running VM (`127.0.0.1:5910+`). Verify Stage 3.2 `ANativeWindow` RFB viewer renders guest framebuffer, `usb-tablet` touch/mouse clicks hit accurate guest coordinates, soft IME & physical keyboard type into guest, and disconnecting leaves VM running. | **`NOT RUN`** |
| **T9** | **Networking Verification (Rootless SLIRP)** | Inside booted guest with `user_slirp`, verify DHCP assigns `10.0.2.15`, DNS resolves via `10.0.2.3`, and outbound HTTPS (`apk update` / `wget`) works. Test `hostfwd=tcp:127.0.0.1:2222-:22` SSH access. Switch to `isolated_slirp` (`restrict=on`) and `disabled` (`-nic none`) and confirm outbound traffic is blocked. | **`NOT RUN`** |
| **T10** | **Audio Verification** | Verify default `Disabled` (`-audiodev none`) boots with zero audio errors. Test `WavDebug` writes a valid WAV file. Test `PulseLoopback` (`127.0.0.1:4713`) with `--audio-drv-list=pa` and record exact behavior/limitations. | **`NOT RUN`** |
| **T11** | **Lifecycle Controls & Process Cleanup** | Exercise `Start` -> `Pause` (QMP `stop`) -> `Resume` (QMP `cont`) -> `Restart` (QMP `system_reset`) -> `Graceful Shutdown` (QMP `system_powerdown`) -> `Force Stop` (verified-PID `SIGKILL`) -> `Clone` -> `Delete` (confirmation dialog). Confirm via `adb shell ps -A | grep qemu` that zero orphan QEMU processes remain. | **`NOT RUN`** |
| **T12** | **Snapshot Management** | Create, restore, and delete snapshots in both offline verified copy mode (SHA-256 checked) and QCOW2 internal mode (`savevm`/`loadvm`/`delvm` or `qemu-img snapshot`). Corrupt a snapshot backup byte and confirm SHA-256 verification blocks restore. | **`NOT RUN`** |
| **T13** | **Memory, Storage & Failure Recovery** | Test low-memory/high-RAM (`4096 MB`) allocation behavior under Android `lmkd`, verify `.part` files are cleaned up on restart after forced mid-operation kill, and confirm missing QEMU binary or corrupted disk transitions VM cleanly to `Error` state with actionable diagnostics. | **`NOT RUN`** |
| **T14** | **Android UI & Lifecycle Stress** | Rotate tablet, background/resume app during running VM, switch tabs (`Machines`, `Storage & Snaps`, `Images`, `Diagnostics`), confirm `BackHandler` and destructive delete confirmation prevent accidental loss, and verify ordinary vs advanced settings separation. | **`NOT RUN`** |
| **T15** | **Regression Verification (Phases 1–4)** | With Phase 5 installed and a VM configured/running on `:10` (`5910`), confirm PRoot Ubuntu 26.04 & Debian 13 terminals, XFCE desktop on `:1` (`5901`), Backups, and Phase 4 Box64/Wine/Graphics screens all continue operating with zero regressions. | **`NOT RUN`** |
| **T16** | **Security & Boundary Verification** | Verify via `ss -tlnp` / `netstat` that VNC (`5910+`), QMP (`4444+`), and SLIRP `hostfwd` bind strictly to `127.0.0.1`. Verify path-traversal IDs (`../escape`) and privileged host ports (`< 1024`) are rejected, and confirm no host folders are exposed into guests. | **`NOT RUN`** |

---

## Artifacts to Collect During Testing
- Full `cargo test --workspace` output.
- Exact `qemu-system-aarch64 --version` and `qemu-system-x86_64 --version` output.
- Measured ARM64 vs x86-64 TCG boot times, RSS (`VmRSS`), and thermal/CPU telemetry on Xiaomi Pad 7.
- `adb shell ps -A | grep -E "qemu|proot|Xvnc"` before and after VM shutdown and app force-stop.
