//! Rust parser using syn for rich AST-based extraction.
//!
//! Extracts:
//! - Functions (async, generics, parameters, return types, doc comments)
//! - Structs (fields, visibility, derives, attributes)
//! - Traits (methods, super traits, associated types)
//! - Impls (trait impls, inherent impls)
//! - Enums (variants with fields)
//! - Constants and statics
//! - Call relationships (function calls within bodies)
//! - Type usage relationships

use proc_macro2::Span;
use syn::{
    spanned::Spanned, visit::Visit, Attribute, FnArg, GenericParam, Generics, ImplItem, Item,
    ItemConst, ItemEnum, ItemFn, ItemImpl, ItemStatic, ItemStruct, ItemTrait, Pat, ReturnType,
    Signature, StaticMutability, TraitItem, Type, Visibility as SynVisibility,
};

use super::result::ParseResult;
use super::traits::{Parser, ParserCapability};
use crate::knowledge::ontology::edges::{
    CallsEdge, ContainsEdge, ExtendsEdge, ImplementsEdge, ReturnsTypeEdge,
};
use crate::knowledge::ontology::nodes::{
    ComplexityMetrics, ConstantEntity, EnumEntity, EnumVariant, FieldInfo, FunctionEntity,
    ImplEntity, Parameter, StructEntity, TraitEntity, Visibility,
};

/// Rust parser using syn for AST-based extraction.
pub struct RustParser {
    /// Whether to extract call relationships (slower, requires body analysis).
    extract_calls: bool,
}

impl RustParser {
    /// Create a new Rust parser with default settings.
    pub fn new() -> Self {
        Self {
            extract_calls: true,
        }
    }

    /// Create a parser without call extraction (faster).
    pub fn without_calls() -> Self {
        Self {
            extract_calls: false,
        }
    }
}

impl Default for RustParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser for RustParser {
    fn parse_file(&self, path: &str, content: &str) -> Result<ParseResult, String> {
        let syntax = syn::parse_file(content).map_err(|e| format!("Parse error: {}", e))?;

        let mut visitor = RustVisitor::new(path, content, self.extract_calls);
        visitor.visit_file(&syntax);

        Ok(visitor.result)
    }

    fn language_name(&self) -> &'static str {
        "Rust"
    }

    fn supported_extensions(&self) -> &[&'static str] {
        &["rs"]
    }

    fn capability(&self) -> ParserCapability {
        ParserCapability::Structural
    }
}

/// Visitor that extracts entities from Rust AST.
struct RustVisitor<'a> {
    result: ParseResult,
    #[allow(dead_code)]
    content: &'a str,
    lines: Vec<&'a str>,
    extract_calls: bool,
    /// Current impl context (for method parent tracking).
    current_impl: Option<String>,
    /// Current impl trait (if trait impl).
    current_impl_trait: Option<String>,
}

impl<'a> RustVisitor<'a> {
    fn new(path: &str, content: &'a str, extract_calls: bool) -> Self {
        Self {
            result: ParseResult::new(path),
            content,
            lines: content.lines().collect(),
            extract_calls,
            current_impl: None,
            current_impl_trait: None,
        }
    }

    /// Get line number from span.
    fn line_of(&self, span: Span) -> u32 {
        span.start().line as u32
    }

    /// Get end line from span.
    fn end_line_of(&self, span: Span) -> u32 {
        span.end().line as u32
    }

    /// Generate entity ID.
    fn entity_id(&self, entity_type: &str, name: &str) -> String {
        format!("{}:{}:{}", entity_type, self.result.file_path, name)
    }

    /// Extract doc comments from attributes.
    fn extract_doc_comment(attrs: &[Attribute]) -> Option<String> {
        let doc_lines: Vec<String> = attrs
            .iter()
            .filter_map(|attr| {
                if attr.path().is_ident("doc") {
                    if let syn::Meta::NameValue(nv) = &attr.meta {
                        if let syn::Expr::Lit(lit) = &nv.value {
                            if let syn::Lit::Str(s) = &lit.lit {
                                return Some(s.value().trim().to_string());
                            }
                        }
                    }
                }
                None
            })
            .collect();

        if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join("\n"))
        }
    }

    /// Extract derive macros from attributes.
    fn extract_derives(attrs: &[Attribute]) -> Vec<String> {
        let mut derives = Vec::new();

        for attr in attrs {
            if attr.path().is_ident("derive") {
                if let Ok(nested) = attr.parse_args_with(
                    syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
                ) {
                    for path in nested {
                        if let Some(ident) = path.get_ident() {
                            derives.push(ident.to_string());
                        } else {
                            derives.push(
                                path.segments
                                    .last()
                                    .map(|s| s.ident.to_string())
                                    .unwrap_or_default(),
                            );
                        }
                    }
                }
            }
        }

        derives
    }

    /// Extract non-derive attributes.
    fn extract_attributes(attrs: &[Attribute]) -> Vec<String> {
        attrs
            .iter()
            .filter(|attr| !attr.path().is_ident("doc") && !attr.path().is_ident("derive"))
            .map(|attr| {
                let path = attr
                    .path()
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                format!("#[{}]", path)
            })
            .collect()
    }

    /// Convert syn Visibility to our Visibility.
    fn convert_visibility(vis: &SynVisibility) -> Visibility {
        match vis {
            SynVisibility::Public(_) => Visibility::Public,
            SynVisibility::Restricted(r) => {
                let path = r
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                match path.as_str() {
                    "crate" => Visibility::PublicCrate,
                    "super" => Visibility::PublicSuper,
                    _ => Visibility::PublicIn,
                }
            }
            SynVisibility::Inherited => Visibility::Private,
        }
    }

    /// Extract generic parameters as strings.
    fn extract_generics(generics: &Generics) -> Vec<String> {
        generics
            .params
            .iter()
            .map(|param| match param {
                GenericParam::Type(t) => {
                    let mut s = t.ident.to_string();
                    if !t.bounds.is_empty() {
                        s.push_str(": ");
                        s.push_str(
                            &t.bounds
                                .iter()
                                .map(|b| quote::quote!(#b).to_string())
                                .collect::<Vec<_>>()
                                .join(" + "),
                        );
                    }
                    s
                }
                GenericParam::Lifetime(l) => l.lifetime.to_string(),
                GenericParam::Const(c) => format!("const {}: {}", c.ident, quote::quote!(#c.ty)),
            })
            .collect()
    }

    /// Extract function parameters.
    fn extract_parameters(sig: &Signature) -> Vec<Parameter> {
        sig.inputs
            .iter()
            .map(|arg| match arg {
                FnArg::Receiver(r) => Parameter {
                    name: "self".to_string(),
                    type_name: if r.reference.is_some() {
                        if r.mutability.is_some() {
                            "&mut self".to_string()
                        } else {
                            "&self".to_string()
                        }
                    } else {
                        "self".to_string()
                    },
                    is_mutable: r.mutability.is_some(),
                    is_reference: r.reference.is_some(),
                },
                FnArg::Typed(t) => {
                    let name = match &*t.pat {
                        Pat::Ident(i) => i.ident.to_string(),
                        _ => "_".to_string(),
                    };
                    let type_name = quote::quote!(#t.ty).to_string();
                    let is_reference = matches!(&*t.ty, Type::Reference(_));
                    let is_mutable = matches!(&*t.pat, Pat::Ident(i) if i.mutability.is_some());
                    Parameter {
                        name,
                        type_name,
                        is_mutable,
                        is_reference,
                    }
                }
            })
            .collect()
    }

    /// Extract return type.
    fn extract_return_type(ret: &ReturnType) -> Option<String> {
        match ret {
            ReturnType::Default => None,
            ReturnType::Type(_, ty) => Some(quote::quote!(#ty).to_string()),
        }
    }

    /// Extract function signature as string.
    fn extract_signature(sig: &Signature) -> String {
        quote::quote!(#sig).to_string()
    }

    /// Calculate basic complexity metrics.
    fn calculate_complexity(&self, start_line: u32, end_line: u32) -> Option<ComplexityMetrics> {
        let start = start_line.saturating_sub(1) as usize;
        let end = (end_line as usize).min(self.lines.len());

        if start >= end {
            return None;
        }

        let code = self.lines[start..end].join("\n");
        let loc = (end - start) as u32;

        // Simple cyclomatic complexity estimation
        let branching_keywords = [
            "if", "else", "match", "for", "while", "loop", "?", "&&", "||",
        ];
        let mut cyclomatic = 1u32;
        for keyword in branching_keywords {
            cyclomatic += code.matches(keyword).count() as u32;
        }

        Some(ComplexityMetrics {
            cyclomatic,
            loc,
            cognitive: None,
        })
    }

    /// Extract function calls from a function body and add them to the result.
    fn extract_calls_from_body(&mut self, caller_id: &str, body: &syn::Block) {
        if !self.extract_calls {
            return;
        }

        let file_path = self.result.file_path.clone();
        let mut extractor = CallExtractor::new(caller_id, &file_path);
        extractor.visit_block(body);

        // Add all extracted calls to the result
        for call in extractor.calls {
            self.result.add_call(call);
        }
    }

    /// Process a struct item.
    fn process_struct(&mut self, item: &ItemStruct) {
        let name = item.ident.to_string();
        let id = self.entity_id("struct", &name);
        let start_line = self.line_of(item.ident.span());
        let end_line = self.end_line_of(item.ident.span());

        // Extract fields
        let fields: Vec<FieldInfo> = item
            .fields
            .iter()
            .map(|f| {
                let field_name = f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default();
                FieldInfo {
                    name: field_name,
                    type_name: quote::quote!(#f.ty).to_string(),
                    visibility: Self::convert_visibility(&f.vis),
                    attributes: Self::extract_attributes(&f.attrs),
                    doc_comment: Self::extract_doc_comment(&f.attrs),
                }
            })
            .collect();

        let entity = StructEntity {
            id: Some(id.clone()),
            name: name.clone(),
            qualified_name: name.clone(),
            file_path: self.result.file_path.clone(),
            start_line,
            end_line,
            visibility: Self::convert_visibility(&item.vis),
            generics: Self::extract_generics(&item.generics),
            fields,
            derives: Self::extract_derives(&item.attrs),
            attributes: Self::extract_attributes(&item.attrs),
            doc_comment: Self::extract_doc_comment(&item.attrs),
        };

        self.result.add_struct(entity);

        // Add contains edge (file contains struct)
        let file_id = format!("file:{}", self.result.file_path);
        self.result.add_contains(ContainsEdge::new(&file_id, &id));
    }

    /// Process a function item.
    fn process_function(&mut self, item: &ItemFn) {
        let name = item.sig.ident.to_string();
        let id = self.entity_id("function", &name);
        let start_line = self.line_of(item.sig.ident.span());
        let end_line = self.end_line_of(item.sig.ident.span());

        let entity = FunctionEntity {
            id: Some(id.clone()),
            name: name.clone(),
            qualified_name: name.clone(),
            file_path: self.result.file_path.clone(),
            start_line,
            end_line,
            signature: Self::extract_signature(&item.sig),
            parent: None,
            visibility: Self::convert_visibility(&item.vis),
            is_async: item.sig.asyncness.is_some(),
            is_unsafe: item.sig.unsafety.is_some(),
            generics: Self::extract_generics(&item.sig.generics),
            parameters: Self::extract_parameters(&item.sig),
            return_type: Self::extract_return_type(&item.sig.output),
            doc_comment: Self::extract_doc_comment(&item.attrs),
            complexity: self.calculate_complexity(start_line, end_line),
        };

        self.result.add_function(entity);

        // Add contains edge
        let file_id = format!("file:{}", self.result.file_path);
        self.result.add_contains(ContainsEdge::new(&file_id, &id));

        // Add return type edge if present
        if let Some(ref ret_type) = Self::extract_return_type(&item.sig.output) {
            let type_id = format!("type:?:{}", ret_type);
            self.result
                .add_returns_type(ReturnsTypeEdge::new(&id, &type_id));
        }

        // Extract calls from function body
        self.extract_calls_from_body(&id, &item.block);
    }

    /// Process a trait item.
    fn process_trait(&mut self, item: &ItemTrait) {
        let name = item.ident.to_string();
        let id = self.entity_id("trait", &name);
        let start_line = self.line_of(item.ident.span());
        let end_line = self.end_line_of(item.ident.span());

        // Extract super traits
        let super_traits: Vec<String> = item
            .supertraits
            .iter()
            .map(|b| quote::quote!(#b).to_string())
            .collect();

        // Categorize methods
        let mut required_methods = Vec::new();
        let mut provided_methods = Vec::new();
        let mut associated_types = Vec::new();

        for item in &item.items {
            match item {
                TraitItem::Fn(m) => {
                    let method_name = m.sig.ident.to_string();
                    if m.default.is_some() {
                        provided_methods.push(method_name);
                    } else {
                        required_methods.push(method_name);
                    }
                }
                TraitItem::Type(t) => {
                    associated_types.push(t.ident.to_string());
                }
                _ => {}
            }
        }

        let entity = TraitEntity {
            id: Some(id.clone()),
            name: name.clone(),
            qualified_name: name.clone(),
            file_path: self.result.file_path.clone(),
            start_line,
            end_line,
            visibility: Self::convert_visibility(&item.vis),
            generics: Self::extract_generics(&item.generics),
            super_traits: super_traits.clone(),
            required_methods,
            provided_methods,
            associated_types,
            doc_comment: Self::extract_doc_comment(&item.attrs),
        };

        self.result.add_trait(entity);

        // Add contains edge
        let file_id = format!("file:{}", self.result.file_path);
        self.result.add_contains(ContainsEdge::new(&file_id, &id));

        // Add extends edges for super traits
        for super_trait in super_traits {
            let super_id = format!("trait:?:{}", super_trait);
            self.result.add_extends(ExtendsEdge::new(&id, &super_id));
        }
    }

    /// Process an impl item.
    fn process_impl(&mut self, item: &ItemImpl) {
        let target_type = quote::quote!(#item.self_ty).to_string();
        let trait_name = item
            .trait_
            .as_ref()
            .map(|(_, path, _)| quote::quote!(#path).to_string());

        let id = if let Some(ref t) = trait_name {
            self.entity_id("impl", &format!("{}_for_{}", t, target_type))
        } else {
            self.entity_id("impl", &target_type)
        };

        let start_line = self.line_of(item.self_ty.span());
        let end_line = self.end_line_of(item.self_ty.span());

        // Extract method names
        let methods: Vec<String> = item
            .items
            .iter()
            .filter_map(|item| {
                if let ImplItem::Fn(m) = item {
                    Some(m.sig.ident.to_string())
                } else {
                    None
                }
            })
            .collect();

        // Extract where clause
        let where_clause = item
            .generics
            .where_clause
            .as_ref()
            .map(|w| quote::quote!(#w).to_string());

        let entity = ImplEntity {
            id: Some(id.clone()),
            target_type: target_type.clone(),
            trait_name: trait_name.clone(),
            file_path: self.result.file_path.clone(),
            start_line,
            end_line,
            generics: Self::extract_generics(&item.generics),
            where_clause,
            methods: methods.clone(),
        };

        self.result.add_impl(entity);

        // Add contains edge
        let file_id = format!("file:{}", self.result.file_path);
        self.result.add_contains(ContainsEdge::new(&file_id, &id));

        // Add implements edge if this is a trait impl
        if let Some(ref trait_name) = trait_name {
            let struct_id = format!("struct:?:{}", target_type);
            let trait_id = format!("trait:?:{}", trait_name);
            let mut edge = ImplementsEdge::new(&struct_id, &trait_id);
            edge.impl_file = Some(self.result.file_path.clone());
            edge.impl_line = Some(start_line);
            self.result.add_implements(edge);
        }

        // Set context for method processing
        self.current_impl = Some(target_type);
        self.current_impl_trait = trait_name;
    }

    /// Process an enum item.
    fn process_enum(&mut self, item: &ItemEnum) {
        let name = item.ident.to_string();
        let id = self.entity_id("enum", &name);
        let start_line = self.line_of(item.ident.span());
        let end_line = self.end_line_of(item.ident.span());

        // Extract variants
        let variants: Vec<EnumVariant> = item
            .variants
            .iter()
            .map(|v| {
                let fields: Vec<FieldInfo> = v
                    .fields
                    .iter()
                    .map(|f| FieldInfo {
                        name: f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default(),
                        type_name: quote::quote!(#f.ty).to_string(),
                        visibility: Self::convert_visibility(&f.vis),
                        attributes: Vec::new(),
                        doc_comment: None,
                    })
                    .collect();

                let discriminant = v
                    .discriminant
                    .as_ref()
                    .map(|(_, expr)| quote::quote!(#expr).to_string());

                EnumVariant {
                    name: v.ident.to_string(),
                    fields,
                    discriminant,
                    doc_comment: Self::extract_doc_comment(&v.attrs),
                }
            })
            .collect();

        let entity = EnumEntity {
            id: Some(id.clone()),
            name: name.clone(),
            qualified_name: name.clone(),
            file_path: self.result.file_path.clone(),
            start_line,
            end_line,
            visibility: Self::convert_visibility(&item.vis),
            generics: Self::extract_generics(&item.generics),
            variants,
            derives: Self::extract_derives(&item.attrs),
            doc_comment: Self::extract_doc_comment(&item.attrs),
        };

        self.result.add_enum(entity);

        // Add contains edge
        let file_id = format!("file:{}", self.result.file_path);
        self.result.add_contains(ContainsEdge::new(&file_id, &id));
    }

    /// Process a constant item.
    fn process_const(&mut self, item: &ItemConst) {
        let name = item.ident.to_string();
        let id = self.entity_id("const", &name);
        let line = self.line_of(item.ident.span());

        let entity = ConstantEntity {
            id: Some(id.clone()),
            name: name.clone(),
            qualified_name: name.clone(),
            file_path: self.result.file_path.clone(),
            line,
            visibility: Self::convert_visibility(&item.vis),
            type_name: quote::quote!(#item.ty).to_string(),
            is_static: false,
            is_mutable: false,
            doc_comment: Self::extract_doc_comment(&item.attrs),
        };

        self.result.add_constant(entity);

        // Add contains edge
        let file_id = format!("file:{}", self.result.file_path);
        self.result.add_contains(ContainsEdge::new(&file_id, &id));
    }

    /// Process a static item.
    fn process_static(&mut self, item: &ItemStatic) {
        let name = item.ident.to_string();
        let id = self.entity_id("static", &name);
        let line = self.line_of(item.ident.span());

        let entity = ConstantEntity {
            id: Some(id.clone()),
            name: name.clone(),
            qualified_name: name.clone(),
            file_path: self.result.file_path.clone(),
            line,
            visibility: Self::convert_visibility(&item.vis),
            type_name: quote::quote!(#item.ty).to_string(),
            is_static: true,
            is_mutable: matches!(item.mutability, StaticMutability::Mut(_)),
            doc_comment: Self::extract_doc_comment(&item.attrs),
        };

        self.result.add_constant(entity);

        // Add contains edge
        let file_id = format!("file:{}", self.result.file_path);
        self.result.add_contains(ContainsEdge::new(&file_id, &id));
    }
}

impl<'ast> Visit<'ast> for RustVisitor<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        match item {
            Item::Struct(s) => self.process_struct(s),
            Item::Fn(f) => self.process_function(f),
            Item::Trait(t) => self.process_trait(t),
            Item::Impl(i) => {
                self.process_impl(i);
                // Continue visiting to process methods inside impl
                syn::visit::visit_item_impl(self, i);
                // Clear context after processing impl
                self.current_impl = None;
                self.current_impl_trait = None;
            }
            Item::Enum(e) => self.process_enum(e),
            Item::Const(c) => self.process_const(c),
            Item::Static(s) => self.process_static(s),
            _ => {
                // Continue visiting for other items
                syn::visit::visit_item(self, item);
            }
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let name = item.sig.ident.to_string();
        let parent = self.current_impl.clone();
        let qualified_name = if let Some(ref p) = parent {
            format!("{}::{}", p, name)
        } else {
            name.clone()
        };

        let id = self.entity_id("function", &qualified_name);
        let start_line = self.line_of(item.sig.ident.span());
        let end_line = self.end_line_of(item.sig.ident.span());

        let entity = FunctionEntity {
            id: Some(id.clone()),
            name,
            qualified_name,
            file_path: self.result.file_path.clone(),
            start_line,
            end_line,
            signature: Self::extract_signature(&item.sig),
            parent,
            visibility: Self::convert_visibility(&item.vis),
            is_async: item.sig.asyncness.is_some(),
            is_unsafe: item.sig.unsafety.is_some(),
            generics: Self::extract_generics(&item.sig.generics),
            parameters: Self::extract_parameters(&item.sig),
            return_type: Self::extract_return_type(&item.sig.output),
            doc_comment: Self::extract_doc_comment(&item.attrs),
            complexity: self.calculate_complexity(start_line, end_line),
        };

        self.result.add_function(entity);

        // Add contains edge (impl contains method)
        if let Some(ref impl_type) = self.current_impl {
            let impl_id = if let Some(ref trait_name) = self.current_impl_trait {
                self.entity_id("impl", &format!("{}_for_{}", trait_name, impl_type))
            } else {
                self.entity_id("impl", impl_type)
            };
            self.result.add_contains(ContainsEdge::new(&impl_id, &id));
        }

        // Extract calls from method body
        self.extract_calls_from_body(&id, &item.block);
    }
}

/// Visitor that extracts function calls from a code block.
struct CallExtractor {
    /// Caller function ID
    caller_id: String,
    /// File path for generating callee IDs (reserved for future cross-file resolution)
    #[allow(dead_code)]
    file_path: String,
    /// Extracted calls
    calls: Vec<CallsEdge>,
    /// Track if we're inside a conditional
    in_conditional: bool,
    /// Track if we're inside a loop
    in_loop: bool,
}

impl CallExtractor {
    fn new(caller_id: &str, file_path: &str) -> Self {
        Self {
            caller_id: caller_id.to_string(),
            file_path: file_path.to_string(),
            calls: Vec::new(),
            in_conditional: false,
            in_loop: false,
        }
    }

    /// Generate a callee ID from a function name.
    /// Uses a placeholder path since we can't resolve imports statically.
    fn callee_id(&self, name: &str) -> String {
        format!("function:?:{}", name)
    }

    /// Add a call edge.
    fn add_call(
        &mut self,
        callee_name: &str,
        call_type: crate::knowledge::ontology::edges::CallType,
    ) {
        let mut edge = CallsEdge::new(&self.caller_id, self.callee_id(callee_name));
        edge.call_type = call_type;
        edge.is_conditional = self.in_conditional;
        edge.is_in_loop = self.in_loop;
        self.calls.push(edge);
    }
}

impl<'ast> syn::visit::Visit<'ast> for CallExtractor {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        // Extract the function name from the call
        if let syn::Expr::Path(path) = &*node.func {
            if let Some(segment) = path.path.segments.last() {
                let name = segment.ident.to_string();
                // Skip macros and common constructors
                if !name.ends_with('!') {
                    use crate::knowledge::ontology::edges::CallType;
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
                    self.add_call(&name, call_type);
                }
            }
        }
        // Continue visiting nested expressions
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let name = node.method.to_string();
        use crate::knowledge::ontology::edges::CallType;
        self.add_call(&name, CallType::Method);
        // Continue visiting nested expressions
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        let was_conditional = self.in_conditional;
        self.in_conditional = true;
        syn::visit::visit_expr_if(self, node);
        self.in_conditional = was_conditional;
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        let was_conditional = self.in_conditional;
        self.in_conditional = true;
        syn::visit::visit_expr_match(self, node);
        self.in_conditional = was_conditional;
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        let was_in_loop = self.in_loop;
        self.in_loop = true;
        syn::visit::visit_expr_loop(self, node);
        self.in_loop = was_in_loop;
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        let was_in_loop = self.in_loop;
        self.in_loop = true;
        syn::visit::visit_expr_while(self, node);
        self.in_loop = was_in_loop;
    }

    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        let was_in_loop = self.in_loop;
        self.in_loop = true;
        syn::visit::visit_expr_for_loop(self, node);
        self.in_loop = was_in_loop;
    }
}

#[cfg(test)]
mod tests {
    use super::super::result::{ParsedEdge, ParsedNode};
    use super::*;

    #[test]
    fn test_parse_struct() {
        let parser = RustParser::new();
        let code = r#"
/// A configuration struct.
#[derive(Debug, Clone)]
pub struct Config {
    /// The name field.
    pub name: String,
    value: i32,
}
"#;
        let result = parser.parse_file("test.rs", code).unwrap();
        assert_eq!(result.nodes.len(), 1);

        if let ParsedNode::Struct(s) = &result.nodes[0] {
            assert_eq!(s.name, "Config");
            assert!(matches!(s.visibility, Visibility::Public));
            assert_eq!(s.fields.len(), 2);
            assert!(s.derives.contains(&"Debug".to_string()));
            assert!(s.doc_comment.as_ref().unwrap().contains("configuration"));
        } else {
            panic!("Expected struct");
        }
    }

    #[test]
    fn test_parse_function() {
        let parser = RustParser::new();
        let code = r#"
/// Process the data.
pub async fn process_data(input: &str, count: usize) -> Result<String, Error> {
    Ok(input.to_string())
}
"#;
        let result = parser.parse_file("test.rs", code).unwrap();
        assert_eq!(result.nodes.len(), 1);

        if let ParsedNode::Function(f) = &result.nodes[0] {
            assert_eq!(f.name, "process_data");
            assert!(f.is_async);
            assert!(matches!(f.visibility, Visibility::Public));
            assert_eq!(f.parameters.len(), 2);
            assert!(f.return_type.is_some());
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn test_parse_trait() {
        let parser = RustParser::new();
        let code = r#"
pub trait Handler: Send + Sync {
    type Output;
    fn handle(&self) -> Self::Output;
    fn default_method(&self) {}
}
"#;
        let result = parser.parse_file("test.rs", code).unwrap();
        assert!(result
            .nodes
            .iter()
            .any(|n| matches!(n, ParsedNode::Trait(_))));

        if let ParsedNode::Trait(t) = &result.nodes[0] {
            assert_eq!(t.name, "Handler");
            assert_eq!(t.super_traits.len(), 2);
            assert_eq!(t.required_methods.len(), 1);
            assert_eq!(t.provided_methods.len(), 1);
            assert_eq!(t.associated_types.len(), 1);
        }
    }

    #[test]
    fn test_parse_impl() {
        let parser = RustParser::new();
        let code = r#"
impl Handler for MyStruct {
    type Output = String;
    fn handle(&self) -> String {
        "handled".to_string()
    }
}
"#;
        let result = parser.parse_file("test.rs", code).unwrap();

        // Should have impl + function
        assert!(!result.nodes.is_empty());
        assert!(result
            .nodes
            .iter()
            .any(|n| matches!(n, ParsedNode::Impl(_))));

        // Should have implements edge
        assert!(result
            .edges
            .iter()
            .any(|e| matches!(e, ParsedEdge::Implements(_))));
    }

    #[test]
    fn test_parse_enum() {
        let parser = RustParser::new();
        let code = r#"
#[derive(Debug)]
pub enum Status {
    Active,
    Inactive { reason: String },
    Pending(u32),
}
"#;
        let result = parser.parse_file("test.rs", code).unwrap();

        if let ParsedNode::Enum(e) = &result.nodes[0] {
            assert_eq!(e.name, "Status");
            assert_eq!(e.variants.len(), 3);
            assert!(e.derives.contains(&"Debug".to_string()));
        } else {
            panic!("Expected enum");
        }
    }

    #[test]
    fn test_call_extraction() {
        let parser = RustParser::new();
        let code = r#"
fn caller() {
    helper_function();
    obj.method_call();
    if condition {
        conditional_call();
    }
}

fn helper_function() {}
"#;
        let result = parser.parse_file("test.rs", code).unwrap();

        // Should have 2 functions
        let functions: Vec<_> = result
            .nodes
            .iter()
            .filter(|n| matches!(n, ParsedNode::Function(_)))
            .collect();
        assert_eq!(functions.len(), 2, "Expected 2 functions");

        // Should have call edges
        let calls: Vec<_> = result
            .edges
            .iter()
            .filter(|e| matches!(e, ParsedEdge::Calls(_)))
            .collect();

        assert!(
            !calls.is_empty(),
            "Expected call edges to be extracted, got {} calls",
            calls.len()
        );

        // Check we have the expected calls
        let call_names: Vec<_> = calls
            .iter()
            .filter_map(|e| {
                if let ParsedEdge::Calls(c) = e {
                    Some(c.to.clone())
                } else {
                    None
                }
            })
            .collect();

        assert!(
            call_names.iter().any(|n| n.contains("helper_function")),
            "Expected helper_function call, got: {:?}",
            call_names
        );
        assert!(
            call_names.iter().any(|n| n.contains("method_call")),
            "Expected method_call, got: {:?}",
            call_names
        );
        assert!(
            call_names.iter().any(|n| n.contains("conditional_call")),
            "Expected conditional_call, got: {:?}",
            call_names
        );
    }
}
