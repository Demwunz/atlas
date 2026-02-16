use crate::Cli;
use crate::preset::Preset;
use anyhow::Result;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::Deserialize;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Parameter structs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct QueryParams {
    /// The task or query describing what you're looking for
    #[schemars(description = "The task or query describing what you're looking for")]
    task: String,

    /// Scoring preset: fast, balanced, deep, thorough
    #[schemars(description = "Scoring preset: fast, balanced, deep, thorough (default: balanced)")]
    preset: Option<String>,

    /// Maximum bytes for token budget
    #[schemars(description = "Maximum bytes for token budget")]
    max_bytes: Option<u64>,

    /// Maximum tokens for token budget
    #[schemars(description = "Maximum tokens for token budget")]
    max_tokens: Option<u64>,

    /// Minimum score threshold (files below this are excluded)
    #[schemars(description = "Minimum score threshold (files below this are excluded)")]
    min_score: Option<f64>,

    /// Return only the top N files
    #[schemars(description = "Return only the top N files")]
    top: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ExplainParams {
    /// The task or query to explain scoring for
    #[schemars(description = "The task or query to explain scoring for")]
    task: String,

    /// Return top N files (default: 10)
    #[schemars(description = "Return top N files (default: 10)")]
    top: Option<usize>,

    /// Scoring preset: fast, balanced, deep, thorough
    #[schemars(description = "Scoring preset: fast, balanced, deep, thorough (default: balanced)")]
    preset: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct IndexParams {
    /// Enable deep indexing with AST chunking
    #[schemars(description = "Enable deep indexing with AST chunking (default: true)")]
    deep: Option<bool>,

    /// Rebuild index from scratch (ignore cache)
    #[schemars(description = "Rebuild index from scratch, ignoring cache")]
    force: Option<bool>,
}

// ---------------------------------------------------------------------------
// TopoServer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TopoServer {
    root: PathBuf,
    tool_router: ToolRouter<TopoServer>,
}

fn parse_preset(s: Option<&str>) -> Preset {
    match s {
        Some("fast") => Preset::Fast,
        Some("deep") => Preset::Deep,
        Some("thorough") => Preset::Thorough,
        _ => Preset::Balanced,
    }
}

// ---------------------------------------------------------------------------
// Sync helpers — called via spawn_blocking from async tool methods
// ---------------------------------------------------------------------------

impl TopoServer {
    fn do_query(&self, params: QueryParams) -> Result<serde_json::Value> {
        let preset = parse_preset(params.preset.as_deref());

        // Auto-index if preset requires it
        if preset.needs_deep_index() {
            self.do_index_inner(true, preset.force_rebuild())?;
        }

        let bundle = topo_scanner::BundleBuilder::new(&self.root).build()?;

        let deep_index = if preset.use_structural_signals() {
            topo_index::load(&self.root)?
        } else {
            None
        };

        let scored =
            super::query::score_files(&params.task, &bundle.files, preset, deep_index.as_ref());

        let effective_min_score = params.min_score.unwrap_or(preset.default_min_score());
        let mut filtered: Vec<topo_core::ScoredFile> = scored
            .into_iter()
            .filter(|f| f.score >= effective_min_score)
            .collect();

        if let Some(n) = params.top {
            filtered.truncate(n);
        }

        let effective_max_bytes = params.max_bytes.unwrap_or(preset.default_max_bytes());
        let budget = topo_core::TokenBudget {
            max_bytes: Some(effective_max_bytes),
            max_tokens: params.max_tokens,
        };
        let budgeted = budget.enforce(&filtered);

        let result = serde_json::json!({
            "query": params.task,
            "preset": preset.as_str(),
            "files": budgeted.iter().map(|f| serde_json::json!({
                "path": f.path,
                "score": f.score,
                "tokens": f.tokens,
                "language": f.language.as_str(),
                "role": f.role.as_str(),
            })).collect::<Vec<_>>(),
            "total_selected": budgeted.len(),
            "total_scanned": bundle.file_count(),
        });

        Ok(result)
    }

    fn do_explain(&self, params: ExplainParams) -> Result<serde_json::Value> {
        let preset = parse_preset(params.preset.as_deref());
        let top = params.top.unwrap_or(10);

        let bundle = topo_scanner::BundleBuilder::new(&self.root).build()?;

        let deep_index = if preset.use_structural_signals() {
            topo_index::load(&self.root)?
        } else {
            None
        };

        let scored =
            super::query::score_files(&params.task, &bundle.files, preset, deep_index.as_ref());

        let display_count = top.min(scored.len());
        let results = &scored[..display_count];

        let output: Vec<serde_json::Value> = results
            .iter()
            .map(|f| {
                serde_json::json!({
                    "path": f.path,
                    "score": f.score,
                    "signals": {
                        "bm25f": f.signals.bm25f,
                        "heuristic": f.signals.heuristic,
                        "pagerank": f.signals.pagerank,
                        "git_recency": f.signals.git_recency,
                    },
                    "tokens": f.tokens,
                    "language": f.language.as_str(),
                    "role": f.role.as_str(),
                })
            })
            .collect();

        Ok(serde_json::Value::Array(output))
    }

    fn do_index(&self, params: IndexParams) -> Result<serde_json::Value> {
        let deep = params.deep.unwrap_or(true);
        let force = params.force.unwrap_or(false);
        self.do_index_inner(deep, force)
    }

    fn do_index_inner(&self, deep: bool, force: bool) -> Result<serde_json::Value> {
        let bundle = topo_scanner::BundleBuilder::new(&self.root).build()?;
        let file_count = bundle.file_count();

        if deep {
            let existing = if force {
                None
            } else {
                topo_index::load(&self.root)?
            };

            let builder = topo_index::IndexBuilder::new(&self.root);
            let (index, reindexed) = builder.build(&bundle.files, existing.as_ref())?;
            let is_incremental = existing.is_some();
            let nothing_changed = is_incremental && reindexed == 0;

            if !nothing_changed {
                topo_index::save(&index, &self.root)?;
            }

            Ok(serde_json::json!({
                "status": "ok",
                "mode": if is_incremental { "incremental" } else { "full" },
                "files_scanned": file_count,
                "files_indexed": index.total_docs,
                "files_changed": reindexed,
            }))
        } else {
            Ok(serde_json::json!({
                "status": "ok",
                "mode": "shallow",
                "files_scanned": file_count,
            }))
        }
    }
}

// ---------------------------------------------------------------------------
// MCP tool definitions
// ---------------------------------------------------------------------------

#[tool_router]
impl TopoServer {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "topo_query",
        description = "Search a codebase and return the most relevant files for a task. Auto-indexes if needed. Returns scored file paths with token counts."
    )]
    async fn topo_query(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<CallToolResult, McpError> {
        let server = self.clone();
        let result = tokio::task::spawn_blocking(move || server.do_query(params))
            .await
            .map_err(|e| McpError::internal_error(format!("join error: {e}"), None))?
            .map_err(|e| McpError::internal_error(format!("{e:#}"), None))?;

        let text = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        name = "topo_explain",
        description = "Show per-file score breakdown for a query, including BM25F, heuristic, PageRank, and git recency signals."
    )]
    async fn topo_explain(
        &self,
        Parameters(params): Parameters<ExplainParams>,
    ) -> Result<CallToolResult, McpError> {
        let server = self.clone();
        let result = tokio::task::spawn_blocking(move || server.do_explain(params))
            .await
            .map_err(|e| McpError::internal_error(format!("join error: {e}"), None))?
            .map_err(|e| McpError::internal_error(format!("{e:#}"), None))?;

        let text = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        name = "topo_index",
        description = "Build or update the codebase index. Deep mode uses AST chunking for better results. Force rebuilds from scratch."
    )]
    async fn topo_index(
        &self,
        Parameters(params): Parameters<IndexParams>,
    ) -> Result<CallToolResult, McpError> {
        let server = self.clone();
        let result = tokio::task::spawn_blocking(move || server.do_index(params))
            .await
            .map_err(|e| McpError::internal_error(format!("join error: {e}"), None))?
            .map_err(|e| McpError::internal_error(format!("{e:#}"), None))?;

        let text = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("{e}"), None))?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

// ---------------------------------------------------------------------------
// ServerHandler
// ---------------------------------------------------------------------------

#[tool_handler]
impl ServerHandler for TopoServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "topo".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: None,
                description: Some(
                    "Fast codebase indexer and file selector for LLM context windows".into(),
                ),
                icons: None,
                website_url: Some("https://github.com/demwunz/topo".into()),
            },
            instructions: Some(
                "Topo indexes codebases and selects the most relevant files for LLM context windows. \
                 Use topo_query to find files relevant to a task, topo_explain to understand scoring, \
                 and topo_index to build or update the index."
                    .to_string(),
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run(cli: &Cli) -> Result<()> {
    let root = cli.repo_root()?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let server = TopoServer::new(root);
        let service = server.serve(stdio()).await?;
        service.waiting().await?;
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_preset_defaults_to_balanced() {
        assert!(matches!(parse_preset(None), Preset::Balanced));
        assert!(matches!(parse_preset(Some("unknown")), Preset::Balanced));
    }

    #[test]
    fn parse_preset_recognizes_all_variants() {
        assert!(matches!(parse_preset(Some("fast")), Preset::Fast));
        assert!(matches!(parse_preset(Some("balanced")), Preset::Balanced));
        assert!(matches!(parse_preset(Some("deep")), Preset::Deep));
        assert!(matches!(parse_preset(Some("thorough")), Preset::Thorough));
    }

    #[test]
    fn do_query_returns_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.rs"), "fn main() {}").unwrap();

        let server = TopoServer::new(dir.path().to_path_buf());
        let params = QueryParams {
            task: "main function".to_string(),
            preset: Some("fast".to_string()),
            max_bytes: None,
            max_tokens: None,
            min_score: None,
            top: None,
        };

        let result = server.do_query(params).unwrap();
        assert!(result.get("files").unwrap().is_array());
        assert!(result.get("total_scanned").unwrap().as_u64().unwrap() > 0);
    }

    #[test]
    fn do_explain_returns_array() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.rs"), "fn main() {}").unwrap();

        let server = TopoServer::new(dir.path().to_path_buf());
        let params = ExplainParams {
            task: "main function".to_string(),
            top: Some(5),
            preset: Some("fast".to_string()),
        };

        let result = server.do_explain(params).unwrap();
        assert!(result.is_array());
    }

    #[test]
    fn do_index_returns_status() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.rs"), "fn main() {}").unwrap();

        let server = TopoServer::new(dir.path().to_path_buf());
        let params = IndexParams {
            deep: Some(true),
            force: Some(false),
        };

        let result = server.do_index(params).unwrap();
        assert_eq!(result.get("status").unwrap(), "ok");
        assert!(result.get("files_scanned").unwrap().as_u64().unwrap() > 0);
    }
}
