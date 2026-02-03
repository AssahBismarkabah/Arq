# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-02-03

### Added

- **AWS Bedrock Support**: Native integration with AWS Bedrock for Claude models
  - Uses the Bedrock Converse API for unified tool calling support
  - Automatic AWS credential chain (environment variables, ~/.aws/credentials, IAM roles)
  - Configure with `provider = "bedrock"` in arq.toml
  - Supports Claude Opus 4.5, Claude 3.5 Sonnet, and other Bedrock models
- **Planning Phase (TUI)**: Full implementation of the second phase in the Researcher → Planner → Agent workflow
  - Generate 2-3 implementation approaches with trade-offs from research findings
  - Select an approach or describe a custom one
  - Generate detailed implementation specification (plan.yaml)
  - Approve or refine the generated plan
- **`arq tui --continue`**: Restore previous session state when reopening TUI
- **Per-tab chat history**: Each tab (Researcher, Planner, Agent) maintains its own messages and scroll position
- **Planning progress indicators**: Visual feedback during approach generation and plan creation
- **Syntax highlighting in chat**: Code highlighting using `syntect` library with support for 100+ languages
- **Agent Tool System**: Modular tools for file, command, and Git operations

### Changed

- Improved LLM error handling with better messages for empty responses
- Planning prompts now include inline JSON format instructions for better provider compatibility
- Auto-advance task phase when saving plan (Research → Planning)
- **Faster scrolling**: j/k now scroll 3 lines, added Page Up/Down (Ctrl+u/d), Home/End (g/G) for navigation
- Improved string handling with character-aware truncation and UTF-8 support
- Adjusted default LLM token limits for better performance

### Fixed

- "Wrong phase: expected Planning, got Research" error when saving plans
- Tab state isolation - switching tabs no longer shows wrong content
- Progress indicators now properly update to complete state
- TUI task state synchronization with CLI operations
- Clippy warnings for Rust 1.93.0 compatibility

## [0.2.1] - 2025-01-31

### Added

- Bold ASCII art banner displayed on `arq --help` and TUI startup
- README header with logo image and GitHub badges (CI, Release, Stars, Activity)

### Changed

- `arq init` now shows actual progress bar with file count instead of spinner
- Updated tagline to "Spec-first AI agent"

## [0.2.0] - 2025-01-31

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

[Unreleased]: https://github.com/AssahBismarkabah/Arq/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/AssahBismarkabah/Arq/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/AssahBismarkabah/Arq/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/AssahBismarkabah/Arq/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/AssahBismarkabah/Arq/releases/tag/v0.1.0
