//! Tree-sitter based parsing utilities shared across language parsers.

use tree_sitter::{Language, Node, Parser as TSParser, Tree};

use super::result::ParseResult;
use super::traits::{Parser, ParserCapability};
use crate::knowledge::ontology::nodes::ComplexityMetrics;

/// Base tree-sitter parser with shared functionality.
pub struct TreeSitterParser {
    language: Language,
    language_name: &'static str,
    extensions: &'static [&'static str],
}

impl TreeSitterParser {
    pub fn new(language: Language, language_name: &'static str, extensions: &'static [&'static str]) -> Self {
        Self { language, language_name, extensions }
    }

    /// Parse source code into a tree-sitter tree.
    pub fn parse_tree(&self, content: &str) -> Result<Tree, String> {
        let mut parser = TSParser::new();
        parser.set_language(&self.language)
            .map_err(|e| format!("Failed to set language: {}", e))?;

        parser.parse(content, None)
            .ok_or_else(|| "Failed to parse content".to_string())
    }

    /// Get text for a node from source content.
    pub fn node_text<'a>(node: &Node, content: &'a str) -> &'a str {
        &content[node.byte_range()]
    }

    /// Get line number (1-based) for a node.
    pub fn node_line(node: &Node) -> u32 {
        node.start_position().row as u32 + 1
    }

    /// Get end line number (1-based) for a node.
    pub fn node_end_line(node: &Node) -> u32 {
        node.end_position().row as u32 + 1
    }

    /// Find child node by field name.
    #[allow(dead_code)]
    pub fn child_by_field<'a>(node: &'a Node, field: &str) -> Option<Node<'a>> {
        node.child_by_field_name(field)
    }

    /// Find all children of a specific kind.
    #[allow(dead_code)]
    pub fn children_of_kind<'a>(node: &'a Node<'a>, kind: &str) -> Vec<Node<'a>> {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .filter(|n| n.kind() == kind)
            .collect()
    }

    /// Calculate basic complexity from node.
    pub fn calculate_complexity(node: &Node, content: &str) -> Option<ComplexityMetrics> {
        let text = Self::node_text(node, content);
        let loc = (node.end_position().row - node.start_position().row + 1) as u32;

        // Simple cyclomatic complexity estimation
        let branching = ["if", "else", "for", "while", "switch", "case", "catch", "?", "&&", "||"];
        let mut cyclomatic = 1u32;
        for keyword in branching {
            cyclomatic += text.matches(keyword).count() as u32;
        }

        Some(ComplexityMetrics {
            cyclomatic,
            loc,
            cognitive: None,
        })
    }
}

impl Parser for TreeSitterParser {
    fn parse_file(&self, _path: &str, _content: &str) -> Result<ParseResult, String> {
        // Base implementation - should be overridden by specific parsers
        Err("Use language-specific parser".to_string())
    }

    fn language_name(&self) -> &'static str {
        self.language_name
    }

    fn supported_extensions(&self) -> &[&'static str] {
        self.extensions
    }

    fn capability(&self) -> ParserCapability {
        ParserCapability::Structural
    }
}

/// Helper to extract doc comments from preceding nodes.
pub fn extract_doc_comment(node: &Node, content: &str) -> Option<String> {
    let mut comments = Vec::new();
    let mut sibling = node.prev_sibling();

    while let Some(s) = sibling {
        if s.kind() == "comment" {
            let text = TreeSitterParser::node_text(&s, content);
            // Strip comment markers
            let cleaned = text
                .trim_start_matches("//")
                .trim_start_matches("///")
                .trim_start_matches("/*")
                .trim_end_matches("*/")
                .trim_start_matches("*")
                .trim_start_matches("#")
                .trim();
            comments.push(cleaned.to_string());
        } else if s.kind() != "comment" {
            break;
        }
        sibling = s.prev_sibling();
    }

    if comments.is_empty() {
        None
    } else {
        comments.reverse();
        Some(comments.join("\n"))
    }
}
