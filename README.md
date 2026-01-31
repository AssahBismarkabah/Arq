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

### Quick Install (Recommended)

**macOS/Linux:**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/AssahBismarkabah/Arq/releases/latest/download/arq-installer.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://github.com/AssahBismarkabah/Arq/releases/latest/download/arq-installer.ps1 | iex
```

**Homebrew (macOS/Linux):**
```bash
brew install AssahBismarkabah/tap/arq
```

### Build from Source

```bash
git clone https://github.com/AssahBismarkabah/arq
cd arq
cargo build --release
```

Or install directly:
```bash
cargo install --path crates/arq-cli
```

## Data Storage

Phase outputs are stored in your project's `.arq/` directory (add to `.gitignore` if desired):

- `.arq/research-doc.md` - Research findings for current task
- `.arq/plan.yaml` - Implementation plan for current task

Internal data (knowledge graph, task metadata) is stored in `~/.arq/` to keep your project clean.

## Configuration

Create an optional `arq.toml` in your project root:

```toml
[llm]
provider = "openai"
model = "gpt-4o"
available_models = [
    "gpt-4o",
    "gpt-4o-mini",
    "o1-preview",
]

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

[knowledge]
db_path = "knowledge.db"
embedding_model = "BGESmallENV15"
search_limit = 20
```

### Reference

| Section | Key | Default | Description |
|---------|-----|---------|-------------|
| `[context]` | `max_file_size` | `102400` | Max bytes per file |
| | `max_total_size` | `512000` | Max total context bytes |
| | `include_extensions` | `[rs,ts,py..]` | File types to scan |
| | `exclude_dirs` | `[node_modules..]` | Directories to skip |
| `[llm]` | `provider` | `openai` | openai, anthropic, ollama |
| | `model` | `gpt-4o` | Model name |
| | `base_url` | — | API endpoint URL |
| | `api_key` | — | API key (prefer env var) |
| | `available_models` | — | Models to show in TUI selector |
| `[storage]` | `data_dir` | `~/.arq` | Base directory for internal data |
| `[knowledge]` | `db_path` | `knowledge.db` | Database directory |
| | `embedding_model` | `BGESmallENV15` | Local embedding model |
| | `search_limit` | `20` | Default search results |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | OpenAI API key |
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OLLAMA_HOST` | Ollama endpoint (e.g., `http://localhost:11434`) |
| `ARQ_LLM_PROVIDER` | Override `llm.provider` |
| `ARQ_LLM_MODEL` | Override `llm.model` |
| `ARQ_DATA_DIR` | Override `storage.data_dir` |

## CLI Commands

```
$ arq --help
Spec-driven AI coding tool

Usage: arq <COMMAND>

Commands:
  new        Start a new task
  status     Show current task status
  list       List all tasks
  delete     Delete a task
  switch     Switch to a different task
  research   Run research phase for current task
  advance    Advance to the next phase
  init       Index codebase into knowledge graph
  search     Search code using semantic search
  kg-status  Show knowledge graph statistics
  graph      Query graph relationships (dependencies and impact)
  tui        Launch interactive TUI chat interface
  serve      Start visualization server for knowledge graph
  help       Print this message or the help of the given subcommand(s)
```

Use `arq <command> --help` for detailed options on any command.

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
