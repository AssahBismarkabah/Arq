//! Parser registry for managing language-specific parsers.

use std::collections::HashMap;
use std::sync::Arc;

use super::csharp::CSharpParser;
use super::go::GoParser;
use super::java::JavaParser;
use super::python::PythonParser;
use super::rust::RustParser;
use super::traits::Parser;
use super::typescript::TypeScriptParser;

/// Registry of language parsers.
///
/// Maps file extensions to their respective parsers.
/// Automatically registers all built-in parsers on creation.
pub struct ParserRegistry {
    /// Extension to parser mapping.
    parsers: HashMap<String, Arc<dyn Parser>>,
}

impl ParserRegistry {
    /// Create a new registry with all built-in parsers.
    pub fn new() -> Self {
        let mut registry = Self {
            parsers: HashMap::new(),
        };

        // Register built-in parsers
        registry.register(Arc::new(RustParser::new()));
        registry.register(Arc::new(TypeScriptParser::typescript()));
        registry.register(Arc::new(TypeScriptParser::javascript()));
        registry.register(Arc::new(PythonParser::new()));
        registry.register(Arc::new(GoParser::new()));
        registry.register(Arc::new(JavaParser::new()));
        registry.register(Arc::new(CSharpParser::new()));

        registry
    }

    /// Register a parser for its supported extensions.
    pub fn register(&mut self, parser: Arc<dyn Parser>) {
        for ext in parser.supported_extensions() {
            self.parsers.insert(ext.to_lowercase(), Arc::clone(&parser));
        }
    }

    /// Get a parser for the given file extension.
    pub fn parser_for_extension(&self, extension: &str) -> Option<Arc<dyn Parser>> {
        self.parsers.get(&extension.to_lowercase()).cloned()
    }

    /// Get a parser for the given file path.
    pub fn parser_for_path(&self, path: &str) -> Option<Arc<dyn Parser>> {
        std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .and_then(|ext| self.parser_for_extension(ext))
    }

    /// Check if any parser can handle the given extension.
    pub fn can_parse(&self, extension: &str) -> bool {
        self.parsers.contains_key(&extension.to_lowercase())
    }

    /// List all supported extensions.
    pub fn supported_extensions(&self) -> Vec<&str> {
        self.parsers.keys().map(|s| s.as_str()).collect()
    }

    /// List all registered parsers with their languages.
    pub fn list_parsers(&self) -> Vec<(&str, &[&'static str])> {
        // Deduplicate parsers (same parser may be registered for multiple extensions)
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        for parser in self.parsers.values() {
            let name = parser.language_name();
            if seen.insert(name) {
                result.push((name, parser.supported_extensions()));
            }
        }

        result
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_rust_parser() {
        let registry = ParserRegistry::new();
        assert!(registry.can_parse("rs"));
        assert!(registry.parser_for_extension("rs").is_some());
    }

    #[test]
    fn test_registry_has_all_parsers() {
        let registry = ParserRegistry::new();

        // Rust
        assert!(registry.can_parse("rs"));

        // TypeScript/JavaScript
        assert!(registry.can_parse("ts"));
        assert!(registry.can_parse("tsx"));
        assert!(registry.can_parse("js"));
        assert!(registry.can_parse("jsx"));
        assert!(registry.can_parse("mjs"));
        assert!(registry.can_parse("cjs"));

        // Python
        assert!(registry.can_parse("py"));
        assert!(registry.can_parse("pyi"));

        // Go
        assert!(registry.can_parse("go"));

        // Java
        assert!(registry.can_parse("java"));

        // C#
        assert!(registry.can_parse("cs"));
    }

    #[test]
    fn test_parser_for_path() {
        let registry = ParserRegistry::new();
        assert!(registry.parser_for_path("src/lib.rs").is_some());
        assert!(registry.parser_for_path("src/main.py").is_some());
        assert!(registry.parser_for_path("src/app.ts").is_some());
        assert!(registry.parser_for_path("Main.java").is_some());
        assert!(registry.parser_for_path("Program.cs").is_some());
        assert!(registry.parser_for_path("main.go").is_some());
        assert!(registry.parser_for_path("unknown.xyz").is_none());
    }

    #[test]
    fn test_case_insensitive() {
        let registry = ParserRegistry::new();
        assert!(registry.can_parse("RS"));
        assert!(registry.can_parse("Rs"));
        assert!(registry.can_parse("PY"));
        assert!(registry.can_parse("TS"));
    }
}
