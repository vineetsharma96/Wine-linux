# Display Benchmarks & Bottleneck Profiling

## 1. Exact Root Causes of 11 FPS in Baseline B

Inspection and profiling of `rust/crates/display/src/client.rs`, `rust/crates/core/src/lib.rs`, `rust/crates/android-bridge/src/lib.rs`, and `DisplayViewportScreen.kt` identified **five compounding bottlenecks**:

1. **Per-Frame 3.68 MB Heap Allocation in JNI (`nativeGetFramebuffer`)**:
   - `android-bridge/src/lib.rs:1157` executes `let mut temp_buf = vec![0u8; array_len];` on **every** poll cycle, zero-filling and dropping `3.68 MB` (`1280x720x4`) or `5.76 MB` (`1600x900x4`) per frame.
2. **4× Redundant Full-Frame Copies Even When 0 Pixels Changed**:
   - `EngineCore::get_framebuffer_slice` copies `client.framebuffer` unconditionally even when `poll_updates()` decoded `0` rectangles.
   - `nativeGetFramebuffer` -> `SetByteArrayRegion(byteArray)` -> `ByteBuffer.put(pixelArray)` -> `Bitmap.copyPixelsFromBuffer(byteBuffer)` -> `SurfaceHolder.lockCanvas()/drawBitmap()` copies `~18.4 MB/frame` (`~202 MB/s` at 11 FPS).
3. **Uncompressed `Raw` Encoding + Per-Pixel Scalar Division**:
   - `RfbClient::connect` negotiates only `Raw (0)`, `CopyRect (1)`, `DesktopSize (-223)`. `Xvnc` sent `18.368 MiB` at `1:1` ratio in ~7 seconds.
   - `RfbClient::decode_raw_rect` allocates a new `Vec<u8>` per rectangle and runs 3 integer divisions (`((r_raw * 255) / red_max)`) per pixel (`2.76M` divisions per `1280x720` frame) even for standard 32bpp `rgb888`.
4. **Synchronous Network + Render Loop + Fixed `delay(33)`**:
   - `DisplayViewportScreen.kt` runs network `poll_updates()` (with `8 ms` socket read timeout), `FramebufferUpdateRequest`, 4× buffer copies, `lockCanvas`/`unlockCanvasAndPost`, and then unconditionally sleeps `delay(33)`.
   - Total loop period: `~35 ms` decode/network + `~20 ms` copies + `~10 ms` `drawBitmap` + `33 ms` sleep = `~98 ms` (**~10.2–11 FPS**).
5. **Compose Recomposition During Pointer Input**:
   - `statusMessage = "Drag: ..."` mutates Compose state on every pointer move, causing `416` UI-thread frames (`14.66%` jank, `15` missed Vsync) during a 5-second drag.

## 2. Command Availability (`tx11start` / `tx11stop`)
- Verified on `6e5b28dd`: `com.termux` (`0.118.3`) has `/data/data/com.termux/files/usr/bin/tx11start` (`6396` bytes) and `/data/data/com.termux/files/usr/bin/tx11stop` (`1920` bytes), and `com.termux.x11` (`1.03.01-d55f1b7-28.09.26`) is installed.
- Both `tx11start` and `tx11stop` executed cleanly in the Termux benchmark (`task-3110`).
