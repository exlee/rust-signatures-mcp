import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { Type } from "@sinclair/typebox";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { resolve, dirname } from "node:path";
import { existsSync } from "node:fs";

const execFileAsync = promisify(execFile);

/** Resolve the path to the rust-signatures-mcp binary. */
function resolveBinary(): string {
  // Check project-local target first (for development)
  const localBin = resolve(import.meta.dirname, "../../../target/release/rust-signatures-mcp");
  if (existsSync(localBin)) return localBin;

  // Fall back to PATH (when installed via cargo install)
  return "rust-signatures-mcp";
}

/** Run the binary and return parsed JSON. */
async function runBin(args: string[], timeoutMs = 30_000): Promise<string> {
  const bin = resolveBinary();
  const { stdout } = await execFileAsync(bin, args, {
    timeout: timeoutMs,
    maxBuffer: 10 * 1024 * 1024, // 10MB
  });
  return stdout;
}

/**
 * Pi extension that provides Rust signature analysis tools
 * by wrapping the rust-signatures-mcp CLI binary.
 *
 * Install globally by copying the `.pi/extensions/rust-signatures/` directory
 * to `~/.pi/agent/extensions/rust-signatures/`.
 */
export default function (pi: ExtensionAPI) {
  pi.registerTool({
    name: "rust_analyze",
    label: "Rust Analyze",
    description:
      "Extract all fn/struct/enum/trait/impl signatures and doc comments from a Rust file or directory. Returns structured JSON with file paths, signature kinds, and doc comments.",
    promptSnippet: "Extract Rust fn/struct/enum/trait/impl signatures with doc comments from source files",
    promptGuidelines: [
      "Use rust_analyze when you need to understand the public API or internal structure of Rust source code.",
      "Prefer rust_analyze over reading full .rs files when you only need signatures and doc comments.",
    ],
    parameters: Type.Object({
      path: Type.String({
        description:
          "Absolute path to a .rs file or directory to scan for Rust signatures.",
      }),
      max_signatures: Type.Optional(
        Type.Integer({
          description:
            "Maximum number of signatures to return. Useful for large crates to limit context size.",
        })
      ),
    }),
    async execute(_toolCallId, params, signal) {
      const args = ["analyze", params.path];
      if (params.max_signatures != null) {
        args.push("--max-signatures", String(params.max_signatures));
      }
      const output = await runBin(args);
      return {
        content: [{ type: "text", text: output }],
        details: {},
      };
    },
  });

  pi.registerTool({
    name: "rust_analyze_package",
    label: "Rust Analyze Package",
    description:
      "Extract signatures from a crate in the local cargo cache by name and optional version, or from a direct file/directory path. Returns structured JSON.",
    promptSnippet: "Extract Rust signatures from cached crates (by name/version) or local paths",
    promptGuidelines: [
      "Use rust_analyze_package to inspect dependencies in the cargo cache without fetching sources.",
      "Provide a crate name (e.g. 'serde') and optionally a version. If the package is a path, provide the absolute path.",
    ],
    parameters: Type.Object({
      package: Type.String({
        description:
          "Crate name (e.g. 'serde') or absolute path to a .rs file or directory.",
      }),
      version: Type.Optional(
        Type.String({
          description:
            "Crate version (e.g. '1.0.228'). Ignored if package is a path.",
        })
      ),
      max_signatures: Type.Optional(
        Type.Integer({
          description:
            "Maximum number of signatures to return.",
        })
      ),
    }),
    async execute(_toolCallId, params, signal) {
      const args = ["analyze-package", params.package];
      if (params.version != null) args.push("--version", params.version);
      if (params.max_signatures != null)
        args.push("--max-signatures", String(params.max_signatures));
      const output = await runBin(args);
      return {
        content: [{ type: "text", text: output }],
        details: {},
      };
    },
  });

  pi.registerTool({
    name: "rust_search_package",
    label: "Rust Search Package",
    description:
      "Find a crate in cargo cache (or use a direct file/directory path) and search its signatures using a regex query. The query is matched case-insensitively against rendered signatures including doc comments. Returns structured JSON.",
    promptSnippet: "Search Rust signatures in cached crates or local paths using regex",
    promptGuidelines: [
      "Use rust_search_package to find specific functions, structs, traits etc. in dependencies.",
      "The query is a regular expression matched against full rendered signatures (including doc comments).",
    ],
    parameters: Type.Object({
      package: Type.String({
        description:
          "Crate name or absolute path to a .rs file or directory.",
      }),
      version: Type.Optional(
        Type.String({
          description:
            "Crate version. Ignored if package is a path.",
        })
      ),
      query: Type.String({
        description:
          "Regular expression to filter signatures. Matched case-insensitively against the full rendered signature including doc comments. Examples: 'process_data', 'async fn\\s+fetch', 'struct.*Config'.",
      }),
    }),
    async execute(_toolCallId, params, signal) {
      const args = ["search-package", params.package];
      if (params.version != null) args.push("--version", params.version);
      args.push(params.query);
      const output = await runBin(args);
      return {
        content: [{ type: "text", text: output }],
        details: {},
      };
    },
  });

  pi.registerTool({
    name: "rust_search_directory",
    label: "Rust Search Directory",
    description:
      "Analyze a Rust file or directory and search its signatures using a regex query. The query is matched case-insensitively against rendered signatures including doc comments. Returns structured JSON.",
    promptSnippet: "Search Rust signatures in local files/directories using regex",
    promptGuidelines: [
      "Use rust_search_directory to find specific signatures in the current project's source code.",
    ],
    parameters: Type.Object({
      path: Type.String({
        description:
          "Absolute path to a .rs file or directory to scan.",
      }),
      query: Type.String({
        description:
          "Regular expression to filter signatures. Examples: 'process_data', 'async fn\\s+fetch', 'struct.*Config'.",
      }),
    }),
    async execute(_toolCallId, params, signal) {
      const output = await runBin(["search-directory", params.path, params.query]);
      return {
        content: [{ type: "text", text: output }],
        details: {},
      };
    },
  });

  pi.registerTool({
    name: "rust_list_files",
    label: "Rust List Files",
    description:
      "List all Rust (.rs) files in a directory (recursively, respecting .gitignore). Returns a JSON list of file paths.",
    promptSnippet: "List all .rs files in a directory",
    parameters: Type.Object({
      path: Type.String({
        description:
          "Absolute path to a .rs file or directory.",
      }),
    }),
    async execute(_toolCallId, params, signal) {
      const output = await runBin(["list-files", params.path]);
      return {
        content: [{ type: "text", text: output }],
        details: {},
      };
    },
  });
}
