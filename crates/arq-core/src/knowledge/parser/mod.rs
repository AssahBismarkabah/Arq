//! Language-agnostic parsing infrastructure for code extraction.
//!
//! Provides a `Parser` trait for extracting ontology entities from source code,
//! with language-specific implementations for Rust (syn), TypeScript/JavaScript,
//! Python, Go, Java, and C# (tree-sitter).
//!
//! ## Components
//!
//! - `Parser` trait - Common interface for all language parsers
//! - `ParserRegistry` - Maps file extensions to appropriate parsers
//! - `ParseResult` - Contains extracted nodes and edges
//!
//! ## Supported Languages
//!
//! - Rust (syn-based, full AST extraction)
//! - TypeScript/JavaScript (tree-sitter)
//! - Python (tree-sitter)
//! - Go (tree-sitter)
//! - Java (tree-sitter)
//! - C# (tree-sitter)

mod csharp;
mod go;
mod java;
mod python;
mod registry;
mod result;
mod rust;
mod traits;
mod treesitter;
mod typescript;

pub use csharp::CSharpParser;
pub use go::GoParser;
pub use java::JavaParser;
pub use python::PythonParser;
pub use registry::ParserRegistry;
pub use result::{ParseResult, ParsedEdge, ParsedNode};
pub use rust::RustParser;
pub use traits::Parser;
pub use typescript::TypeScriptParser;
