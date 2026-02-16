use crate::Cli;
use anyhow::Result;

pub fn run(cli: &Cli) -> Result<()> {
    let root = cli.repo_root()?;
    let index_path = atlas_index::index_path(&root);

    if !index_path.exists() {
        anyhow::bail!(
            "No index found at {}. Run `atlas index --deep` first.",
            index_path.display()
        );
    }

    let metadata = std::fs::metadata(&index_path)?;
    let file_size = metadata.len();

    let index = atlas_index::load(&root)?.ok_or_else(|| anyhow::anyhow!("Failed to load index"))?;

    // Collect language stats
    let mut lang_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut total_chunks: usize = 0;
    let mut total_terms: usize = 0;

    for entry in index.files.values() {
        total_chunks += entry.chunks.len();
        total_terms += entry.term_frequencies.len();
    }

    // Count files by extension
    for path in index.files.keys() {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("(none)");
        *lang_counts.entry(ext.to_string()).or_default() += 1;
    }

    println!("Index: {}", index_path.display());
    println!("Format: rkyv binary");
    println!(
        "Size: {:.1} MB ({} bytes)",
        file_size as f64 / 1_048_576.0,
        file_size
    );
    println!("Version: {}", index.version);
    println!("Files: {}", index.total_docs);
    println!("Chunks: {}", total_chunks);
    println!("Unique terms: {}", index.doc_frequencies.len());
    println!("Terms (file-level): {}", total_terms);
    println!("Avg doc length: {:.1}", index.avg_doc_length);
    println!();

    // Top extensions by file count
    let mut sorted_langs: Vec<_> = lang_counts.into_iter().collect();
    sorted_langs.sort_by(|a, b| b.1.cmp(&a.1));

    println!("Files by extension:");
    for (ext, count) in sorted_langs.iter().take(15) {
        println!("  .{ext:<12} {count:>6}");
    }
    if sorted_langs.len() > 15 {
        let rest: usize = sorted_langs[15..].iter().map(|(_, c)| c).sum();
        println!("  (other)       {rest:>6}");
    }

    Ok(())
}
