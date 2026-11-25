# Virtualization Architecture (Phase 5)

## 1. Architectural Independence
Android PC Environment maintains **three cleanly separated execution engines** under the single Rust `EngineCore` (`rust/crates/core`):

| Subsystem | Engine | Primary Purpose | Isolation Model |
|---|---|---|---|
| **Linux Environments (Phase 1–3)** | Rootless PRoot (`androidpc-linux-runtime`) | ARM64 Ubuntu 26.04 & Debian 13 userspace + XFCE (`:1` / `5901`) | Userspace syscall translation (shares Android kernel) |
| **Windows Compatibility (Phase 4)** | Box64 `0.4.4` + Wine `11.0` + Turnip/VirGL/DXVK/VKD3D | Windows x86/x64 apps inside PRoot Linux | Wine prefixes inside PRoot |
| **Full Virtual Machines (Phase 5)** | **QEMU `11.1.3` (`androidpc-vm`)** | Full guest OSes with independent kernels (ARM64 & x86-64) | Full machine emulation / virtualization (`qemu-system-*`, VNC `5910+`, QMP `4444+`) |

Starting, pausing, crashing, or deleting a QEMU VM never stops or mutates PRoot Linux containers, XFCE `:1`, or Wine prefixes.

## 2. Rust Crate Structure (`rust/crates/vm`)
All virtualization orchestration lives in Rust; Kotlin is strictly limited to UI, Android lifecycle, and JNI forwarding:
- `config.rs`: Pinned versions (`QEMU 11.1.3`, `EDK2 edk2-stable202608`, `libslirp 4.9.7`), `VmProfile`, `CpuConfig`, `FirmwareConfig`, `StorageDeviceConfig`, `NetworkConfig`, `DisplayVmConfig`, `AudioVmConfig`, `BootConfig`, validation, and legacy `VmConfig` adapter.
- `qemu_engine.rs`: `QemuCommandBuilder` (hardened rootless CLI generation) and `QemuSupervisor` (lifecycle, persistence, cloning, bounded graceful-to-kill escalation with `/proc/<pid>/stat` start-time PID-reuse protection).
- `storage.rs`: `VmStorageManager` (pure-Rust QCOW2 v3 & sparse RAW disk creation, `qemu-img` integration, header inspection, grow-only resize, SHA-256 verified import/export, interrupted `.part` cleanup).
- `network.rs`: `VmNetworkBuilder` (rootless SLIRP NAT, isolated SLIRP `restrict=on`, loopback-only `hostfwd`, or disabled NIC).
- `display_audio.rs`: `VmDisplayAudioBuilder` (`virtio-gpu-pci`/`virtio-vga`/`ramfb`, localhost VNC `5910+`, `qemu-xhci` + `usb-tablet` + `usb-kbd`, headless serial, and modular audio).
- `monitor.rs`: `QmpClient` (JSON QMP over `127.0.0.1:<qmp_port>` for `query-status`, `stop`, `cont`, `system_powerdown`, `system_reset`, `quit`, and `human-monitor-command`).
- `snapshot.rs`: `VmSnapshotManager` (internal QCOW2 snapshots via QMP/`qemu-img` and offline SHA-256 verified copy snapshots).
- `images.rs`: `VmImageCatalog` (curated official upstream HTTPS guest image portals & checksum feeds).
- `diagnostics.rs`: `KvmCapabilities`, `AvfCapabilities`, `QemuEngineCapabilities`, `VmProcessTelemetry`, and bounded `VmLogBuffer`.

## 3. Core, JNI & Android Integration
- **Core facade**: `rust/crates/core/src/vm_engine.rs` + `"QEMU Virtual Machines"` health check in `rust/crates/core/src/health.rs`.
- **JNI bridge**: 10 panic-safe (`catch_unwind`) JSON JNI exports in `rust/crates/android-bridge/src/vm_jni.rs`, preserving all legacy Phase 1 `nativeCreateVm`/`nativeStartVm`/`nativeStopVm`/`nativeDeleteVm`/`nativeListVms`/`nativeProbeKvmSupport` exports.
- **Android UI**: `VmEngineBridge.kt` -> `VirtualMachinesScreen.kt` + `VmDialogsAndTabs.kt` -> reuses `DisplayViewportScreen.kt` (Stage 3.2 `ANativeWindow` RFB viewer) via `onOpenVmDisplay(vncPort)`.
