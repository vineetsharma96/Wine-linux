use std::fs::{File, OpenOptions};
use std::io::{Read, Seek};
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
        let scratch_path = destination_dir.with_extension("tar.tmp");
        let mut decoded = OpenOptions::new().create_new(true).read(true).write(true).open(&scratch_path)?;
        let result = (|| {
            let mut source = std::io::BufReader::new(File::open(archive_path)?);
            lzma_rs::xz_decompress(&mut source, &mut decoded)
                .map_err(|error| FilesystemError::InvalidArchive(error.to_string()))?;
            decoded.rewind()?;
            Self::extract_tar_stream(&mut decoded, destination_dir)
        })();
        drop(decoded);
        let cleanup = std::fs::remove_file(scratch_path);
        let entries = result?;
        cleanup?;
        Ok(entries)
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

            let dest_file_path = PathSecurity::sanitize_and_resolve(destination_dir, &path)?;
            match entry.unpack_in(destination_dir) {
                Ok(true) => {}
                Ok(false) => {
                    return Err(FilesystemError::InvalidArchive(format!(
                        "Cannot extract {}", path.display()
                    )));
                }
                Err(err) if cfg!(target_os = "android")
                    && entry.header().entry_type().is_hard_link()
                    && err.kind() == std::io::ErrorKind::PermissionDenied =>
                {
                    let link = entry.link_name()?.ok_or_else(|| {
                        FilesystemError::InvalidArchive("Hardlink has no target".into())
                    })?;
                    let source = PathSecurity::sanitize_and_resolve(destination_dir, &link)?;
                    let root = destination_dir.canonicalize()?;
                    let source = source.canonicalize()?;
                    let parent = dest_file_path.parent().unwrap_or(destination_dir).canonicalize()?;
                    if !source.starts_with(&root) || !parent.starts_with(&root) || !source.is_file() {
                        return Err(FilesystemError::InvalidArchive("Unsafe hardlink target".into()));
                    }
                    let mut target = std::fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(&dest_file_path)?;
                    std::io::copy(&mut File::open(&source)?, &mut target)?;
                    target.set_permissions(source.metadata()?.permissions())?;
                }
                Err(err) => return Err(err.into()),
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
        let entries = std::fs::read_dir(destination_dir)?.collect::<Result<Vec<_>, _>>()?;
        if entries.len() == 1 && entries[0].file_type()?.is_dir() {
            let nested = entries[0].path();
            if nested.join("bin").exists() || nested.join("usr").exists() {
                for child in std::fs::read_dir(&nested)? {
                    let child = child?;
                    let target = destination_dir.join(child.file_name());
                    std::fs::rename(child.path(), target)?;
                }
                std::fs::remove_dir(&nested)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flatten_single_rootfs_dir() {
        let tmp = std::env::temp_dir().join("androidpc_flatten_test");
        let _ = std::fs::remove_dir_all(&tmp);
        let nested = tmp.join("debian-bookworm-aarch64");
        std::fs::create_dir_all(nested.join("bin")).unwrap();
        std::fs::write(nested.join("bin").join("bash"), b"elf").unwrap();

        ArchiveExtractor::flatten_single_rootfs_dir(&tmp).unwrap();
        assert!(tmp.join("bin").join("bash").exists());
        assert!(!nested.exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
