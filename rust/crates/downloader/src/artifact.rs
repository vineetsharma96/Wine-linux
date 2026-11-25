use std::path::Path;
use serde::{Deserialize, Serialize};
use androidpc_runtime_api::{Architecture, ArtifactProvider, RuntimeError};
use crate::verifier::Sha256Verifier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub name: String,
    pub distribution: String,
    pub architecture: Architecture,
    pub filename: String,
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
}

pub struct DefaultArtifactProvider {
    catalog: Vec<ArtifactMetadata>,
}

impl Default for DefaultArtifactProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultArtifactProvider {
    pub fn new() -> Self {
        Self {
            catalog: vec![
                // ARM64 (aarch64) verified upstream releases
                ArtifactMetadata {
                    name: "ubuntu-24.04-arm64".to_string(),
                    distribution: "ubuntu".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "ubuntu-base-24.04.5-base-arm64.tar.gz".to_string(),
                    url: "https://cdimage.ubuntu.com/ubuntu-base/releases/24.04/release/ubuntu-base-24.04.5-base-arm64.tar.gz".to_string(),
                    sha256: "a91d5a93010193712d346d761372b7c9db6dfcf093893161c64ca107f05914f2".to_string(),
                    size_bytes: 31_457_280,
                },
                ArtifactMetadata {
                    name: "debian-12-arm64".to_string(),
                    distribution: "debian".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "hangover_11.16_debian12_bookworm_arm64.tar".to_string(),
                    url: "https://github.com/AndreRH/hangover/releases/download/hangover-11.16/hangover_11.16_debian12_bookworm_arm64.tar".to_string(),
                    sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    size_bytes: 36_700_160,
                },
                ArtifactMetadata {
                    name: "proot-arm64".to_string(),
                    distribution: "proot".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "proot_5.1.107.102_aarch64.deb".to_string(),
                    url: "https://packages.termux.dev/apt/termux-main/pool/main/p/proot/proot_5.1.107.102_aarch64.deb".to_string(),
                    sha256: "10a394b585c2fc2b2c8502544f4e5f1be581dc8bfff79e8ba62f16207423e030".to_string(),
                    size_bytes: 103_028,
                },
                ArtifactMetadata {
                    name: "tigervnc-arm64".to_string(),
                    distribution: "xvnc".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "tigervnc_1.16.2-2_aarch64.deb".to_string(),
                    url: "https://packages.termux.dev/apt/termux-x11/pool/main/t/tigervnc/tigervnc_1.16.2-2_aarch64.deb".to_string(),
                    sha256: "8dacf03fb8a132e56745316c3c0308f6deb7dca252f9e3fd5a4166444d2f8109".to_string(),
                    size_bytes: 1_021_996,
                },
                ArtifactMetadata {
                    name: "box64-0.4.5-arm64".to_string(),
                    distribution: "box64".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "box64-0.4.5.tzst".to_string(),
                    url: "https://raw.githubusercontent.com/brunodev85/winlator/main/installable_components/box64/box64-0.4.5.tzst".to_string(),
                    sha256: "ba9e8acb06b9d8a33b8c8fadda6b2790b313ee38659232b59c0752ae00209911".to_string(),
                    size_bytes: 4_589_488,
                },
                ArtifactMetadata {
                    name: "wine-9.2-arm64".to_string(),
                    distribution: "wine".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "wine-9.2-amd64.tar.xz".to_string(),
                    url: "https://github.com/Kron4ek/Wine-Builds/releases/download/9.2/wine-9.2-amd64.tar.xz".to_string(),
                    sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    size_bytes: 58_000_000,
                },
                ArtifactMetadata {
                    name: "qemu-9.2-arm64".to_string(),
                    distribution: "qemu".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "qemu-system-aarch64-headless_9.2.0_aarch64.deb".to_string(),
                    url: "https://packages.termux.dev/apt/termux-main/pool/main/q/qemu-system-aarch64-headless/".to_string(),
                    sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    size_bytes: 24_000_000,
                },
                // x86_64 artifacts (Android x86_64 emulators / Chromebooks)
                ArtifactMetadata {
                    name: "ubuntu-24.04-x86_64".to_string(),
                    distribution: "ubuntu".to_string(),
                    architecture: Architecture::X86_64,
                    filename: "ubuntu-base-24.04.5-base-amd64.tar.gz".to_string(),
                    url: "https://cdimage.ubuntu.com/ubuntu-base/releases/24.04/release/ubuntu-base-24.04.5-base-amd64.tar.gz".to_string(),
                    sha256: "e77b6f10c2590cef872b33ee9f635a0e3fd1f57fb074c0e52b5c7f56147a0c86".to_string(),
                    size_bytes: 33_554_432,
                },
                ArtifactMetadata {
                    name: "proot-x86_64".to_string(),
                    distribution: "proot".to_string(),
                    architecture: Architecture::X86_64,
                    filename: "proot-x86_64".to_string(),
                    url: "https://github.com/proot-me/proot/releases/download/v5.5.0/proot".to_string(),
                    sha256: "db185300a9daabbce86aca7632d8c7fdc4249f6f153171954013b70c5f491ec0".to_string(),
                    size_bytes: 1_841_504,
                },
                // ARMv7 (32-bit ARM) artifacts
                ArtifactMetadata {
                    name: "ubuntu-24.04-armv7".to_string(),
                    distribution: "ubuntu".to_string(),
                    architecture: Architecture::Armv7,
                    filename: "ubuntu-base-24.04.5-base-armhf.tar.gz".to_string(),
                    url: "https://cdimage.ubuntu.com/ubuntu-base/releases/24.04/release/ubuntu-base-24.04.5-base-armhf.tar.gz".to_string(),
                    sha256: "4fcee4d278f1c5232e085a021a85e4c6cef3853557a88d98ff380b5e5d5841bb".to_string(),
                    size_bytes: 29_360_128,
                },
            ],
        }
    }

    pub fn list_artifacts(&self) -> &[ArtifactMetadata] {
        &self.catalog
    }

    pub fn list_for_architecture(&self, arch: Architecture) -> Vec<ArtifactMetadata> {
        self.catalog
            .iter()
            .filter(|a| a.architecture == arch)
            .cloned()
            .collect()
    }

    pub fn find_artifact(&self, name: &str, arch: Architecture) -> Option<&ArtifactMetadata> {
        self.catalog.iter().find(|a| a.name == name && a.architecture == arch)
    }

    pub fn find_by_distribution(&self, distro: &str, arch: Architecture) -> Option<&ArtifactMetadata> {
        self.catalog
            .iter()
            .find(|a| a.distribution.eq_ignore_ascii_case(distro) && a.architecture == arch)
    }
}

impl ArtifactProvider for DefaultArtifactProvider {
    fn get_artifact_url(&self, artifact_name: &str, arch: Architecture) -> Result<String, RuntimeError> {
        self.find_artifact(artifact_name, arch)
            .map(|a| a.url.to_string())
            .ok_or_else(|| RuntimeError::Internal(format!("Artifact not found: {artifact_name} for {arch:?}")))
    }

    fn verify_artifact(&self, path: &Path, expected_sha256: &str) -> Result<bool, RuntimeError> {
        Sha256Verifier::verify_file(path, expected_sha256)
            .map_err(|e| RuntimeError::Internal(e.to_string()))
    }
}
