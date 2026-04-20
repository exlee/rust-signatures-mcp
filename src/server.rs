use std::path::Path;
use rmcp::{model::*, handler::server::router::tool::ToolRouter, tool};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::analyze::analyze_to_json;
use crate::registry::find_package_dir;
use crate::search::search_signatures_json;
use crate::types::{AnalyzeResult, SearchResult};

#[derive(Debug, Deserialize, JsonSchema)]
struct AnalyzeArgs {
    #[schemars(description = "File or directory path to scan for Rust signatures")]
    path: String,
    #[schemars(description = "Optional maximum number of signatures to return. Useful for large crates to limit context size.")]
    max_signatures: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AnalyzePackageArgs {
    #[schemars(description = "Crate name (e.g. 'serde', 'clap_derive'). Alternatively, provide a direct file or directory path.")]
    package: String,
    #[schemars(description = "Optional version (e.g. '1.0.228'). Defaults to latest cached version. Ignored if package is a path.")]
    version: Option<String>,
    #[schemars(description = "Optional maximum number of signatures to return. Useful for large crates to limit context size.")]
    max_signatures: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchPackageArgs {
    #[schemars(description = "Crate name to search in. Alternatively, provide a direct file or directory path.")]
    package: String,
    #[schemars(description = "Optional version. Defaults to latest cached version. Ignored if package is a path.")]
    version: Option<String>,
    #[schemars(description = "Search string to filter signatures (case-insensitive)")]
    query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchDirectoryArgs {
    #[schemars(description = "File or directory path to scan for Rust files")]
    path: String,
    #[schemars(description = "Search string to filter signatures (case-insensitive)")]
    query: String,
}

#[derive(Clone)]
pub struct RustSigServer {
    tool_router: ToolRouter<Self>,
}

impl RustSigServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[rmcp::tool_router]
impl RustSigServer {
    #[tool(description = "Extract all fn/struct/enum/trait/impl signatures and doc comments from a Rust file or all Rust files in a directory. Returns structured JSON.")]
    async fn analyze_rust(&self, params: rmcp::handler::server::wrapper::Parameters<AnalyzeArgs>) -> String {
        let AnalyzeArgs { path, max_signatures } = params.0;
        analyze_to_json(&path, max_signatures)
    }

    #[tool(description = "Extract signatures from a crate in the local cargo cache by name and optional version, or from a direct file/directory path. Returns structured JSON.")]
    async fn analyze_package(&self, params: rmcp::handler::server::wrapper::Parameters<AnalyzePackageArgs>) -> String {
        let AnalyzePackageArgs { package, version, max_signatures } = params.0;
        let target = Path::new(&package);
        if target.exists() {
            return analyze_to_json(&package, max_signatures);
        }
        match find_package_dir(&package, version.as_deref()) {
            Ok(dir) => analyze_to_json(dir.to_str().unwrap_or_default(), max_signatures),
            Err(e) => serde_json::to_string(&AnalyzeResult::Error { message: e }).unwrap(),
        }
    }

    #[tool(description = "Find a crate in cargo cache (or use a direct file/directory path) and search its signatures for a given string. Returns structured JSON.")]
    async fn search_package_signatures(&self, params: rmcp::handler::server::wrapper::Parameters<SearchPackageArgs>) -> String {
        let SearchPackageArgs { package, version, query } = params.0;
        let target = Path::new(&package);
        if target.exists() {
            return search_signatures_json(&package, &query);
        }
        match find_package_dir(&package, version.as_deref()) {
            Ok(dir) => search_signatures_json(dir.to_str().unwrap_or_default(), &query),
            Err(e) => serde_json::to_string(&SearchResult::Error { message: e }).unwrap(),
        }
    }

    #[tool(description = "Analyze a Rust file or directory and search its signatures for a given string. Returns structured JSON.")]
    async fn search_directory_signatures(&self, params: rmcp::handler::server::wrapper::Parameters<SearchDirectoryArgs>) -> String {
        let SearchDirectoryArgs { path, query } = params.0;
        let target = Path::new(&path);
        if !target.exists() {
            return serde_json::to_string(&SearchResult::Error {
                message: format!("Path not found: {}", path),
            }).unwrap();
        }
        search_signatures_json(&path, &query)
    }
}

#[rmcp::tool_handler]
impl rmcp::ServerHandler for RustSigServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Analyzes Rust source files and extracts signatures with doc comments. Returns structured JSON. Can analyze local directories or crates from cargo cache.".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
