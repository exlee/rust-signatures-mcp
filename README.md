# rust-signatures-mcp

MCP server that extracts and searches Rust function, struct, enum, trait, and impl signatures with doc comments from local source files or crates in the Cargo cache.

## Install

```
cargo install rust-signatures-mcp
```

## Tools

| Tool | Description |
|------|-------------|
| `analyze_rust` | Extract signatures from a local file or directory |
| `analyze_package` | Extract signatures from a crate in the Cargo cache (or a local path) |
| `search_package_signatures` | Search signatures in a cached crate or local path |
| `search_directory_signatures` | Analyze a local file/directory and search signatures |

## Configuration

Add to your MCP client configuration (e.g., Claude Desktop `claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "rust-signatures": {
      "command": "rust-signatures-mcp"
    }
  }
}
```

## License

MIT
