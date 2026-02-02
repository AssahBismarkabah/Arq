![Arq](assets/Arq.png)

![GitHub Release](https://img.shields.io/github/v/release/AssahBismarkabah/Arq?label=latest%20release)
[![CI](https://github.com/AssahBismarkabah/Arq/actions/workflows/ci.yml/badge.svg)](https://github.com/AssahBismarkabah/Arq/actions/workflows/ci.yml)
[![Release](https://github.com/AssahBismarkabah/Arq/actions/workflows/release.yml/badge.svg)](https://github.com/AssahBismarkabah/Arq/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
![GitHub Repo stars](https://img.shields.io/github/stars/AssahBismarkabah/Arq?style=flat)
![GitHub commit activity](https://img.shields.io/github/commit-activity/m/AssahBismarkabah/Arq)

# Spec-first AI agent

Arq is a next-generation AI coding engine designed for deep codebase understanding and high-precision ai assisted development. Unliketraditional AI coding tools that rely on simple RAG (Retrieval-Augmented Generation), 

Arq builds a comprehensive ** Knowlege Graph** of your project, enabling it to reason about architectural patterns, dependencies, andcross-file impacts before enabling you to make technically sound decisions before writing a single line of code.


## 🏗 Philosophy: The Three-Phase Workflow

Arq enforces a disciplined, spec-driven engineering process to eliminate hallucinations and ensure technical correctness.

1.  **Research**: Arq analyzes the codebase using its knowledge graph to validate the feasibility of a task, identify relevant patterns, and map out dependencies.
2.  **Planning**: Based on the research, Arq generates a detailed technical specification and execution plan.
3.  **Implementation**: An autonomous agent executes the approved plan, producing code that respects the project's existing architecture and idioms.

---

## Core Technologies

### Semantic Knowledge Graph
Built on **SurrealDB**, Arq's knowledge graph goes beyond simple text chunks. It uses **Tree-sitter** to parse your code into a rich ontology of entities:
*   **Structural Nodes**: Files, Modules, Structs, Traits, Enums.
*   **Behavioral Nodes**: Functions, Methods, Constants.
*   **Relational Edges**: `Calls`, `DependsOn`, `Implements`, `Contains`.

### Smart Context Gathering
Instead of flooding the LLM with irrelevant files, Arq's **Smart Context** algorithm:
1.  Performs **semantic vector search** to find relevant code entry points.
2.  Traverses the **knowledge graph** to pull in critical dependencies and upstream callers.
3.  Synthesizes a "context package" that gives the LLM a 360-degree view of the target logic.

### Local-First & High Performance
*   **Rust-powered core** for maximum efficiency.
*   **Local Vector Embeddings** (BGE-Small) ensure your code stays private.
*   **RocksDB storage** for lightning-fast graph queries.

---

## Key Features

*   **Multi-Language Support**: Native parsing for **Rust, TypeScript, JavaScript, Python, Go, Java, and C#**.
*   **Interactive TUI**: A terminal-based collaborative environment for real-time task management.
*   **Graph Visualizer**: A web-based interactive tool to explore your project's architecture and the AI's internal representation.
*   **Spec-Driven**: Ensures deep understanding before generation, reducing iteration loops.

---

## 📦 Installation

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

---

##  Getting Started

1. **Configure your LLM provider**:
   ```bash
   export OPENAI_API_KEY="sk-..."
   ```

2. **Initialize your project**:
   ```bash
   arq init
   ```
   *This indexes your codebase into the local knowledge graph.*

3. **Start a new task**:
   ```bash
   arq new "Implement JWT authentication handler"
   ```

4. **Execute the workflow**:
   ```bash
   arq research  # Phase 1: Analyze codebase
   arq advance   # Phase 2: Create execution plan
   arq advance   # Phase 3: Generate code
   ```

---

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

---

## 🛠 CLI Commands

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

---

## Contributing

We welcome contributions! Please see our [GitHub Issues](https://github.com/AssahBismarkabah/Arq/issues) for bug reports and feature requests.

## 📄 License

Arq is released under the [Apache License 2.0](LICENSE).
