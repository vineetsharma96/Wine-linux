# Windows Application Manager (Phase 4.6)

*Last updated: 2026-11-25*
*Rust crate: `rust/crates/windows-apps` (`androidpc-windows-apps`) · UI: `WindowsAppsTab.kt`*

## 1. Per-Application Profile Model (`WindowsAppProfile`)

Every registered Windows application is stored as a JSON profile under `<base_data_dir>/windows_apps/<app_id>.json` with strict ID validation (`^[a-zA-Z0-9_-]{1,64}$`):

| Field | Purpose |
|---|---|
| `app_id` | Unique sanitized identifier |
| `name` | Human-readable application name |
| `executable_path` | Windows (`C:\...`) or Unix guest path to `.exe` or `.msi` |
| `install_dir` | Working/installation directory inside the prefix |
| `prefix_id` | Bound isolated Wine prefix ID |
| `environment_id` | Bound Ubuntu/Debian ARM64 Linux environment ID |
| `wine_version` | Pinned Wine version (`11.0`) |
| `box64_overrides` | Optional per-app Box64 environment overrides |
| `graphics_backend` | Preferred graphics backend (`turnip`, `virgl`, `software`) |
| `resolution_width` / `resolution_height` | Virtual desktop window geometry (`explorer /desktop=...`) |
| `launch_args` | Command-line arguments passed to the Windows executable |
| `env_vars` | Per-app environment variables (credentials redacted from logs) |
| `compatibility_notes` | User/engineer notes on known quirks or overrides |
| `created_at_epoch_ms` / `last_launched_at_epoch_ms` | Lifecycle timestamps |
| `shortcut_created` | Tracks whether an XFCE `.desktop` shortcut was generated |

## 2. XFCE Desktop Shortcut Generation

`WindowsAppLibrary::write_xfce_desktop_shortcut` writes a standard Freedesktop `.desktop` entry under `<guest_rootfs>/home/user/Desktop/androidpc-win-<app_id>.desktop` (and `applications/`):
- Safely escapes shell/Exec characters and strips control characters/newlines to prevent desktop-entry injection.
- Routes clicks directly through the configured Box64 + Wine prefix launch command on `DISPLAY=:1`.

## 3. Per-App Diagnostic Log Buffer

`WindowsAppLibrary` maintains a bounded per-app log ring buffer (`DEFAULT_MAX_LOG_LINES = 400`, persisted to `<base_data_dir>/windows_apps/logs/<app_id>.log` with credential-key redaction) so stdout/stderr from Box64, Wine, DXVK, and VKD3D can be inspected per application without unbounded storage growth.

## 4. Supported Lifecycle Operations

- **Register / Edit** (`nativeRegisterWindowsApp` / `WindowsAppsTabContent` Add EXE/MSI dialog)
- **Launch** (`nativeLaunchWindowsApp` → binary check → graphics & DX gate → Box64 + Wine launch command → `TrackedWindowsProcess` registration)
- **Stop / Force Stop** (`nativeTerminateWindowsApp` → scoped `wineserver -k` / `-k9`)
- **Remove** (`nativeRemoveWindowsApp` → removes profile, XFCE `.desktop` shortcut, and per-app log buffer while leaving the Wine prefix intact unless the user explicitly deletes the prefix)
