# Stage 3 Display Benchmarks (Xiaomi Pad 7 `2410CRP4CI`)

## Stage 3.2 protocol

**Experimentally verified.** Xiaomi Pad 7, model `2410CRP4CI`, Android 16, HyperOS `OS3.0.305.0.WOZINXM`, kernel `6.1.138`, ARM64, charging, 3200×2136 physical landscape at 440 DPI. SurfaceFlinger reported a nominal 120 Hz display mode during sampling. Built-in rows used the same Ubuntu 26.04.1 XFCE guest, match-sized 1920×1080 desktop, and loopback TigerVNC 1.15.0. Termux:X11 used its installed 3200×1840 root geometry, with the same stimulus window. Each short run drew a 1024×640 Xlib window at 30 Hz for 360 draws. The extended run submitted 3,600 draws in 120 seconds.

Run [measure_display.ps1](../../scripts/measure_display.ps1) with the app viewport already visible, the ADB path, guest UUID, and an output directory. Change the renderer with the viewport toolbar and collect one fresh run per mode. [display_workload.py](../../scripts/display_workload.py) uses Python 3 and the guest's libX11, with no packages required in the app. For the external reference, copy the workload and [Termux launcher](../../scripts/run_termux_display_workload.sh) to shared Downloads, start the existing X server, arm the launcher from Termux, switch to its viewer, and run the harness with `-External`. The harness signals the waiting workload and collects the named display layer, device settings, memory, process snapshots, and CPU/GPU samples. It clears and restores the TimeStats collection state for the measurement. [Android measurement method](https://developer.android.com/games/optimize/framerate)

| Path | Displayed FPS | Present interval p50 / p90 / p99 | Acquire-to-present p50 / p90 / p99 | SurfaceFlinger drops | Viewer / X server CPU | Viewer RSS | Device GPU busy |
| --- | ---: | --- | --- | ---: | --- | ---: | ---: |
| Existing staged Rust RFB, paired trial | 25.31 | 33 / 50 / 86 ms | 15 / 19 / 20 ms | 0 | 19% / 31% | 363.4 MiB | 5.71% |
| Direct Rust RFB, paired trial | 29.02 | 33 / 42 / 58 ms | 16 / 19 / 21 ms | 0 | 17% / 19% | 360.7 MiB | 3.83% |
| Canvas RFB fallback, paired trial | 24.63 | 33 / 66 / 86 ms | 16 / 20 / 20 ms | 0 | 26% / 35% | 370.7 MiB | 15.27% |
| Termux + Termux:X11, synchronized reference | 30.20 | 33 / 50 / 50 ms | 17 / 26 / 29 ms | 0 | 17% / 34% | 356.8 MiB | 7.06% |
| Direct Rust RFB, 120-second trial | 25.42 | 33 / 58 / 74 ms | 16 / 19 / 20 ms | 0 | 18% / 35% | 376.1 MiB | Unrecorded |

The paired rows use the main SurfaceView BLAST layer by event count. CPU is the median of the sampled process percentages, in units where one core is 100%. The samples do not sum guest or display-service overhead. GPU busy is the median of readable KGSL busy/elapsed samples for the device. Memory is the viewer's post-workload RSS. A second short trial returned 26.61 FPS for the staged baseline and 26.38 FPS for the direct prototype. This variation limits the throughput conclusion from a single pair. The extended run finished without an app restart and presented 3,042 buffers. The difference between Xlib draws and posted Android buffers includes coalescing upstream of the compositor.

**Confirmed by code inspection.** At 1920×1080 RGBA8888, the previous native path explicitly copied 15.82 MiB per changed framebuffer across its staging and window buffers. The direct path copies 7.91 MiB once. Canvas additionally stages through a direct JNI buffer and a Bitmap before drawing. These numbers describe explicit application copies. They exclude decoding, X server traffic and platform compositor copies.

The paired native producer log counted 367 successful posts and copied 3,044,044,800 bytes. Mean measured CPU copy time was 0.56 ms per post. The native submission path averaged 3.01 ms including geometry checks, buffer lock, copy and post. Producer counters cover the surface lifetime, while the compositor table covers only the workload collection window.

The input probe focused the X11 stimulus window and received two key presses, three button presses and 33 motion events after injecting Android touch, mouse and keyboard input. Sending the app Home and resuming it created a new BLAST layer while preserving the Xvnc and XFCE identities. The foreground viewport switched from 1920×1080 to 1280×720 through the resize button.

The final Canvas build also kept the same guest processes through a landscape-to-portrait rotation and back. After resizing the guest to 1280×720 it rendered another Xlib workload on the same session.

The JNI ImageReader test measured 668 ms from launch to server readiness and 1,003 ms from launch to receipt of the first native image at 1280×720. It then recreated the surface and received a resized 1600×900 image over the same desktop session. ImageReader is an offscreen consumer. These timings do not include a physical display post.

**Unknown.** Physical input-to-photon time and per-app GPU utilization were outside the available measurement path. The compositor acquire-to-present percentile measures a later stage and cannot stand in for input latency. The test exercises a single device and an intentionally synthetic 30 Hz source.

## Historical drag benchmark

The historical experiment used a different workload and a mixture of presentation counters. Its results retain context for the original 11 FPS report.

| Backend / Stage | Resolution | App CPU % | App RSS (MiB) | UI-Thread `gfxinfo` Frames / Missed Vsync | Server Updates (5s drag) | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| **A: Termux + Termux:X11 (`tx11start`)** | Native | `0.9%` + `11.7%` | `169` + `252` | `0` / `0` | N/A (`libXlorie` + `BLAST`) | Reference baseline; requires external `com.termux` + `com.termux.x11`. |
| **B: Baseline `Xvnc` + Kotlin `Bitmap`/`Canvas`** | `1280x720` | `45.3%` | `303.7` | `416` / `15` | `72` updates (`325` rects) | Measured `11 FPS` during drag. |
| **D1: Stage 2 `DirectByteBuffer` + 32bpp Fast Path** | `1280x720` | `36.1%` | `251.2` | `20` / `0` | `101` updates (`628` rects) | `-20.3%` CPU, `-52.5 MiB` RSS, `0` missed Vsync. |
| **D2: Stage 3 Rust `ANativeWindow` Native Blit** | `1920x1080` | `22.2%` | `257.1` | `14` / `0` | `171` updates (`718` rects, `9.36 Mpx`) | `-51.0%` CPU vs Baseline at `2.25x` pixels; `23 fps` HUD during drag; dynamic resize (`1280x720`) verified. |
