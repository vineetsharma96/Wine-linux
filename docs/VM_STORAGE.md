# Virtual Machine Storage & Snapshots (`VM_STORAGE.md`)

## 1. Supported Disk Formats & Buses
- **QCOW2 v3 (Default)**: Copy-on-write sparse format (`64 KiB` clusters) supporting internal snapshots and minimal initial host storage usage. `VmStorageManager::create_disk` uses staged `qemu-img create -f qcow2` when present, and falls back to `write_minimal_qcow2_v3()` (pure-Rust specification-compliant QCOW2 v3 header + refcount table + refcount block + L1 table) so valid QCOW2 disks can always be created atomically via `.part` staging.
- **RAW (Sparse)**: Thin-provisioned raw block files created via `set_len()`. Lower metadata overhead; relies on offline SHA-256 verified file snapshots.
- **Virtual Disk Buses**: `virtio_blk` (default, lowest overhead), `virtio_scsi` (multi-disk & ARM64 CD-ROM), `sata`, and `ide`.

## 2. Disk Operations & Safety Rules
- **Attach / Detach**: Secondary disks can be attached or detached only while the VM is stopped. Detaching the primary/only disk is blocked.
- **Grow-Only Resize**: `VmStorageManager::resize_disk` grows RAW images directly (`set_len`) or QCOW2 images via `qemu-img resize`. **Shrinking is explicitly blocked** to prevent silent guest filesystem corruption.
- **Verified Import / Export**:
  - `import_disk_image` validates `disk_id`, verifies optional SHA-256 checksum, inspects disk headers (`QCOW2_MAGIC` / size), and copies atomically via `.part` + `rename`.
  - `export_disk_image` copies atomically and writes a companion `.sha256` file.
- **Corruption & Integrity Inspection**: `inspect_disk` validates file presence, `QFI\xfb` magic, QCOW2 version (`2` or `3`), cluster bits (`9..=21`), virtual size, and actual on-disk bytes.

## 3. Snapshot Architecture (`rust/crates/vm/src/snapshot.rs`)
1. **Live QCOW2 Internal Snapshot**: When the VM is running/paused on a QCOW2 disk, `VmSnapshotManager` issues QMP `human-monitor-command` (`savevm`, `loadvm`, `delvm`).
2. **Offline `qemu-img` Internal Snapshot**: When stopped on QCOW2 with `qemu-img` staged, runs `qemu-img snapshot -c/-a/-d`.
3. **Offline Verified Copy Snapshot**: When on RAW disks (or when `qemu-img` is not yet staged), atomically copies the stopped primary disk to `snapshots/<snap_id>.disk`, stores its SHA-256 digest in `<snap_id>.json`, and re-verifies SHA-256 before any restore.
