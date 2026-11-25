use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path traversal blocked for entry '{0}'")]
    PathTraversal(String),

    #[error("SHA-256 mismatch for '{file}': expected {expected}, actual {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },
}

/// Validates an archive entry path against Zip-Slip / directory traversal attacks
/// and returns the canonical relative path inside `dest_root`.
pub fn resolve_safe_path(dest_root: &Path, entry_path: &Path) -> Result<PathBuf, SecurityError> {
    if entry_path.is_absolute() {
        return Err(SecurityError::PathTraversal(entry_path.display().to_string()));
    }

    let mut clean_rel = PathBuf::new();
    for comp in entry_path.components() {
        match comp {
            Component::Normal(part) => clean_rel.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(SecurityError::PathTraversal(entry_path.display().to_string()));
            }
        }
    }

    Ok(dest_root.join(clean_rel))
}

/// Computes the lowercase hex SHA-256 digest of a file using streaming 64 KB reads.
pub fn compute_file_sha256(path: &Path) -> Result<String, SecurityError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Verifies a file against an expected hex SHA-256 digest.
pub fn verify_file_sha256(path: &Path, expected_hex: &str) -> Result<(), SecurityError> {
    let actual = compute_file_sha256(path)?;
    let expected = expected_hex.trim().to_ascii_lowercase();
    if actual != expected {
        return Err(SecurityError::ChecksumMismatch {
            file: path.display().to_string(),
            expected,
            actual,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_safe_path_blocks_traversal() {
        let root = Path::new("/data/sandbox/rootfs");
        assert!(resolve_safe_path(root, Path::new("../../system/bin/sh")).is_err());
        assert!(resolve_safe_path(root, Path::new("/etc/passwd")).is_err());

        let ok = resolve_safe_path(root, Path::new("./usr/bin/env")).expect("valid path");
        assert_eq!(ok, root.join("usr").join("bin").join("env"));
    }

    #[test]
    fn test_verify_file_sha256() {
        let tmp = std::env::temp_dir().join("androidpc_sec_sha_test.txt");
        std::fs::write(&tmp, b"abc").unwrap();
        // NIST SHA-256("abc")
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(verify_file_sha256(&tmp, expected).is_ok());
        assert!(verify_file_sha256(&tmp, "0000").is_err());
        let _ = std::fs::remove_file(&tmp);
    }
}
