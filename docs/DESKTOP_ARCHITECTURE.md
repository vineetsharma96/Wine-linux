# Desktop Architecture (Phase 3)

Android PC Environment runs a real X11 + XFCE 4.20 desktop inside the rootless PRoot guest userspace, streamed over loopback RFB (`127.0.0.1:5901`) to a Rust VNC client and rendered onto an Android `SurfaceView`.

## Data & Control Flow
```
XFCE 4.20 (xfce4-session, xfwm4, xfce4-panel, xfdesktop4, xfce4-terminal)
  -> Xvnc (tigervnc-standalone-server 1.15.0, bound to 127.0.0.1:5901)
  -> Rust RfbClient (RFB 003.008, Raw/CopyRect/DesktopSize, ClientCutText/ServerCutText)
  -> JNI NativeBridge -> Android SurfaceView (SurfaceHolder.lockCanvas / drawBitmap)
```

## Key Rootless PRoot Adaptations
1. **Guest-hosted Xvnc**: Rather than running a standalone Termux `Xvnc` binary (which failed on Android 16 with `libcrypto.so.3` missing), `EngineCore::start_desktop_session` supervises `/usr/bin/Xvnc` inside the selected Ubuntu/Debian guest via PRoot (`--kill-on-exit --link2symlink`).
2. **Short `/tmp` Bind Mount (`<base_data_dir>/dtmp:/tmp`)**: Linux `AF_UNIX` sockets (`sun_path` limit 108 bytes) fail with `EINVAL` if `/tmp` resolves inside deep app-private `rootfs` paths. Binding `<base_data_dir>/dtmp` to `/tmp` keeps `/tmp/.X11-unix/X1` and `/tmp/dbus-*` under 65 bytes.
3. **Glycin Loader Compatibility (`bwrap` shim)**: Ubuntu 26.04's `gdk-pixbuf` invokes `glycin` via `bwrap --unshare-all`, which fails inside unprivileged PRoot. The desktop launcher prepends `/usr/local/lib/androidpc-bin/bwrap` to `PATH` to forward `glycin-loaders` directly inside the existing PRoot/Android sandbox.
4. **Declarative Profiles**: `rust/crates/display/src/profiles.rs` defines versioned `Minimal`, `Standard`, and `Development` profiles installable via `EngineCore::install_desktop_profile`.
