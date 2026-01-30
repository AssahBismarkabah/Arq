//! Code entity nodes: Functions, Structs, Traits, Implementations.
//!
//! These represent the core code constructs that make up a codebase.

use serde::{Deserialize, Serialize};

// =============================================================================
// FUNCTION ENTITY
// =============================================================================

/// A function or method in the codebase.
///
/// Represents both standalone functions and struct methods.
/// The `parent` field indicates if this is a method (has parent) or function (no parent).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionEntity {
    /// Unique identifier (e.g., "function:file.rs:my_function")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Function name
    pub name: String,

    /// Full qualified name (e.g., "module::submodule::function")
    pub qualified_name: String,

    /// File containing this function
    pub file_path: String,

    /// Start line number
    pub start_line: u32,

    /// End line number
    pub end_line: u32,

    /// Function signature (e.g., "fn process(data: &Data) -> Result<Output>")
    pub signature: String,

    /// Parent struct/impl if this is a method
    pub parent: Option<String>,

    /// Visibility (pub, pub(crate), private)
    pub visibility: Visibility,

    /// Whether the function is async
    pub is_async: bool,

    /// Whether the function is unsafe
    pub is_unsafe: bool,

    /// Generic parameters (e.g., ["T", "U: Clone"])
    pub generics: Vec<String>,

    /// Parameter types
    pub parameters: Vec<Parameter>,

    /// Return type
    pub return_type: Option<String>,

    /// Documentation comment
    pub doc_comment: Option<String>,

    /// Complexity metrics
    pub complexity: Option<ComplexityMetrics>,
}

/// A function parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_name: String,
    pub is_mutable: bool,
    pub is_reference: bool,
}

/// Code complexity metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Cyclomatic complexity
    pub cyclomatic: u32,
    /// Lines of code
    pub loc: u32,
    /// Cognitive complexity
    pub cognitive: Option<u32>,
}

// =============================================================================
// STRUCT ENTITY
// =============================================================================

/// A struct or class in the codebase.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructEntity {
    /// Unique identifier
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,

    /// Struct name
    #[serde(default)]
    pub name: String,

    /// Full qualified name
    #[serde(default)]
    pub qualified_name: String,

    /// File containing this struct
    #[serde(default)]
    pub file_path: String,

    /// Start line number
    #[serde(default)]
    pub start_line: u32,

    /// End line number
    #[serde(default)]
    pub end_line: u32,

    /// Visibility
    #[serde(default)]
    pub visibility: Visibility,

    /// Generic parameters
    #[serde(default)]
    pub generics: Vec<String>,

    /// Fields
    #[serde(default)]
    pub fields: Vec<FieldInfo>,

    /// Derive macros applied
    #[serde(default)]
    pub derives: Vec<String>,

    /// Attributes (e.g., #[serde(rename_all = "camelCase")])
    #[serde(default)]
    pub attributes: Vec<String>,

    /// Documentation comment
    #[serde(default)]
    pub doc_comment: Option<String>,
}

/// Information about a struct field.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FieldInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub type_name: String,
    #[serde(default)]
    pub visibility: Visibility,
    #[serde(default)]
    pub attributes: Vec<String>,
    #[serde(default)]
    pub doc_comment: Option<String>,
}

// =============================================================================
// TRAIT ENTITY
// =============================================================================

/// A trait or interface in the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitEntity {
    /// Unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Trait name
    pub name: String,

    /// Full qualified name
    pub qualified_name: String,

    /// File containing this trait
    pub file_path: String,

    /// Start line number
    pub start_line: u32,

    /// End line number
    pub end_line: u32,

    /// Visibility
    pub visibility: Visibility,

    /// Generic parameters
    pub generics: Vec<String>,

    /// Super traits (traits this extends)
    pub super_traits: Vec<String>,

    /// Required methods
    pub required_methods: Vec<String>,

    /// Provided (default) methods
    pub provided_methods: Vec<String>,

    /// Associated types
    pub associated_types: Vec<String>,

    /// Documentation comment
    pub doc_comment: Option<String>,
}

// =============================================================================
// IMPL ENTITY
// =============================================================================

/// An implementation block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplEntity {
    /// Unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// The type being implemented for
    pub target_type: String,

    /// The trait being implemented (None for inherent impl)
    pub trait_name: Option<String>,

    /// File containing this impl
    pub file_path: String,

    /// Start line number
    pub start_line: u32,

    /// End line number
    pub end_line: u32,

    /// Generic parameters
    pub generics: Vec<String>,

    /// Where clause constraints
    pub where_clause: Option<String>,

    /// Methods in this impl block
    pub methods: Vec<String>,
}

// =============================================================================
// ENUM ENTITY
// =============================================================================

/// An enum type in the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumEntity {
    /// Unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Enum name
    pub name: String,

    /// Full qualified name
    pub qualified_name: String,

    /// File containing this enum
    pub file_path: String,

    /// Start line number
    pub start_line: u32,

    /// End line number
    pub end_line: u32,

    /// Visibility
    pub visibility: Visibility,

    /// Generic parameters
    pub generics: Vec<String>,

    /// Variants
    pub variants: Vec<EnumVariant>,

    /// Derive macros applied
    pub derives: Vec<String>,

    /// Documentation comment
    pub doc_comment: Option<String>,
}

/// An enum variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<FieldInfo>,
    pub discriminant: Option<String>,
    pub doc_comment: Option<String>,
}

// =============================================================================
// CONSTANT ENTITY
// =============================================================================

/// A constant or static in the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantEntity {
    /// Unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Constant name
    pub name: String,

    /// Full qualified name
    pub qualified_name: String,

    /// File containing this constant
    pub file_path: String,

    /// Line number
    pub line: u32,

    /// Visibility
    pub visibility: Visibility,

    /// Type
    pub type_name: String,

    /// Whether this is static (vs const)
    pub is_static: bool,

    /// Whether this is mutable (static mut)
    pub is_mutable: bool,

    /// Documentation comment
    pub doc_comment: Option<String>,
}

// =============================================================================
// COMMON TYPES
// =============================================================================

/// Visibility level of a code entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Public (accessible from anywhere)
    Public,
    /// Public within crate
    PublicCrate,
    /// Public within parent module
    PublicSuper,
    /// Public within specific path (path stored in separate field)
    PublicIn,
    /// Private (default)
    #[default]
    Private,
}

impl Visibility {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pub" | "public" => Self::Public,
            "pub(crate)" => Self::PublicCrate,
            "pub(super)" => Self::PublicSuper,
            s if s.starts_with("pub(in") => Self::PublicIn,
            _ => Self::Private,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "pub",
            Self::PublicCrate => "pub(crate)",
            Self::PublicSuper => "pub(super)",
            Self::PublicIn => "pub(in ...)",
            Self::Private => "private",
        }
    }
}
