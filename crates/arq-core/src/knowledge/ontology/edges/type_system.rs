//! Type system edges: relationships that define type hierarchies and usage.
//!
//! These edges represent how types relate to each other:
//! - IMPLEMENTS: A implements B (struct implements trait)
//! - EXTENDS: A extends B (trait extends trait)
//! - USES_TYPE: A uses type B (parameter, field)
//! - RETURNS_TYPE: A returns type B
//! - HAS_FIELD: A has field of type B

use serde::{Deserialize, Serialize};

// =============================================================================
// IMPLEMENTS EDGE
// =============================================================================

/// A IMPLEMENTS B: Type A implements trait/interface B.
///
/// Critical for understanding polymorphism and contract compliance.
///
/// Examples:
/// - Struct IMPLEMENTS Trait
/// - Class IMPLEMENTS Interface
/// - Type IMPLEMENTS Protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementsEdge {
    /// Source node ID (implementor)
    pub from: String,
    /// Target node ID (trait/interface)
    pub to: String,
    /// File where implementation is defined
    pub impl_file: Option<String>,
    /// Line number of impl block
    pub impl_line: Option<u32>,
    /// Generic parameters used in impl
    pub generics: Vec<String>,
    /// Where clause constraints
    pub where_clause: Option<String>,
    /// Whether this is a blanket impl
    pub is_blanket: bool,
}

impl ImplementsEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            impl_file: None,
            impl_line: None,
            generics: Vec::new(),
            where_clause: None,
            is_blanket: false,
        }
    }

    pub fn at(mut self, file: impl Into<String>, line: u32) -> Self {
        self.impl_file = Some(file.into());
        self.impl_line = Some(line);
        self
    }

    pub fn blanket(mut self) -> Self {
        self.is_blanket = true;
        self
    }
}

// =============================================================================
// EXTENDS EDGE
// =============================================================================

/// A EXTENDS B: Type A extends/inherits from B.
///
/// Examples:
/// - Trait EXTENDS SuperTrait
/// - Class EXTENDS BaseClass
/// - Interface EXTENDS BaseInterface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendsEdge {
    /// Source node ID (subtype)
    pub from: String,
    /// Target node ID (supertype)
    pub to: String,
    /// Extension type
    pub extension_type: ExtensionType,
}

/// Type of extension relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionType {
    /// Trait bound (trait: SuperTrait)
    #[default]
    TraitBound,
    /// Class inheritance
    ClassInheritance,
    /// Interface extension
    InterfaceExtension,
    /// Type alias
    TypeAlias,
}

impl ExtendsEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            extension_type: ExtensionType::TraitBound,
        }
    }

    pub fn inheritance(mut self) -> Self {
        self.extension_type = ExtensionType::ClassInheritance;
        self
    }
}

// =============================================================================
// USES_TYPE EDGE
// =============================================================================

/// A USES_TYPE B: Entity A uses type B in some capacity.
///
/// Tracks type dependencies for impact analysis.
///
/// Examples:
/// - Function USES_TYPE Type (parameter)
/// - Struct USES_TYPE Type (field type)
/// - Variable USES_TYPE Type (declaration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsesTypeEdge {
    /// Source node ID (user)
    pub from: String,
    /// Target node ID (type being used)
    pub to: String,
    /// How the type is used
    pub usage: TypeUsage,
    /// Whether the type is behind a reference
    pub is_reference: bool,
    /// Whether the type is optional (`Option<T>`)
    pub is_optional: bool,
    /// Whether the type is in a collection (`Vec<T>`, etc.)
    pub is_collection: bool,
}

/// How a type is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TypeUsage {
    /// Function parameter
    #[default]
    Parameter,
    /// Struct field
    Field,
    /// Local variable
    Variable,
    /// Generic constraint
    Constraint,
    /// Cast target
    Cast,
    /// Associated type
    Associated,
}

impl UsesTypeEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            usage: TypeUsage::Parameter,
            is_reference: false,
            is_optional: false,
            is_collection: false,
        }
    }

    pub fn as_field(mut self) -> Self {
        self.usage = TypeUsage::Field;
        self
    }

    pub fn as_reference(mut self) -> Self {
        self.is_reference = true;
        self
    }

    pub fn optional(mut self) -> Self {
        self.is_optional = true;
        self
    }
}

// =============================================================================
// RETURNS_TYPE EDGE
// =============================================================================

/// A RETURNS_TYPE B: Function A has return type B.
///
/// Tracks return type relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnsTypeEdge {
    /// Source node ID (function)
    pub from: String,
    /// Target node ID (return type)
    pub to: String,
    /// Whether wrapped in Result
    pub is_result: bool,
    /// Whether wrapped in Option
    pub is_option: bool,
    /// Whether this is a Future
    pub is_future: bool,
    /// Whether this is an impl Trait return
    pub is_impl_trait: bool,
}

impl ReturnsTypeEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            is_result: false,
            is_option: false,
            is_future: false,
            is_impl_trait: false,
        }
    }

    pub fn result(mut self) -> Self {
        self.is_result = true;
        self
    }

    pub fn future(mut self) -> Self {
        self.is_future = true;
        self
    }
}

// =============================================================================
// HAS_FIELD EDGE
// =============================================================================

/// A HAS_FIELD B: Struct A has a field of type B.
///
/// Specifically tracks struct composition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasFieldEdge {
    /// Source node ID (struct)
    pub from: String,
    /// Target node ID (field type)
    pub to: String,
    /// Field name
    pub field_name: String,
    /// Field index (order in struct)
    pub field_index: u32,
    /// Whether field is public
    pub is_public: bool,
    /// Whether field is mutable (for interior mutability patterns)
    pub is_mutable: bool,
}

impl HasFieldEdge {
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        field_name: impl Into<String>,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            field_name: field_name.into(),
            field_index: 0,
            is_public: false,
            is_mutable: false,
        }
    }

    pub fn at_index(mut self, index: u32) -> Self {
        self.field_index = index;
        self
    }

    pub fn public(mut self) -> Self {
        self.is_public = true;
        self
    }
}
