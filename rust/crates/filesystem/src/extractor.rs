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

    pub fn extract_tar_xz(archive_path: &Path, destination_dir: &Path) -> Result<u64, FilesystemError> {
        std::fs::create_dir_all(destination_dir)?;
        let file = File::open(archive_path)?;
        let mut buf_reader = std::io::BufReader::new(file);
        let mut decompressed = Vec::new();
        lzma_rs::xz_decompress(&mut buf_reader, &mut decompressed)
            .map_err(|e| FilesystemError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())))?;
        Self::extract_tar_stream(std::io::Cursor::new(decompressed), destination_dir)
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
            // Unpack entry with Android-safe hardlink fallback (SELinux blocks link() in app sandboxes)
            if entry.header().entry_type() == tar::EntryType::Link {
                if let Err(err) = entry.unpack(&dest_file_path) {
                    if err.kind() == std::io::ErrorKind::PermissionDenied {
                        if let Ok(Some(link_target)) = entry.link_name() {
                            let resolved_target = PathSecurity::sanitize_and_resolve(
                                destination_dir,
                                link_target.as_ref(),
                            )?;
                            if resolved_target.exists() {
                                std::fs::copy(&resolved_target, &dest_file_path)?;
                            }
                        }
                    } else {
                        return Err(err.into());
                    }
                }
            } else {
                entry.unpack(&dest_file_path)?;
            }
            entries_extracted += 1;
        }

        info!("Successfully extracted {} entries to {}", entries_extracted, destination_dir.display());
        Self::flatten_single_rootfs_dir(destination_dir)?;
        Ok(entries_extracted)
    }

    /// If an archive (such as Termux proot-distro tarballs) wraps the rootfs inside a single
    /// top-level directory (e.g. `debian-bookworm-aarch64/bin`), promote its contents up one level.
    fn flatten_single_rootfs_dir(destination_dir: &Path) -> Result<(), FilesystemError> {
        if destination_dir.join("bin").exists() || destination_dir.join("usr").exists() {
            return Ok(());
        }
        let entries: Vec<_> = std::fs::read_dir(destination_dir)?
            .filter_map(|e| e.ok())
            .collect();
        if entries.len() == 1 && entries[0].path().is_dir() {
            let nested = entries[0].path();
            if nested.join("bin").exists() || nested.join("usr").exists() {
                for child in std::fs::read_dir(&nested)?.flatten() {
                    let target = destination_dir.join(child.file_name());
                    if !target.exists() {
                        let _ = std::fs::rename(child.path(), target);
                    }
                }
                let _ = std::fs::remove_dir_all(&nested);
            }
        }
        Ok(())
    }
}
