# Termux:X11 Integration Analysis

## Public Architecture Summary
- `com.termux.x11` (`1.03.01-d55f1b7-28.09.26` verified on `6e5b28dd`) embeds `libXlorie.so`, an Android-tuned X.Org server that renders directly into an Android `SurfaceView` backed by `BLAST` / `BufferQueue` and `libandroid-shmem` / Unix FD passing.
- On `6e5b28dd`, `tx11start` launches `termux-x11 :0` (`11.7%` CPU, `252 MiB` RSS) and `com.termux.x11` (`0.9%` CPU, `169 MiB` RSS) with `0` UI-thread jank.

## Integration Constraints on Non-Rooted Android
1. **Package Visibility**: `AndroidManifest.xml` must declare `<queries><package android:name="com.termux.x11" /></queries>` for API 30+ detection.
2. **App-Private Storage Isolation**: Android SELinux (`app_data_file`) prevents our app (`com.androidpc.environment`) from reading/writing `/data/data/com.termux/files/usr/tmp/.X11-unix/X0` or executing `/data/data/com.termux/files/usr/bin/tx11start` directly.
3. **Supported Integration Path**:
   - Detect & launch `com.termux.x11` via `PackageManager`.
   - Allow users running `termux-x11 :0 -ac` (with TCP or abstract/bridged socket where configured) or `tx11start`/`tx11stop` inside Termux to use Termux:X11 as an external display backend, with clear onboarding and troubleshooting in the app.
   - Keep Integrated `Xvnc` (and future in-app `Lorie`/`ANativeWindow`) as the zero-dependency standalone backend.
