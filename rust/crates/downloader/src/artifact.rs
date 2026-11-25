use std::path::Path;
use serde::{Deserialize, Serialize};
use androidpc_runtime_api::{Architecture, ArtifactProvider, RuntimeError};
use crate::verifier::Sha256Verifier;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RootfsManifest {
    schema_version: u32,
    rootfs: Vec<RootfsEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RootfsEntry {
    distribution: String,
    release: String,
    artifact_version: String,
    architecture: Architecture,
    filename: String,
    url: String,
    archive_format: String,
    sha256: String,
    size_bytes: u64,
    min_app_version_code: u32,
}

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
    expected_releases: std::collections::HashMap<String, String>,
}

impl Default for DefaultArtifactProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultArtifactProvider {
    pub fn from_signed_rootfs(
        manifest: &[u8],
        signature: &[u8],
        public_key: &str,
        app_version_code: u32,
    ) -> Result<Self, RuntimeError> {
        let invalid = |reason: &str| RuntimeError::InitializationFailed(format!("Rootfs catalog: {reason}"));
        if manifest.len() > 64 * 1024 || !androidpc_security::verify_ed25519_manifest(manifest, signature, public_key) {
            return Err(invalid("release signature verification failed"));
        }
        let manifest: RootfsManifest = serde_json::from_slice(manifest)?;
        if manifest.schema_version != 1 || manifest.rootfs.len() != 2 {
            return Err(invalid("expected schema 1 with Ubuntu and Debian artifacts"));
        }
        let mut provider = Self::new();
        provider.catalog.retain(|item| !matches!(item.distribution.as_str(), "ubuntu" | "debian"));
        for entry in manifest.rootfs {
            let token = |value: &str| !value.is_empty() && value.len() <= 128
                && value.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'));
            if !matches!(entry.distribution.as_str(), "ubuntu" | "debian")
                || entry.architecture != Architecture::Aarch64
                || provider.catalog.iter().any(|item| item.distribution == entry.distribution)
                || !token(&entry.release) || !token(&entry.artifact_version) || !token(&entry.filename)
                || !matches!(entry.archive_format.as_str(), "tar.gz" | "tar.xz")
                || !entry.filename.ends_with(&format!(".{}", entry.archive_format))
                || !entry.url.starts_with("https://") || entry.url.len() > 2048
                || !entry.url[8..].split('/').next().is_some_and(|host| {
                    host.contains('.') && host.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'))
                })
                || entry.url.bytes().any(|b| !b.is_ascii_graphic() || matches!(b, b'@' | b'\\' | b'#'))
                || entry.size_bytes == 0
                || entry.min_app_version_code == 0 || entry.min_app_version_code > app_version_code
                || entry.sha256.len() != 64 || !entry.sha256.bytes().all(|b| b.is_ascii_hexdigit())
                || entry.sha256.bytes().all(|b| b == b'0')
            {
                return Err(invalid("invalid or incompatible artifact fields"));
            }
            provider.expected_releases.insert(entry.distribution.clone(), entry.release.clone());
            provider.catalog.push(ArtifactMetadata {
                name: format!("{}-{}-{}", entry.distribution, entry.release, entry.artifact_version),
                distribution: entry.distribution,
                architecture: entry.architecture,
                filename: entry.filename,
                url: entry.url,
                sha256: entry.sha256.to_ascii_lowercase(),
                size_bytes: entry.size_bytes,
            });
        }
        Ok(provider)
    }

    pub fn new() -> Self {
        Self {
            expected_releases: [("ubuntu".to_string(), "26.04".to_string()), ("debian".to_string(), "13".to_string())].into(),
            catalog: vec![
                // ARM64 (aarch64) verified upstream releases
                ArtifactMetadata {
                    name: "ubuntu-26.04-arm64".to_string(),
                    distribution: "ubuntu".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "ubuntu-base-26.04.1-base-arm64.tar.gz".to_string(),
                    url: "https://cdimage.ubuntu.com/ubuntu-base/releases/26.04/release/ubuntu-base-26.04.1-base-arm64.tar.gz".to_string(),
                    sha256: "5a1906794ced63a71a8119c3f211ef5f0bbe0a243001b4bbd41fdf80c5b219fd".to_string(),
                    size_bytes: 35_092_106,
                },
                ArtifactMetadata {
                    name: "debian-13-arm64".to_string(),
                    distribution: "debian".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "debian-13.8-slim-arm64.tar.gz".to_string(),
                    url: "https://raw.githubusercontent.com/debuerreotype/docker-debian-artifacts/657d5664bd8e7d6833fa48225b8110774cc17666/trixie/slim/oci/blobs/rootfs.tar.gz".to_string(),
                    sha256: "48fb00926fe468d83ca04ee6775f36350d17e0f328dfcb03aadb5de0c52ae3f6".to_string(),
                    size_bytes: 30_198_034,
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
                    name: "box64-0.4.4-arm64".to_string(),
                    distribution: "box64".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "box64-v0.4.4-linux-arm64.tar.gz".to_string(),
                    url: "https://github.com/ptitSeb/box64/archive/refs/tags/v0.4.4.tar.gz".to_string(),
                    sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    size_bytes: 0,
                },
                ArtifactMetadata {
                    name: "wine-11.0-wow64-amd64".to_string(),
                    distribution: "wine".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "wine-11.0-wow64-amd64.tar.xz".to_string(),
                    url: "https://dl.winehq.org/wine/source/11.0/wine-11.0.tar.xz".to_string(),
                    sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    size_bytes: 0,
                },
                ArtifactMetadata {
                    name: "mesa-turnip-26.3.1-arm64".to_string(),
                    distribution: "turnip".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "mesa-26.3.1-turnip-arm64.tar.gz".to_string(),
                    url: "https://archive.mesa3d.org/mesa-26.3.1.tar.xz".to_string(),
                    sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    size_bytes: 0,
                },
                ArtifactMetadata {
                    name: "virglrenderer-1.3.0-arm64".to_string(),
                    distribution: "virgl".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "virglrenderer-1.3.0-arm64.tar.gz".to_string(),
                    url: "https://gitlab.freedesktop.org/virgl/virglrenderer/-/archive/1.3.0/virglrenderer-1.3.0.tar.gz".to_string(),
                    sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    size_bytes: 0,
                },
                ArtifactMetadata {
                    name: "dxvk-2.7.1".to_string(),
                    distribution: "dxvk".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "dxvk-2.7.1.tar.gz".to_string(),
                    url: "https://github.com/doitsujin/dxvk/releases/download/v2.7.1/dxvk-2.7.1.tar.gz".to_string(),
                    sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    size_bytes: 0,
                },
                ArtifactMetadata {
                    name: "vkd3d-proton-3.1".to_string(),
                    distribution: "vkd3d-proton".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "vkd3d-proton-3.1.tar.zst".to_string(),
                    url: "https://github.com/HansKristian-Work/vkd3d-proton/releases/download/v3.1/vkd3d-proton-3.1.tar.zst".to_string(),
                    sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    size_bytes: 0,
                },
                ArtifactMetadata {
                    name: "qemu-11.1.3-arm64".to_string(),
                    distribution: "qemu".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "qemu-11.1.3.tar.xz".to_string(),
                    url: "https://download.qemu.org/qemu-11.1.3.tar.xz".to_string(),
                    sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    size_bytes: 0,
                },
                ArtifactMetadata {
                    name: "edk2-aavmf-202608".to_string(),
                    distribution: "qemu-firmware".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "edk2-stable202608-aavmf.tar.gz".to_string(),
                    url: "https://github.com/tianocore/edk2/archive/refs/tags/edk2-stable202608.tar.gz".to_string(),
                    sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    size_bytes: 0,
                },
                ArtifactMetadata {
                    name: "edk2-ovmf-202608".to_string(),
                    distribution: "qemu-firmware".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "edk2-stable202608-ovmf.tar.gz".to_string(),
                    url: "https://github.com/tianocore/edk2/archive/refs/tags/edk2-stable202608.tar.gz".to_string(),
                    sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    size_bytes: 0,
                },
                ArtifactMetadata {
                    name: "libslirp-4.9.7".to_string(),
                    distribution: "libslirp".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "libslirp-v4.9.7.tar.gz".to_string(),
                    url: "https://gitlab.freedesktop.org/slirp/libslirp/-/archive/v4.9.7/libslirp-v4.9.7.tar.gz".to_string(),
                    sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                    size_bytes: 0,
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

    pub fn expected_release(&self, distro: &str) -> Option<&str> {
        self.expected_releases.get(distro).map(String::as_str)
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

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn signed_catalog_rejects_tampering_duplicate_and_incompatible_artifacts() {
        let key = SigningKey::from_bytes(&[42; 32]);
        let public_key = hex::encode(key.verifying_key().as_bytes());
        let manifest = |guest: &str, minimum_version: u32| {
            let artifact = |distribution| serde_json::json!({
                "distribution": distribution, "release": "1", "artifact_version": "1",
                "architecture": "aarch64", "filename": "example.tar.gz",
                "url": "https://example.com/example.tar.gz", "archive_format": "tar.gz",
                "sha256": "b".repeat(64), "size_bytes": 123, "min_app_version_code": minimum_version
            });
            serde_json::to_vec(&serde_json::json!({
                "schema_version": 1, "rootfs": [artifact("ubuntu"), artifact(guest)]
            })).unwrap()
        };
        let bytes = manifest("debian", 1);
        let sig = key.sign(&bytes).to_bytes();
        let catalog = DefaultArtifactProvider::from_signed_rootfs(&bytes, &sig, &public_key, 1).unwrap();
        assert_eq!(catalog.find_by_distribution("debian", Architecture::Aarch64).unwrap().size_bytes, 123);

        for invalid in [manifest("ubuntu", 1), manifest("debian", 2)] {
            let invalid_sig = key.sign(&invalid).to_bytes();
            assert!(DefaultArtifactProvider::from_signed_rootfs(&invalid, &invalid_sig, &public_key, 1).is_err());
        }
        let mut changed = bytes;
        changed.push(b' ');
        assert!(DefaultArtifactProvider::from_signed_rootfs(&changed, &sig, &public_key, 1).is_err());
        assert!(DefaultArtifactProvider::from_signed_rootfs(&changed, &sig[..63], &public_key, 1).is_err());
    }
}
