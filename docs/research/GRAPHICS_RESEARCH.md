# Graphics & Display Research

## Confirmed Architecture Comparisons
- **Integrated `Xvnc` + `RfbClient`**: Self-contained inside app-private PRoot. Even after Stage 2 optimizations (`DirectByteBuffer`, 32bpp fast path, dirty-skip), RFB `Raw` over loopback TCP incurs `~36.1%` app CPU and `18.5 MiB` / 5s bandwidth.
- **Termux:X11 (`libXlorie.so`)**: Runs an Android-tuned X.Org server in-process inside Android, rendering to `SurfaceView` via `BLAST` (`BufferQueue`) and `libandroid-shmem` / Unix FD passing. Measured `12.6%` total CPU and `0` UI-thread `gfxinfo` frames on Xiaomi Pad 7.
- **Winlator (Public Architecture)**: Pairs `Lorie` + `shmPath` + `SurfaceView` with **Turnip** (Adreno Vulkan) + **DXVK/VKD3D** on Snapdragon devices, or **VirGL/Zink** on non-Adreno GPUs.
- **XFCE Compositor Note**: Running a software compositor (`xfwm4` compositing) on top of Android's `SurfaceFlinger` adds double-compositing overhead on mobile devices.
