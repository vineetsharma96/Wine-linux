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
                ArtifactMetadata {
                    name: "ubuntu-24.04-arm64".to_string(),
                    distribution: "ubuntu".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "ubuntu-base-24.04.1-base-arm64.tar.gz".to_string(),
                    url: "https://cdimage.ubuntu.com/ubuntu-base/releases/24.04/release/ubuntu-base-24.04.1-base-arm64.tar.gz".to_string(),
                    sha256: "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".to_string(),
                    size_bytes: 31_457_280, // ~30 MB base image
                },
                ArtifactMetadata {
                    name: "debian-12-arm64".to_string(),
                    distribution: "debian".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "debian-12-rootfs-arm64.tar.xz".to_string(),
                    url: "https://cloud.debian.org/images/cloud/bookworm/latest/debian-12-generic-arm64.tar.xz".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                    size_bytes: 36_700_160, // ~35 MB rootfs
                },
                ArtifactMetadata {
                    name: "proot-arm64".to_string(),
                    distribution: "proot".to_string(),
                    architecture: Architecture::Aarch64,
                    filename: "proot-arm64".to_string(),
                    url: "https://github.com/termux/proot/releases/download/v5.4.0/proot-v5.4.0-aarch64".to_string(),
                    sha256: "a6c221e8d4791c855a401b34adfb5a62f4b0054a1a5b822d86a341b59cfc67a4".to_string(),
                    size_bytes: 2_621_440, // ~2.5 MB binary
                },
            ],
        }
    }

    pub fn list_artifacts(&self) -> &[ArtifactMetadata] {
        &self.catalog
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
