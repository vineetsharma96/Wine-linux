# Stage 3 Display Architecture Decision

## Stage 3.2 decision

Keep Xvnc/RFB as the built-in transport and choose Rust ANativeWindow for presentation. The completed vertical slice removes the native staging framebuffer and sends one incremental request at a time. SurfaceView retains Android's window lifecycle and BufferQueue synchronization. A Canvas presenter consumes the same RFB framebuffer when selected or when surface setup fails. The existing desktop supervisor, input path, RandR controls, and clipboard protocol are shared. [Evaluation](DISPLAY_RENDERER_EVALUATION.md) · [Implementation and lifecycle](ANDROID_NATIVE_RENDERING.md)

This choice carries forward the existing Xvnc and XFCE compatibility, and it keeps an independently testable display path with limited copying. An embedded DDX, an EGL upload path and dedicated shared-buffer transport can each be reconsidered when measurement justifies their integration. The supported external Termux workflow remains separate. [Termux:X11 analysis](TERMUX_X11_ANALYSIS.md) · [Benchmarks](DISPLAY_BENCHMARKS.md)

## Evaluated Options
- **Architecture A (In-app `libXlorie`)**: Highest native X11 FPS (`60–120 FPS`), zero TCP, `ASharedMemory` + `EGL`/`ANativeWindow`, but carries **GPL-3.0** licensing and full X.Org CMake build/maintenance cost.
- **Architecture B (`libandroid-shmem`)**: **BSD-3-Clause** System V SHM (`ASharedMemory` + `SCM_RIGHTS`) helper; already bundled in `jniLibs/arm64-v8a/libandroid-shmem.so`, not a renderer.
- **Architecture C (External `Termux:X11` socket bridge)**: Works when user runs `tx11start` in Termux (`com.termux.x11` launched via `<queries>`), but cannot be embedded or automated standalone without `com.termux` due to Android UID/SELinux isolation.
- **Architecture D (Rust `ANativeWindow` Native Surface Renderer + Dedicated RFB Thread)**: Keeps self-contained `Xvnc` inside PRoot, eliminates Kotlin `Bitmap` + `copyPixelsFromBuffer` + `Canvas.drawBitmap` + per-frame JNI calls by having Rust lock and blit directly to `ANativeWindow` (`ANativeWindow_fromSurface`, `ANativeWindow_setBuffersGeometry`, `ANativeWindow_lock`, `ANativeWindow_unlockAndPost`) with hardware `SurfaceFlinger` scaling.

## Decision
1. **Primary Self-Contained Display Renderer (Stage 3 Vertical Slice)**: Implement **Architecture D — Rust `ANativeWindow` Native Surface Renderer** (`rust/crates/android-bridge` + `androidpc-display`) with a dedicated Rust RFB reader/presenter thread, keeping `DirectByteBuffer`/`SurfaceView` fallback intact.
2. **Optional External High-FPS Backend**: Retain **Termux:X11** (`Backend B: Launch Termux:X11`) for users with `com.termux` + `com.termux.x11`.
3. **Future Phase**: Re-evaluate bundling `libXlorie` (`GPL-3.0`) if Wine/DXVK in Phase 4 requires zero-TCP `AHardwareBuffer` X11 sharing.

## Stage 3.1 session ownership

**Confirmed.** The previous launcher replaced itself with XFCE via `exec`, taking its Xvnc cleanup trap out of the active process tree. The tablet had a former app-owned Xvnc with parent PID 1 and an RFB listener on port 5901. The bundled PRoot command already uses `--kill-on-exit`. The upstream Termux PRoot tracer handles `SIGQUIT` by killing its tracees. [Termux PRoot source](https://github.com/termux/proot/blob/master/src/tracee/event.c) · [TigerVNC Xvnc documentation](https://tigervnc.org/doc/Xvnc.html)

The managed desktop now has a dedicated PRoot child and Unix process group. Each launch tracks its environment ID, unique desktop ID, elapsed time, control directory, and observed child PIDs with Linux start times. The launcher gives XFCE time to complete logout on its private D-Bus session, then terminates its Xvnc child. The app closes the session's pipe to request shutdown. If PRoot stays alive, Rust signals the owned tracer with `SIGQUIT` and then escalates to the process group. Disconnect preserves the desktop. Exit Desktop closes the owned desktop. Stop Linux closes the desktop, runtime, and terminals for the selected environment. [D-Bus session lifetime](https://dbus.freedesktop.org/doc/dbus-run-session.1.html) · [Android activity lifecycle](https://developer.android.com/guide/components/activities/activity-lifecycle)

The renderer's `Mutex` import now shares the Android-only compilation condition used by the presenter. [XFCE session manager docs](https://docs.xfce.org/xfce/xfce4-session/start) · [Rust conditional compilation](https://doc.rust-lang.org/reference/conditional-compilation.html)

**Limit.** An abrupt tracer or kernel failure can prevent normal cleanup. The process group covers processes that stay in it. An untraced process that also escaped the group falls outside that guarantee.
