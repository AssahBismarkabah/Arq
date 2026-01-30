//! Go parser using tree-sitter.

use tree_sitter::Node;

use super::result::ParseResult;
use super::traits::{Parser, ParserCapability};
use super::treesitter::{extract_doc_comment, TreeSitterParser};
use crate::knowledge::ontology::edges::{CallType, CallsEdge, ContainsEdge};
use crate::knowledge::ontology::nodes::{
    FieldInfo, FunctionEntity, Parameter, StructEntity, TraitEntity, Visibility,
};

/// Go parser using tree-sitter.
pub struct GoParser {
    base: TreeSitterParser,
}

impl GoParser {
    pub fn new() -> Self {
        Self {
            base: TreeSitterParser::new(tree_sitter_go::LANGUAGE.into(), "Go", &["go"]),
        }
    }

    fn extract_function(&self, node: &Node, content: &str, path: &str) -> Option<FunctionEntity> {
        let name_node = node.child_by_field_name("name")?;
        let name = TreeSitterParser::node_text(&name_node, content).to_string();

        let params = self.extract_parameters(node, content);
        let return_type = node
            .child_by_field_name("result")
            .map(|n| TreeSitterParser::node_text(&n, content).to_string());

        // Check for receiver (method)
        let receiver = node.child_by_field_name("receiver").and_then(|r| {
            // Extract type from receiver
            let mut cursor = r.walk();
            let result = r
                .children(&mut cursor)
                .find(|c| c.kind() == "type_identifier" || c.kind() == "pointer_type")
                .map(|t| TreeSitterParser::node_text(&t, content).to_string());
            result
        });

        let qualified_name = if let Some(ref recv) = receiver {
            format!("{}.{}", recv.trim_start_matches('*'), name)
        } else {
            name.clone()
        };

        Some(FunctionEntity {
            id: Some(format!("function:{}:{}", path, qualified_name)),
            name,
            qualified_name,
            file_path: path.to_string(),
            start_line: TreeSitterParser::node_line(node),
            end_line: TreeSitterParser::node_end_line(node),
            signature: self.build_signature(node, content),
            parent: receiver,
            visibility: self.extract_visibility(&name_node, content),
            is_async: false,
            is_unsafe: false,
            generics: Vec::new(),
            parameters: params,
            return_type,
            doc_comment: extract_doc_comment(node, content),
            complexity: TreeSitterParser::calculate_complexity(node, content),
        })
    }

    fn extract_struct(&self, node: &Node, content: &str, path: &str) -> Option<StructEntity> {
        // type_spec -> name + type (struct_type)
        let name_node = node.child_by_field_name("name")?;
        let name = TreeSitterParser::node_text(&name_node, content).to_string();

        let type_node = node.child_by_field_name("type")?;
        if type_node.kind() != "struct_type" {
            return None;
        }

        let fields = self.extract_struct_fields(&type_node, content);

        Some(StructEntity {
            id: Some(format!("struct:{}:{}", path, name)),
            name: name.clone(),
            qualified_name: name,
            file_path: path.to_string(),
            start_line: TreeSitterParser::node_line(node),
            end_line: TreeSitterParser::node_end_line(&type_node),
            visibility: self.extract_visibility(&name_node, content),
            generics: Vec::new(),
            fields,
            derives: Vec::new(),
            attributes: Vec::new(),
            doc_comment: extract_doc_comment(node, content),
        })
    }

    fn extract_interface(&self, node: &Node, content: &str, path: &str) -> Option<TraitEntity> {
        let name_node = node.child_by_field_name("name")?;
        let name = TreeSitterParser::node_text(&name_node, content).to_string();

        let type_node = node.child_by_field_name("type")?;
        if type_node.kind() != "interface_type" {
            return None;
        }

        let mut required_methods = Vec::new();
        let mut cursor = type_node.walk();
        for child in type_node.children(&mut cursor) {
            if child.kind() == "method_spec" {
                if let Some(method_name) = child.child_by_field_name("name") {
                    required_methods
                        .push(TreeSitterParser::node_text(&method_name, content).to_string());
                }
            }
        }

        Some(TraitEntity {
            id: Some(format!("trait:{}:{}", path, name)),
            name: name.clone(),
            qualified_name: name,
            file_path: path.to_string(),
            start_line: TreeSitterParser::node_line(node),
            end_line: TreeSitterParser::node_end_line(&type_node),
            visibility: self.extract_visibility(&name_node, content),
            generics: Vec::new(),
            super_traits: Vec::new(),
            required_methods,
            provided_methods: Vec::new(),
            associated_types: Vec::new(),
            doc_comment: extract_doc_comment(node, content),
        })
    }

    fn extract_parameters(&self, node: &Node, content: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "parameter_declaration" {
                    let names: Vec<String> = child
                        .children(&mut child.walk())
                        .filter(|c| c.kind() == "identifier")
                        .map(|n| TreeSitterParser::node_text(&n, content).to_string())
                        .collect();

                    let type_name = child
                        .child_by_field_name("type")
                        .map(|n| TreeSitterParser::node_text(&n, content).to_string())
                        .unwrap_or_else(|| "interface{}".to_string());

                    for name in names {
                        params.push(Parameter {
                            name,
                            type_name: type_name.clone(),
                            is_mutable: false,
                            is_reference: type_name.starts_with('*'),
                        });
                    }
                }
            }
        }

        params
    }

    fn extract_struct_fields(&self, struct_node: &Node, content: &str) -> Vec<FieldInfo> {
        let mut fields = Vec::new();

        let mut cursor = struct_node.walk();
        for child in struct_node.children(&mut cursor) {
            if child.kind() == "field_declaration_list" {
                let mut list_cursor = child.walk();
                for field in child.children(&mut list_cursor) {
                    if field.kind() == "field_declaration" {
                        let names: Vec<String> = field
                            .children(&mut field.walk())
                            .filter(|c| c.kind() == "field_identifier")
                            .map(|n| TreeSitterParser::node_text(&n, content).to_string())
                            .collect();

                        let type_name = field
                            .child_by_field_name("type")
                            .map(|n| TreeSitterParser::node_text(&n, content).to_string())
                            .unwrap_or_default();

                        for name in names {
                            let visibility = if name
                                .chars()
                                .next()
                                .map(|c| c.is_uppercase())
                                .unwrap_or(false)
                            {
                                Visibility::Public
                            } else {
                                Visibility::Private
                            };

                            fields.push(FieldInfo {
                                name,
                                type_name: type_name.clone(),
                                visibility,
                                attributes: Vec::new(),
                                doc_comment: None,
                            });
                        }
                    }
                }
            }
        }

        fields
    }

    fn build_signature(&self, node: &Node, content: &str) -> String {
        TreeSitterParser::node_text(node, content)
            .lines()
            .next()
            .unwrap_or("")
            .trim_end_matches('{')
            .trim()
            .to_string()
    }

    fn extract_visibility(&self, name_node: &Node, content: &str) -> Visibility {
        let name = TreeSitterParser::node_text(name_node, content);
        if name
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            Visibility::Public
        } else {
            Visibility::Private
        }
    }

    /// Extract function calls from a node.
    fn extract_calls(&self, node: &Node, content: &str, caller_id: &str, result: &mut ParseResult) {
        self.extract_calls_recursive(node, content, caller_id, result);
    }

    fn extract_calls_recursive(
        &self,
        node: &Node,
        content: &str,
        caller_id: &str,
        result: &mut ParseResult,
    ) {
        if node.kind() == "call_expression" {
            // Get the function being called
            if let Some(func_node) = node.child_by_field_name("function") {
                let (callee_name, call_type) = match func_node.kind() {
                    "selector_expression" => {
                        // Method call: obj.Method()
                        if let Some(field) = func_node.child_by_field_name("field") {
                            (
                                TreeSitterParser::node_text(&field, content).to_string(),
                                CallType::Method,
                            )
                        } else {
                            return;
                        }
                    }
                    "identifier" => {
                        // Direct call: Func()
                        let name = TreeSitterParser::node_text(&func_node, content).to_string();
                        let call_type = if name
                            .chars()
                            .next()
                            .map(|c| c.is_uppercase())
                            .unwrap_or(false)
                        {
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
            "function_declaration" | "method_declaration" => {
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
            "type_spec" => {
                // Could be struct or interface
                if let Some(s) = self.extract_struct(&node, content, path) {
                    let id = s.id.clone();
                    result.add_struct(s);
                    if let Some(ref struct_id) = id {
                        let file_id = format!("file:{}", path);
                        result.add_contains(ContainsEdge::new(&file_id, struct_id));
                    }
                } else if let Some(iface) = self.extract_interface(&node, content, path) {
                    let id = iface.id.clone();
                    result.add_trait(iface);
                    if let Some(ref iface_id) = id {
                        let file_id = format!("file:{}", path);
                        result.add_contains(ContainsEdge::new(&file_id, iface_id));
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

impl Default for GoParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser for GoParser {
    fn parse_file(&self, path: &str, content: &str) -> Result<ParseResult, String> {
        let tree = self.base.parse_tree(content)?;
        let mut result = ParseResult::new(path);

        self.process_node(tree.root_node(), content, path, &mut result);

        Ok(result)
    }

    fn language_name(&self) -> &'static str {
        "Go"
    }

    fn supported_extensions(&self) -> &[&'static str] {
        &["go"]
    }

    fn capability(&self) -> ParserCapability {
        ParserCapability::Structural
    }
}
