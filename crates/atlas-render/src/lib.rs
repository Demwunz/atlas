//! JSONL v0.3, JSON, and human-readable output rendering.

mod jsonl;

pub use jsonl::JsonlWriter;

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::{FileRole, Language, ScoredFile, SignalBreakdown};

    fn sample_files() -> Vec<ScoredFile> {
        vec![
            ScoredFile {
                path: "src/auth/middleware.rs".to_string(),
                score: 0.95,
                signals: SignalBreakdown {
                    bm25f: 0.8,
                    heuristic: 0.7,
                    ..Default::default()
                },
                tokens: 1200,
                language: Language::Rust,
                role: FileRole::Implementation,
            },
            ScoredFile {
                path: "src/auth/handler.rs".to_string(),
                score: 0.72,
                signals: SignalBreakdown {
                    bm25f: 0.5,
                    heuristic: 0.6,
                    ..Default::default()
                },
                tokens: 800,
                language: Language::Rust,
                role: FileRole::Implementation,
            },
        ]
    }

    #[test]
    fn jsonl_output_has_three_lines() {
        let files = sample_files();
        let output = JsonlWriter::new("auth middleware", "balanced")
            .max_bytes(Some(100_000))
            .min_score(0.01)
            .render(&files, 358)
            .unwrap();

        let lines: Vec<&str> = output.trim().lines().collect();
        assert_eq!(lines.len(), 4); // header + 2 files + footer
    }

    #[test]
    fn jsonl_header_contains_version() {
        let files = sample_files();
        let output = JsonlWriter::new("test query", "balanced")
            .render(&files, 100)
            .unwrap();

        let first_line = output.lines().next().unwrap();
        let header: serde_json::Value = serde_json::from_str(first_line).unwrap();
        assert_eq!(header["Version"], "0.3");
    }

    #[test]
    fn jsonl_header_contains_query() {
        let files = sample_files();
        let output = JsonlWriter::new("auth middleware", "balanced")
            .render(&files, 100)
            .unwrap();

        let first_line = output.lines().next().unwrap();
        let header: serde_json::Value = serde_json::from_str(first_line).unwrap();
        assert_eq!(header["Query"], "auth middleware");
    }

    #[test]
    fn jsonl_file_entries_have_required_fields() {
        let files = sample_files();
        let output = JsonlWriter::new("test", "balanced")
            .render(&files, 100)
            .unwrap();

        let lines: Vec<&str> = output.trim().lines().collect();
        let file_entry: serde_json::Value = serde_json::from_str(lines[1]).unwrap();

        assert!(file_entry["Path"].is_string());
        assert!(file_entry["Score"].is_number());
        assert!(file_entry["Tokens"].is_number());
        assert!(file_entry["Language"].is_string());
        assert!(file_entry["Role"].is_string());
    }

    #[test]
    fn jsonl_footer_has_totals() {
        let files = sample_files();
        let output = JsonlWriter::new("test", "balanced")
            .render(&files, 358)
            .unwrap();

        let last_line = output.trim().lines().last().unwrap();
        let footer: serde_json::Value = serde_json::from_str(last_line).unwrap();

        assert_eq!(footer["TotalFiles"], 2);
        assert_eq!(footer["TotalTokens"], 2000); // 1200 + 800
        assert_eq!(footer["ScannedFiles"], 358);
    }

    #[test]
    fn jsonl_empty_files_produces_header_and_footer() {
        let output = JsonlWriter::new("test", "balanced").render(&[], 0).unwrap();

        let lines: Vec<&str> = output.trim().lines().collect();
        assert_eq!(lines.len(), 2); // header + footer
    }

    #[test]
    fn jsonl_each_line_is_valid_json() {
        let files = sample_files();
        let output = JsonlWriter::new("test", "balanced")
            .render(&files, 100)
            .unwrap();

        for line in output.trim().lines() {
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
            assert!(parsed.is_ok(), "Invalid JSON line: {line}");
        }
    }

    #[test]
    fn jsonl_max_bytes_in_header() {
        let output = JsonlWriter::new("test", "balanced")
            .max_bytes(Some(50_000))
            .render(&[], 0)
            .unwrap();

        let first_line = output.lines().next().unwrap();
        let header: serde_json::Value = serde_json::from_str(first_line).unwrap();
        assert_eq!(header["Budget"]["MaxBytes"], 50_000);
    }

    #[test]
    fn jsonl_preset_in_header() {
        let output = JsonlWriter::new("test", "deep").render(&[], 0).unwrap();

        let first_line = output.lines().next().unwrap();
        let header: serde_json::Value = serde_json::from_str(first_line).unwrap();
        assert_eq!(header["Preset"], "deep");
    }
}
