//! Parse result types containing extracted ontology entities.

use crate::knowledge::ontology::edges::{
    CallsEdge, ContainsEdge, ExtendsEdge, HasFieldEdge, ImplementsEdge, ImportsEdge,
    ReturnsTypeEdge, UsesTypeEdge,
};
use crate::knowledge::ontology::nodes::{
    ConstantEntity, EnumEntity, FunctionEntity, ImplEntity, StructEntity, TraitEntity,
};

/// Result of parsing a source file.
///
/// Contains all extracted nodes (entities) and edges (relations).
#[derive(Debug, Default)]
pub struct ParseResult {
    /// File path that was parsed.
    pub file_path: String,

    /// Extracted nodes (entities).
    pub nodes: Vec<ParsedNode>,

    /// Extracted edges (relations).
    pub edges: Vec<ParsedEdge>,

    /// Parse warnings (non-fatal issues).
    pub warnings: Vec<String>,
}

impl ParseResult {
    /// Create a new parse result for the given file.
    pub fn new(file_path: impl Into<String>) -> Self {
        Self {
            file_path: file_path.into(),
            ..Default::default()
        }
    }

    /// Add a function entity.
    pub fn add_function(&mut self, func: FunctionEntity) {
        self.nodes.push(ParsedNode::Function(func));
    }

    /// Add a struct entity.
    pub fn add_struct(&mut self, s: StructEntity) {
        self.nodes.push(ParsedNode::Struct(s));
    }

    /// Add a trait entity.
    pub fn add_trait(&mut self, t: TraitEntity) {
        self.nodes.push(ParsedNode::Trait(t));
    }

    /// Add an impl entity.
    pub fn add_impl(&mut self, i: ImplEntity) {
        self.nodes.push(ParsedNode::Impl(i));
    }

    /// Add an enum entity.
    pub fn add_enum(&mut self, e: EnumEntity) {
        self.nodes.push(ParsedNode::Enum(e));
    }

    /// Add a constant entity.
    pub fn add_constant(&mut self, c: ConstantEntity) {
        self.nodes.push(ParsedNode::Constant(c));
    }

    /// Add a calls edge.
    pub fn add_call(&mut self, edge: CallsEdge) {
        self.edges.push(ParsedEdge::Calls(edge));
    }

    /// Add a contains edge.
    pub fn add_contains(&mut self, edge: ContainsEdge) {
        self.edges.push(ParsedEdge::Contains(edge));
    }

    /// Add an implements edge.
    pub fn add_implements(&mut self, edge: ImplementsEdge) {
        self.edges.push(ParsedEdge::Implements(edge));
    }

    /// Add an extends edge.
    pub fn add_extends(&mut self, edge: ExtendsEdge) {
        self.edges.push(ParsedEdge::Extends(edge));
    }

    /// Add a uses_type edge.
    pub fn add_uses_type(&mut self, edge: UsesTypeEdge) {
        self.edges.push(ParsedEdge::UsesType(edge));
    }

    /// Add a returns_type edge.
    pub fn add_returns_type(&mut self, edge: ReturnsTypeEdge) {
        self.edges.push(ParsedEdge::ReturnsType(edge));
    }

    /// Add a has_field edge.
    pub fn add_has_field(&mut self, edge: HasFieldEdge) {
        self.edges.push(ParsedEdge::HasField(edge));
    }

    /// Add an imports edge.
    pub fn add_imports(&mut self, edge: ImportsEdge) {
        self.edges.push(ParsedEdge::Imports(edge));
    }

    /// Add a parse warning.
    pub fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    /// Get statistics about the parse result.
    pub fn stats(&self) -> ParseStats {
        let mut stats = ParseStats::default();

        for node in &self.nodes {
            match node {
                ParsedNode::Function(_) => stats.functions += 1,
                ParsedNode::Struct(_) => stats.structs += 1,
                ParsedNode::Trait(_) => stats.traits += 1,
                ParsedNode::Impl(_) => stats.impls += 1,
                ParsedNode::Enum(_) => stats.enums += 1,
                ParsedNode::Constant(_) => stats.constants += 1,
            }
        }

        for edge in &self.edges {
            match edge {
                ParsedEdge::Calls(_) => stats.calls += 1,
                ParsedEdge::Contains(_) => stats.contains += 1,
                ParsedEdge::Implements(_) => stats.implements += 1,
                ParsedEdge::Extends(_) => stats.extends += 1,
                ParsedEdge::UsesType(_) => stats.uses_type += 1,
                ParsedEdge::ReturnsType(_) => stats.returns_type += 1,
                ParsedEdge::HasField(_) => stats.has_field += 1,
                ParsedEdge::Imports(_) => stats.imports += 1,
            }
        }

        stats.warnings = self.warnings.len();
        stats
    }
}

/// A parsed node (entity) from the source code.
#[derive(Debug, Clone)]
pub enum ParsedNode {
    Function(FunctionEntity),
    Struct(StructEntity),
    Trait(TraitEntity),
    Impl(ImplEntity),
    Enum(EnumEntity),
    Constant(ConstantEntity),
}

impl ParsedNode {
    /// Get the entity ID.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Function(f) => f.id.as_deref(),
            Self::Struct(s) => s.id.as_deref(),
            Self::Trait(t) => t.id.as_deref(),
            Self::Impl(i) => i.id.as_deref(),
            Self::Enum(e) => e.id.as_deref(),
            Self::Constant(c) => c.id.as_deref(),
        }
    }

    /// Get the entity name.
    pub fn name(&self) -> &str {
        match self {
            Self::Function(f) => &f.name,
            Self::Struct(s) => &s.name,
            Self::Trait(t) => &t.name,
            Self::Impl(i) => &i.target_type,
            Self::Enum(e) => &e.name,
            Self::Constant(c) => &c.name,
        }
    }

    /// Get the file path.
    pub fn file_path(&self) -> &str {
        match self {
            Self::Function(f) => &f.file_path,
            Self::Struct(s) => &s.file_path,
            Self::Trait(t) => &t.file_path,
            Self::Impl(i) => &i.file_path,
            Self::Enum(e) => &e.file_path,
            Self::Constant(c) => &c.file_path,
        }
    }

    /// Get the start line.
    pub fn start_line(&self) -> u32 {
        match self {
            Self::Function(f) => f.start_line,
            Self::Struct(s) => s.start_line,
            Self::Trait(t) => t.start_line,
            Self::Impl(i) => i.start_line,
            Self::Enum(e) => e.start_line,
            Self::Constant(c) => c.line,
        }
    }

    /// Get the end line.
    pub fn end_line(&self) -> u32 {
        match self {
            Self::Function(f) => f.end_line,
            Self::Struct(s) => s.end_line,
            Self::Trait(t) => t.end_line,
            Self::Impl(i) => i.end_line,
            Self::Enum(e) => e.end_line,
            Self::Constant(c) => c.line,
        }
    }

    /// Get the node type name.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Function(_) => "function",
            Self::Struct(_) => "struct",
            Self::Trait(_) => "trait",
            Self::Impl(_) => "impl",
            Self::Enum(_) => "enum",
            Self::Constant(_) => "constant",
        }
    }
}

/// A parsed edge (relation) from the source code.
#[derive(Debug, Clone)]
pub enum ParsedEdge {
    Calls(CallsEdge),
    Contains(ContainsEdge),
    Implements(ImplementsEdge),
    Extends(ExtendsEdge),
    UsesType(UsesTypeEdge),
    ReturnsType(ReturnsTypeEdge),
    HasField(HasFieldEdge),
    Imports(ImportsEdge),
}

impl ParsedEdge {
    /// Get the source node ID.
    pub fn from(&self) -> &str {
        match self {
            Self::Calls(e) => &e.from,
            Self::Contains(e) => &e.from,
            Self::Implements(e) => &e.from,
            Self::Extends(e) => &e.from,
            Self::UsesType(e) => &e.from,
            Self::ReturnsType(e) => &e.from,
            Self::HasField(e) => &e.from,
            Self::Imports(e) => &e.from,
        }
    }

    /// Get the target node ID.
    pub fn to(&self) -> &str {
        match self {
            Self::Calls(e) => &e.to,
            Self::Contains(e) => &e.to,
            Self::Implements(e) => &e.to,
            Self::Extends(e) => &e.to,
            Self::UsesType(e) => &e.to,
            Self::ReturnsType(e) => &e.to,
            Self::HasField(e) => &e.to,
            Self::Imports(e) => &e.to,
        }
    }

    /// Get the edge type name.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Calls(_) => "calls",
            Self::Contains(_) => "contains",
            Self::Implements(_) => "implements",
            Self::Extends(_) => "extends",
            Self::UsesType(_) => "uses_type",
            Self::ReturnsType(_) => "returns_type",
            Self::HasField(_) => "has_field",
            Self::Imports(_) => "imports",
        }
    }
}

/// Statistics about a parse result.
#[derive(Debug, Default, Clone)]
pub struct ParseStats {
    pub functions: usize,
    pub structs: usize,
    pub traits: usize,
    pub impls: usize,
    pub enums: usize,
    pub constants: usize,
    pub calls: usize,
    pub contains: usize,
    pub implements: usize,
    pub extends: usize,
    pub uses_type: usize,
    pub returns_type: usize,
    pub has_field: usize,
    pub imports: usize,
    pub warnings: usize,
}

impl std::fmt::Display for ParseStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Nodes:")?;
        writeln!(f, "  Functions: {}", self.functions)?;
        writeln!(f, "  Structs:   {}", self.structs)?;
        writeln!(f, "  Traits:    {}", self.traits)?;
        writeln!(f, "  Impls:     {}", self.impls)?;
        writeln!(f, "  Enums:     {}", self.enums)?;
        writeln!(f, "  Constants: {}", self.constants)?;
        writeln!(f, "Edges:")?;
        writeln!(f, "  Calls:     {}", self.calls)?;
        writeln!(f, "  Contains:  {}", self.contains)?;
        writeln!(f, "  Implements:{}", self.implements)?;
        writeln!(f, "  UsesType:  {}", self.uses_type)?;
        if self.warnings > 0 {
            writeln!(f, "Warnings: {}", self.warnings)?;
        }
        Ok(())
    }
}
