use crate::config::{AudioVmConfig, DisplayVmConfig, VmArch, VmAudioBackend, VmDisplayBackend};

/// Builds QEMU display, input, and audio CLI arguments integrated with the existing
/// AndroidPC RFB (`androidpc-display`) and Loopback Audio (`androidpc-audio`) subsystems.
pub struct VmDisplayAudioBuilder;

impl VmDisplayAudioBuilder {
    pub fn build_display_and_input_args(arch: VmArch, display: &DisplayVmConfig) -> Vec<String> {
        let mut args = Vec::new();

        match display.backend {
            VmDisplayBackend::HeadlessSerial => {
                args.extend(["-display".to_string(), "none".to_string()]);
                args.extend(["-nographic".to_string()]);
                return args;
            }
            VmDisplayBackend::VncLocalhost => {
                // Attach virtual GPU device appropriate for guest architecture
                let gpu = if display.gpu_device.trim().is_empty() {
                    match arch {
                        VmArch::Aarch64 => "virtio-gpu-pci",
                        VmArch::X86_64 => "virtio-vga",
                    }
                } else {
                    display.gpu_device.trim()
                };
                match gpu {
                    "virtio-gpu-pci" => {
                        args.extend([
                            "-device".to_string(),
                            format!(
                                "virtio-gpu-pci,xres={},yres={}",
                                display.width, display.height
                            ),
                        ]);
                    }
                    "virtio-vga" => {
                        args.extend([
                            "-device".to_string(),
                            format!(
                                "virtio-vga,xres={},yres={}",
                                display.width, display.height
                            ),
                        ]);
                    }
                    "ramfb" => {
                        args.extend(["-device".to_string(), "ramfb".to_string()]);
                    }
                    other => {
                        args.extend(["-vga".to_string(), other.to_string()]);
                    }
                }

                // Bind VNC strictly to localhost loopback (ports 5910+ to never collide with XFCE :1 / 5901)
                args.extend([
                    "-display".to_string(),
                    format!("vnc=127.0.0.1:{}", display.vnc_display),
                ]);

                // Attach USB XHCI controller + absolute USB tablet + keyboard for accurate Android touch/trackpad mapping
                if display.enable_usb_tablet {
                    args.extend(["-device".to_string(), "qemu-xhci,id=xhci".to_string()]);
                    args.extend(["-device".to_string(), "usb-tablet,bus=xhci.0".to_string()]);
                    args.extend(["-device".to_string(), "usb-kbd,bus=xhci.0".to_string()]);
                }
            }
        }

        args
    }

    pub fn build_audio_args(audio: &AudioVmConfig) -> Vec<String> {
        match audio.backend {
            VmAudioBackend::Disabled => {
                vec!["-audiodev".to_string(), "none,id=snd0".to_string()]
            }
            VmAudioBackend::PulseLoopback => {
                let mut args = vec![
                    "-audiodev".to_string(),
                    format!("pa,id=snd0,server={}", audio.pulse_server),
                ];
                match audio.device_model.as_str() {
                    "virtio-sound" => {
                        args.extend([
                            "-device".to_string(),
                            "virtio-sound-pci,audiodev=snd0".to_string(),
                        ]);
                    }
                    _ => {
                        args.extend(["-device".to_string(), "intel-hda".to_string()]);
                        args.extend([
                            "-device".to_string(),
                            "hda-duplex,audiodev=snd0".to_string(),
                        ]);
                    }
                }
                args
            }
            VmAudioBackend::WavDebug => {
                vec![
                    "-audiodev".to_string(),
                    "wav,id=snd0,path=vm_audio_debug.wav".to_string(),
                    "-device".to_string(),
                    "intel-hda".to_string(),
                    "-device".to_string(),
                    "hda-duplex,audiodev=snd0".to_string(),
                ]
            }
        }
    }
}
