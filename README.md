# rust-signatures-mcp

MCP server and CLI tool that extracts and searches Rust function, struct, enum, trait, and impl signatures with doc comments from local source files or crates in the Cargo cache.

## Install

```
cargo install rust-signatures-mcp
```

By default this installs with the `mcp` feature enabled (includes MCP server support). For a lighter CLI-only install:

```
cargo install rust-signatures-mcp --no-default-features
```

## CLI Usage

The binary supports subcommands for direct use from the command line or from other tools:

```
# Extract signatures from a file or directory
rust-signatures-mcp analyze /path/to/src --max-signatures 50

# Extract signatures from a cached crate
rust-signatures-mcp analyze-package serde --version 1.0.228

# Search signatures in a cached crate using regex
rust-signatures-mcp search-package tokio --version 1 --query "async fn\\s+spawn"

# Search signatures in a local directory
rust-signatures-mcp search-directory /path/to/project/src --query "impl.*Read"

# List all .rs files in a directory
rust-signatures-mcp list-files /path/to/project/src

# Run as MCP server (stdio transport) — requires default features
rust-signatures-mcp serve
```

## MCP Server

Add to your MCP client configuration (e.g., Claude Desktop `claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "rust-signatures": {
      "command": "rust-signatures-mcp",
      "args": ["serve"]
    }
  }
}
```

## Pi Extension

This repository includes a Pi extension (`./rust-signatures-pi/`) that provides Rust signature analysis tools directly to Pi without requiring an MCP server.

### Setup

Copy the extension to your global Pi extensions directory:

```bash
cp -r ./rust-signatures-pi ~/.pi/agent/extensions/rust-signatures
```

Or symlink for development:

```bash
ln -s "$(pwd)/rust-signatures-pi" ~/.pi/agent/extensions/rust-signatures
```

The extension looks for the binary at `target/release/rust-signatures-mcp` (relative to the extension) first, then falls back to `PATH`.

### Tools Registered

| Tool | Description |
|------|-------------|
| `rust_analyze` | Extract signatures from a local file or directory |
| `rust_analyze_package` | Extract signatures from a cached crate or local path |
| `rust_search_package` | Search signatures in a cached crate or local path |
| `rust_search_directory` | Search signatures in local files/directories |
| `rust_list_files` | List all .rs files in a directory |

## License

MIT
