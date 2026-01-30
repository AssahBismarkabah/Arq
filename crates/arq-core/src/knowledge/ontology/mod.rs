//! Knowledge Graph Ontology
//!
//! Defines the semantic schema for the code knowledge graph with rich types
//! for representing code entities and their relationships.
//!
//! ## Modules
//!
//! - `nodes/` - Entity types: Code (Function, Struct, Trait), API (Endpoint, Schema),
//!   Structure (File, Module), Test (TestCase, TestSuite)
//! - `edges/` - Relationship types: Structural (CONTAINS, IMPORTS), Behavioral (CALLS),
//!   TypeSystem (IMPLEMENTS, EXTENDS), API (EXPOSES, MAPS_TO)
//!
//! ## Design Principles
//!
//! - Semantic clarity with well-defined node/edge types
//! - Domain separation between Code, API, Structure, and Tests
//! - Composable types for rich graph queries
//! - Extensible without breaking existing code

pub mod edges;
pub mod nodes;

pub use edges::*;
pub use nodes::*;

use serde::{Deserialize, Serialize};

/// Marker trait for all node types in the knowledge graph.
pub trait Node: Send + Sync {
    /// The table name in SurrealDB for this node type.
    fn table_name() -> &'static str;

    /// Human-readable type name for display.
    fn type_name(&self) -> &'static str;

    /// Unique identifier within the graph.
    fn node_id(&self) -> Option<String>;
}

/// Marker trait for all edge types in the knowledge graph.
pub trait Edge: Send + Sync {
    /// The table name in SurrealDB for this edge type.
    fn table_name() -> &'static str;

    /// Human-readable relationship name for display.
    fn relation_name(&self) -> &'static str;
}

/// Categories of nodes for filtering and organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeCategory {
    /// Code entities (functions, structs, traits)
    Code,
    /// API entities (endpoints, schemas)
    Api,
    /// Structural entities (files, modules)
    Structure,
    /// Test entities (test cases, suites)
    Test,
}

/// Categories of edges for filtering and organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeCategory {
    /// Structural relationships (contains, belongs_to)
    Structural,
    /// Behavioral relationships (calls, returns)
    Behavioral,
    /// Type system relationships (implements, extends)
    TypeSystem,
    /// API relationships (exposes, maps_to)
    Api,
}
