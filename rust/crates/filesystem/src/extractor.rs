use std::fs::File;
use std::io::Read;
use std::path::Path;
use flate2::read::GzDecoder;
use tar::Archive;
use tracing::info;
use crate::{FilesystemError, PathSecurity};

pub struct ArchiveExtractor;

impl ArchiveExtractor {
    /// Extracts a tar or tar.gz archive safely into the destination directory with traversal protection.
    pub fn extract_tar_gz(archive_path: &Path, destination_dir: &Path) -> Result<u64, FilesystemError> {
        std::fs::create_dir_all(destination_dir)?;
        let file = File::open(archive_path)?;
        let gz = GzDecoder::new(file);
        Self::extract_tar_stream(gz, destination_dir)
    }

    pub fn extract_tar(archive_path: &Path, destination_dir: &Path) -> Result<u64, FilesystemError> {
        std::fs::create_dir_all(destination_dir)?;
        let file = File::open(archive_path)?;
        Self::extract_tar_stream(file, destination_dir)
    }

    fn extract_tar_stream<R: Read>(reader: R, destination_dir: &Path) -> Result<u64, FilesystemError> {
        let mut archive = Archive::new(reader);
        let mut entries_extracted: u64 = 0;

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?.to_path_buf();

            // Validate Zip-Slip defense
            let dest_file_path = PathSecurity::sanitize_and_resolve(destination_dir, &path)?;

            // Ensure parent directory exists
            if let Some(parent) = dest_file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Unpack entry
            entry.unpack(&dest_file_path)?;
            entries_extracted += 1;
        }

        info!("Successfully extracted {} entries to {}", entries_extracted, destination_dir.display());
        Ok(entries_extracted)
    }
}
