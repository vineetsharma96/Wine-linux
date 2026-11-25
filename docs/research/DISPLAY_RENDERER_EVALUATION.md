# Stage 3.2 display renderer evaluation

| Candidate | Performance and compatibility | Integration and maintenance | Decision |
| --- | --- | --- | --- |
| Lorie/Xlorie in the APK | Its X.Org DDX and Android graphics path share graphics buffers. No independent build of that path ran in this app. | Integrating the Termux:X11 source distribution means maintaining its X.Org build and satisfying GPL-3.0 distribution obligations alongside the application's current licensing. Java, native X server, and Android integration are coupled. | Leave external. Use upstream as a reference for shared buffers, fences, RandR, input, and clipboard. |
| Rust and ANativeWindow | One CPU copy from the decoded framebuffer to a native surface buffer after RFB decoding. Available from the NDK and verified on the primary Android ARM64 device. | Rust owns rendering and framing. Kotlin binds the Surface lifetime to the presenter. The separate implementation boundary is the presenter. | Primary. Keep Canvas over the same RFB connection for compatibility. |
| libandroid-shmem | Emulates a System V IPC primitive. It can help a local X11 client and server share memory if both implement that path. The existing RFB stream still requires decoding. | BSD-3-Clause upstream. Adding it to this renderer would require a new producer and transport contract, ownership, and synchronization. | Leave it outside the display data path. |
| Termux:X11 external | Installed companion workflow runs on the tablet. Its measurements include the external Termux and X server environment. | It owns its own processes, input and clipboard receivers. Introducing it as the default would change the app's rootfs ownership and lifecycle contract. | Retain the optional external launch. |

**Confirmed.** RFB remains the display transport for both built-in presenters. Touch, pointer and keyboard events continue through RFB and guest resize continues through RandR. Clipboard uses RFB cut-text. The native presenter only submits dirty generations and the request stream keeps one incremental request in flight. [Android renderer](../../rust/crates/android-bridge/src/native_display.rs) · [RFB client](../../rust/crates/display/src/client.rs)

**Experimentally verified.** The evaluation starts from the checked-in ANativeWindow prototype, including its staging copy. The measurements and device configuration are in the benchmark record. [Stage 3.2 benchmark protocol](DISPLAY_BENCHMARKS.md#stage-32-protocol)

**Hypothesis.** EGL with damage-aware uploads may help workloads that update small rectangles or need shader scaling. Vulkan or AHardwareBuffer sharing would be more useful if a producer can hand off frames in a shareable format. These paths need paired measurements that include uploads, fences and compositor behavior.

**Unknown.** No glass-to-glass input latency, cross-device thermal curve, or integrated-Xlorie result was established by this evaluation.
