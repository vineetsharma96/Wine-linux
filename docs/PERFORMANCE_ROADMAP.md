# Display Performance Roadmap

## Stage 1 — Investigation & Baseline (Completed)
- Reproduced 11 FPS on Xiaomi Pad 7 (`2410CRP4CI`, Android 16, 120 Hz) and benchmarked against Termux:X11 (`tx11start` / `tx11stop`).
- Identified 5 bottlenecks: per-frame `3.68 MB` JNI `Vec` allocation, 4× full-frame copies even on idle frames, `Raw`-only encoding + per-pixel integer division, synchronous network/render loop + `delay(33)`, and Compose recomposition on drag.

## Stage 2 — Optimized VNC Pipeline (Target: 30–60 FPS)
1. Dedicated Rust RFB reader thread + generation counter + dirty-skip.
2. Fast-path 32bpp pixel conversion (zero heap allocs, zero divisions for `8888`).
3. Direct `ByteBuffer` / `ANativeWindow` transfer (eliminate `vec!` + `ByteArray` + `Bitmap` intermediate copies).
4. Decouple Compose HUD state from pointer drag events.

## Stage 3 — Termux:X11 Backend & Future Native `Lorie` Renderer
1. Expose Termux:X11 external backend selector & onboarding (`DISPLAY=:0` / `tx11start` / `tx11stop` guidance).
2. Evaluate bundling `libXlorie` + `libandroid-shmem` + `ANativeWindow` in-app for 60–120 FPS self-contained X11 rendering without external APKs.
