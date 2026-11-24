use std::fs::File;
use std::io::Read;
use std::path::Path;
use sha2::{Digest, Sha256};
use crate::DownloaderError;

pub struct Sha256Verifier {
    hasher: Sha256,
}

impl Default for Sha256Verifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256Verifier {
    pub fn new() -> Self {
        Self {
            hasher: Sha256::new(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    pub fn finalize_hex(self) -> String {
        hex::encode(self.hasher.finalize())
    }

    pub fn hash_bytes(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    pub fn hash_file(path: &Path) -> Result<String, DownloaderError> {
        let mut file = File::open(path)?;
        let mut verifier = Self::new();
        let mut buffer = [0u8; 64 * 1024];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            verifier.update(&buffer[..bytes_read]);
        }

        Ok(verifier.finalize_hex())
    }

    pub fn verify_file(path: &Path, expected_sha256: &str) -> Result<bool, DownloaderError> {
        let actual = Self::hash_file(path)?;
        Ok(actual.eq_ignore_ascii_case(expected_sha256.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_known_vectors() {
        // Known NIST vector: sha256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let empty_hash = Sha256Verifier::hash_bytes(b"");
        assert_eq!(
            empty_hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        // Known vector: sha256("hello world")
        let hello_hash = Sha256Verifier::hash_bytes(b"hello world");
        assert_eq!(
            hello_hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}
