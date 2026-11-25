# Virtual Machine Display & Input (`VM_DISPLAY.md`)

## 1. Reuse of Stage 3.2 Native Display Pipeline
Rather than introducing a competing renderer, Phase 5 QEMU VMs reuse the existing Stage 3.2 **Rust RFB `003.008` + Android `ANativeWindow`** display stack (`rust/crates/display`, `rust/crates/android-bridge/src/native_display.rs`, `DisplayViewportScreen.kt`):
- PRoot XFCE desktop binds strictly to `:1` (`127.0.0.1:5901`).
- QEMU VMs allocate independent localhost VNC displays starting at `:10` (`127.0.0.1:5910`, `5911`, ...), completely eliminating port collisions.
- Tapping **Display** on a VM card calls `onOpenVmDisplay(vncPort)`, routing `MainActivity.kt` into `DisplayViewportScreen` with `managedDesktop = false`, so disconnecting the VM viewport never interferes with PRoot XFCE.

## 2. Virtual GPU & Input Devices (`rust/crates/vm/src/display_audio.rs`)
| Guest Arch | Default Virtual GPU | Input Hardware | Resolution |
|---|---|---|---|
| **ARM64 (`aarch64`)** | `virtio-gpu-pci,xres=1280,yres=720` (optional `ramfb` for early UEFI fallback) | `qemu-xhci` + `usb-tablet` + `usb-kbd` | Configurable (default `1280x720`) |
| **x86-64 (`x86_64`)** | `virtio-vga,xres=1280,yres=720` (optional `std`/`bochs`) | `qemu-xhci` + `usb-tablet` + `usb-kbd` | Configurable (default `1280x720`) |
| **Headless Serial** | `-display none -nographic` | Serial console / SSH | N/A |

## 3. Input & Clipboard Behavior
- **Absolute Touch & Trackpad**: `usb-tablet` on `xhci.0` gives 1:1 absolute pointer alignment between Android touch/mouse coordinates in `DisplayViewportScreen` and the guest OS cursor, avoiding relative-pointer drift.
- **Keyboard & Modifiers**: Android soft IME, hardware keyboards, and modifier keys (`Ctrl`, `Alt`, `Shift`, `Super`, `F1..F11`) flow over RFB `KeyEvent` to QEMU's `usb-kbd`.
- **Clipboard**: RFB `ClientCutText` / `ServerCutText` is supported at the protocol layer when the guest OS runs a VNC-aware clipboard agent (`vncconfig` or SPICE/VNC helper); bare console guests do not share clipboard.
- **3D Acceleration Honesty**: Phase 5 uses 2D VirtIO/framebuffer rendering over localhost VNC. Hardware 3D `virgl` GPU passthrough inside QEMU on stock Android is **not** claimed or enabled in Phase 5.
