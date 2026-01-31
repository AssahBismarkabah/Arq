# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-01-31

### Added

- **Semantic Knowledge Graph**: Built on SurrealDB with Tree-sitter parsing for multi-language support (Rust, TypeScript, JavaScript, Python, Go, Java, C#)
- **Three-Phase Workflow**: Research, Planning, and Implementation phases for disciplined code generation
- **Smart Context Gathering**: Semantic vector search with BGE-Small embeddings for relevant code discovery
- **Interactive TUI**: Terminal-based chat interface for real-time task management
- **Graph Visualizer**: Web-based tool to explore project architecture (`arq serve`)
- **CLI Commands**:
  - `arq init` - Index codebase into knowledge graph with progress indicators
  - `arq new` - Start a new task from natural language prompt
  - `arq research` - Execute research phase
  - `arq advance` - Progress to next phase
  - `arq search` - Semantic code search
  - `arq graph` - Query dependencies and impact
  - `arq tui` - Launch interactive TUI
  - `arq serve` - Start visualization server
  - `arq kg-status` - Knowledge graph statistics
  - `arq kg-clear` - Clear knowledge graph database
  - `arq upgrade` - Show upgrade instructions
- **Configuration**: `arq.toml` for project-level settings with environment variable overrides
- **Cross-Platform Support**: macOS (Intel & Apple Silicon), Linux, Windows

### Contributors

- @AssahBismarkabah

[Unreleased]: https://github.com/AssahBismarkabah/Arq/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/AssahBismarkabah/Arq/releases/tag/v0.1.0

### Added

- add progress indicators for knowledge graph operations (https://github.com/AssahBismarkabah/Arq/issues/1)[#1]

