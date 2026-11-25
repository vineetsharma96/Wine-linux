# Input, Keyboard, Clipboard, Audio, Security & Performance

## Input System (`docs/INPUT_SYSTEM.md` summary)
- **Direct Touch**: Tap maps screen coordinates (accounting for zoom) to guest `(x, y)` left-click; long-press sends right-click (button mask `4`); drag sends continuous button-1 motion.
- **Trackpad & Relative Modes**: Drag moves the guest pointer without holding left button (`1.0x` in Trackpad, `1.5x` in Relative); tap/long-press clicks at the current pointer position.
- **Physical Mouse**: `PointerEventType.Move/Press/Release/Scroll` maps primary/secondary/tertiary (`isTertiaryPressed`) buttons and vertical scroll wheel (`8` / `16`).

## Keyboard Support
- Soft keyboard input via `IME` (`BasicTextField` bridge) and physical keyboard via `onPreviewKeyEvent` translate Android keycodes and Unicode characters to X11 `KeySym`s.
- Floating toolbar provides sticky `Ctrl`, `Alt`, `Shift`, plus `Super`, `Esc`, `Tab`, `Enter`, `Bksp`, arrow keys, and `F1`/`F2`/`F4`/`F5`/`F11`.
- `BackHandler` intercepts Android Back with a confirmation dialog (`Disconnect Desktop?`) to prevent accidental session termination.

## Audio Engine
- `AudioTcpSink` (`127.0.0.1:4713`) + `AudioBridgePlayer` (`AudioTrack` `44.1 kHz` stereo `PCM_16BIT`) with `setMuted()` toggle. Guest desktop sets `PULSE_SERVER=127.0.0.1:4713`. Audio runs independently and does not block display startup.

## Display Security & Performance
- `Xvnc` binds exclusively to `-interface 127.0.0.1` (`-ac -pn`).
- `RfbClient` validates dimensions (`<= 8192x8192`), `Raw` rect sizes (`<= 64 MiB`), and clipboard sizes (`<= 1 MiB` server / `256 KiB` client).
- `SurfaceView` rendering avoids per-frame Compose recomposition; measured 11 FPS on `6e5b28dd` during interactive XFCE session at `1280x720`.
