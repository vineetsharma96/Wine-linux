pub mod artifact;
pub mod verifier;

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, info};

pub use artifact::{ArtifactMetadata, DefaultArtifactProvider};
pub use verifier::Sha256Verifier;

#[derive(Debug, Error)]
pub enum DownloaderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Integrity check failed for '{file}': expected {expected}, actual {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    #[error("Unknown artifact: {0}")]
    UnknownArtifact(String),

    #[error("Download destination already exists: {0}")]
    DestinationExists(PathBuf),

    #[error("Incomplete download: expected {expected} bytes, received {actual} bytes")]
    Incomplete { expected: u64, actual: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub artifact_name: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub percent: f32,
    pub is_complete: bool,
}

pub struct DownloadManager;

impl DownloadManager {
    /// Streams reader content into destination_path.part, updates SHA-256 on the fly,
    /// and atomically renames to destination_path if and only if the hash matches expected_sha256.
    pub fn download_stream<R: Read, F: Fn(u64, u64)>(
        reader: &mut R,
        destination_path: &Path,
        expected_sha256: &str,
        total_size: u64,
        progress_callback: F,
    ) -> Result<PathBuf, DownloaderError> {
        let part_path = destination_path.with_extension("part");
        if let Some(parent) = part_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = File::create(&part_path)?;
        let mut verifier = Sha256Verifier::new();

        let mut buffer = [0u8; 64 * 1024]; // 64KB buffer
        let mut bytes_downloaded: u64 = 0;

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            file.write_all(&buffer[..bytes_read])?;
            verifier.update(&buffer[..bytes_read]);
            bytes_downloaded += bytes_read as u64;

            progress_callback(bytes_downloaded, total_size);
        }

        file.flush()?;
        drop(file);

        let computed_hash = verifier.finalize_hex();
        let expected_normalized = expected_sha256.trim().to_lowercase();

        if computed_hash != expected_normalized {
            error!(
                "Checksum mismatch for {}: expected {}, computed {}",
                destination_path.display(),
                expected_normalized,
                computed_hash
            );
            let _ = std::fs::remove_file(&part_path);
            return Err(DownloaderError::ChecksumMismatch {
                file: destination_path.display().to_string(),
                expected: expected_normalized,
                actual: computed_hash,
            });
        }

        // Atomically rename .part to destination
        std::fs::rename(&part_path, destination_path)?;
        info!(
            "Artifact verified and written successfully to {}",
            destination_path.display()
        );

        Ok(destination_path.to_path_buf())
    }
}
