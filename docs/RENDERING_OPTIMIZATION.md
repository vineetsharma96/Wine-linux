# Rendering Optimization Plan

## 1. Rust Display Engine Optimizations (`androidpc-display`, `androidpc-core`, `androidpc-android-bridge`)
1. **Dedicated RFB Receiver/Decoder Thread**: Decouple TCP socket I/O and rectangle decoding from the Android render loop; coalesce dirty updates into a double-buffered RGBA framebuffer with a `dirty: AtomicBool` / generation counter.
2. **Zero-Allocation JNI Transfer (`GetDirectBufferAddress`)**: Replace `nativeGetFramebuffer(ByteArray)` (which allocates `vec![0u8; 3.68MB]` every frame) with a persistent `DirectByteBuffer` or `ANativeWindow_lock` write only when `generation` advances.
3. **Fast-Path 32bpp Pixel Conversion**: When `bits_per_pixel == 32` and `red_max == 255, green_max == 255, blue_max == 255` (the default `Xvnc` `rgb888` format), replace the per-pixel division loop and per-rect `Vec` allocation with direct slice/u32 conversion into the framebuffer.
4. **Dirty-Skip**: Return `0` immediately when no rectangles arrived since the last frame so Kotlin skips `copyPixelsFromBuffer` and `lockCanvas`.

## 2. Android Rendering Optimizations (`DisplayViewportScreen.kt`)
1. **Separate Render Loop from Compose State**: Remove `statusMessage = "Drag: ..."` updates on every pointer move; update HUD text only on discrete state changes or 1 Hz FPS ticks.
2. **Frame Pacing Without Fixed `delay(33)`**: Let the `SurfaceView` render loop block on a dirty/vsync signal or `16 ms` target cadence (`~60 FPS` ceiling) rather than sleeping `33 ms` after decode+copy.
3. **Future `ANativeWindow` Native Blit**: Pass `Surface` directly to Rust via `ANativeWindow_fromSurface` for zero-copy native buffer presentation.
