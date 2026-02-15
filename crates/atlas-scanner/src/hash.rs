use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Compute SHA-256 hash of a file's contents.
pub fn sha256_file(path: &Path) -> anyhow::Result<[u8; 32]> {
    let contents = fs::read(path)?;
    Ok(sha256_bytes(&contents))
}

/// Compute SHA-256 hash of a byte slice.
pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}
