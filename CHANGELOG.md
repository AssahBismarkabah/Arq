# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `arq kg-clear` - Clear knowledge graph database
- `arq upgrade` - Show upgrade instructions for latest version
- `arq --version` - Display current version
- Progress indicators for `arq init` operations
- GitHub release notes auto-generation from PRs

### Changed

- Improved CLI help description

## [0.1.0] - 2025-01-31

### Added

- **Semantic Knowledge Graph**: Built on SurrealDB with Tree-sitter parsing for multi-language support (Rust, TypeScript, JavaScript, Python, Go, Java, C#)
- **Three-Phase Workflow**: Research, Planning, and Implementation phases for disciplined code generation
- **Smart Context Gathering**: Semantic vector search with BGE-Small embeddings for relevant code discovery
- **Interactive TUI**: Terminal-based chat interface for real-time task management
- **Graph Visualizer**: Web-based tool to explore project architecture (`arq serve`)
- **CLI Commands**: init, new, research, advance, search, graph, tui, serve, kg-status, list, status, switch, delete
- **Configuration**: `arq.toml` for project-level settings with environment variable overrides
- **Cross-Platform Support**: macOS (Intel & Apple Silicon), Linux, Windows

### Contributors

- @AssahBismarkabah

[Unreleased]: https://github.com/AssahBismarkabah/Arq/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/AssahBismarkabah/Arq/releases/tag/v0.1.0
