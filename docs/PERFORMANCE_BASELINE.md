# Performance Baseline — Xiaomi Pad 7 (`2410CRP4CI`)

## Test Environment
- **Device**: Xiaomi Pad 7 (`2410CRP4CI`, Snapdragon ARM64)
- **Android Version**: Android 16
- **Kernel**: `6.1.138-android14-11-g0c3d559bcd85-ab14529422 #1 SMP PREEMPT aarch64`
- **Panel**: `2136 x 3200`, `440 dpi`, active render rate `120.00001 Hz` (modes: 30, 48, 50, 60, 90, 120, 144 Hz)
- **Linux Guest**: Ubuntu 26.04.1 ARM64 (`resolute`), XFCE 4.20 (`xfce4-session` 4.20.4-1, `xfwm4` 4.20.0-1build1, `xfce4-panel` 4.20.7-1)
- **Workload**: 5-second repeated pointer drag across desktop (`800,800` <-> `1800,1200`) at `1280x720`, 32bpp `rgb888`

## Measured Baseline Comparison

| Metric | Baseline A: Termux + Termux:X11 (`tx11start`) | Baseline B: Android PC + XFCE + Xvnc |
| --- | --- | --- |
| **Termux / App Build** | `com.termux` `0.118.3` + `com.termux.x11` `1.03.01-d55f1b7-28.09.26` | `com.androidpc.environment.debug` `0.1.0-alpha.0` + `tigervnc-standalone-server` `1.15.0+dfsg-2build1` |
| **Measured Desktop FPS** | Native `SurfaceFlinger` BLAST flips (no UI-thread throttle) | **11 FPS** (`Drag: (321, 270) • 11fps`) |
| **Host App / X Server CPU %** | `com.termux.x11`: `0.9%` + `termux-x11 :0`: `11.7%` (**12.6% total**) | `com.androidpc.environment.debug`: **45.3%** + `Xvnc`: `4.7%` + `libproot.so`: `14.3%` (**64.3% total**) |
| **Memory (RSS)** | `com.termux.x11`: `169 MiB`, `termux-x11 :0`: `252 MiB` | `com.androidpc.environment.debug`: `304 MiB`, `Xvnc`: `41 MiB`, `libproot.so`: `4.6 MiB` |
| **Android `gfxinfo` UI Jank** | `0` UI-thread frames (`0` missed Vsync) | `416` frames, `61` janky (`14.66%`), `356` legacy janky (`85.58%`), `15` missed Vsync |
| **UI Frame Latency (p50 / p90 / p99)** | N/A (native `SurfaceView` `BufferQueue` bypasses View/Compose UI thread) | `14 ms` / `24 ms` / `27 ms` |
| **Framebuffer Bandwidth** | Zero TCP; local Unix socket + SHM/FD | `18.368 MiB` over `72` updates (`~10.3` updates/s), `325` rects, `1:1` Raw uncompressed |

## Post-Optimization Results (Stage 2: DirectByteBuffer + 32bpp Fast Path + Dirty-Skip)
Measured on the identical Xiaomi Pad 7 (`2410CRP4CI`, Android 16, `1280x720`, 5-second drag workload):
- **App CPU %**: **36.1%** (down from `45.3%`, `-20%`)
- **App RSS**: **251.2 MiB** (down from `303.7 MiB`, `-52.5 MiB`)
- **UI-Thread `gfxinfo`**: **20 frames**, **0 Missed Vsync** (down from `416` frames and `15` Missed Vsync)
- **Xvnc Updates Delivered**: **101 updates**, **628 rects** (`18.498 MiB` Raw) vs `72` updates / `325` rects previously (`+40%` update throughput)
- **Idle-Frame Overhead**: `0` bytes copied and `0` `SurfaceView` redraws when `generation == last_gen` (`0fps` reported during idle windows after input stops).
