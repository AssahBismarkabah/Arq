//! TypeScript and JavaScript parser using tree-sitter.

use tree_sitter::Node;

use super::result::ParseResult;
use super::traits::{Parser, ParserCapability};
use super::treesitter::{extract_doc_comment, TreeSitterParser};
use crate::knowledge::ontology::edges::{CallType, CallsEdge, ContainsEdge};
use crate::knowledge::ontology::nodes::{
    EnumEntity, FieldInfo, FunctionEntity, Parameter, StructEntity, TraitEntity, Visibility,
};

/// TypeScript parser using tree-sitter.
pub struct TypeScriptParser {
    base: TreeSitterParser,
    #[allow(dead_code)]
    is_typescript: bool,
}

impl TypeScriptParser {
    /// Create a TypeScript parser.
    pub fn typescript() -> Self {
        Self {
            base: TreeSitterParser::new(
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                "TypeScript",
                &["ts", "tsx"],
            ),
            is_typescript: true,
        }
    }

    /// Create a JavaScript parser.
    pub fn javascript() -> Self {
        Self {
            base: TreeSitterParser::new(
                tree_sitter_javascript::LANGUAGE.into(),
                "JavaScript",
                &["js", "jsx", "mjs", "cjs"],
            ),
            is_typescript: false,
        }
    }

    fn extract_function(&self, node: &Node, content: &str, path: &str) -> Option<FunctionEntity> {
        let name_node = node.child_by_field_name("name")?;
        let name = TreeSitterParser::node_text(&name_node, content).to_string();

        let params = self.extract_parameters(node, content);
        let return_type = node.child_by_field_name("return_type").map(|n| {
            TreeSitterParser::node_text(&n, content)
                .trim_start_matches(':')
                .trim()
                .to_string()
        });

        let is_async = node.children(&mut node.walk()).any(|c| c.kind() == "async");

        Some(FunctionEntity {
            id: Some(format!("function:{}:{}", path, name)),
            name: name.clone(),
            qualified_name: name,
            file_path: path.to_string(),
            start_line: TreeSitterParser::node_line(node),
            end_line: TreeSitterParser::node_end_line(node),
            signature: TreeSitterParser::node_text(node, content)
                .lines()
                .next()
                .unwrap_or("")
                .to_string(),
            parent: None,
            visibility: self.extract_visibility(node, content),
            is_async,
            is_unsafe: false,
            generics: self.extract_generics(node, content),
            parameters: params,
            return_type,
            doc_comment: extract_doc_comment(node, content),
            complexity: TreeSitterParser::calculate_complexity(node, content),
        })
    }

    fn extract_class(&self, node: &Node, content: &str, path: &str) -> Option<StructEntity> {
        let name_node = node.child_by_field_name("name")?;
        let name = TreeSitterParser::node_text(&name_node, content).to_string();

        let fields = self.extract_class_fields(node, content);

        Some(StructEntity {
            id: Some(format!("struct:{}:{}", path, name)),
            name: name.clone(),
            qualified_name: name,
            file_path: path.to_string(),
            start_line: TreeSitterParser::node_line(node),
            end_line: TreeSitterParser::node_end_line(node),
            visibility: self.extract_visibility(node, content),
            generics: self.extract_generics(node, content),
            fields,
            derives: Vec::new(),
            attributes: Vec::new(),
            doc_comment: extract_doc_comment(node, content),
        })
    }

    fn extract_interface(&self, node: &Node, content: &str, path: &str) -> Option<TraitEntity> {
        let name_node = node.child_by_field_name("name")?;
        let name = TreeSitterParser::node_text(&name_node, content).to_string();

        let mut required_methods = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "method_signature" {
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
            end_line: TreeSitterParser::node_end_line(node),
            visibility: self.extract_visibility(node, content),
            generics: self.extract_generics(node, content),
            super_traits: Vec::new(),
            required_methods,
            provided_methods: Vec::new(),
            associated_types: Vec::new(),
            doc_comment: extract_doc_comment(node, content),
        })
    }

    fn extract_enum(&self, node: &Node, content: &str, path: &str) -> Option<EnumEntity> {
        let name_node = node.child_by_field_name("name")?;
        let name = TreeSitterParser::node_text(&name_node, content).to_string();

        let mut variants = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "enum_assignment" || child.kind() == "property_identifier" {
                let variant_name = TreeSitterParser::node_text(&child, content);
                variants.push(crate::knowledge::ontology::nodes::EnumVariant {
                    name: variant_name.to_string(),
                    fields: Vec::new(),
                    discriminant: None,
                    doc_comment: None,
                });
            }
        }

        Some(EnumEntity {
            id: Some(format!("enum:{}:{}", path, name)),
            name: name.clone(),
            qualified_name: name,
            file_path: path.to_string(),
            start_line: TreeSitterParser::node_line(node),
            end_line: TreeSitterParser::node_end_line(node),
            visibility: self.extract_visibility(node, content),
            generics: Vec::new(),
            variants,
            derives: Vec::new(),
            doc_comment: extract_doc_comment(node, content),
        })
    }

    fn extract_parameters(&self, node: &Node, content: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "required_parameter" || child.kind() == "optional_parameter" {
                    let name = child
                        .child_by_field_name("pattern")
                        .map(|n| TreeSitterParser::node_text(&n, content).to_string())
                        .unwrap_or_default();
                    let type_name = child
                        .child_by_field_name("type")
                        .map(|n| TreeSitterParser::node_text(&n, content).to_string())
                        .unwrap_or_else(|| "any".to_string());

                    params.push(Parameter {
                        name,
                        type_name,
                        is_mutable: false,
                        is_reference: false,
                    });
                }
            }
        }

        params
    }

    fn extract_class_fields(&self, node: &Node, content: &str) -> Vec<FieldInfo> {
        let mut fields = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "public_field_definition" || child.kind() == "field_definition" {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| TreeSitterParser::node_text(&n, content).to_string())
                    .unwrap_or_default();
                let type_name = child
                    .child_by_field_name("type")
                    .map(|n| TreeSitterParser::node_text(&n, content).to_string())
                    .unwrap_or_else(|| "any".to_string());

                fields.push(FieldInfo {
                    name,
                    type_name,
                    visibility: Visibility::Public,
                    attributes: Vec::new(),
                    doc_comment: None,
                });
            }
        }

        fields
    }

    fn extract_generics(&self, node: &Node, content: &str) -> Vec<String> {
        node.child_by_field_name("type_parameters")
            .map(|params| {
                let mut cursor = params.walk();
                params
                    .children(&mut cursor)
                    .filter(|c| c.kind() == "type_parameter")
                    .map(|c| TreeSitterParser::node_text(&c, content).to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn extract_visibility(&self, node: &Node, content: &str) -> Visibility {
        let text = TreeSitterParser::node_text(node, content);
        if text.contains("export") || text.contains("public") {
            Visibility::Public
        } else if text.contains("private") {
            Visibility::Private
        } else if text.contains("protected") {
            Visibility::PublicSuper
        } else {
            Visibility::Private
        }
    }

    /// Extract function calls from a node (function body).
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
        match node.kind() {
            "call_expression" => {
                // Get the function being called
                if let Some(func_node) = node.child_by_field_name("function") {
                    let (callee_name, call_type) = match func_node.kind() {
                        "member_expression" => {
                            // Method call: obj.method()
                            if let Some(prop) = func_node.child_by_field_name("property") {
                                (
                                    TreeSitterParser::node_text(&prop, content).to_string(),
                                    CallType::Method,
                                )
                            } else {
                                return;
                            }
                        }
                        "identifier" => {
                            // Direct call: func()
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
            "new_expression" => {
                // Constructor call: new Foo()
                if let Some(constructor) = node.child_by_field_name("constructor") {
                    let name = TreeSitterParser::node_text(&constructor, content).to_string();
                    let callee_id = format!("function:?:{}", name);
                    let mut edge = CallsEdge::new(caller_id, callee_id);
                    edge.call_type = CallType::Constructor;
                    result.add_call(edge);
                }
            }
            _ => {}
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_calls_recursive(&child, content, caller_id, result);
        }
    }

    fn process_node(&self, node: Node, content: &str, path: &str, result: &mut ParseResult) {
        match node.kind() {
            "function_declaration" | "function" | "arrow_function" | "method_definition" => {
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
            "class_declaration" | "class" => {
                if let Some(class) = self.extract_class(&node, content, path) {
                    let id = class.id.clone();
                    result.add_struct(class);
                    if let Some(ref class_id) = id {
                        let file_id = format!("file:{}", path);
                        result.add_contains(ContainsEdge::new(&file_id, class_id));
                    }
                }
            }
            "interface_declaration" => {
                if let Some(iface) = self.extract_interface(&node, content, path) {
                    let id = iface.id.clone();
                    result.add_trait(iface);
                    if let Some(ref iface_id) = id {
                        let file_id = format!("file:{}", path);
                        result.add_contains(ContainsEdge::new(&file_id, iface_id));
                    }
                }
            }
            "enum_declaration" => {
                if let Some(enum_entity) = self.extract_enum(&node, content, path) {
                    let id = enum_entity.id.clone();
                    result.add_enum(enum_entity);
                    if let Some(ref enum_id) = id {
                        let file_id = format!("file:{}", path);
                        result.add_contains(ContainsEdge::new(&file_id, enum_id));
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

impl Parser for TypeScriptParser {
    fn parse_file(&self, path: &str, content: &str) -> Result<ParseResult, String> {
        let tree = self.base.parse_tree(content)?;
        let mut result = ParseResult::new(path);

        self.process_node(tree.root_node(), content, path, &mut result);

        Ok(result)
    }

    fn language_name(&self) -> &'static str {
        self.base.language_name()
    }

    fn supported_extensions(&self) -> &[&'static str] {
        self.base.supported_extensions()
    }

    fn capability(&self) -> ParserCapability {
        ParserCapability::Structural
    }
}
