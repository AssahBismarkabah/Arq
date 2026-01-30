//! Structure entity nodes: Files, Modules, Packages.
//!
//! These represent the organizational structure of a codebase.

use serde::{Deserialize, Serialize};

// =============================================================================
// FILE ENTITY
// =============================================================================

/// A source file in the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntity {
    /// Unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Relative path from project root
    pub path: String,

    /// File name without path
    pub name: String,

    /// File extension
    pub extension: String,

    /// Programming language
    pub language: Language,

    /// SHA256 hash of contents (for change detection)
    pub content_hash: String,

    /// File size in bytes
    pub size: u64,

    /// Lines of code
    pub lines: u32,

    /// Module this file belongs to
    pub module: Option<String>,

    /// Package this file belongs to
    pub package: Option<String>,

    /// Import statements
    pub imports: Vec<ImportInfo>,

    /// Export statements (for JS/TS)
    pub exports: Vec<String>,

    /// When this file was last indexed
    pub indexed_at: String,
}

/// Programming language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    Kotlin,
    CSharp,
    Cpp,
    C,
    Ruby,
    Php,
    Swift,
    Yaml,
    Json,
    Toml,
    Markdown,
    #[default]
    Unknown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "ts" | "tsx" => Self::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Self::JavaScript,
            "py" | "pyi" => Self::Python,
            "go" => Self::Go,
            "java" => Self::Java,
            "kt" | "kts" => Self::Kotlin,
            "cs" => Self::CSharp,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Self::Cpp,
            "c" | "h" => Self::C,
            "rb" => Self::Ruby,
            "php" => Self::Php,
            "swift" => Self::Swift,
            "yaml" | "yml" => Self::Yaml,
            "json" => Self::Json,
            "toml" => Self::Toml,
            "md" | "markdown" => Self::Markdown,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Python => "python",
            Self::Go => "go",
            Self::Java => "java",
            Self::Kotlin => "kotlin",
            Self::CSharp => "csharp",
            Self::Cpp => "cpp",
            Self::C => "c",
            Self::Ruby => "ruby",
            Self::Php => "php",
            Self::Swift => "swift",
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Toml => "toml",
            Self::Markdown => "markdown",
            Self::Unknown => "unknown",
        }
    }
}

/// An import statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    /// The imported module/package
    pub source: String,
    /// Specific items imported (empty = wildcard or default)
    pub items: Vec<String>,
    /// Whether this is a re-export
    pub is_reexport: bool,
    /// Line number
    pub line: u32,
}

// =============================================================================
// MODULE ENTITY
// =============================================================================

/// A module in the codebase.
///
/// Represents a logical grouping of code (Rust mod, JS/TS module, Python package).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleEntity {
    /// Unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Module name
    pub name: String,

    /// Full path (e.g., "crate::module::submodule")
    pub path: String,

    /// File defining this module
    pub file_path: String,

    /// Parent module (None for root)
    pub parent: Option<String>,

    /// Visibility
    pub visibility: ModuleVisibility,

    /// Child modules
    pub children: Vec<String>,

    /// Files in this module
    pub files: Vec<String>,

    /// Re-exported items
    pub reexports: Vec<String>,

    /// Documentation
    pub doc_comment: Option<String>,
}

/// Module visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModuleVisibility {
    Public,
    Internal,
    #[default]
    Private,
}

// =============================================================================
// PACKAGE ENTITY
// =============================================================================

/// A package/crate in the codebase.
///
/// Represents the highest-level unit of code distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageEntity {
    /// Unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Package name
    pub name: String,

    /// Version string
    pub version: String,

    /// Package type
    pub package_type: PackageType,

    /// Root path relative to workspace
    pub root_path: String,

    /// Manifest file path (Cargo.toml, package.json, etc.)
    pub manifest_path: String,

    /// Entry points
    pub entry_points: Vec<EntryPoint>,

    /// Direct dependencies
    pub dependencies: Vec<DependencyInfo>,

    /// Dev dependencies
    pub dev_dependencies: Vec<DependencyInfo>,

    /// Authors
    pub authors: Vec<String>,

    /// License
    pub license: Option<String>,

    /// Description
    pub description: Option<String>,

    /// Repository URL
    pub repository: Option<String>,
}

/// Type of package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageType {
    /// Rust library crate
    RustLib,
    /// Rust binary crate
    RustBin,
    /// Rust proc-macro crate
    RustProcMacro,
    /// NPM package
    Npm,
    /// Python package
    Python,
    /// Go module
    Go,
    /// Java/Maven artifact
    Maven,
    /// Generic/unknown
    Unknown,
}

/// An entry point to a package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoint {
    pub name: String,
    pub path: String,
    pub entry_type: EntryPointType,
}

/// Type of entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryPointType {
    Library,
    Binary,
    Example,
    Bench,
    Test,
    Main,
}

/// Information about a dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub name: String,
    pub version: String,
    pub source: DependencySource,
    pub optional: bool,
    pub features: Vec<String>,
}

/// Source of a dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencySource {
    /// From a registry (crates.io, npm, pypi)
    Registry { registry: String },
    /// From git
    Git { url: String, branch: Option<String> },
    /// From local path
    Path { path: String },
    /// Workspace member
    Workspace,
}
