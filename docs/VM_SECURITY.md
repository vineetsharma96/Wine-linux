# Virtual Machine Security & Trust Boundaries (`VM_SECURITY.md`)

## 1. Honest Security Boundary Statement
QEMU inside Android PC Environment runs strictly as an **unprivileged process under the Android app UID** (`untrusted_app` SELinux domain):
- **What QEMU provides**: Guest OS emulation isolation — guest kernel panics, package managers, and disk mutations are contained inside the VM's virtual hardware and `<app_files_dir>/vms/<vm_id>/` disk images.
- **What QEMU does NOT provide**: An absolute hardware/hypervisor security boundary against hostile guest-escape exploits. Because there is no privileged pKVM/AVF boundary available to ordinary third-party apps, a QEMU emulator bug executes with the same Android app UID privileges as the host app itself. **Do not treat a rootless Android QEMU VM as a hardened malware sandbox.**

## 2. Concrete Hardening Controls Implemented
1. **App-Private Storage Isolation**: All VM profiles, QCOW2/RAW disks, UEFI NVRAM (`AAVMF_VARS.fd` / `OVMF_VARS.fd`), and snapshots live strictly inside `<app_files_dir>/vms/<vm_id>/`. `validate_vm_id()` and `validate_snapshot_id()` allow only `1..=64` ASCII alphanumeric/dash/underscore characters (blocking path traversal `../`).
2. **No Default Host-Folder Sharing**: `QemuCommandBuilder` passes `-nodefaults` and `scripts/build_qemu_arm64.sh` configures `--enable-virtfs=no`. Host Android `/sdcard` or app data directories are never mounted into guests by default.
3. **Loopback-Only Management & Display**:
   - VNC binds strictly to `127.0.0.1:<5910+>` (never `0.0.0.0`).
   - QMP binds strictly to `127.0.0.1:<4444+>` (never exposed on LAN/Wi-Fi).
   - SLIRP `hostfwd` binds strictly to `127.0.0.1` on unprivileged host ports `>= 1024`.
4. **No Root / No SELinux Bypass**: The engine never invokes `su`, never requests privileged `MANAGE_VIRTUAL_MACHINE`, and never attempts to alter `/dev/kvm` or `/dev/net/tun` permissions.
5. **Verified Images & Integrity**:
   - No copyrighted OS ISOs or proprietary firmware are bundled.
   - `VmStorageManager::import_disk_image` verifies SHA-256 checksums and inspects disk headers before committing via atomic `.part` + `rename`.
   - Offline snapshots record and re-verify SHA-256 digests before restoring.
6. **PID-Reuse-Safe Process Termination**: Before sending `SIGTERM` or `SIGKILL`, `QemuSupervisor` verifies `/proc/<pid>/stat` field 22 (`starttime` ticks) matches the process spawned by the supervisor.
7. **Destructive Confirmation**: Deleting a VM in `VirtualMachinesScreen.kt` requires explicit confirmation in `ConfirmDeleteVmDialog`.
