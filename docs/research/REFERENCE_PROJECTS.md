# Reference Projects Catalog

| Project | Repository / Official Site | License | Architectures | Android Compat | Decision | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| **PRoot (Termux fork)** | [termux/proot](https://github.com/termux/proot) | GPL-2.0 | `aarch64`, `arm`, `x86_64`, `i686` | API 24+ (via `jniLibs`) | **Adopt** | Bundled as `libproot.so` + `libproot_loader.so` in `jniLibs/arm64-v8a`. |
| **Termux:X11 (`libXlorie`)** | [termux/termux-x11](https://github.com/termux/termux-x11) | GPL-3.0 | `aarch64`, `arm`, `x86_64` | API 24+ | **Integrate (External) / Study (Native)** | `libXlorie.so` + `SurfaceView` + `BLAST` achieves zero UI-thread jank; external launch via `<queries>` supported, native `libXlorie` bundling evaluated for Stage 3. |
| **TigerVNC (`Xvnc`)** | [TigerVNC/tigervnc](https://github.com/TigerVNC/tigervnc) | GPL-2.0 | `aarch64`, `x86_64`, etc. | Inside PRoot guest | **Adopt (Fallback/Remote)** | `1.15.0` from Ubuntu/Debian APT runs cleanly inside PRoot with `dtmp:/tmp`; self-contained without external APKs. |
| **Winlator** | [brunodev85/winlator](https://github.com/brunodev85/winlator) | MIT (app; bundles GPL/LGPL components) | `aarch64` | API 26+ | **Study / Adapt** | Uses Lorie X server + `shmPath` + `SurfaceView` + Turnip/VirGL + Box64/Wine. |
| **Box64** | [ptitSeb/box64](https://github.com/ptitSeb/box64) | MIT | `aarch64` host -> `x86_64` guest | Android / Linux | **Adopt (Phase 4)** | Dynamic x86_64-to-ARM64 recompiler for Wine/Linux x86_64 binaries. |
| **Wine** | [winehq.org](https://www.winehq.org/) | LGPL-2.1+ | `x86_64`, `aarch64` | Inside PRoot + Box64 | **Adopt (Phase 4)** | Windows API translation layer. |
| **Mesa (Turnip / Zink / VirGL)** | [mesa3d.org](https://docs.mesa3d.org/) | MIT | `aarch64` | Adreno (Turnip) / General (VirGL) | **Study / Adapt (Phase 4)** | Vulkan/OpenGL acceleration inside guest containers. |
