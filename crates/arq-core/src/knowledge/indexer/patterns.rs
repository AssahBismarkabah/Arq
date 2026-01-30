//! Regex patterns and keywords for code extraction.

/// Default file extensions to index.
pub const DEFAULT_EXTENSIONS: &[&str] = &[
    // Systems
    "rs", "c", "cpp", "h", "hpp", "go", // JVM
    "java", "kt", "scala", // .NET
    "cs", "fs", // Scripting
    "py", "rb", "php", // JavaScript
    "js", "ts", "tsx", "jsx", "vue", "svelte", // Mobile
    "swift",  // Functional
    "ml", "hs", "ex", "exs", "clj", // Web
    "html", "css", "scss", // Config
    "yaml", "yml", "toml", "json", // Docs
    "md",   // Database
    "sql",
];

/// Maximum chunk size in characters.
pub const MAX_CHUNK_SIZE: usize = 1000;

/// Chunk overlap in characters.
pub const CHUNK_OVERLAP: usize = 100;

/// Keywords to filter when extracting struct/class names.
pub const STRUCT_KEYWORDS: &[&str] = &["pub", "export", "public", "class", "struct", "abstract"];

/// Keywords to filter when extracting function names.
pub const FUNCTION_KEYWORDS: &[&str] = &[
    "pub",
    "async",
    "fn",
    "function",
    "export",
    "const",
    "let",
    "var",
    "def",
    "func",
    "public",
    "private",
    "protected",
    "static",
];

/// Common language keywords that look like function calls but aren't.
pub const NON_FUNCTION_KEYWORDS: &[&str] = &[
    "if", "for", "while", "match", "switch", "return", "new", "void", "int", "string", "bool",
];

/// Keywords to filter when extracting function calls.
pub const CALL_KEYWORDS: &[&str] = &[
    "if", "for", "while", "match", "switch", "return", "new", "fn", "function", "def", "func",
    "class", "struct", "enum", "type", "import", "from", "use", "pub", "async", "await",
];

/// Regex patterns for struct/class extraction.
/// Format: (pattern, description)
pub const STRUCT_PATTERNS: &[&str] = &[
    // Rust struct
    r"(?m)^(\s*)(pub\s+)?struct\s+(\w+)",
    // Rust enum
    r"(?m)^(\s*)(pub\s+)?enum\s+(\w+)",
    // Class (JS/TS/Python/Java/C#)
    r"(?m)^(\s*)(export\s+|public\s+|abstract\s+)?(class)\s+(\w+)",
    // Go struct
    r"(?m)^(\s*)type\s+(\w+)\s+struct",
    // Interface (TS/Go/Java)
    r"(?m)^(\s*)(export\s+|public\s+)?interface\s+(\w+)",
];

/// Regex patterns for function extraction.
/// Format: (pattern, is_simple_extraction)
pub const FUNCTION_PATTERNS: &[(&str, bool)] = &[
    // Rust fn
    (r"(?m)^(\s*)(pub\s+)?(async\s+)?fn\s+(\w+)", true),
    // JS/TS function
    (r"(?m)^(\s*)(export\s+)?(async\s+)?function\s+(\w+)", true),
    // JS/TS arrow function assignment
    (
        r"(?m)^(\s*)(export\s+)?(const|let|var)\s+(\w+)\s*=\s*(async\s*)?\([^)]*\)\s*=>",
        false,
    ),
    // Python def
    (r"(?m)^(\s*)(async\s+)?def\s+(\w+)", true),
    // Go func
    (r"(?m)^(\s*)func\s+(\([^)]*\)\s+)?(\w+)", false),
    // Java/C# method
    (
        r"(?m)^(\s*)(public|private|protected)?\s*(static\s+)?(async\s+)?(\w+)\s+(\w+)\s*\(",
        false,
    ),
];

/// Pattern for extracting function calls.
pub const CALL_PATTERN: &str = r"\b(\w+)\s*\(";
