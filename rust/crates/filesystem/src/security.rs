use std::path::{Component, Path, PathBuf};
use crate::FilesystemError;

pub struct PathSecurity;

impl PathSecurity {
    /// Validates that a target entry path does not escape the destination base directory (Zip-Slip defense).
    pub fn sanitize_and_resolve(base_dir: &Path, entry_path: &Path) -> Result<PathBuf, FilesystemError> {
        // Reject absolute paths in archive
        if entry_path.is_absolute() {
            return Err(FilesystemError::PathTraversalAttempt {
                base: base_dir.to_path_buf(),
                target: entry_path.to_path_buf(),
            });
        }

        // Check for parent directory references ("..")
        for component in entry_path.components() {
            match component {
                Component::ParentDir => {
                    return Err(FilesystemError::PathTraversalAttempt {
                        base: base_dir.to_path_buf(),
                        target: entry_path.to_path_buf(),
                    });
                }
                Component::Prefix(_) => {
                    return Err(FilesystemError::PathTraversalAttempt {
                        base: base_dir.to_path_buf(),
                        target: entry_path.to_path_buf(),
                    });
                }
                _ => {}
            }
        }

        let resolved = base_dir.join(entry_path);
        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_traversal_detection() {
        let base = Path::new("/data/local/tmp/rootfs");

        // Valid relative paths
        let safe1 = PathSecurity::sanitize_and_resolve(base, Path::new("bin/bash")).unwrap();
        assert_eq!(safe1, base.join("bin/bash"));

        let safe2 = PathSecurity::sanitize_and_resolve(base, Path::new("etc/passwd")).unwrap();
        assert_eq!(safe2, base.join("etc/passwd"));

        // Malicious path traversal attempts (Zip-Slip)
        let malicious1 = PathSecurity::sanitize_and_resolve(base, Path::new("../evil.sh"));
        assert!(matches!(malicious1, Err(FilesystemError::PathTraversalAttempt { .. })));

        let malicious2 = PathSecurity::sanitize_and_resolve(base, Path::new("bin/../../etc/shadow"));
        assert!(matches!(malicious2, Err(FilesystemError::PathTraversalAttempt { .. })));
    }
}
