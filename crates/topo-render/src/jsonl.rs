use serde::Serialize;
use std::io::Write;
use topo_core::ScoredFile;

/// Writes scored files in JSONL v0.3 format.
pub struct JsonlWriter {
    query: String,
    preset: String,
    max_bytes: Option<u64>,
    min_score: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct Header {
    version: String,
    query: String,
    preset: String,
    budget: Budget,
    min_score: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct Budget {
    #[serde(skip_serializing_if = "Option::is_none")]
    max_bytes: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct FileEntry {
    path: String,
    score: f64,
    tokens: u64,
    language: String,
    role: String,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct Footer {
    total_files: usize,
    total_tokens: u64,
    scanned_files: usize,
}

impl JsonlWriter {
    pub fn new(query: &str, preset: &str) -> Self {
        Self {
            query: query.to_string(),
            preset: preset.to_string(),
            max_bytes: None,
            min_score: 0.0,
        }
    }

    pub fn max_bytes(mut self, max_bytes: Option<u64>) -> Self {
        self.max_bytes = max_bytes;
        self
    }

    pub fn min_score(mut self, min_score: f64) -> Self {
        self.min_score = min_score;
        self
    }

    /// Render scored files as JSONL v0.3 string.
    pub fn render(&self, files: &[ScoredFile], scanned_count: usize) -> anyhow::Result<String> {
        let mut buf = Vec::new();
        self.write_to(&mut buf, files, scanned_count)?;
        Ok(String::from_utf8(buf)?)
    }

    /// Write JSONL v0.3 output to a writer.
    pub fn write_to(
        &self,
        writer: &mut dyn Write,
        files: &[ScoredFile],
        scanned_count: usize,
    ) -> anyhow::Result<()> {
        // Header
        let header = Header {
            version: "0.3".to_string(),
            query: self.query.clone(),
            preset: self.preset.clone(),
            budget: Budget {
                max_bytes: self.max_bytes,
            },
            min_score: self.min_score,
        };
        serde_json::to_writer(&mut *writer, &header)?;
        writeln!(writer)?;

        // File entries
        let mut total_tokens = 0u64;
        for file in files {
            let entry = FileEntry {
                path: file.path.clone(),
                score: file.score,
                tokens: file.tokens,
                language: file.language.as_str().to_string(),
                role: file.role.as_str().to_string(),
            };
            serde_json::to_writer(&mut *writer, &entry)?;
            writeln!(writer)?;
            total_tokens += file.tokens;
        }

        // Footer
        let footer = Footer {
            total_files: files.len(),
            total_tokens,
            scanned_files: scanned_count,
        };
        serde_json::to_writer(&mut *writer, &footer)?;
        writeln!(writer)?;

        Ok(())
    }
}
