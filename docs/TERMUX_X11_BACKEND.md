# Termux:X11 External Display Backend

- **Rust Contract (`rust/crates/display/src/termux_x11.rs`)**: `TermuxX11Backend` implements `DisplayBackend` for `DisplayType::TermuxX11`, checking `/data/data/com.termux/files/usr/tmp/.X11-unix/X0`.
- **Android Integration (`AndroidManifest.xml` & `DashboardScreen.kt`)**:
  - `AndroidManifest.xml` declares `<queries><package android:name="com.termux.x11" /></queries>` for Android 11+ (API 30+) package visibility.
  - `DashboardScreen` checks `PackageManager.getPackageInfo("com.termux.x11", 0)`: when installed (verified on `6e5b28dd`), displays `Backend B: Launch Termux:X11` and launches `com.termux.x11/com.termux.x11.MainActivity`; when absent, shows setup requirements.
- **Limitation**: On unrooted Android, third-party app private directories (`/data/data/com.termux`) are isolated by SELinux unless bridged explicitly; Integrated Xvnc is the primary self-contained backend.
