# Arq

Arq is a spec-driven AI coding tool that ensures developers understand their codebase before generating code. It enforces a three-phase workflow: Research, Planning, and Implementation.

## Quick Start

```bash
# Set your LLM provider
export OPENAI_API_KEY="sk-..."  # or ANTHROPIC_API_KEY or OLLAMA_HOST

# Initialize knowledge graph (semantic search)
arq init

# Create and run a task
arq new "Add user authentication"
arq research
arq status

# Search your codebase semantically
arq search "authentication handler"
```

## The Problem

AI generates code faster than developers can understand it. This creates knowledge debt that compounds over time, unmaintainable systems nobody fully understands, and erosion of engineering judgment and pattern recognition.

Arq solves this by enforcing understanding before generation.

## Core Principles

**Understand Before You Build.** No code generation without validated understanding. The research phase ensures the AI and developer share the same mental model of the codebase.

**Human Decides, AI Executes.** Architectural decisions happen in planning, made by humans. The agent phase only implements what was explicitly approved.

**Spec as Contract.** The plan.yaml is a contract. The agent is bound to it. Deviations are flagged. No surprises, no scope creep, no hidden changes.

**Transparency Over Speed.** Every AI action is visible. Side-by-side comparison of intent vs output. Conformance checking validates execution matches plan.

## Installation

```bash
cargo install --path crates/arq-cli
```

Or build from source:

```bash
git clone https://github.com/AssahBismarkabah/arq
cd arq
cargo build --release
```

## Data Storage

Phase outputs are stored in your project's `.arq/` directory (add to `.gitignore` if desired):

- `.arq/research-doc.md` - Research findings for current task
- `.arq/plan.yaml` - Implementation plan for current task

Internal data (knowledge graph, task metadata) is stored in `~/.arq/` to keep your project clean.

## Configuration

Create an optional `arq.toml` in your project root. See [Configuration Reference](#configuration-reference) for all options.

```toml
# Example configuration
[context]
max_file_size = 102400
max_total_size = 512000
include_extensions = [
    "rs",
    "ts",
    "py",
    "go",
    "js",
    "tsx",
    "jsx",
]
exclude_dirs = [
    "node_modules",
    "target",
    ".git",
    "dist",
    "build",
]

[llm]
provider = "openai"
model = "gpt-4o"

[knowledge]
search_limit = 20
```

### Configuration Reference

#### `[context]` - Codebase scanning settings

| Parameter | Default | Description |
|-----------|---------|-------------|
| `max_file_size` | `102400` | Max bytes per file |
| `max_total_size` | `512000` | Max total context bytes |
| `include_extensions` | `[rs,ts,py..]` | File types to scan |
| `exclude_dirs` | `[node_modules..]` | Directories to skip |

#### `[llm]` - LLM provider settings

| Parameter | Default | Description |
|-----------|---------|-------------|
| `provider` | `openai` | openai, anthropic, ollama |
| `model` | `gpt-4o` | Model name |
| `base_url` | (per provider) | API endpoint URL |
| `api_key` | (from env) | API key (prefer env var) |

#### `[storage]` - Data persistence settings

| Parameter | Default | Description |
|-----------|---------|-------------|
| `data_dir` | `~/.arq` | Base directory |
| `tasks_dir` | `tasks` | Tasks subdirectory |
| `task_file` | `task.json` | Task metadata file |
| `research_file` | `research-doc.md` | Research output file |
| `plan_file` | `plan.yaml` | Plan output file |

#### `[knowledge]` - Knowledge graph settings

| Parameter | Default | Description |
|-----------|---------|-------------|
| `db_path` | `knowledge.db` | Database directory |
| `embedding_model` | `BGESmallENV15` | Local embedding model |
| `max_chunk_size` | `1000` | Max chunk size (chars) |
| `chunk_overlap` | `100` | Overlap between chunks |
| `search_limit` | `20` | Default search results |

### Environment Variables

Environment variables override `arq.toml` settings:

| Variable | Overrides |
|----------|-----------|
| `ARQ_LLM_PROVIDER` | `llm.provider` |
| `ARQ_LLM_MODEL` | `llm.model` |
| `ARQ_LLM_BASE_URL` | `llm.base_url` |
| `ARQ_LLM_API_KEY` | `llm.api_key` |
| `ARQ_DATA_DIR` | `storage.data_dir` |

Provider-specific keys (auto-detected):

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | For OpenAI provider |
| `ANTHROPIC_API_KEY` | For Anthropic provider |
| `OLLAMA_HOST` | For Ollama provider (e.g., `http://localhost:11434`) |

## CLI Commands

### Task Management

| Command | Description |
|---------|-------------|
| `arq new "<prompt>"` | Create a new task |
| `arq status` | Show current task status |
| `arq list` | List all tasks |
| `arq switch <id>` | Switch to a task |
| `arq delete <id>` | Delete a task |
| `arq research` | Run research phase |
| `arq advance` | Advance to next phase |

### Knowledge Graph

| Command | Description |
|---------|-------------|
| `arq init` | Index codebase into knowledge graph |
| `arq init --force` | Force re-index (rebuilds from scratch) |
| `arq search "<query>"` | Semantic code search |
| `arq search "<query>" -l 5` | Search with result limit |
| `arq kg-status` | Show knowledge graph statistics |

The knowledge graph enables semantic search (find code by meaning, not just keywords), faster research (relevant context ~5KB vs full dump ~500KB), and local embeddings (no API calls for embedding generation).

## Development

```bash
git clone https://github.com/AssahBismarkabah/arq
cd arq
cargo build --release
cargo test
```

## Contributing

Contributions welcome. Report bugs via GitHub issues, submit PRs against main branch, and follow existing code style.

## License

Apache License 2.0. See LICENSE file.
