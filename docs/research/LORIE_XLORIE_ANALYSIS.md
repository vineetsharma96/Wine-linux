# Lorie / Xlorie Native Renderer Analysis

## Stage 3.2 upstream audit

**Confirmed.** At source revision `24ce2076fe68eff1599da286c5c162f7693c0501`, Lorie is built as an X.Org Device Dependent X subsystem and Android app glue. CMake 3.22 or newer, Python, Bison, patches, the NDK, and a group of linked X.Org and supporting subprojects are part of the cross-build. The source uses hidden visibility for server objects. Java-native methods in LorieView, including surface/window changes, viewport, filtering, and clipboard calls, are part of the app contract. Inspection established no stable public renderer ABI or versioned standalone package. [CMake build](https://github.com/termux/termux-x11/blob/24ce2076fe68eff1599da286c5c162f7693c0501/lorie/src/main/cpp/CMakeLists.txt) · [LorieView JNI](https://github.com/termux/termux-x11/blob/24ce2076fe68eff1599da286c5c162f7693c0501/lorie/src/main/java/com/termux/x11/LorieView.java)

The Android viewer has separate rendering and cursor surfaces. Renderer code imports supported hardware buffers through EGL and samples them as OpenGL ES textures, with a fence wait path for GPU synchronization. Buffer and IPC code manage shared file descriptors and maps. Android input becomes X keyboard, mouse and touch events through the DDX input path. Window configuration and refresh changes propagate to the X server, and clipboard code translates Android clipboard state and X selections. These pieces depend on the DDX, JNI and IPC contract. [Renderer](https://github.com/termux/termux-x11/blob/24ce2076fe68eff1599da286c5c162f7693c0501/lorie/src/main/cpp/lorie/renderer.cpp) · [Input](https://github.com/termux/termux-x11/blob/24ce2076fe68eff1599da286c5c162f7693c0501/lorie/src/main/cpp/lorie/InitInput.c) · [Clipboard](https://github.com/termux/termux-x11/blob/24ce2076fe68eff1599da286c5c162f7693c0501/lorie/src/main/cpp/lorie/clipboard.c)

**Confirmed.** The upstream distribution is GPL-3.0 and also contains component-level notices. Reusing source or shipping a linked derivative requires a distribution and compliance decision. Its lifecycle history includes an EGL context failure during an Android window transition, and current open work changes direct DRI3 buffer access. [Upstream license](https://github.com/termux/termux-x11/blob/24ce2076fe68eff1599da286c5c162f7693c0501/LICENSE) · [Lifecycle issue](https://github.com/termux/termux-x11/issues/382) · [DRI3 pull request](https://github.com/termux/termux-x11/pull/1095)

**Hypothesis.** An original producer that can supply hardware buffers could adopt the same broad techniques using public Android APIs. That would require measuring allocation, fences and damage tracking with our own transport.

**Experimentally verified.** The installed public Termux:X11 APK contained an ARM64 `libXlorie.so` with 1,986 dynamic definitions in `llvm-nm -D --defined-only`. These included `JNI_OnLoad` and X.Org symbols such as `CompositeGlyphs` and `fbImageGlyphBlt`. That symbol table is consistent with a native X server bundle. It provides no evidence of a supported renderer ABI. Compatibility across independent Xlorie builds remains open.

`llvm-readelf -d` reported Android, Media NDK, OpenGL ES 2, EGL, logging, math, compression, and libc dependencies. These are the binary dependencies of the installed build. The source audit used the upstream revision cited above.

## Source Inspection (`lorie/src/main/cpp/`)
- **Confirmed (`CMakeLists.txt`)**: `libXlorie` is **not** a small standalone library. It compiles a full X.Org X11 server stack (`xorgproto`, `fontenc`, `md`, `pixman`, `tirpc`, `xshmfence`, `Xdmcp`, `Xau`, `Xfont2`, `xkbcomp`, `xserver`) plus Android glue (`cmdentrypoint.cpp`, `renderer.cpp`, `buffer.c`, `InitInput.c`, `InitOutput.c`, `clipboard.c`).
- **Confirmed (`buffer.c`, `renderer.cpp`)**:
  - Allocates shared regions via `ASharedMemory_create` (API 26+) with `memfd_create` fallback (`LorieBuffer_createRegion`).
  - Wraps `AHardwareBuffer` via `eglGetNativeClientBufferANDROID` into `EGLImage` / OpenGL ES 2.0 `GL_TEXTURE_2D`.
  - Renders to `ANativeWindow` via `EGLSurface` with `eglClientWaitSyncKHR` GPU fences.
- **Licensing Implication**: Adopting `libXlorie` directly in-app introduces **GPL-3.0** copyleft obligations for the native X server component and requires maintaining a CMake X.Org cross-build.
