//! Default values for Arq configuration.
//!
//! All hardcoded defaults are centralized here for easy maintenance.

// ============================================================================
// Context Defaults
// ============================================================================

/// Maximum size of a single file to include in context (100 KB).
pub const DEFAULT_MAX_FILE_SIZE: u64 = 100 * 1024;

/// Maximum total context size (500 KB).
pub const DEFAULT_MAX_TOTAL_SIZE: u64 = 500 * 1024;

/// Default file extensions to include in context gathering.
pub const DEFAULT_EXTENSIONS: &[&str] = &[
    // Rust
    "rs", "toml",
    // JavaScript/TypeScript
    "js", "ts", "jsx", "tsx", "mjs", "cjs",
    // Python
    "py", "pyi",
    // Go
    "go", "mod", "sum",
    // Java/Kotlin
    "java", "kt", "kts",
    // C/C++
    "c", "h", "cpp", "hpp", "cc", "hh",
    // C#
    "cs", "csproj",
    // Ruby
    "rb", "rake", "gemspec",
    // PHP
    "php",
    // Swift
    "swift",
    // Web
    "html", "css", "scss", "sass", "less",
    // Config/Data
    "json", "yaml", "yml", "xml",
    // Shell
    "sh", "bash", "zsh",
    // Documentation
    "md", "txt", "rst",
    // Build files
    "Makefile", "Dockerfile", "Containerfile",
];

/// Default directories to exclude from context gathering.
pub const DEFAULT_EXCLUDE_DIRS: &[&str] = &[
    // Version control
    ".git",
    ".svn",
    ".hg",
    // Dependencies
    "node_modules",
    "vendor",
    "venv",
    ".venv",
    "env",
    "__pycache__",
    ".pytest_cache",
    // Build outputs
    "target",
    "build",
    "dist",
    "out",
    "bin",
    "obj",
    // IDE/Editor
    ".idea",
    ".vscode",
    ".vs",
    // Arq's own data
    ".arq",
    // Other common excludes
    "coverage",
    ".coverage",
    ".nyc_output",
    ".next",
    ".nuxt",
    ".cache",
];

/// Default file patterns to exclude.
pub const DEFAULT_EXCLUDE_PATTERNS: &[&str] = &[
    "*.lock",
    "*.log",
    "*.min.js",
    "*.min.css",
    "*.map",
    "*.pyc",
    "*.pyo",
    "*.class",
    "*.o",
    "*.a",
    "*.so",
    "*.dll",
    "*.exe",
    "*.bin",
    "*.png",
    "*.jpg",
    "*.jpeg",
    "*.gif",
    "*.ico",
    "*.svg",
    "*.woff",
    "*.woff2",
    "*.ttf",
    "*.eot",
    "*.mp3",
    "*.mp4",
    "*.wav",
    "*.pdf",
    "*.zip",
    "*.tar",
    "*.gz",
    "*.rar",
];

// ============================================================================
// LLM Defaults
// ============================================================================

/// Default LLM provider.
pub const DEFAULT_LLM_PROVIDER: &str = "openai";

/// Default max tokens for LLM responses.
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

// OpenAI defaults
/// Default OpenAI API URL.
pub const DEFAULT_OPENAI_URL: &str = "https://api.openai.com/v1";
/// Default OpenAI model.
pub const DEFAULT_OPENAI_MODEL: &str = "gpt-4o";

// Anthropic defaults
/// Default Anthropic API URL.
pub const DEFAULT_ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
/// Default Anthropic model.
pub const DEFAULT_ANTHROPIC_MODEL: &str = "claude-sonnet-4-20250514";
/// Default Anthropic API version.
pub const DEFAULT_ANTHROPIC_API_VERSION: &str = "2023-06-01";

// Ollama defaults
/// Default Ollama API URL.
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434/v1";
/// Default Ollama model.
pub const DEFAULT_OLLAMA_MODEL: &str = "llama3";

// OpenRouter defaults
/// Default OpenRouter API URL.
pub const DEFAULT_OPENROUTER_URL: &str = "https://openrouter.ai/api/v1";

// ============================================================================
// Storage Defaults
// ============================================================================

/// Default data directory.
pub const DEFAULT_DATA_DIR: &str = ".arq";

/// Default tasks subdirectory.
pub const DEFAULT_TASKS_DIR: &str = "tasks";

/// Default task file name.
pub const DEFAULT_TASK_FILE: &str = "task.json";

/// Default research document file name.
pub const DEFAULT_RESEARCH_FILE: &str = "research-doc.md";

/// Default plan file name.
pub const DEFAULT_PLAN_FILE: &str = "plan.yaml";

/// Default current task pointer file name.
pub const DEFAULT_CURRENT_FILE: &str = "current";

// ============================================================================
// Research Defaults
// ============================================================================

/// Default error context length in error messages.
pub const DEFAULT_ERROR_CONTEXT_LENGTH: usize = 500;

/// Default word limit for task name derivation.
pub const DEFAULT_TASK_NAME_WORDS: usize = 5;

// ============================================================================
// System Prompts
// ============================================================================

/// Default system prompt for the research phase.
pub const DEFAULT_RESEARCH_SYSTEM_PROMPT: &str = r#"You are a code analyst helping a developer understand a codebase before making changes.

Your task is to analyze the provided codebase and create a research document that will help the developer understand:
1. The relevant parts of the codebase for their task
2. Dependencies and relationships between components
3. Existing patterns and conventions used
4. A suggested approach for implementing the task

Be thorough but concise. Focus on what's relevant to the task at hand.

IMPORTANT: Output your analysis as valid JSON matching this exact structure:
{
  "summary": "A 2-3 sentence summary of your findings",
  "findings": [
    {
      "title": "Finding title",
      "description": "Detailed description of the finding",
      "related_files": ["path/to/file1.rs", "path/to/file2.rs"]
    }
  ],
  "dependencies": [
    {
      "name": "Dependency name",
      "description": "What it does and why it's relevant",
      "is_external": true
    }
  ],
  "suggested_approach": "A clear, actionable description of how to implement the task"
}

Only output the JSON, no additional text."#;
