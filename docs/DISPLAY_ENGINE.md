# Display Engine (`androidpc-display` & `androidpc-core::desktop`)

- **Session Supervisor (`rust/crates/core/src/desktop.rs`)**:
  - `start_desktop_session(env_id, display_num, width, height, dpi, port)`: checks `/usr/bin/Xvnc` and `/usr/bin/xfce4-session`, writes `/usr/local/bin/androidpc-start-desktop.sh`, spawns PRoot with `--kill-on-exit`, and polls `127.0.0.1:<port>` until RFB is ready.
  - `stop_desktop_session()`: disconnects `RfbClient` and sends `SIGTERM` (with `SIGKILL` fallback) to the supervised PRoot process, terminating `Xvnc`, `dbus-daemon`, and `xfce4-session`.
  - `resize_desktop_session(width, height)`: invokes `xrandr --fb <w>x<h>` inside the active display session, triggering `Xvnc` `DesktopSize` (`-223`) updates.
- **Android Rendering (`DisplayViewportScreen.kt`)**:
  - Uses `AndroidView` + `SurfaceView` (`SurfaceHolder.lockCanvas()` -> `drawBitmap()` -> `unlockCanvasAndPost()`) on `Dispatchers.Default` at ~30 FPS without per-frame Compose recomposition.
  - Reports actual measured FPS (`framesInWindow` per 1-second window).
