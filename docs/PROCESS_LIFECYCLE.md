# Windows Process & Resource Lifecycle (Phase 4.8)

*Last updated: 2026-11-25*
*Rust modules: `rust/crates/wine/src/manager.rs`, `rust/crates/core/src/windows_compat.rs`*

## 1. Scoped Process Tracking (`TrackedWindowsProcess`)

Every Windows application launch creates a `TrackedWindowsProcess` record indexed by `(app_id, prefix_id, environment_id)`:
- `app_id` & `app_name`: Owning Windows application profile.
- `prefix_id` & `environment_id`: Isolated Wine prefix and parent Ubuntu/Debian PRoot environment.
- `wine_pid` & `box64_pid`: Tracked PIDs when spawned live.
- `state`: `Launching`, `Running`, `Stopping`, `Exited`, `Crashed`.
- `memory_rss_bytes`: Read from `/proc/<pid>/status` (`VmRSS:`) when the process is alive and accessible inside the Android app UID sandbox; returns `None` (`Unavailable`) if not measurable—never fabricated.
- `exit_code` & `exit_reason`: Captured upon process exit or signal termination.
- `started_at_epoch_ms`: Launch timestamp.

## 2. Scoped Termination Policy (No Broad Kills)

**Hard Rule**: Stopping a Windows application MUST NEVER execute `killall`, `pkill wine`, or terminate processes outside the target application's isolated `WINEPREFIX`.

1. **Graceful Stop (`force = false`)**:
   - Invokes `wineserver -k` with `WINEPREFIX=<exact_isolated_prefix_path>`.
   - Waits within a bounded window for Wine child processes in that prefix to exit cleanly and flush registry hives (`system.reg`, `user.reg`).
2. **Force Stop (`force = true`)**:
   - Invokes `wineserver -k9` strictly with `WINEPREFIX=<exact_isolated_prefix_path>`, then sends `SIGKILL` only to tracked PIDs registered under that `app_id`.
3. **Isolation Guarantee**:
   - Other Wine prefixes continue running unaffected.
   - The parent Ubuntu/Debian PRoot container, PTY terminals, and XFCE/Xvnc desktop session remain alive and uninterrupted.

## 3. Android Lifecycle Interactions

- **Activity Backgrounding / Screen Rotation**: Windows processes run inside the Rust engine / foreground `LinuxRuntimeService` scope so switching Android apps or rotating Xiaomi Pad 7 does not kill running Windows sessions.
- **Process Death / App Force-Stop**: On restart, `EngineCore` reconciles persisted prefix/app metadata against live `/proc/<pid>` state so stale PIDs are never mistaken for running applications.
- **Prefix Deletion Guard**: `WinePrefixManager` and `EngineCore` block deleting any prefix that currently has an active `TrackedWindowsProcess` in `Launching` or `Running` state.
