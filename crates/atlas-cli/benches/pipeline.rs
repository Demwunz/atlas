//! Benchmark harness: measures scan → score → render pipeline performance.
//!
//! Run with: cargo bench -p atlas-cli
//!
//! This uses Rust's built-in test harness benchmarks.
//! For production benchmarks, consider criterion.

use std::fs;
use std::time::Instant;

use atlas_core::{ScoredFile, TokenBudget};
use atlas_render::JsonlWriter;
use atlas_scanner::BundleBuilder;
use atlas_score::HybridScorer;

fn create_synthetic_repo(file_count: usize) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    fs::create_dir_all(root.join("src")).unwrap();

    for i in 0..file_count {
        let lang = match i % 5 {
            0 => (
                "rs",
                "fn handler_{i}() {{\n    let x = {i};\n    println!(\"{{x}}\");\n}}\n",
            ),
            1 => ("py", "def handler_{i}():\n    x = {i}\n    print(x)\n"),
            2 => (
                "go",
                "func handler_{i}() {{\n    x := {i}\n    fmt.Println(x)\n}}\n",
            ),
            3 => (
                "js",
                "function handler_{i}() {{\n    const x = {i};\n    console.log(x);\n}}\n",
            ),
            _ => (
                "ts",
                "export function handler_{i}(): void {{\n    const x = {i};\n}}\n",
            ),
        };
        let content = lang.1.replace("{i}", &i.to_string());
        let path = root.join(format!("src/module_{i}.{}", lang.0));
        fs::write(path, content).unwrap();
    }

    dir
}

fn bench_scan(dir: &std::path::Path) -> atlas_core::Bundle {
    BundleBuilder::new(dir).build().unwrap()
}

fn bench_score(task: &str, files: &[atlas_core::FileInfo]) -> Vec<ScoredFile> {
    let scorer = HybridScorer::new(task);
    scorer.score(files)
}

fn bench_budget(files: &[ScoredFile], max_bytes: u64) -> Vec<ScoredFile> {
    let budget = TokenBudget {
        max_bytes: Some(max_bytes),
        max_tokens: None,
    };
    budget.enforce(files)
}

fn bench_render(task: &str, files: &[ScoredFile], scanned: usize, max_bytes: u64) -> String {
    JsonlWriter::new(task, "balanced")
        .max_bytes(Some(max_bytes))
        .min_score(0.01)
        .render(files, scanned)
        .unwrap()
}

fn run_benchmark(label: &str, file_count: usize, task: &str) {
    let dir = create_synthetic_repo(file_count);
    let iterations = 5;

    // Warmup
    let bundle = bench_scan(dir.path());
    let _ = bench_score(task, &bundle.files);

    // Scan benchmark
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = bench_scan(dir.path());
    }
    let scan_ms = start.elapsed().as_millis() as f64 / iterations as f64;

    // Score benchmark
    let bundle = bench_scan(dir.path());
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = bench_score(task, &bundle.files);
    }
    let score_ms = start.elapsed().as_millis() as f64 / iterations as f64;

    // Budget + Render benchmark
    let scored = bench_score(task, &bundle.files);
    let start = Instant::now();
    for _ in 0..iterations {
        let budgeted = bench_budget(&scored, 100_000);
        let _ = bench_render(task, &budgeted, bundle.file_count(), 100_000);
    }
    let render_ms = start.elapsed().as_millis() as f64 / iterations as f64;

    let total_ms = scan_ms + score_ms + render_ms;

    println!("{label}:");
    println!("  Files:  {file_count}");
    println!("  Scan:   {scan_ms:.1}ms");
    println!("  Score:  {score_ms:.1}ms");
    println!("  Render: {render_ms:.1}ms");
    println!("  Total:  {total_ms:.1}ms");
    println!();
}

fn main() {
    println!("Atlas Pipeline Benchmarks");
    println!("=========================\n");

    run_benchmark("Small repo (50 files)", 50, "handler authentication");
    run_benchmark("Medium repo (200 files)", 200, "handler authentication");
    run_benchmark("Large repo (1000 files)", 1000, "handler authentication");

    println!("Done.");
}
