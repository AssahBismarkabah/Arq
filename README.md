# Arq

Arq is a spec-driven AI coding tool that ensures developers understand their codebase before generating code. It enforces a three-phase workflow: Research, Planning, and Implementation.

## Quick Start

```bash
# Set your LLM provider
export OPENAI_API_KEY="sk-..."  # or ANTHROPIC_API_KEY or OLLAMA_HOST

# Create and run a task
arq new "Add user authentication"
arq research
arq status
```

## The Problem

AI generates code faster than developers can understand it. This creates:

- Knowledge debt that compounds over time
- Unmaintainable systems nobody fully understands
- Erosion of engineering judgment and pattern recognition

Arq solves this by enforcing understanding before generation.

## Phase Documentation

Detailed documentation for each phase:

- [Research Phase](docs-concepts/research-phase)
- [Planning Phase](docs-concepts/planning-phase)
- [Agent Phase](docs-concepts/agent-phase)

## Core Principles

### Understand Before You Build

No code generation without validated understanding. Research phase ensures the AI and developer share the same mental model of the codebase.

### Human Decides, AI Executes

Architectural decisions happen in Planning, made by humans. Agent phase only implements what was explicitly approved.

### Spec as Contract

The plan.yaml is a contract. Agent is bound to it. Deviations are flagged. No surprises, no scope creep, no hidden changes.

### Transparency Over Speed

Every AI action is visible. Side-by-side comparison of intent vs output. Conformance checking validates execution matches plan.

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

## Project Structure

When using Arq, artifacts are saved in `.arq/` directory:

```
project/
├── src/
├── .arq/
│   └── tasks/{task-id}/
│       ├── task.json         # Task metadata
│       ├── research-doc.md   # Research findings
│       └── plan.yaml         # Approved specification
└── arq.toml                  # Configuration (optional)
```

These files are git-tracked, creating an audit trail of decisions.

## Configuration

Arq configuration in `arq.toml`:

```toml
[context]
max_file_size = 102400
max_total_size = 512000
include_extensions = ["rs", "ts", "py", "go", "js", "tsx", "jsx"]
exclude_dirs = ["node_modules", "target", ".git", "dist", "build"]

[llm]
provider = "openai"       # openai, anthropic, ollama
model = "gpt-4o"
base_url = "https://api.openai.com/v1"  # for custom endpoints
api_key = "sk-..."        # or use environment variable

[storage]
data_dir = ".arq"
```

### Configuration Reference

#### `[context]` - Codebase scanning settings

| Parameter | Default | Status | Description |
|-----------|---------|--------|-------------|
| `max_file_size` | `102400` | OK | Max bytes per file |
| `max_total_size` | `512000` | OK | Max total context bytes |
| `include_extensions` | `[rs,ts,py..]` | OK | File types to scan |
| `exclude_dirs` | `[node_mod..]` | OK | Directories to skip |
| `exclude_patterns` | `[]` | PLANNED | Glob patterns to skip |

#### `[llm]` - LLM provider settings

| Parameter | Default | Status | Description |
|-----------|---------|--------|-------------|
| `provider` | `openai` | OK | openai, anthropic, ollama |
| `model` | `gpt-4o` | OK | Model name |
| `base_url` | (per provider) | OK | API endpoint URL |
| `api_key` | (from env) | OK | API key (prefer env var) |
| `max_tokens` | `4096` | PLANNED | Max response tokens |
| `api_version` | `2023-06-01` | PLANNED | Anthropic API version |

#### `[storage]` - Data persistence settings

| Parameter | Default | Status | Description |
|-----------|---------|--------|-------------|
| `data_dir` | `.arq` | OK | Base directory |
| `tasks_dir` | `tasks` | OK | Tasks subdirectory |
| `task_file` | `task.json` | OK | Task metadata file |
| `research_file` | `research-doc.md` | OK | Research output file |
| `plan_file` | `plan.yaml` | OK | Plan output file |

#### `[research]` - Research phase settings

| Parameter | Default | Status | Description |
|-----------|---------|--------|-------------|
| `system_prompt` | (built-in) | PLANNED | Custom system prompt |
| `error_context_len` | `500` | PLANNED | Error message length |

### Environment Variables

Environment variables override `arq.toml` settings:

| Variable | Overrides |
|----------|-----------|
| `ARQ_LLM_PROVIDER` | `llm.provider` |
| `ARQ_LLM_MODEL` | `llm.model` |
| `ARQ_LLM_BASE_URL` | `llm.base_url` |
| `ARQ_LLM_API_KEY` | `llm.api_key` |
| `ARQ_LLM_MAX_TOKENS` | `llm.max_tokens` |
| `ARQ_MAX_FILE_SIZE` | `context.max_file_size` |
| `ARQ_MAX_TOTAL_SIZE` | `context.max_total_size` |
| `ARQ_DATA_DIR` | `storage.data_dir` |

Provider-specific keys (auto-detected):

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | For OpenAI provider |
| `ANTHROPIC_API_KEY` | For Anthropic provider |
| `OLLAMA_HOST` | For Ollama provider (e.g., `http://localhost:11434`) |

## Development

### Building from Source

```bash
git clone https://github.com/AssahBismarkabah/arq
cd arq
cargo build --release
```

### Running Tests

```bash
cargo test
```

## Contributing

Contributions welcome. See CONTRIBUTING.md for guidelines.

- Report bugs via GitHub issues
- Submit PRs against main branch
- Follow existing code style

## License

Apache License 2.0. See LICENSE file.
