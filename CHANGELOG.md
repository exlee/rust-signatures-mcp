# Changelog

## [Unreleased]

## [0.2.0] - 2026-04-22

### Added
- CLI subcommands (`analyze`, `analyze-package`, `search-package`, `search-directory`, `list-files`, `serve`) for direct use from the command line
- Pi extension (`rust-signatures-pi/`) providing 5 tools (`rust_analyze`, `rust_analyze_package`, `rust_search_package`, `rust_search_directory`, `rust_list_files`)
- Categorized markdown output grouping signatures by kind (`### Functions`, `### Structs`, `### Enums`, `### Traits`, `### Impls`)
- `prettyplease` crate for idiomatic Rust code formatting of signatures

### Changed
- CLI output switched from JSON to categorized Markdown format
- MCP server support made optional via `mcp` feature flag (still default on); `--no-default-features` builds a CLI-only binary
- `rmcp` dependency gated behind the `mcp` feature
- JSON output functions gated behind `#[cfg(feature = "mcp")]` to eliminate dead code in CLI-only builds

## [0.1.1] - 2026-04-21

### ADDED

- Regexp support in queries

### CHANGED

- Results are served as JSON
- Tools note that path should be provided as absolute where applicable
 
### MAINTENANCE

- Update rmcp to `1.5.0`
- Split one file into modules

