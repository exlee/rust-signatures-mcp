use rmcp::{model::*, tool};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::Path;

use crate::analyze::{analyze_to_json, list_rust_files_json};
use crate::registry::find_package_dir;
use crate::search::search_signatures_json;
use crate::types::{AnalyzeResult, FileListResult, SearchResult};

#[derive(Debug, Deserialize, JsonSchema)]
struct AnalyzeArgs {
    #[schemars(
        description = "Absolute path to a .rs file or directory to scan for Rust signatures. Must be an absolute path (e.g. /home/user/project/src)."
    )]
    path: String,
    #[schemars(
        description = "Optional maximum number of signatures to return. Useful for large crates to limit context size."
    )]
    max_signatures: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AnalyzePackageArgs {
    #[schemars(
        description = "Crate name (e.g. 'serde', 'clap_derive'). Alternatively, an absolute path to a .rs file or directory. Must be an absolute path if providing a path (e.g. /home/user/project/src)."
    )]
    package: String,
    #[schemars(
        description = "Optional version (e.g. '1.0.228'). Defaults to latest cached version. Ignored if package is a path."
    )]
    version: Option<String>,
    #[schemars(
        description = "Optional maximum number of signatures to return. Useful for large crates to limit context size."
    )]
    max_signatures: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchPackageArgs {
    #[schemars(
        description = "Crate name to search in. Alternatively, an absolute path to a .rs file or directory. Must be an absolute path if providing a path (e.g. /home/user/project/src)."
    )]
    package: String,
    #[schemars(
        description = "Optional version. Defaults to latest cached version. Ignored if package is a path."
    )]
    version: Option<String>,
    #[schemars(
        description = "Regular expression (regex) to filter signatures. Matched case-insensitively against the full rendered signature including doc comments. Examples: 'process_data', 'async fn\\s+fetch', 'struct.*Config'."
    )]
    query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchDirectoryArgs {
    #[schemars(
        description = "Absolute path to a .rs file or directory to scan for Rust files. Must be an absolute path (e.g. /home/user/project/src)."
    )]
    path: String,
    #[schemars(
        description = "Regular expression (regex) to filter signatures. Matched case-insensitively against the full rendered signature including doc comments. Examples: 'process_data', 'async fn\\s+fetch', 'struct.*Config'."
    )]
    query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListFilesArgs {
    #[schemars(
        description = "Absolute path to a .rs file or directory to list Rust files from. Must be an absolute path (e.g. /home/user/project/src)."
    )]
    path: String,
}

#[derive(Clone)]
pub struct RustSigServer {}

impl RustSigServer {
    pub fn new() -> Self {
        Self {}
    }
}

#[rmcp::tool_router]
impl RustSigServer {
    #[tool(
        description = "Extract all fn/struct/enum/trait/impl signatures and doc comments from a Rust file or all Rust files in a directory. The path parameter must be an absolute path (e.g. /home/user/project/src). Returns structured JSON."
    )]
    async fn analyze_rust(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<AnalyzeArgs>,
    ) -> String {
        let AnalyzeArgs {
            path,
            max_signatures,
        } = params.0;
        analyze_to_json(&path, max_signatures)
    }

    #[tool(
        description = "Extract signatures from a crate in the local cargo cache by name and optional version, or from a direct file/directory path. When providing a path, it must be an absolute path (e.g. /home/user/project/src). Returns structured JSON."
    )]
    async fn analyze_package(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<AnalyzePackageArgs>,
    ) -> String {
        let AnalyzePackageArgs {
            package,
            version,
            max_signatures,
        } = params.0;
        let target = Path::new(&package);
        if target.exists() {
            return analyze_to_json(&package, max_signatures);
        }
        match find_package_dir(&package, version.as_deref()) {
            Ok(dir) => analyze_to_json(dir.to_str().unwrap_or_default(), max_signatures),
            Err(e) => serde_json::to_string(&AnalyzeResult::Error { message: e }).unwrap(),
        }
    }

    #[tool(
        description = "Find a crate in cargo cache (or use a direct file/directory path) and search its signatures using a regex query. When providing a path, it must be an absolute path (e.g. /home/user/project/src). The query parameter is a regular expression matched case-insensitively against rendered signatures (including doc comments). Returns structured JSON."
    )]
    async fn search_package_signatures(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<SearchPackageArgs>,
    ) -> String {
        let SearchPackageArgs {
            package,
            version,
            query,
        } = params.0;
        let target = Path::new(&package);
        if target.exists() {
            return search_signatures_json(&package, &query);
        }
        match find_package_dir(&package, version.as_deref()) {
            Ok(dir) => search_signatures_json(dir.to_str().unwrap_or_default(), &query),
            Err(e) => serde_json::to_string(&SearchResult::Error { message: e }).unwrap(),
        }
    }

    #[tool(
        description = "Analyze a Rust file or directory and search its signatures using a regex query. The path parameter must be an absolute path (e.g. /home/user/project/src). The query parameter is a regular expression matched case-insensitively against rendered signatures (including doc comments). Returns structured JSON."
    )]
    async fn search_directory_signatures(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<SearchDirectoryArgs>,
    ) -> String {
        let SearchDirectoryArgs { path, query } = params.0;
        let target = Path::new(&path);
        if !target.exists() {
            return serde_json::to_string(&SearchResult::Error {
                message: format!("Path not found: {}", path),
            })
            .unwrap();
        }
        search_signatures_json(&path, &query)
    }

    #[tool(
        description = "List all Rust (.rs) files in a directory (recursively, respecting .gitignore). Returns a plain list of file paths. The path parameter must be an absolute path (e.g. /home/user/project/src)."
    )]
    async fn list_project_files(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<ListFilesArgs>,
    ) -> String {
        let ListFilesArgs { path } = params.0;
        let target = Path::new(&path);
        if !target.exists() {
            return serde_json::to_string(&FileListResult::Error {
                message: format!("Path not found: {}", path),
            })
            .unwrap();
        }
        list_rust_files_json(&path)
    }
}

#[rmcp::tool_handler]
impl rmcp::ServerHandler for RustSigServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Analyzes Rust source files and extracts signatures with doc comments. Returns structured JSON. Can analyze local directories or crates from cargo cache. IMPORTANT: All path parameters must be absolute paths (e.g. /home/user/project/src), not relative paths.")
    }
}
