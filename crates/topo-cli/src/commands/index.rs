use crate::Cli;
use anyhow::Result;
use topo_index::IndexBuilder;
use topo_scanner::BundleBuilder;

pub fn run(cli: &Cli, deep: bool, force: bool) -> Result<()> {
    let root = cli.repo_root()?;

    if !cli.is_quiet() {
        eprintln!(
            "Indexing {} (mode: {})...",
            root.display(),
            if deep { "deep" } else { "shallow" }
        );
    }

    // Scan the repository
    let bundle = BundleBuilder::new(&root).build()?;

    if !cli.is_quiet() {
        eprintln!(
            "Scanned {} files (fingerprint: {})",
            bundle.file_count(),
            &bundle.fingerprint[..12]
        );
    }

    if deep {
        // Load existing index (unless force rebuild)
        let existing = if force {
            None
        } else {
            topo_index::load(&root)?
        };

        // Build index, skipping unchanged files when existing index is available
        let builder = IndexBuilder::new(&root);
        let (index, reindexed) = builder.build(&bundle.files, existing.as_ref())?;

        let is_incremental = existing.is_some();
        let nothing_changed = is_incremental && reindexed == 0;

        if !cli.is_quiet() {
            if is_incremental {
                eprintln!(
                    "Incremental update: {} files indexed ({} changed)",
                    index.total_docs, reindexed
                );
            } else {
                eprintln!("Full index build: {} files indexed", index.total_docs);
            }
        }

        if nothing_changed {
            if !cli.is_quiet() {
                eprintln!(
                    "Index unchanged at {}",
                    topo_index::index_path(&root).display()
                );
            }
        } else {
            topo_index::save(&index, &root)?;

            if !cli.is_quiet() {
                eprintln!("Index saved to {}", topo_index::index_path(&root).display());
            }
        }
    }

    if !cli.is_quiet() {
        eprintln!("Done.");
    }

    Ok(())
}
