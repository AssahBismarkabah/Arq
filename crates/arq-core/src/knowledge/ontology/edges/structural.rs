//! Structural edges: relationships that define code organization.
//!
//! These edges represent how code is organized and structured:
//! - CONTAINS: Parent contains child (file contains function)
//! - BELONGS_TO: Child belongs to parent (function belongs to file)
//! - IMPORTS: A imports B (file imports module)
//! - EXPORTS: A exports B (module exports function)
//! - DEPENDS_ON: A depends on B (package depends on package)

use serde::{Deserialize, Serialize};

// =============================================================================
// CONTAINS EDGE
// =============================================================================

/// A CONTAINS B: Parent entity contains child entity.
///
/// Examples:
/// - File CONTAINS Function
/// - Module CONTAINS Struct
/// - Struct CONTAINS Field
/// - Package CONTAINS Module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainsEdge {
    /// Source node ID (parent)
    pub from: String,
    /// Target node ID (child)
    pub to: String,
    /// Order within parent (for maintaining source order)
    pub order: Option<u32>,
    /// Whether this is the primary container
    pub is_primary: bool,
}

impl ContainsEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            order: None,
            is_primary: true,
        }
    }

    pub fn with_order(mut self, order: u32) -> Self {
        self.order = Some(order);
        self
    }
}

// =============================================================================
// BELONGS_TO EDGE
// =============================================================================

/// A BELONGS_TO B: Child entity belongs to parent entity.
///
/// Inverse of CONTAINS. Useful for upward traversal.
///
/// Examples:
/// - Function BELONGS_TO File
/// - Method BELONGS_TO Struct
/// - Struct BELONGS_TO Module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BelongsToEdge {
    /// Source node ID (child)
    pub from: String,
    /// Target node ID (parent)
    pub to: String,
    /// Membership type
    pub membership: MembershipType,
}

/// Type of membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MembershipType {
    /// Defined directly in parent
    #[default]
    Direct,
    /// Re-exported from parent
    Reexport,
    /// Nested within parent
    Nested,
}

impl BelongsToEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            membership: MembershipType::Direct,
        }
    }
}

// =============================================================================
// IMPORTS EDGE
// =============================================================================

/// A IMPORTS B: A imports/uses B.
///
/// Examples:
/// - File IMPORTS Module
/// - Module IMPORTS ExternalCrate
/// - Function IMPORTS Type (via use statement)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportsEdge {
    /// Source node ID (importer)
    pub from: String,
    /// Target node ID (imported)
    pub to: String,
    /// Import alias (if renamed)
    pub alias: Option<String>,
    /// Whether this is a wildcard import (use foo::*)
    pub is_wildcard: bool,
    /// Specific items imported (empty = all or default)
    pub items: Vec<String>,
    /// Line number of import statement
    pub line: Option<u32>,
}

impl ImportsEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            alias: None,
            is_wildcard: false,
            items: Vec::new(),
            line: None,
        }
    }

    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }

    pub fn with_items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self
    }
}

// =============================================================================
// EXPORTS EDGE
// =============================================================================

/// A EXPORTS B: A makes B publicly available.
///
/// Examples:
/// - Module EXPORTS Function
/// - Package EXPORTS Module
/// - File EXPORTS Type (via pub mod or pub use)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportsEdge {
    /// Source node ID (exporter)
    pub from: String,
    /// Target node ID (exported)
    pub to: String,
    /// Export name (may differ from original name)
    pub export_name: Option<String>,
    /// Visibility level of export
    pub visibility: ExportVisibility,
    /// Whether this is a re-export
    pub is_reexport: bool,
}

/// Visibility of an export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExportVisibility {
    #[default]
    Public,
    Crate,
    Super,
    Restricted,
}

impl ExportsEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            export_name: None,
            visibility: ExportVisibility::Public,
            is_reexport: false,
        }
    }
}

// =============================================================================
// DEPENDS_ON EDGE
// =============================================================================

/// A DEPENDS_ON B: A requires B to function.
///
/// Higher-level dependency relationship between packages/modules.
///
/// Examples:
/// - Package DEPENDS_ON ExternalPackage
/// - Module DEPENDS_ON Module
/// - Service DEPENDS_ON Service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependsOnEdge {
    /// Source node ID (dependent)
    pub from: String,
    /// Target node ID (dependency)
    pub to: String,
    /// Type of dependency
    pub dependency_type: DependencyType,
    /// Version constraint (for packages)
    pub version_constraint: Option<String>,
    /// Whether this is optional
    pub is_optional: bool,
    /// Features required (for Rust crates)
    pub features: Vec<String>,
}

/// Type of dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    /// Runtime dependency
    #[default]
    Runtime,
    /// Development/test dependency
    Dev,
    /// Build-time dependency
    Build,
    /// Peer dependency (npm)
    Peer,
    /// Optional dependency
    Optional,
}

impl DependsOnEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            dependency_type: DependencyType::Runtime,
            version_constraint: None,
            is_optional: false,
            features: Vec::new(),
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version_constraint = Some(version.into());
        self
    }

    pub fn dev(mut self) -> Self {
        self.dependency_type = DependencyType::Dev;
        self
    }
}
