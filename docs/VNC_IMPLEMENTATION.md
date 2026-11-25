# VNC / RFB Implementation (`rust/crates/display/src/client.rs`)

- **Protocol Version**: RFB `003.008` (`RFB_VERSION_3_8`).
- **Security Negotiation**: Enforces `RfbSecurityType::None` (`1`) strictly on loopback (`127.0.0.1`) and rejects unsupported security types (no silent fallback).
- **Encodings**: Requests `Raw (0)`, `CopyRect (1)`, and `DesktopSize (-223)`.
- **Clipboard**: Supports `ClientCutText (6)` (`send_clipboard`) and `ServerCutText (3)` (`handle_server_cut_text` / `take_server_clipboard`).
- **Bounds & Overflow Protection**:
  - Rejects `width == 0`, `height == 0`, or dimensions `> 8192` (`MAX_FRAMEBUFFER_DIM`).
  - Uses `checked_mul` for framebuffer and `Raw` rectangle byte lengths; rejects `Raw` rects `> 64 MiB` and `ServerCutText` payloads `> 1 MiB`.
