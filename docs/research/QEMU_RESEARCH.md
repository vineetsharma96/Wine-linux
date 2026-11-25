# QEMU Technical Investigation (`11.1.3`)

*Research date: 2026-11-25*
*Official sources: [qemu.org](https://www.qemu.org/), [QEMU GitLab](https://gitlab.com/qemu-project/qemu), [libslirp](https://gitlab.freedesktop.org/slirp/libslirp), [TianoCore EDK2](https://github.com/tianocore/edk2)*

## 1. Upstream Release & Licensing Selection
| Component | Pinned Version | Upstream Source | License | Role in Android PC Environment |
|---|---|---|---|---|
| **QEMU** | **`11.1.3`** | `https://download.qemu.org/qemu-11.1.3.tar.xz` | GPL-2.0-only (softmmu/tcg core) + LGPL-2.1 / MIT / BSD modules | System virtualization & emulation (`qemu-system-aarch64`, `qemu-system-x86_64`, `qemu-img`) |
| **libslirp** | **`4.9.7`** | `https://gitlab.freedesktop.org/slirp/libslirp` | BSD-3-Clause | Rootless user-mode TCP/UDP NAT networking (`-netdev user`) |
| **EDK2 UEFI** | **`edk2-stable202608`** | `https://github.com/tianocore/edk2` | BSD-2-Clause-Patent | Open-source ARM64 (`AAVMF_CODE.fd`) & x86-64 (`OVMF_CODE.fd`) UEFI firmware |

**Redistribution & Compliance Notes**:
- QEMU softmmu binaries are governed by GPL-2.0-only. Any binary distribution of `qemu-system-aarch64`, `qemu-system-x86_64`, or `qemu-img` must preserve complete corresponding source availability and license notices.
- No copyrighted guest OS ISOs or proprietary BIOS/UEFI blobs are bundled in the repository or APK.

## 2. System Emulation vs. User-Mode Emulation
- **System emulation (`qemu-system-*`)** emulates complete virtual hardware (CPU, GIC/APIC, PCIe bus, MMU, VirtIO block/net/GPU, UEFI/BIOS firmware) capable of booting a real guest kernel and full OS. This is the core of Phase 5 (`rust/crates/vm`).
- **User-mode emulation (`qemu-*-static`)** translates single userspace binaries and Linux syscalls without a guest kernel. Android PC Environment intentionally does **not** use QEMU user-mode emulation for x86-64 Linux/Windows apps because Phase 4 already uses **Box64 `v0.4.4`** with ARM64 Dynarec + native library wrapping inside PRoot.

## 3. Target Architectures on ARM64 Android Host
### ARM64 Guest (`qemu-system-aarch64`)
- **Machine model**: `virt,gic-version=3` (QEMU canonical ARM64 virtual platform with GICv3 interrupt controller and PCIe/VirtIO support).
- **CPU model**: `cortex-a72` under TCG software translation (`host` ONLY if `/dev/kvm` is actually R/W accessible without root).
- **Firmware**: EDK2 `AAVMF_CODE.fd` (read-only pflash) + per-VM writable `AAVMF_VARS.fd`, or `-kernel` / `-initrd` direct Linux kernel boot.
- **Devices**: `virtio-blk-pci` / `virtio-scsi-pci`, `virtio-net-pci`, `virtio-gpu-pci`, `qemu-xhci` + `usb-tablet` + `usb-kbd`.

### x86-64 Guest (`qemu-system-x86_64`)
- **Machine model**: `q35` (modern PCIe chipset with AHCI/VirtIO compatibility).
- **CPU model**: `max` under multi-threaded TCG.
- **Firmware**: EDK2 `OVMF_CODE.fd` / `OVMF_VARS.fd` (UEFI) or SeaBIOS (legacy PC BIOS).
- **Performance honesty**: An x86-64 guest on an ARM64 Android host **always** runs under TCG binary translation (never KVM). Cross-architecture system emulation incurs heavy translation overhead and cannot reach native ARM64 or PRoot/Box64 speeds.

## 4. Acceleration, TCG & Memory Considerations
- **Multi-threaded TCG (`-accel tcg,thread=multi,tb-size=128`)**: Default execution backend for rootless Android devices where `/dev/kvm` is absent or SELinux-denied. Bounds translation-block cache (`tb-size`) to reduce Android `lmkd` OOM pressure.
- **Opportunistic KVM (`-accel kvm`)**: Probed at runtime via read/write open of `/dev/kvm`. Enabled exclusively for `aarch64` guests when the user selects KVM and `/dev/kvm` is genuinely accessible; never assumed and never applied to `x86_64` guests on ARM64.
- **Guest RAM discipline**: Android devices enforce strict per-app memory limits (`lmkd`). VM RAM is validated (`256 MB ..= 16384 MB`) and defaults to conservative presets (`1024 MB`) to reduce OOM kills.

## 5. Control Plane (QMP), Display, Storage & Audio
- **QMP (`-qmp tcp:127.0.0.1:<port>,server,nowait`)**: Provides deterministic JSON control (`query-status`, `stop`, `cont`, `system_powerdown`, `system_reset`, `quit`, and `human-monitor-command` for internal QCOW2 `savevm`/`loadvm`/`delvm`).
- **Display (`-display vnc=127.0.0.1:<10+>`)**: Reuses the existing Stage 3.2 `androidpc-display` RFB `003.008` + `ANativeWindow` viewer on ports `5910+`, avoiding collision with PRoot XFCE `:1` (`5901`).
- **Storage (`QCOW2 v3` & `RAW`)**: `VmStorageManager` supports both `qemu-img` and pure-Rust QCOW2 v3 header creation, inspection, grow-only resize, SHA-256 import/export, and `.part` staging cleanup.
- **Audio**: Modular (`Disabled` default, `PulseLoopback` `tcp:127.0.0.1:4713` when QEMU is built with `--audio-drv-list=pa`, and `WavDebug`).
