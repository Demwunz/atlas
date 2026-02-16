use crate::hash;
use topo_core::FileInfo;

/// Generate a deterministic fingerprint for a repository based on its file listing.
///
/// The fingerprint is a hex-encoded SHA-256 hash of all file paths and sizes,
/// sorted alphabetically. This ensures the same repo state always produces the
/// same fingerprint, regardless of scan order.
pub fn generate(files: &[FileInfo]) -> String {
    let mut entries: Vec<String> = files
        .iter()
        .map(|f| format!("{}:{}", f.path, f.size))
        .collect();
    entries.sort();

    let combined = entries.join("\n");
    let hash = hash::sha256_bytes(combined.as_bytes());
    hex_encode(&hash)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use topo_core::{FileRole, Language};

    fn make_file(path: &str, size: u64) -> FileInfo {
        FileInfo {
            path: path.to_string(),
            size,
            language: Language::Other,
            role: FileRole::Other,
            sha256: [0u8; 32],
        }
    }

    #[test]
    fn fingerprint_deterministic() {
        let files = vec![make_file("a.rs", 100), make_file("b.rs", 200)];
        let fp1 = generate(&files);
        let fp2 = generate(&files);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn fingerprint_order_independent() {
        let files_a = vec![make_file("b.rs", 200), make_file("a.rs", 100)];
        let files_b = vec![make_file("a.rs", 100), make_file("b.rs", 200)];
        assert_eq!(generate(&files_a), generate(&files_b));
    }

    #[test]
    fn fingerprint_changes_with_new_file() {
        let files1 = vec![make_file("a.rs", 100)];
        let files2 = vec![make_file("a.rs", 100), make_file("b.rs", 200)];
        assert_ne!(generate(&files1), generate(&files2));
    }

    #[test]
    fn fingerprint_changes_with_size_change() {
        let files1 = vec![make_file("a.rs", 100)];
        let files2 = vec![make_file("a.rs", 200)];
        assert_ne!(generate(&files1), generate(&files2));
    }

    #[test]
    fn fingerprint_changes_with_rename() {
        let files1 = vec![make_file("a.rs", 100)];
        let files2 = vec![make_file("b.rs", 100)];
        assert_ne!(generate(&files1), generate(&files2));
    }

    #[test]
    fn fingerprint_empty_files() {
        let fp = generate(&[]);
        assert!(!fp.is_empty());
        assert_eq!(fp.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn fingerprint_is_hex_string() {
        let files = vec![make_file("a.rs", 100)];
        let fp = generate(&files);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(fp.len(), 64);
    }
}
