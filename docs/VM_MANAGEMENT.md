# Virtual Machine Lifecycle & Management (`VM_MANAGEMENT.md`)

## 1. Persistent Directory Layout
Each VM is isolated under `<app_files_dir>/vms/<vm_id>/`:
```text
vms/<vm_id>/
  vm_profile.json          # Atomic-saved VmProfile (written via .part + rename)
  disk0.qcow2              # Primary virtual disk (or disk0.img for RAW)
  AAVMF_VARS.fd            # Optional per-VM UEFI NVRAM (or OVMF_VARS.fd)
  snapshots/
    <snap_id>.json         # Snapshot metadata + SHA-256 digest
    <snap_id>.disk         # Offline verified snapshot copy (when applicable)
```
On startup, `QemuSupervisor::reload_persisted()` scans `vms/`, removes interrupted `.part` staging files, loads `vm_profile.json` (with automatic backward-compatible migration from legacy `vm.json`), and resets persisted VMs safely to `Stopped`.

## 2. Lifecycle Operations
| Action | Mechanism | Process Safety |
|---|---|---|
| **Create** | Validates ID, RAM, vCPUs, disk size, and port rules; allocates non-colliding VNC (`5910+`) and QMP (`4444+`) ports; creates QCOW2 v3 or RAW disk atomically | Rejects duplicate IDs and invalid characters |
| **Start** | Spawns `qemu-system-aarch64` or `qemu-system-x86_64` via `DefaultProcessSupervisor`, records PID and `/proc/<pid>/stat` field 22 `starttime` ticks | Prevents double-start of an alive owned PID |
| **Pause / Resume** | QMP `stop` / QMP `cont` over `127.0.0.1:<qmp_port>` | Only permitted from `Running` / `Paused` |
| **Restart** | QMP `system_reset` | Non-destructive guest reboot |
| **Graceful Shutdown** | QMP `system_powerdown` -> bounded wait (`200ms..15000ms`) -> QMP `quit` -> `SIGTERM` -> `SIGKILL` | Checks `starttime` ticks before every signal (eliminates PID-reuse hazard) |
| **Force Stop** | Verified-PID `SIGKILL` | Immediate termination when guest hangs |
| **Clone** | Copies stopped VM disks via `.part` staging, assigns new VNC/QMP ports and timestamps | Blocks cloning while running/paused to avoid dirty disk copies |
| **Delete** | Requires explicit destructive confirmation dialog in Android UI; terminates owned PID if alive; deletes `vms/<vm_id>/` recursively | Reclaims disk and snapshot space |

## 3. UI Ergonomics
`VirtualMachinesScreen.kt` separates ordinary controls (Name, Arch, RAM/CPU/Disk sliders, Start/Pause/Shutdown/Display) from collapsible **Advanced Hardware Settings** (Firmware, SLIRP Network Mode, Display Backend, Audio Backend) inside `CreateVmDialog`, and organizes management into four tabs: **Machines**, **Storage & Snaps**, **Images**, and **Diagnostics**.
