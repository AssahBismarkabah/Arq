//! Core parser trait for language-agnostic code extraction.

use super::result::ParseResult;

/// Language-agnostic parser trait.
///
/// Implement this trait for each language to extract rich ontology entities
/// from source code. Each parser is responsible for:
///
/// 1. **Node extraction**: Functions, structs, traits, enums, constants
/// 2. **Edge extraction**: Calls, implements, uses_type, contains
/// 3. **Metadata**: Doc comments, visibility, generics, attributes
///
/// # Example Implementation
///
/// ```ignore
/// impl Parser for RustParser {
///     fn parse_file(&self, path: &str, content: &str) -> Result<ParseResult, String> {
///         let syntax = syn::parse_file(content)?;
///         // Extract nodes and edges from AST...
///     }
///
///     fn language_name(&self) -> &'static str { "Rust" }
///     fn supported_extensions(&self) -> &[&'static str] { &["rs"] }
/// }
/// ```
pub trait Parser: Send + Sync {
    /// Parse a source file and extract ontology entities.
    ///
    /// # Arguments
    /// * `path` - Relative path to the file (used for entity IDs)
    /// * `content` - Source code content
    ///
    /// # Returns
    /// * `Ok(ParseResult)` - Extracted nodes and edges
    /// * `Err(String)` - Parse error message
    fn parse_file(&self, path: &str, content: &str) -> Result<ParseResult, String>;

    /// Human-readable language name.
    fn language_name(&self) -> &'static str;

    /// File extensions this parser handles.
    fn supported_extensions(&self) -> &[&'static str];

    /// Check if this parser can handle the given file extension.
    fn can_parse(&self, extension: &str) -> bool {
        self.supported_extensions()
            .iter()
            .any(|ext| ext.eq_ignore_ascii_case(extension))
    }

    /// Parse capability level.
    ///
    /// Returns how deeply this parser can analyze code:
    /// - `Basic`: Simple regex-based extraction (names, positions)
    /// - `Structural`: AST-based extraction (full signatures, generics)
    /// - `Semantic`: Type-resolved analysis (call targets, type usage)
    fn capability(&self) -> ParserCapability {
        ParserCapability::Structural
    }
}

/// Level of parsing capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParserCapability {
    /// Basic regex extraction - names and line numbers only.
    Basic,
    /// AST-based extraction - full signatures, generics, visibility.
    Structural,
    /// Semantic analysis - resolved types, call targets.
    Semantic,
}

impl std::fmt::Display for ParserCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "Basic"),
            Self::Structural => write!(f, "Structural"),
            Self::Semantic => write!(f, "Semantic"),
        }
    }
}
