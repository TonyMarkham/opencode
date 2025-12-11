# OpenCode Tool Summary

This document summarizes the built-in tools that the OpenCode server makes available to agents for toolcalling. Tool availability in a given session depends on agent configuration and permissions.

## Core filesystem and editing tools

- `read`  
  Read files from the workspace (or, with explicit permission, from outside it), optionally by line range; supports image attachments when the model can handle images.

- `write`  
  Write or overwrite entire files with provided content.

- `edit`  
  Apply targeted text replacements within a single file (`oldString` → `newString`), optionally replacing all occurrences; computes and records a diff and respects edit permissions.

- `multiedit`  
  Perform multiple coordinated edits across files using a higher-level edit description format.

- `list`  
  List files and directories under a given path, respecting workspace boundaries.

- `glob`  
  Perform pattern-based file discovery (e.g. `src/**/*.ts`) using fast globbing.

- `grep`  
  Search file contents for a regular expression pattern, scoped to the workspace.

- `patch`  
  Apply unified diff-style patches to files, updating them and recording changes.

## Shell and process tools

- `bash`  
  Execute shell commands in the project directory with tree-sitter-based parsing for arguments, safety checks, and permission prompts for dangerous or external-path operations.

- `batch`  
  Run multiple tool calls in parallel within a single assistant turn, tracking individual tool states and outputs (with some tools disallowed inside batch for safety).

## Code and language tools

- `codesearch`  
  Query a remote Exa MCP endpoint to retrieve high-signal code/API context snippets for a natural-language query.

- `lsp_hover`  
  Use the language server to fetch hover information (type info, docs, etc.) for a given file/range.

- `lsp_diagnostics`  
  Fetch diagnostics (errors, warnings) from configured language servers for files in the workspace.

## Web and external information tools

- `webfetch`  
  Fetch the content of a URL (HTTP/HTTPS), convert it (e.g. to markdown), and return it for use as model context.

- `websearch`  
  Perform a web search via Exa’s MCP endpoint and return textual search results/snippets tuned for LLM consumption.

## Planning, orchestration, and task tools

- `task`  
  Launch subagents / background tasks that can perform multi-step work (e.g. exploration, analysis) and return summarized results back into the main session.

- `todowrite`  
  Create or update a structured todo list (task items with status and priority) stored per project/session.

- `todoread`  
  Read the current todo list to help the model stay aligned with user-requested tasks.

## Utility and internal tools

- `invalid`  
  Internal tool used when a tool call is invalid or not recognized; helps surface better error messages and steer the model away from unsupported tools.

- `ls` (exposed as `list` tool)  
  The `list` tool replaces traditional `ls`/`dir` shell usage for safe, structured directory listing.

> Note: Additional tools discovered from configured MCP servers are **not** listed here, as they depend on your `mcp` configuration (`Config.mcp`). Use `GET /mcp/status` or the TUI sidebar to inspect active MCP servers and their provided tools in a given environment.
