# Termux:X11 Architecture Analysis (`termux/termux-x11`)

## Stage 3.2 upstream audit

**Confirmed.** The supported install is the Android APK plus a companion package from the Termux graphical repository. The package supplies the launcher and the X server entry runs with the launching environment. The normal APK and shared-UID APK are separate build flavors. The normal build's `standalone` flavor refers to APK UID placement. The regular workflow still uses the companion, and the shared-UID variant requires matching signatures. [Setup and build](https://github.com/termux/termux-x11/tree/24ce2076fe68eff1599da286c5c162f7693c0501) · [Build flavors](https://github.com/termux/termux-x11/blob/24ce2076fe68eff1599da286c5c162f7693c0501/lorie-app/build.gradle)

The X server accepts X11 clients through its configured display socket. For a PRoot guest, upstream documents sharing the launching environment's temporary directory or setting TMPDIR and XKB data appropriately. Android viewer control and frame handoff use separate native calls, Android broadcasts with a Binder, and FD transfers. Launching the viewer activity alone does not bootstrap the server. The documented stock-Android path is the external companion. Android PC Environment can open its activity and leaves command execution with the launching environment. Termux's general RUN_COMMAND service has its own permission and opt-in contract. [Command entry point](https://github.com/termux/termux-x11/blob/24ce2076fe68eff1599da286c5c162f7693c0501/lorie/src/main/java/com/termux/x11/CmdEntryPoint.java) · [Termux RUN_COMMAND](https://github.com/termux/termux-app/wiki/RUN_COMMAND-Intent)

**Experimentally verified.** The device had Termux 0.118.3 and Termux:X11 1.03.01-d55f1b7-28.09.26. Starting the existing external XFCE session exposed a separate Termux:X11 BLAST layer and a native 3200×1840 X11 desktop. The upstream nightly tag inspected pointed to `c5d09202`. The benchmark used the already-installed version. [Nightlies](https://github.com/termux/termux-x11/releases) · [Results](DISPLAY_BENCHMARKS.md)

**Unknown.** A self-contained companion-free embedding in our app has no verified supported API. The published external integration remains suitable for users who already maintain a Termux session.

## Source Inspection (`G:\tmp\ref\termux-x11`, GPL-3.0)
- **Confirmed**: `com.termux.x11` (`1.03.01-d55f1b7-28.09.26` on `6e5b28dd`) is licensed under **GPL-3.0** (`LICENSE`).
- **Confirmed (`cmdentrypoint.cpp`, `CmdEntryPoint.java`)**: `termux-x11` is started via `app_process` inside Termux's UID, creates an X11 server thread inside `libXlorie`, binds Unix sockets inside Termux's private `TMPDIR`, and sends an `IBinder`/FD broadcast to `com.termux.x11`.
- **Confirmed (`LorieView.java`)**: Uses Android `SurfaceView` and passes `Surface` down to native C++ (`ANativeWindow_fromSurface`).
- **Confirmed (Integration Constraint)**: Android SELinux (`app_data_file`) and UID isolation prevent a third-party app (`com.androidpc.environment`) from reading `/data/data/com.termux` sockets or injecting `CmdEntryPoint` without user-managed Termux execution (`tx11start` / `tx11stop`).
