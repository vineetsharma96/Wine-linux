# Virtual Machine Audio (`VM_AUDIO.md`)

## 1. Modular Audio Architecture (`rust/crates/vm/src/display_audio.rs`)
QEMU VM audio is designed as an optional, cleanly decoupled backend controlled per VM profile (`AudioVmConfig`):

| Backend | QEMU CLI Configuration | Status & Notes |
|---|---|---|
| **`Disabled` (Default)** | `-audiodev none,id=snd0` | Zero overhead, zero audio-device stalls during boot |
| **`PulseLoopback`** | `-audiodev pa,id=snd0,server=tcp:127.0.0.1:4713` + `intel-hda`/`hda-duplex` (or `virtio-sound-pci`) | Connects to existing `androidpc-audio` `AudioTcpSink` + Android `AudioBridgePlayer` (`AudioTrack`) |
| **`WavDebug`** | `-audiodev wav,id=snd0,path=vm_audio_debug.wav` + `intel-hda`/`hda-duplex` | Offline PCM capture inside VM directory for deterministic debugging |

## 2. Explicit Platform Limitations
1. **QEMU build flag prerequisite**: `PulseLoopback` requires QEMU `11.1.3` compiled with `--audio-drv-list=pa,wav` and `libpulse` client support (`scripts/build_qemu_arm64.sh`).
2. **Protocol mismatch caveat**: The existing Phase 1–3 `AudioTcpSink` accepts raw PCM over `127.0.0.1:4713`; full PulseAudio protocol negotiation requires either a PulseAudio TCP server bridge or a dedicated raw-PCM QEMU audio driver. Accordingly, VM audio defaults to **`Disabled`** and is explicitly marked **unverified on device (`NOT RUN`)** until tested end-to-end by the independent testing agent.
3. **Android audio focus & latency**: On-device playback is subject to Android `AudioTrack` buffering, Doze power restrictions, and TCG CPU load (guest audio underruns are expected under heavy TCG emulation).
