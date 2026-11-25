# Wine Prefix Management (Phase 4.2)

*Last updated: 2026-11-25*

Every Windows application or group of related applications runs inside an **isolated Wine prefix** managed by `WinePrefixManager` (`rust/crates/wine/src/prefix.rs`). Prefixes never share `drive_c`, Windows registry hives (`system.reg`, `user.reg`, `userdef.reg`), DLL overrides, or `wineserver` sockets.

## 1. On-Disk Directory Structure

```text
<base_data_dir>/wine_prefixes/
  <prefix_id>/
    prefix.json                # WinePrefixMetadata (version, winver, overrides, gfx, deps)
    drive_c/
      Program Files/
      Program Files (x86)/
      users/user/
      windows/
        system32/              # 64-bit DLLs (including staged DXVK/VKD3D x64)
        syswow64/              # 32-bit DLLs (including staged DXVK/VKD3D x32)
    dosdevices/
      c: -> ../drive_c
```

Prefix IDs are validated strictly (`^[a-zA-Z0-9_-]{1,64}$`) to prevent path traversal (`..`, `/`, `\`).

## 2. Supported Lifecycle Operations

Exposed via `EngineCore` (`rust/crates/core/src/windows_compat.rs`), JNI (`nativeCreateWinePrefix`, `nativeManageWinePrefix`), and `WineAndPrefixesTabContent`:

1. **Create (`create_prefix`)**: Allocates isolated directory tree, initializes `drive_c` and `dosdevices`, writes `prefix.json`, and prepares `wineboot --init` command.
2. **Configure (`configure_prefix`)**: Updates Windows version (`win11`, `win10`, `win7`), DLL overrides, environment variables, graphics backend preference, and DXVK/VKD3D toggles.
3. **Clone (`clone_prefix`)**: Performs a deep recursive copy of an existing prefix into a new validated `prefix_id` with independent inodes and rewritten `prefix.json` metadata.
4. **Backup (`backup_prefix`)**: Packages the isolated prefix into `<base_data_dir>/backups/wine_prefixes/<prefix_id>-<timestamp>.tar.gz` and computes a companion `.sha256` cryptographic digest file.
5. **Restore (`restore_prefix`)**: Verifies the `.sha256` digest **before** extraction, validates every archive entry path against Zip-Slip traversal (`..` or absolute paths rejected via `androidpc-security`), extracts into an atomic staging directory, and commits only on full success.
6. **Repair (`repair_prefix`)**: Re-creates missing `drive_c/windows/{system32,syswow64}` directories, restores `dosdevices/c:`, verifies `prefix.json` readability, and prepares `wineboot -u` without destroying user data in `drive_c/users`.
7. **Delete (`delete_prefix`)**: Refuses deletion if active `TrackedWindowsProcess` entries are still running in that prefix; otherwise removes only `<prefix_dir>`.

## 3. Isolation & Failure Recovery Guarantees

- Corrupting DLL overrides or registry hives inside Prefix A has zero effect on Prefix B or the host Ubuntu/Debian rootfs.
- `wineserver -k` / `wineserver -k9` is always executed with `WINEPREFIX` set to the exact prefix path.
- Backup/restore round-trip and SHA-256 tampering rejection are covered by Rust unit tests in `rust/crates/wine/src/lib.rs`.
