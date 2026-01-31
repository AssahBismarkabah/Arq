# Arq: Spec-Driven AI Code Engineering

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange.svg)](https://www.rust-lang.org/)

Arq is a next-generation AI coding engine designed for deep codebase understanding and high-precision code generation. Unlike traditional AI coding tools that rely on simple RAG (Retrieval-Augmented Generation), Arq builds a comprehensive **Semantic Knowledge Graph** of your project, enabling it to reason about architectural patterns, dependencies, and cross-file impacts before writing a single line of code.

---

## 🏗 Philosophy: The Three-Phase Workflow

Arq enforces a disciplined, spec-driven engineering process to eliminate hallucinations and ensure technical correctness.

1.  **Research**: Arq analyzes the codebase using its knowledge graph to validate the feasibility of a task, identify relevant patterns, and map out dependencies.
2.  **Planning**: Based on the research, Arq generates a detailed technical specification and execution plan.
3.  **Implementation**: An autonomous agent executes the approved plan, producing code that respects the project's existing architecture and idioms.

---

## 🧠 Core Technologies

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

## 🚀 Key Features

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

## 🎯 Getting Started

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

## 🛠 Configuration

Create an optional `arq.toml` in your project root to customize Arq's behavior:

```toml
[llm]
provider = "openai"
model = "gpt-4o"
available_models = ["gpt-4o", "gpt-4o-mini", "o1-preview"]

[context]
max_file_size = 102400  # 100KB
include_extensions = ["rs", "ts", "py", "go", "java", "cs"]
exclude_dirs = ["node_modules", "target", ".git", "dist"]

[knowledge]
db_path = "knowledge.db"
search_limit = 20
```

### Configuration Reference

| Section | Key | Default | Description |
|---------|-----|---------|-------------|
| `[llm]` | `provider` | `openai` | `openai`, `anthropic`, `ollama` |
| | `model` | `gpt-4o` | Primary model for generation |
| | `available_models` | — | Models for TUI selector |
| `[context]` | `include_extensions` | — | File types to index |
| `[knowledge]` | `db_path` | `knowledge.db` | Local database location |
| | `embedding_model` | `BGESmallENV15` | Local embedding model used |

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

## 🤝 Contributing

We welcome contributions! Please see our [GitHub Issues](https://github.com/AssahBismarkabah/Arq/issues) for bug reports and feature requests.

## 📄 License

Arq is released under the [Apache License 2.0](LICENSE).
