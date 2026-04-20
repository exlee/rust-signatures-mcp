use std::fs;
use std::path::{Path, PathBuf};
use quote::quote;
use rmcp::{model::*, handler::server::router::tool::ToolRouter, ServiceExt, tool};
use schemars::JsonSchema;
use serde::Deserialize;
use syn::{visit::Visit, Attribute, File, ItemEnum, ItemFn, ItemImpl, ItemStruct, ItemTrait, Lit, Meta};
use ignore::WalkBuilder;

fn extract_docs(attrs: &[Attribute]) -> Vec<String> {
    attrs.iter().filter_map(|a| {
        if !a.path().is_ident("doc") { return None; }
        if let Meta::NameValue(nv) = &a.meta {
            if let syn::Expr::Lit(expr_lit) = &nv.value {
                if let Lit::Str(s) = &expr_lit.lit {
                    return Some(s.value().trim().to_string());
                }
            }
        }
        None
    }).collect()
}

struct SignatureCollector {
    output: String,
}

impl SignatureCollector {
    fn new() -> Self { Self { output: String::new() } }

    fn push_docs(&mut self, docs: &[String]) {
        for line in docs {
            self.output.push_str(&format!("/// {}\n", line));
        }
    }
}

impl<'ast> Visit<'ast> for SignatureCollector {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        self.push_docs(&extract_docs(&i.attrs));
        let sig = &i.sig;
        self.output.push_str(&format!("fn   {}\n", quote! { #sig }));
    }

    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        self.push_docs(&extract_docs(&i.attrs));
        let (name, generics) = (&i.ident, &i.generics);
        self.output.push_str(&format!("struct {}{}\n", name, quote! { #generics }));
    }

    fn visit_item_enum(&mut self, i: &'ast ItemEnum) {
        self.push_docs(&extract_docs(&i.attrs));
        let (name, generics) = (&i.ident, &i.generics);
        self.output.push_str(&format!("enum  {}{} {{\n", name, quote! { #generics }));
        for variant in &i.variants {
            let vdocs = extract_docs(&variant.attrs);
            for line in &vdocs {
                self.output.push_str(&format!("  /// {}\n", line));
            }
            let vname = &variant.ident;
            match &variant.fields {
                syn::Fields::Named(f) => {
                    let fields: Vec<_> = f.named.iter().map(|f| {
                        let (fname, ty) = (&f.ident, &f.ty);
                        quote! { #fname: #ty }
                    }).collect();
                    self.output.push_str(&format!("  {} {{ {} }}\n", vname, quote! { #(#fields),* }));
                }
                syn::Fields::Unnamed(f) => {
                    let types: Vec<_> = f.unnamed.iter().map(|f| { let ty = &f.ty; quote! { #ty } }).collect();
                    self.output.push_str(&format!("  {}({})\n", vname, quote! { #(#types),* }));
                }
                syn::Fields::Unit => {
                    self.output.push_str(&format!("  {}\n", vname));
                }
            }
        }
        self.output.push_str("}\n");
    }

    fn visit_item_trait(&mut self, i: &'ast ItemTrait) {
        self.push_docs(&extract_docs(&i.attrs));
        let (name, generics) = (&i.ident, &i.generics);
        self.output.push_str(&format!("trait  {}{}\n", name, quote! { #generics }));
    }

    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        let ty = &i.self_ty;
        if let Some((_, trait_, _)) = &i.trait_ {
            self.output.push_str(&format!("impl   {} for {}\n", quote! { #trait_ }, quote! { #ty }));
        } else {
            self.output.push_str(&format!("impl   {}\n", quote! { #ty }));
        }
        syn::visit::visit_item_impl(self, i);
    }
}

fn analyze_file(path: &Path) -> Option<String> {
    let src = fs::read_to_string(path).ok()?;
    let ast: File = syn::parse_file(&src).ok()?;
    let mut collector = SignatureCollector::new();
    collector.visit_file(&ast);
    Some(format!("=== {} ===\n{}{}", path.display(), collector.output, collector.output.is_empty().then_some("\n").unwrap_or("")))
}

fn analyze_path(target: &str) -> String {
    let path = Path::new(target);
    if !path.exists() {
        return format!("Path not found: {}", target);
    }
    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            return "Not a Rust file.".to_string();
        }
        return analyze_file(path).unwrap_or_else(|| "Failed to parse file.".to_string());
    }
    let mut result = String::new();
    for entry in WalkBuilder::new(target).build().filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        if entry_path.extension().and_then(|e| e.to_str()) != Some("rs") { continue; }
        if let Some(file_output) = analyze_file(entry_path) {
            result.push_str(&file_output);
            result.push('\n');
        }
    }
    if result.is_empty() {
        result.push_str("No Rust files found.");
    }
    result
}

fn cargo_registry_src() -> PathBuf {
    let cargo_home = std::env::var("CARGO_HOME").unwrap_or_else(|_| {
        std::env::var("HOME")
            .map(|h| format!("{}/.cargo", h))
            .unwrap_or_else(|_| ".cargo".into())
    });
    PathBuf::from(cargo_home).join("registry/src")
}

fn find_package_dir(package: &str, version: Option<&str>) -> Result<PathBuf, String> {
    let registry_src = cargo_registry_src();
    let normalized = package.replace('_', "-");

    let index_dirs: Vec<PathBuf> = match fs::read_dir(&registry_src) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|n| n.starts_with("index.crates.io-"))
                    .unwrap_or(false)
            })
            .map(|e| e.path())
            .collect(),
        Err(e) => return Err(format!("Cannot read cargo registry src: {}", e)),
    };

    if index_dirs.is_empty() {
        return Err("No crates.io index found in cargo registry. Run cargo build first.".into());
    }

    let prefix = format!("{}-", normalized);

    for index_dir in &index_dirs {
        let entries: Vec<_> = match fs::read_dir(index_dir) {
            Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
            Err(_) => continue,
        };

        let matching: Vec<(PathBuf, String)> = entries
            .into_iter()
            .filter_map(|e| {
                let name = e.file_name().to_str()?.to_string();
                let ver = name.strip_prefix(&prefix)?.to_string();
                if !e.path().is_dir() { return None; }
                Some((e.path(), ver))
            })
            .collect();

        if matching.is_empty() {
            continue;
        }

        if let Some(ver) = version {
            if let Some((path, _)) = matching.iter().find(|(_, v)| v.starts_with(ver)) {
                return Ok(path.clone());
            }
        } else {
            let best = matching
                .into_iter()
                .filter(|(_, v)| {
                    v.split('+').next()
                        .and_then(|sv| semver::Version::parse(sv).ok())
                        .is_some()
                })
                .max_by(|a, b| {
                    let va = semver::Version::parse(a.1.split('+').next().unwrap_or(&a.1)).ok();
                    let vb = semver::Version::parse(b.1.split('+').next().unwrap_or(&b.1)).ok();
                    match (va, vb) {
                        (Some(a), Some(b)) => a.cmp(&b),
                        (Some(_), None) => std::cmp::Ordering::Greater,
                        (None, Some(_)) => std::cmp::Ordering::Less,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });

            if let Some((path, _ver)) = best {
                return Ok(path);
            }
        }
    }

    match version {
        Some(v) => Err(format!("Package {} v{} not found in cargo cache", package, v)),
        None => Err(format!("Package {} not found in cargo cache", package)),
    }
}

fn search_signatures(content: &str, query: &str) -> String {
    let query_lower = query.to_lowercase();
    let mut result = String::new();
    let mut count = 0u32;

    for line in content.lines() {
        if line.to_lowercase().contains(&query_lower) {
            result.push_str(line);
            result.push('\n');
            count += 1;
        }
    }

    if count == 0 {
        return format!("No signatures matching \"{}\" found.", query);
    }

    format!("Found {} matching line(s):\n\n{}", count, result)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AnalyzeArgs {
    #[schemars(description = "File or directory path to scan for Rust signatures")]
    path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AnalyzePackageArgs {
    #[schemars(description = "Crate name (e.g. 'serde', 'clap_derive'). Alternatively, provide a direct file or directory path.")]
    package: String,
    #[schemars(description = "Optional version (e.g. '1.0.228'). Defaults to latest cached version. Ignored if package is a path.")]
    version: Option<String>,
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
struct RustSigServer {
    tool_router: ToolRouter<Self>,
}

impl RustSigServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[rmcp::tool_router]
impl RustSigServer {
    /// Analyze Rust file(s) at a given path and return their signatures and docstrings
    #[tool(description = "Extract all fn/struct/enum/trait/impl signatures and doc comments from a Rust file or all Rust files in a directory")]
    async fn analyze_rust(&self, params: rmcp::handler::server::wrapper::Parameters<AnalyzeArgs>) -> String {
        let AnalyzeArgs { path } = params.0;
        analyze_path(&path)
    }

    /// Analyze a crate from the local cargo cache, or a direct file/directory path
    #[tool(description = "Extract signatures from a crate in the local cargo cache by name and optional version, or from a direct file/directory path")]
    async fn analyze_package(&self, params: rmcp::handler::server::wrapper::Parameters<AnalyzePackageArgs>) -> String {
        let AnalyzePackageArgs { package, version } = params.0;
        let target = Path::new(&package);
        if target.exists() {
            return analyze_path(&package);
        }
        match find_package_dir(&package, version.as_deref()) {
            Ok(dir) => analyze_path(dir.to_str().unwrap_or_default()),
            Err(e) => e,
        }
    }

    /// Search signatures in a cached crate or direct path
    #[tool(description = "Find a crate in cargo cache (or use a direct file/directory path) and search its signatures for a given string")]
    async fn search_package_signatures(&self, params: rmcp::handler::server::wrapper::Parameters<SearchPackageArgs>) -> String {
        let SearchPackageArgs { package, version, query } = params.0;
        let target = Path::new(&package);
        let sigs = if target.exists() {
            analyze_path(&package)
        } else {
            match find_package_dir(&package, version.as_deref()) {
                Ok(dir) => analyze_path(dir.to_str().unwrap_or_default()),
                Err(e) => return e,
            }
        };
        search_signatures(&sigs, &query)
    }

    /// Search signatures in a file or directory
    #[tool(description = "Analyze a Rust file or directory and search its signatures for a given string")]
    async fn search_directory_signatures(&self, params: rmcp::handler::server::wrapper::Parameters<SearchDirectoryArgs>) -> String {
        let SearchDirectoryArgs { path, query } = params.0;
        let target = Path::new(&path);
        if !target.exists() {
            return format!("Path not found: {}", path);
        }
        let sigs = analyze_path(&path);
        search_signatures(&sigs, &query)
    }
}

#[rmcp::tool_handler]
impl rmcp::ServerHandler for RustSigServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Analyzes Rust source files and extracts signatures with doc comments. Can analyze local directories or crates from cargo cache.".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = rmcp::transport::stdio();
    let server = RustSigServer::new();
    server.serve(transport).await?.waiting().await?;
    Ok(())
}
