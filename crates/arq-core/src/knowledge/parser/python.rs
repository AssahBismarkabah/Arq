//! Python parser using tree-sitter.

use tree_sitter::Node;

use super::result::ParseResult;
use super::traits::{Parser, ParserCapability};
use super::treesitter::TreeSitterParser;
use crate::knowledge::ontology::edges::{CallsEdge, CallType, ContainsEdge};
use crate::knowledge::ontology::nodes::{
    FunctionEntity, StructEntity, Visibility, FieldInfo, Parameter,
};

/// Python parser using tree-sitter.
pub struct PythonParser {
    base: TreeSitterParser,
}

impl PythonParser {
    pub fn new() -> Self {
        Self {
            base: TreeSitterParser::new(
                tree_sitter_python::LANGUAGE.into(),
                "Python",
                &["py", "pyi"],
            ),
        }
    }

    fn extract_function(&self, node: &Node, content: &str, path: &str) -> Option<FunctionEntity> {
        let name_node = node.child_by_field_name("name")?;
        let name = TreeSitterParser::node_text(&name_node, content).to_string();

        let params = self.extract_parameters(node, content);
        let return_type = node.child_by_field_name("return_type")
            .map(|n| TreeSitterParser::node_text(&n, content).to_string());

        let is_async = node.kind() == "async_function_definition" ||
            TreeSitterParser::node_text(node, content).starts_with("async ");

        // Check for decorators
        let _decorators: Vec<String> = node.children(&mut node.walk())
            .filter(|c| c.kind() == "decorator")
            .map(|d| TreeSitterParser::node_text(&d, content).to_string())
            .collect();

        let visibility = self.extract_visibility(&name);

        Some(FunctionEntity {
            id: Some(format!("function:{}:{}", path, name)),
            name: name.clone(),
            qualified_name: name,
            file_path: path.to_string(),
            start_line: TreeSitterParser::node_line(node),
            end_line: TreeSitterParser::node_end_line(node),
            signature: self.build_signature(node, content),
            parent: None,
            visibility,
            is_async,
            is_unsafe: false,
            generics: Vec::new(),
            parameters: params,
            return_type,
            doc_comment: self.extract_docstring(node, content),
            complexity: TreeSitterParser::calculate_complexity(node, content),
        })
    }

    fn extract_class(&self, node: &Node, content: &str, path: &str) -> Option<StructEntity> {
        let name_node = node.child_by_field_name("name")?;
        let name = TreeSitterParser::node_text(&name_node, content).to_string();

        let fields = self.extract_class_fields(node, content);

        // Extract base classes
        let bases: Vec<String> = node.child_by_field_name("superclasses")
            .map(|sc| {
                let mut cursor = sc.walk();
                sc.children(&mut cursor)
                    .filter(|c| c.kind() == "identifier" || c.kind() == "attribute")
                    .map(|c| TreeSitterParser::node_text(&c, content).to_string())
                    .collect()
            })
            .unwrap_or_default();

        Some(StructEntity {
            id: Some(format!("struct:{}:{}", path, name)),
            name: name.clone(),
            qualified_name: name,
            file_path: path.to_string(),
            start_line: TreeSitterParser::node_line(node),
            end_line: TreeSitterParser::node_end_line(node),
            visibility: Visibility::Public,
            generics: Vec::new(),
            fields,
            derives: bases,
            attributes: Vec::new(),
            doc_comment: self.extract_docstring(node, content),
        })
    }

    fn extract_parameters(&self, node: &Node, content: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                match child.kind() {
                    "identifier" => {
                        let name = TreeSitterParser::node_text(&child, content).to_string();
                        if name != "self" && name != "cls" {
                            params.push(Parameter {
                                name,
                                type_name: "Any".to_string(),
                                is_mutable: false,
                                is_reference: false,
                            });
                        }
                    }
                    "typed_parameter" | "default_parameter" | "typed_default_parameter" => {
                        let name = child.child_by_field_name("name")
                            .or_else(|| child.children(&mut child.walk()).find(|c| c.kind() == "identifier"))
                            .map(|n| TreeSitterParser::node_text(&n, content).to_string())
                            .unwrap_or_default();

                        let type_name = child.child_by_field_name("type")
                            .map(|n| TreeSitterParser::node_text(&n, content).to_string())
                            .unwrap_or_else(|| "Any".to_string());

                        if !name.is_empty() && name != "self" && name != "cls" {
                            params.push(Parameter {
                                name,
                                type_name,
                                is_mutable: false,
                                is_reference: false,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        params
    }

    fn extract_class_fields(&self, node: &Node, content: &str) -> Vec<FieldInfo> {
        let mut fields = Vec::new();

        // Look for __init__ method and extract self.x assignments
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "block" {
                let mut block_cursor = child.walk();
                for stmt in child.children(&mut block_cursor) {
                    if stmt.kind() == "function_definition" {
                        let name = stmt.child_by_field_name("name")
                            .map(|n| TreeSitterParser::node_text(&n, content));
                        if name == Some("__init__") {
                            // Parse __init__ for self.x assignments
                            self.extract_init_fields(&stmt, content, &mut fields);
                        }
                    }
                }
            }
        }

        fields
    }

    fn extract_init_fields(&self, init_node: &Node, content: &str, fields: &mut Vec<FieldInfo>) {
        let mut cursor = init_node.walk();
        self.walk_for_assignments(init_node, content, fields, &mut cursor);
    }

    fn walk_for_assignments(&self, node: &Node, content: &str, fields: &mut Vec<FieldInfo>, _cursor: &mut tree_sitter::TreeCursor) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "expression_statement" {
                if let Some(assign) = child.children(&mut child.walk()).find(|c| c.kind() == "assignment") {
                    if let Some(left) = assign.child_by_field_name("left") {
                        let left_text = TreeSitterParser::node_text(&left, content);
                        if left_text.starts_with("self.") {
                            let field_name = left_text.trim_start_matches("self.").to_string();
                            if !fields.iter().any(|f| f.name == field_name) {
                                fields.push(FieldInfo {
                                    name: field_name,
                                    type_name: "Any".to_string(),
                                    visibility: Visibility::Public,
                                    attributes: Vec::new(),
                                    doc_comment: None,
                                });
                            }
                        }
                    }
                }
            }
            self.walk_for_assignments(&child, content, fields, &mut child.walk());
        }
    }

    fn build_signature(&self, node: &Node, content: &str) -> String {
        let first_line = TreeSitterParser::node_text(node, content)
            .lines()
            .next()
            .unwrap_or("");
        first_line.trim_end_matches(':').to_string()
    }

    fn extract_visibility(&self, name: &str) -> Visibility {
        if name.starts_with("__") && !name.ends_with("__") {
            Visibility::Private
        } else if name.starts_with("_") {
            Visibility::PublicCrate // Convention for "internal"
        } else {
            Visibility::Public
        }
    }

    fn extract_docstring(&self, node: &Node, content: &str) -> Option<String> {
        // Look for string as first statement in body
        let body = node.child_by_field_name("body")?;
        let mut cursor = body.walk();
        let first_stmt = body.children(&mut cursor).next()?;

        if first_stmt.kind() == "expression_statement" {
            let mut stmt_cursor = first_stmt.walk();
            let string_node = first_stmt.children(&mut stmt_cursor).find(|c| c.kind() == "string");
            if let Some(string_node) = string_node {
                let text = TreeSitterParser::node_text(&string_node, content);
                // Remove quotes
                let cleaned = text
                    .trim_start_matches("\"\"\"")
                    .trim_end_matches("\"\"\"")
                    .trim_start_matches("'''")
                    .trim_end_matches("'''")
                    .trim_start_matches('"')
                    .trim_end_matches('"')
                    .trim();
                return Some(cleaned.to_string());
            }
        }

        None
    }

    /// Extract function calls from a node.
    fn extract_calls(&self, node: &Node, content: &str, caller_id: &str, result: &mut ParseResult) {
        self.extract_calls_recursive(node, content, caller_id, result);
    }

    fn extract_calls_recursive(&self, node: &Node, content: &str, caller_id: &str, result: &mut ParseResult) {
        if node.kind() == "call" {
            // Get the function being called
            if let Some(func_node) = node.child_by_field_name("function") {
                let (callee_name, call_type) = match func_node.kind() {
                    "attribute" => {
                        // Method call: obj.method()
                        if let Some(attr) = func_node.child_by_field_name("attribute") {
                            (TreeSitterParser::node_text(&attr, content).to_string(), CallType::Method)
                        } else {
                            return;
                        }
                    }
                    "identifier" => {
                        // Direct call: func()
                        let name = TreeSitterParser::node_text(&func_node, content).to_string();
                        let call_type = if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                            CallType::Constructor
                        } else {
                            CallType::Direct
                        };
                        (name, call_type)
                    }
                    _ => return,
                };

                let callee_id = format!("function:?:{}", callee_name);
                let mut edge = CallsEdge::new(caller_id, callee_id);
                edge.call_type = call_type;
                result.add_call(edge);
            }
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_calls_recursive(&child, content, caller_id, result);
        }
    }

    fn process_node(&self, node: Node, content: &str, path: &str, result: &mut ParseResult) {
        match node.kind() {
            "function_definition" | "async_function_definition" => {
                if let Some(func) = self.extract_function(&node, content, path) {
                    let id = func.id.clone();
                    result.add_function(func);
                    if let Some(ref func_id) = id {
                        let file_id = format!("file:{}", path);
                        result.add_contains(ContainsEdge::new(&file_id, func_id));
                        // Extract calls from function body
                        if let Some(body) = node.child_by_field_name("body") {
                            self.extract_calls(&body, content, func_id, result);
                        }
                    }
                }
            }
            "class_definition" => {
                if let Some(class) = self.extract_class(&node, content, path) {
                    let id = class.id.clone();
                    result.add_struct(class);
                    if let Some(ref class_id) = id {
                        let file_id = format!("file:{}", path);
                        result.add_contains(ContainsEdge::new(&file_id, class_id));
                    }
                }
            }
            _ => {}
        }

        // Recursively process children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.process_node(child, content, path, result);
        }
    }
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser for PythonParser {
    fn parse_file(&self, path: &str, content: &str) -> Result<ParseResult, String> {
        let tree = self.base.parse_tree(content)?;
        let mut result = ParseResult::new(path);

        self.process_node(tree.root_node(), content, path, &mut result);

        Ok(result)
    }

    fn language_name(&self) -> &'static str {
        "Python"
    }

    fn supported_extensions(&self) -> &[&'static str] {
        &["py", "pyi"]
    }

    fn capability(&self) -> ParserCapability {
        ParserCapability::Structural
    }
}
