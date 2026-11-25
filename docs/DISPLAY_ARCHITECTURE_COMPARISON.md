# Display Architecture Comparison

| Architecture | Rendering Path | Measured / Expected FPS | CPU Overhead | Compatibility & Security | Complexity |
| --- | --- | --- | --- | --- | --- |
| **Option A: Termux:X11 (`libXlorie`)** | Guest X11 -> `libXlorie` (`SurfaceView` + BLAST `BufferQueue` + SHM/FD) | **60–120 FPS** (panel refresh-coupled, 0 UI-thread jank) | Low (`~12.6%` total on Pad 7) | Requires external `com.termux.x11` APK; Android SELinux isolates `/data/data/com.termux` from third-party apps unless bridged via supported IPC/abstract socket or user-managed Termux session. | Low (external backend) / High (if embedding `libXlorie` directly) |
| **Option B: Optimized VNC (`Xvnc` + Rust `RfbClient`)** | Guest `Xvnc` -> `127.0.0.1` RFB -> Dedicated Rust decode thread -> Direct `ByteBuffer` / `ANativeWindow` | **30–60 FPS** target (currently 11 FPS unoptimized) | Medium (eliminating 4× copies, `vec!` allocs, and scalar divisions cuts CPU by ~3×) | 100% self-contained in app-private PRoot sandbox; no external APK; supports remote/VM (`QEMU`) too. | Medium |
| **Option C: Native Integrated X11 (`libXlorie` / `ANativeWindow` in-app)** | Guest X11 (`DISPLAY=:0`) -> In-process native X server -> `ANativeWindow` / `AHardwareBuffer` | **60–120 FPS** | Low | Fully self-contained, zero TCP, direct `Surface` / `HardwareBuffer`; requires building/maintaining `libXlorie` C/NDK + `libandroid-shmem` inside APK `jniLibs`. | High |
| **Winlator Reference (Public Architecture)** | Wine/Box64 -> Turnip (Adreno Vulkan) / VirGL -> Lorie X Server + `shmPath` -> `SurfaceView` | 60+ FPS | Low–Medium | Demonstrates that `Lorie` + `shmPath` + `SurfaceView` eliminates TCP/VNC overhead; Turnip/Zink/VirGL handles 3D GPU acceleration. | High |
| **Xerj Platform Evaluation** | N/A (Code search & project memory engine on `127.0.0.1:9200`, `all-MiniLM-L6-v2`) | N/A | N/A | Xerj is a developer knowledge/index tool in this workspace, **not** a Linux/Android display server or graphics compositor. | N/A |

## Architectural Decision
1. **Immediate Milestone**: Support **Option A (`Termux:X11` external backend)** for users with `com.termux.x11` installed **and** optimize **Option B (`Xvnc` + Rust `RfbClient` -> `ANativeWindow` / direct `ByteBuffer`)** to hit `30–60 FPS` self-contained without external APKs.
2. **Future Native Integrated Milestone (Option C)**: Adopt a `Lorie`-style in-process X server + `ANativeWindow` / `AHardwareBuffer` (`shmPath`) renderer in Rust/NDK so our standalone app matches `Termux:X11` and Winlator frame pacing without external app dependencies.
