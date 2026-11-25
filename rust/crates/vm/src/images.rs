use serde::{Deserialize, Serialize};
use crate::config::{FirmwareKind, VmArch};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmGuestImageEntry {
    pub id: String,
    pub display_name: String,
    pub distribution: String,
    pub version: String,
    pub arch: VmArch,
    pub media_type: String,
    pub recommended_firmware: FirmwareKind,
    pub min_ram_mb: u32,
    pub min_disk_mb: u64,
    pub official_download_url: String,
    pub checksum_url: String,
    pub license: String,
    pub redistribution_policy: String,
    pub notes: String,
}

/// Curated catalog of official upstream guest OS images.
///
/// Android PC Environment NEVER bundles copyrighted operating systems or proprietary firmware.
/// Every entry links directly to the vendor's official HTTPS release and SHA-256 checksum feed.
pub struct VmImageCatalog;

impl VmImageCatalog {
    pub fn list_official_images() -> Vec<VmGuestImageEntry> {
        vec![
            VmGuestImageEntry {
                id: "alpine-virt-aarch64".to_string(),
                display_name: "Alpine Linux Virtual (ARM64)".to_string(),
                distribution: "alpine".to_string(),
                version: "3.21".to_string(),
                arch: VmArch::Aarch64,
                media_type: "iso".to_string(),
                recommended_firmware: FirmwareKind::AavmfUefi,
                min_ram_mb: 512,
                min_disk_mb: 1024,
                official_download_url: "https://alpinelinux.org/downloads/".to_string(),
                checksum_url: "https://dl-cdn.alpinelinux.org/alpine/latest-stable/releases/aarch64/latest-releases.yaml".to_string(),
                license: "MIT / GPL-2.0 (Open Source)".to_string(),
                redistribution_policy: "Official upstream HTTPS download + SHA-256 verification".to_string(),
                notes: "Lightweight ARM64 baseline recommended for validating QEMU TCG virt + VirtIO boot speed.".to_string(),
            },
            VmGuestImageEntry {
                id: "debian-13-netinst-arm64".to_string(),
                display_name: "Debian 13 Trixie Netinst (ARM64)".to_string(),
                distribution: "debian".to_string(),
                version: "13".to_string(),
                arch: VmArch::Aarch64,
                media_type: "iso".to_string(),
                recommended_firmware: FirmwareKind::AavmfUefi,
                min_ram_mb: 1024,
                min_disk_mb: 4096,
                official_download_url: "https://cdimage.debian.org/debian-cd/current/arm64/iso-cd/".to_string(),
                checksum_url: "https://cdimage.debian.org/debian-cd/current/arm64/iso-cd/SHA256SUMS".to_string(),
                license: "DFSG Open Source".to_string(),
                redistribution_policy: "Official Debian CD image + signed SHA256SUMS".to_string(),
                notes: "Full ARM64 Linux guest with kernel, systemd, and VirtIO drivers out of the box.".to_string(),
            },
            VmGuestImageEntry {
                id: "ubuntu-26.04-server-arm64".to_string(),
                display_name: "Ubuntu 26.04 LTS Server (ARM64)".to_string(),
                distribution: "ubuntu".to_string(),
                version: "26.04 LTS".to_string(),
                arch: VmArch::Aarch64,
                media_type: "iso".to_string(),
                recommended_firmware: FirmwareKind::AavmfUefi,
                min_ram_mb: 2048,
                min_disk_mb: 8192,
                official_download_url: "https://cdimage.ubuntu.com/releases/26.04/release/".to_string(),
                checksum_url: "https://cdimage.ubuntu.com/releases/26.04/release/SHA256SUMS".to_string(),
                license: "Ubuntu Open Source".to_string(),
                redistribution_policy: "Official Canonical release + signed SHA256SUMS".to_string(),
                notes: "Requires AAVMF UEFI firmware and >= 2 GB RAM.".to_string(),
            },
            VmGuestImageEntry {
                id: "alpine-virt-x86_64".to_string(),
                display_name: "Alpine Linux Virtual (x86-64 TCG Emulated)".to_string(),
                distribution: "alpine".to_string(),
                version: "3.21".to_string(),
                arch: VmArch::X86_64,
                media_type: "iso".to_string(),
                recommended_firmware: FirmwareKind::OvmfUefi,
                min_ram_mb: 512,
                min_disk_mb: 1024,
                official_download_url: "https://alpinelinux.org/downloads/".to_string(),
                checksum_url: "https://dl-cdn.alpinelinux.org/alpine/latest-stable/releases/x86_64/latest-releases.yaml".to_string(),
                license: "MIT / GPL-2.0 (Open Source)".to_string(),
                redistribution_policy: "Official upstream HTTPS download + SHA-256 verification".to_string(),
                notes: "Cross-architecture x86-64 TCG emulation on ARM64; suitable for testing q35 + OVMF/SeaBIOS, NOT native speed.".to_string(),
            },
            VmGuestImageEntry {
                id: "debian-13-netinst-amd64".to_string(),
                display_name: "Debian 13 Trixie Netinst (x86-64 TCG Emulated)".to_string(),
                distribution: "debian".to_string(),
                version: "13".to_string(),
                arch: VmArch::X86_64,
                media_type: "iso".to_string(),
                recommended_firmware: FirmwareKind::OvmfUefi,
                min_ram_mb: 1024,
                min_disk_mb: 4096,
                official_download_url: "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/".to_string(),
                checksum_url: "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/SHA256SUMS".to_string(),
                license: "DFSG Open Source".to_string(),
                redistribution_policy: "Official Debian CD image + signed SHA256SUMS".to_string(),
                notes: "Runs via QEMU TCG x86-64 software translation on ARM64 hosts.".to_string(),
            },
        ]
    }
}
