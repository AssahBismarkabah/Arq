//! Java parser using tree-sitter.

use tree_sitter::Node;

use super::result::ParseResult;
use super::traits::{Parser, ParserCapability};
use super::treesitter::{TreeSitterParser, extract_doc_comment};
use crate::knowledge::ontology::edges::{CallsEdge, CallType, ContainsEdge};
use crate::knowledge::ontology::nodes::{
    EnumEntity, EnumVariant, FunctionEntity, StructEntity, TraitEntity,
    Visibility, FieldInfo, Parameter,
};

/// Java parser using tree-sitter.
pub struct JavaParser {
    base: TreeSitterParser,
}

impl JavaParser {
    pub fn new() -> Self {
        Self {
            base: TreeSitterParser::new(
                tree_sitter_java::LANGUAGE.into(),
                "Java",
                &["java"],
            ),
        }
    }

    fn extract_method(&self, node: &Node, content: &str, path: &str, class_name: Option<&str>) -> Option<FunctionEntity> {
        let name_node = node.child_by_field_name("name")?;
        let name = TreeSitterParser::node_text(&name_node, content).to_string();

        let params = self.extract_parameters(node, content);
        let return_type = node.child_by_field_name("type")
            .map(|n| TreeSitterParser::node_text(&n, content).to_string());

        let qualified_name = if let Some(class) = class_name {
            format!("{}.{}", class, name)
        } else {
            name.clone()
        };

        // Check modifiers for async/static
        let modifiers = self.extract_modifiers(node, content);
        let visibility = self.modifiers_to_visibility(&modifiers);

        Some(FunctionEntity {
            id: Some(format!("function:{}:{}", path, qualified_name)),
            name,
            qualified_name,
            file_path: path.to_string(),
            start_line: TreeSitterParser::node_line(node),
            end_line: TreeSitterParser::node_end_line(node),
            signature: self.build_signature(node, content),
            parent: class_name.map(String::from),
            visibility,
            is_async: modifiers.contains(&"synchronized".to_string()),
            is_unsafe: modifiers.contains(&"native".to_string()),
            generics: self.extract_type_parameters(node, content),
            parameters: params,
            return_type,
            doc_comment: self.extract_javadoc(node, content),
            complexity: TreeSitterParser::calculate_complexity(node, content),
        })
    }

    fn extract_class(&self, node: &Node, content: &str, path: &str) -> Option<StructEntity> {
        let name_node = node.child_by_field_name("name")?;
        let name = TreeSitterParser::node_text(&name_node, content).to_string();

        let fields = self.extract_class_fields(node, content);
        let modifiers = self.extract_modifiers(node, content);

        // Extract superclass
        let mut derives = Vec::new();
        if let Some(superclass) = node.child_by_field_name("superclass") {
            derives.push(TreeSitterParser::node_text(&superclass, content).to_string());
        }

        // Extract interfaces
        if let Some(interfaces) = node.child_by_field_name("interfaces") {
            let mut cursor = interfaces.walk();
            for child in interfaces.children(&mut cursor) {
                if child.kind() == "type_identifier" || child.kind() == "generic_type" {
                    derives.push(TreeSitterParser::node_text(&child, content).to_string());
                }
            }
        }

        Some(StructEntity {
            id: Some(format!("struct:{}:{}", path, name)),
            name: name.clone(),
            qualified_name: name,
            file_path: path.to_string(),
            start_line: TreeSitterParser::node_line(node),
            end_line: TreeSitterParser::node_end_line(node),
            visibility: self.modifiers_to_visibility(&modifiers),
            generics: self.extract_type_parameters(node, content),
            fields,
            derives,
            attributes: modifiers,
            doc_comment: self.extract_javadoc(node, content),
        })
    }

    fn extract_interface(&self, node: &Node, content: &str, path: &str) -> Option<TraitEntity> {
        let name_node = node.child_by_field_name("name")?;
        let name = TreeSitterParser::node_text(&name_node, content).to_string();

        let modifiers = self.extract_modifiers(node, content);

        // Extract method signatures
        let mut required_methods = Vec::new();
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "method_declaration" {
                    if let Some(method_name) = child.child_by_field_name("name") {
                        required_methods.push(TreeSitterParser::node_text(&method_name, content).to_string());
                    }
                }
            }
        }

        // Extract extended interfaces
        let mut super_traits = Vec::new();
        if let Some(extends) = node.child_by_field_name("extends_interfaces") {
            let mut cursor = extends.walk();
            for child in extends.children(&mut cursor) {
                if child.kind() == "type_identifier" || child.kind() == "generic_type" {
                    super_traits.push(TreeSitterParser::node_text(&child, content).to_string());
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
            visibility: self.modifiers_to_visibility(&modifiers),
            generics: self.extract_type_parameters(node, content),
            super_traits,
            required_methods,
            provided_methods: Vec::new(),
            associated_types: Vec::new(),
            doc_comment: self.extract_javadoc(node, content),
        })
    }

    fn extract_enum(&self, node: &Node, content: &str, path: &str) -> Option<EnumEntity> {
        let name_node = node.child_by_field_name("name")?;
        let name = TreeSitterParser::node_text(&name_node, content).to_string();

        let modifiers = self.extract_modifiers(node, content);

        let mut variants = Vec::new();
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "enum_constant" {
                    if let Some(const_name) = child.child_by_field_name("name") {
                        variants.push(EnumVariant {
                            name: TreeSitterParser::node_text(&const_name, content).to_string(),
                            fields: Vec::new(),
                            discriminant: None,
                            doc_comment: None,
                        });
                    }
                }
            }
        }

        Some(EnumEntity {
            id: Some(format!("enum:{}:{}", path, name)),
            name: name.clone(),
            qualified_name: name,
            file_path: path.to_string(),
            start_line: TreeSitterParser::node_line(node),
            end_line: TreeSitterParser::node_end_line(node),
            visibility: self.modifiers_to_visibility(&modifiers),
            generics: Vec::new(),
            variants,
            derives: Vec::new(),
            doc_comment: self.extract_javadoc(node, content),
        })
    }

    fn extract_parameters(&self, node: &Node, content: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "formal_parameter" || child.kind() == "spread_parameter" {
                    let name = child.child_by_field_name("name")
                        .map(|n| TreeSitterParser::node_text(&n, content).to_string())
                        .unwrap_or_default();

                    let type_name = child.child_by_field_name("type")
                        .map(|n| TreeSitterParser::node_text(&n, content).to_string())
                        .unwrap_or_else(|| "Object".to_string());

                    let modifiers = self.extract_child_modifiers(&child, content);

                    params.push(Parameter {
                        name,
                        type_name,
                        is_mutable: !modifiers.contains(&"final".to_string()),
                        is_reference: false,
                    });
                }
            }
        }

        params
    }

    fn extract_class_fields(&self, node: &Node, content: &str) -> Vec<FieldInfo> {
        let mut fields = Vec::new();

        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                if child.kind() == "field_declaration" {
                    let modifiers = self.extract_child_modifiers(&child, content);
                    let visibility = self.modifiers_to_visibility(&modifiers);

                    let type_name = child.child_by_field_name("type")
                        .map(|n| TreeSitterParser::node_text(&n, content).to_string())
                        .unwrap_or_default();

                    // Get variable declarators
                    let mut declarator_cursor = child.walk();
                    for decl in child.children(&mut declarator_cursor) {
                        if decl.kind() == "variable_declarator" {
                            if let Some(name_node) = decl.child_by_field_name("name") {
                                fields.push(FieldInfo {
                                    name: TreeSitterParser::node_text(&name_node, content).to_string(),
                                    type_name: type_name.clone(),
                                    visibility: visibility.clone(),
                                    attributes: modifiers.clone(),
                                    doc_comment: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        fields
    }

    fn extract_modifiers(&self, node: &Node, content: &str) -> Vec<String> {
        let mut modifiers = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "modifiers" {
                let mut mod_cursor = child.walk();
                for modifier in child.children(&mut mod_cursor) {
                    let text = TreeSitterParser::node_text(&modifier, content);
                    if !text.starts_with('@') {
                        modifiers.push(text.to_string());
                    }
                }
            }
        }

        modifiers
    }

    fn extract_child_modifiers(&self, node: &Node, content: &str) -> Vec<String> {
        self.extract_modifiers(node, content)
    }

    fn extract_type_parameters(&self, node: &Node, content: &str) -> Vec<String> {
        node.child_by_field_name("type_parameters")
            .map(|params| {
                let mut cursor = params.walk();
                params.children(&mut cursor)
                    .filter(|c| c.kind() == "type_parameter")
                    .map(|c| TreeSitterParser::node_text(&c, content).to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn modifiers_to_visibility(&self, modifiers: &[String]) -> Visibility {
        if modifiers.contains(&"public".to_string()) {
            Visibility::Public
        } else if modifiers.contains(&"private".to_string()) {
            Visibility::Private
        } else if modifiers.contains(&"protected".to_string()) {
            Visibility::PublicSuper
        } else {
            Visibility::PublicCrate // package-private
        }
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

    fn extract_javadoc(&self, node: &Node, content: &str) -> Option<String> {
        // Look for block_comment immediately before
        let mut sibling = node.prev_sibling();
        while let Some(s) = sibling {
            if s.kind() == "block_comment" {
                let text = TreeSitterParser::node_text(&s, content);
                if text.starts_with("/**") {
                    // Parse Javadoc
                    let cleaned = text
                        .trim_start_matches("/**")
                        .trim_end_matches("*/")
                        .lines()
                        .map(|l| l.trim().trim_start_matches('*').trim())
                        .filter(|l| !l.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n");
                    return Some(cleaned);
                }
            } else if s.kind() != "line_comment" {
                break;
            }
            sibling = s.prev_sibling();
        }

        extract_doc_comment(node, content)
    }

    /// Extract function calls from a node.
    fn extract_calls(&self, node: &Node, content: &str, caller_id: &str, result: &mut ParseResult) {
        self.extract_calls_recursive(node, content, caller_id, result);
    }

    fn extract_calls_recursive(&self, node: &Node, content: &str, caller_id: &str, result: &mut ParseResult) {
        match node.kind() {
            "method_invocation" => {
                // Get method name
                if let Some(name_node) = node.child_by_field_name("name") {
                    let callee_name = TreeSitterParser::node_text(&name_node, content).to_string();
                    let callee_id = format!("function:?:{}", callee_name);
                    let mut edge = CallsEdge::new(caller_id, callee_id);
                    edge.call_type = CallType::Method;
                    result.add_call(edge);
                }
            }
            "object_creation_expression" => {
                // Constructor call: new Foo()
                if let Some(type_node) = node.child_by_field_name("type") {
                    let callee_name = TreeSitterParser::node_text(&type_node, content).to_string();
                    let callee_id = format!("function:?:{}", callee_name);
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

    fn process_node(&self, node: Node, content: &str, path: &str, result: &mut ParseResult, class_context: Option<&str>) {
        match node.kind() {
            "method_declaration" => {
                if let Some(func) = self.extract_method(&node, content, path, class_context) {
                    let id = func.id.clone();
                    result.add_function(func);
                    if let Some(ref func_id) = id {
                        let file_id = format!("file:{}", path);
                        result.add_contains(ContainsEdge::new(&file_id, func_id));
                        // Extract calls from method body
                        if let Some(body) = node.child_by_field_name("body") {
                            self.extract_calls(&body, content, func_id, result);
                        }
                    }
                }
            }
            "class_declaration" => {
                if let Some(class) = self.extract_class(&node, content, path) {
                    let class_name = class.name.clone();
                    let id = class.id.clone();
                    result.add_struct(class);
                    if let Some(ref class_id) = id {
                        let file_id = format!("file:{}", path);
                        result.add_contains(ContainsEdge::new(&file_id, class_id));
                    }

                    // Process methods within class context
                    if let Some(body) = node.child_by_field_name("body") {
                        let mut cursor = body.walk();
                        for child in body.children(&mut cursor) {
                            self.process_node(child, content, path, result, Some(&class_name));
                        }
                    }
                    return; // Don't recurse normally for classes
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
            self.process_node(child, content, path, result, class_context);
        }
    }
}

impl Default for JavaParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser for JavaParser {
    fn parse_file(&self, path: &str, content: &str) -> Result<ParseResult, String> {
        let tree = self.base.parse_tree(content)?;
        let mut result = ParseResult::new(path);

        self.process_node(tree.root_node(), content, path, &mut result, None);

        Ok(result)
    }

    fn language_name(&self) -> &'static str {
        "Java"
    }

    fn supported_extensions(&self) -> &[&'static str] {
        &["java"]
    }

    fn capability(&self) -> ParserCapability {
        ParserCapability::Structural
    }
}
