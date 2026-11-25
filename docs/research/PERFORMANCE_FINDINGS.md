# Performance Findings Summary

- **Reference Device**: Xiaomi Pad 7 (`2410CRP4CI`, Android 16, Kernel `6.1.138`, `2136x3200` @ `120 Hz`).
- **Root Causes of 11 FPS**: (1) Per-frame `3.68 MB` `vec![]` allocation in `nativeGetFramebuffer`, (2) 4× redundant full-frame copies even on idle frames, (3) uncompressed `Raw` encoding + 3 scalar divisions per pixel, (4) synchronous TCP poll + `delay(33)` loop, (5) Compose `statusMessage` recompositions on pointer drag.
- **Stage 2 Optimization Results**: App CPU `45.3%` -> `36.1%` (`-20.3%`), App RSS `303.7 MiB` -> `251.2 MiB` (`-52.5 MiB`), UI-thread `gfxinfo` `416` frames (`15` missed Vsync) -> `20` frames (`0` missed Vsync), Xvnc updates `72` -> `101` (`+40.3%`).
- **Full Data**: See [PERFORMANCE_BASELINE.md](../PERFORMANCE_BASELINE.md) and [DISPLAY_BENCHMARKS.md](../DISPLAY_BENCHMARKS.md).
