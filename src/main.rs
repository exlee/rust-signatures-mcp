mod analyze;
mod collector;
mod registry;
mod search;
#[cfg(feature = "mcp")]
mod server;
mod types;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "rust-signatures",
    about = "Extract and search Rust signatures from source files and cached crates"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract all fn/struct/enum/trait/impl signatures and doc comments from a Rust file or directory
    Analyze {
        /// Absolute path to a .rs file or directory to scan
        path: String,
        /// Maximum number of signatures to return
        #[arg(long)]
        max_signatures: Option<usize>,
    },
    /// Extract signatures from a crate in the local cargo cache by name, or from a local path
    AnalyzePackage {
        /// Crate name or absolute path to a .rs file/directory
        package: String,
        /// Crate version (ignored for local paths)
        #[arg(long)]
        version: Option<String>,
        /// Maximum number of signatures to return
        #[arg(long)]
        max_signatures: Option<usize>,
    },
    /// Search signatures in a crate from cargo cache or local path using a regex
    SearchPackage {
        /// Crate name or absolute path to a .rs file/directory
        package: String,
        /// Crate version (ignored for local paths)
        #[arg(long)]
        version: Option<String>,
        /// Regular expression to filter signatures (case-insensitive)
        query: String,
    },
    /// Analyze a local file/directory and search signatures using a regex
    SearchDirectory {
        /// Absolute path to a .rs file or directory
        path: String,
        /// Regular expression to filter signatures (case-insensitive)
        query: String,
    },
    /// List all Rust (.rs) files in a directory
    ListFiles {
        /// Absolute path to a .rs file or directory
        path: String,
    },
    /// Run as MCP server (stdio transport)
    #[cfg(feature = "mcp")]
    Serve,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze {
            path,
            max_signatures,
        } => {
            println!("{}", analyze::analyze_to_markdown(&path, max_signatures));
        }
        Commands::AnalyzePackage {
            package,
            version,
            max_signatures,
        } => {
            let target = std::path::Path::new(&package);
            if target.exists() {
                println!("{}", analyze::analyze_to_markdown(&package, max_signatures));
            } else {
                match registry::find_package_dir(&package, version.as_deref()) {
                    Ok(dir) => println!(
                        "{}",
                        analyze::analyze_to_markdown(
                            dir.to_str().unwrap_or_default(),
                            max_signatures
                        )
                    ),
                    Err(e) => println!("Error: {}", e),
                }
            }
        }
        Commands::SearchPackage {
            package,
            version,
            query,
        } => {
            let target = std::path::Path::new(&package);
            if target.exists() {
                println!(
                    "{}",
                    search::search_signatures_to_markdown(&package, &query)
                );
            } else {
                match registry::find_package_dir(&package, version.as_deref()) {
                    Ok(dir) => {
                        println!(
                            "{}",
                            search::search_signatures_to_markdown(
                                dir.to_str().unwrap_or_default(),
                                &query
                            )
                        )
                    }
                    Err(e) => println!("Error: {}", e),
                }
            }
        }
        Commands::SearchDirectory { path, query } => {
            let target = std::path::Path::new(&path);
            if !target.exists() {
                println!("Error: Path not found: {}", path);
            } else {
                println!("{}", search::search_signatures_to_markdown(&path, &query));
            }
        }
        Commands::ListFiles { path } => {
            let target = std::path::Path::new(&path);
            if !target.exists() {
                println!("Error: Path not found: {}", path);
            } else {
                println!("{}", analyze::list_rust_files_to_markdown(&path));
            }
        }
        #[cfg(feature = "mcp")]
        Commands::Serve => {
            #[cfg(feature = "mcp")]
            {
                use rmcp::ServiceExt;
                use server::RustSigServer;

                let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                rt.block_on(async {
                    let transport = rmcp::transport::stdio();
                    let server = RustSigServer::new();
                    server
                        .serve(transport)
                        .await
                        .expect("Failed to serve")
                        .waiting()
                        .await
                        .expect("Failed to wait");
                });
            }
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::analyze::analyze_to_json;

    #[test]
    fn analyze_self_returns_structured_json() {
        let json = analyze_to_json("src/main.rs", Some(5));
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["type"], "success");
        assert_eq!(parsed["files"][0]["path"], "src/main.rs");
        let sigs = &parsed["files"][0]["signatures"];
        assert!(sigs.as_array().unwrap().len() <= 5);
    }
}
