# Android Virtualization Platform Research (KVM, AVF / pKVM & Rootless Constraints)

*Research date: 2026-11-25*
*Target platform: Stock non-rooted ARM64 Android (Android 14–16, including Xiaomi Pad 7 / Snapdragon 7+ Gen 3)*

## 1. Android Virtualization Framework (AVF / pKVM) Reality
Android 13+ introduced the Android Virtualization Framework (AVF) built on Linux protected KVM (`pKVM`) and `crosvm`. On Android 16, Google uses AVF internally to power the privileged system Linux Terminal app.

However, official Android platform security boundaries impose hard constraints on ordinary third-party apps:
1. **`android.permission.MANAGE_VIRTUAL_MACHINE` is `@SystemApi` / signature-privileged.** Ordinary Play Store / sideloaded apps running in the `untrusted_app` SELinux domain **cannot** hold this permission.
2. **No public unprivileged AVF hypervisor API exists** for ordinary third-party apps to boot arbitrary user-supplied guest OS images (Alpine, Debian, Ubuntu, x86-64 ISOs).
3. Consequently, `androidpc-vm::AvfCapabilities::report()` explicitly reports `ordinary_app_accessible = false` so users and tests receive an accurate technical explanation rather than misleading claims.

## 2. `/dev/kvm` Accessibility on Stock Android Devices
On most stock, non-rooted consumer Android devices (including Xiaomi Pad 7):
- Either `/dev/kvm` is not compiled/exposed by the vendor GKI kernel, **or**
- `/dev/kvm` exists for system/pKVM usage, but Android SELinux policy denies `open(O_RDWR)` to `untrusted_app` processes (`EACCES`).
- Android PC Environment **never** assumes `/dev/kvm` is accessible, never requests root (`su`), and never attempts to weaken SELinux. Instead, `KvmCapabilities::probe()` checks existence and actual R/W open capability at runtime and falls back automatically to QEMU multi-threaded TCG.

## 3. Rootless Execution & Android Process Constraints
Running QEMU inside an ordinary Android application UID introduces specific engineering constraints:
- **SELinux `W^X` / executable staging**: Binaries must be executed from executable storage (`nativeLibraryDir` or app-private executable runtime directories allowed by targetSdk/device policy).
- **Memory & `lmkd` (Low Memory Killer Daemon)**: Android aggressively terminates background or high-RSS processes. QEMU's `-m` guest RAM allocation plus TCG translation buffer (`tb-size`) counts directly against the app's memory footprint. Running multiple large VMs simultaneously risks Android OOM termination.
- **Background lifecycle**: Long-running VMs should be accompanied by an Android Foreground Service (`LinuxRuntimeService`) to reduce background throttling while respecting Android power management.
- **No `CAP_NET_ADMIN` / no `/dev/net/tun`**: TAP devices, Linux bridges, and raw sockets (including unprivileged ICMP ping) are unavailable without root. All VM networking uses QEMU user-mode networking (`libslirp 4.9.7`) with loopback-only (`127.0.0.1`) port forwarding on unprivileged host ports (`>= 1024`).

## 4. Reference Implementations Reviewed (Lawful Public Sources)
- **QEMU upstream (`gitlab.com/qemu-project/qemu`)**: Canonical `virt` ARM64 machine, `q35` x86-64 machine, multi-threaded TCG (`tcg,thread=multi`), QMP JSON protocol, QCOW2 v3 specification.
- **Termux QEMU packages (`termux/termux-packages`)**: Confirmed practical Android userspace QEMU compilation flags (`--enable-tcg`, `--enable-slirp`, `--enable-vnc`, headless/VNC display, no GTK/SDL/Xen) and confirmed TCG-only execution on stock Android devices.
- **Limbo / Vectras / UTM (public open-source design patterns)**: Confirmed best practices for separating ordinary VM creation controls from advanced hardware/firmware/network toggles, using `virtio-gpu-pci` + `usb-tablet` for touchscreen pointer accuracy over VNC, and keeping guest disk images inside app-private storage.
