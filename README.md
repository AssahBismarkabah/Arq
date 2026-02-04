<p align="center">
  <img src="assets/Arq.png" alt="Arq">
</p>


# Spec-first AI coding agent with deep codebase understanding

[![Release](https://img.shields.io/github/v/release/AssahBismarkabah/Arq?label=release)](https://github.com/AssahBismarkabah/Arq/releases)
[![CI](https://github.com/AssahBismarkabah/Arq/actions/workflows/ci.yml/badge.svg)](https://github.com/AssahBismarkabah/Arq/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Stars](https://img.shields.io/github/stars/AssahBismarkabah/Arq?style=flat)](https://github.com/AssahBismarkabah/Arq/stargazers)
[![Commits](https://img.shields.io/github/commit-activity/m/AssahBismarkabah/Arq)](https://github.com/AssahBismarkabah/Arq/commits)
![Language](https://img.shields.io/github/languages/top/AssahBismarkabah/Arq)
![Repo Size](https://img.shields.io/github/repo-size/AssahBismarkabah/Arq)


Arq builds a **Knowledge Graph** of your codebase, enabling it to reason about architectural patterns, dependencies, and cross-file impacts before writing code. Unlike traditional AI tools that rely on simple RAG, Arq ensures technically sound decisions through a disciplined three-phase workflow.

arq Analyzes the codebase to validate feasibility, identify patterns, and map dependencies , Generates a detailed technical specification and execution plan, Executes the approved plan, respecting existing architecture and idioms.


## Installation

### Quick Install (macOS/Linux)
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/AssahBismarkabah/Arq/releases/latest/download/arq-installer.sh | sh
```

### Windows (PowerShell)
```powershell
irm https://github.com/AssahBismarkabah/Arq/releases/latest/download/arq-installer.ps1 | iex
```

### Homebrew
```bash
brew install AssahBismarkabah/tap/arq
```

## Getting Started

1. **Configure your LLM provider**:
   ```bash
   export OPENAI_API_KEY="sk-..."
   ```

2. **Initialize your project**:
   ```bash
   arq init
   ```
   *This indexes your codebase into the local knowledge graph.*

3. **Launch the interactive TUI**:
   ```bash
   arq tui
   ```

<p align="center">
  <img src="assets/arq.gif" alt="Arq TUI Demo" width="800">
</p>

## Configuration

Create an optional `arq.toml` in your project root to customize Arq's behavior:

```toml
[llm]
provider = "openai"          # openai, anthropic, ollama, openrouter
model = "gpt-4o"
base_url = "https://api.openai.com/v1"  # Optional: custom endpoint
max_tokens = 8192            # Max tokens for LLM responses
available_models = [         # Models shown in TUI selector
    "gpt-4o",
    "gpt-4o-mini",
    "o1-preview"
]

[context]
max_file_size = 102400       # 100KB - skip files larger than this
max_total_size = 512000      # 500KB - total context budget
include_extensions = [
    "rs", "ts", "py", "go", "java", "cs"
]
exclude_dirs = [
    "node_modules", "target", ".git", "dist"
]
exclude_patterns = [
    "*.lock", "*.min.js", "*.map"
]

[storage]
data_dir = "~/.arq"          # Where Arq stores task data

[research]
# Custom system prompt for the research phase (optional)
# system_prompt = "You are a code analyst..."

[knowledge]
db_path = "knowledge.db"     # Knowledge graph database location
embedding_model = "Xenova/bge-small-en-v1.5"  # See supported models below
max_chunk_size = 1000        # Characters per chunk (larger = more context)
chunk_overlap = 100          # Overlap between chunks
search_limit = 20            # Max semantic search results
```

### Configuration Reference

| Section | Key | Default | Description |
|---------|-----|---------|-------------|
| `[llm]` | `provider` | `openai` | Provider: `openai`, `anthropic`, `ollama`, `openrouter` |
| `[llm]` | `model` | Provider default | Model name (e.g., `gpt-4o`, `claude-sonnet-4-20250514`) |
| `[llm]` | `base_url` | Provider default | API endpoint URL |
| `[llm]` | `api_key` | From env | API key (prefer env vars: `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`) |
| `[llm]` | `max_tokens` | `8192` | Maximum tokens for LLM responses |
| `[llm]` | `api_version` | `2023-06-01` | API version (Anthropic only) |
| `[llm]` | `available_models` | `[]` | Models to show in TUI model selector |
| `[context]` | `max_file_size` | `102400` | Skip files larger than this (bytes) |
| `[context]` | `max_total_size` | `512000` | Total context size budget (bytes) |
| `[context]` | `include_extensions` | Many | File extensions to include (without dot) |
| `[context]` | `exclude_dirs` | Many | Directories to skip (e.g., `node_modules`) |
| `[context]` | `exclude_patterns` | Many | Glob patterns to exclude (e.g., `*.lock`) |
| `[storage]` | `data_dir` | `~/.arq` | Base directory for Arq data |
| `[storage]` | `tasks_dir` | `tasks` | Subdirectory for task metadata |
| `[research]` | `system_prompt` | Built-in | Custom system prompt for research LLM calls |
| `[knowledge]` | `db_path` | `knowledge.db` | Database file location (relative to project dir) |
| `[knowledge]` | `embedding_model` | `Xenova/bge-small-en-v1.5` | Embedding model for semantic search (see below) |
| `[knowledge]` | `max_chunk_size` | `1000` | Max characters per code chunk for indexing |
| `[knowledge]` | `chunk_overlap` | `100` | Overlap between chunks (preserves context) |
| `[knowledge]` | `search_limit` | `20` | Max search results for semantic queries |

**Note:** Changing `embedding_model` requires re-indexing: `arq init --force`

### Supported Embedding Models

| Model Code | Dimensions | Description |
|------------|------------|-------------|
| `Xenova/bge-small-en-v1.5` | 384 | **Default** - Fast English model, optimized for code |
| `Xenova/bge-base-en-v1.5` | 768 | Base English model |
| `Xenova/bge-large-en-v1.5` | 1024 | Large English model (higher accuracy) |
| `jinaai/jina-embeddings-v2-base-code` | 768 | Specialized for code search |
| `nomic-ai/nomic-embed-text-v1.5` | 768 | 8192 context length |
| `intfloat/multilingual-e5-small` | 384 | Multilingual support |
| `intfloat/multilingual-e5-base` | 768 | Multilingual support |
| `Xenova/bge-small-zh-v1.5` | 512 | Chinese language model |
| `Xenova/bge-large-zh-v1.5` | 1024 | Chinese language model |

Quantized variants (smaller, faster) are available by adding `-onnx-Q` suffix or using `model_quantized.onnx`.

### Environment Variables

Arq supports environment variable overrides:

```bash
# LLM Configuration
export ARQ_LLM_PROVIDER="anthropic"
export ARQ_LLM_MODEL="claude-sonnet-4-20250514"
export ARQ_LLM_BASE_URL="https://api.anthropic.com/v1/messages"
export ARQ_LLM_API_KEY="your-key"
export ARQ_LLM_MAX_TOKENS="8192"

# Provider-specific keys (fallback)
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENROUTER_API_KEY="sk-or-..."

# Context limits
export ARQ_MAX_FILE_SIZE="102400"
export ARQ_MAX_TOTAL_SIZE="512000"

# Storage
export ARQ_DATA_DIR="~/.arq"
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `init` | Index codebase into the local knowledge graph |
| `new` | Initialize a new task from a natural language prompt |
| `research` | Execute the research phase to analyze the codebase and context |
| `advance` | Progress the current task to the next phase (Research -> Planning -> Agent) |
| `status` | Display the current task's progress and active phase |
| `search` | Perform semantic vector search across the indexed codebase |
| `tui` | Launch the interactive terminal user interface |
| `serve` | Start the web-based knowledge graph visualization server |
| `graph` | Query specific graph relationships (dependencies/impact) via CLI |
| `kg-status` | Show detailed statistics about the indexed knowledge graph |
| `list` | List all tasks managed by Arq |
| `switch` | Switch the active context to a different task |
| `delete` | Remove a task and its associated artifacts |

## Contributing

Contributions welcome. See [GitHub Issues](https://github.com/AssahBismarkabah/Arq/issues) for bug reports and feature requests.

## License

Apache License 2.0. See [LICENSE](LICENSE) for details.
