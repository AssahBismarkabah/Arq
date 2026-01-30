//! Node types for the knowledge graph.
//!
//! Nodes represent entities in the codebase. They are organized by domain:
//!
//! - **Code**: Functions, Structs, Traits, Implementations
//! - **API**: Endpoints, Schemas, Operations
//! - **Structure**: Files, Modules, Packages
//! - **Test**: Test cases, Test suites

mod api;
mod code;
mod structure;
mod test;

pub use api::*;
pub use code::*;
pub use structure::*;
pub use test::*;

use super::NodeCategory;
use serde::{Deserialize, Serialize};

/// A unified node type that can hold any entity in the knowledge graph.
///
/// This enum allows for type-safe handling of different node types
/// while maintaining a unified interface for graph operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "node_type")]
pub enum GraphNode {
    // === Code Nodes ===
    /// A function or method
    Function(FunctionEntity),
    /// A struct or class
    Struct(StructEntity),
    /// A trait or interface
    Trait(TraitEntity),
    /// An implementation block
    Impl(ImplEntity),
    /// An enum type
    Enum(EnumEntity),
    /// A constant or static
    Constant(ConstantEntity),

    // === API Nodes ===
    /// A REST/GraphQL endpoint
    Endpoint(EndpointEntity),
    /// A data schema/DTO
    Schema(SchemaEntity),
    /// An API operation
    Operation(OperationEntity),

    // === Structure Nodes ===
    /// A source file
    File(FileEntity),
    /// A module (Rust mod, JS module)
    Module(ModuleEntity),
    /// A package/crate
    Package(PackageEntity),

    // === Test Nodes ===
    /// A single test case
    TestCase(TestCaseEntity),
    /// A test suite/module
    TestSuite(TestSuiteEntity),
}

impl GraphNode {
    /// Get the category of this node.
    pub fn category(&self) -> NodeCategory {
        match self {
            Self::Function(_)
            | Self::Struct(_)
            | Self::Trait(_)
            | Self::Impl(_)
            | Self::Enum(_)
            | Self::Constant(_) => NodeCategory::Code,

            Self::Endpoint(_) | Self::Schema(_) | Self::Operation(_) => NodeCategory::Api,

            Self::File(_) | Self::Module(_) | Self::Package(_) => NodeCategory::Structure,

            Self::TestCase(_) | Self::TestSuite(_) => NodeCategory::Test,
        }
    }

    /// Get a human-readable type name.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Function(_) => "Function",
            Self::Struct(_) => "Struct",
            Self::Trait(_) => "Trait",
            Self::Impl(_) => "Impl",
            Self::Enum(_) => "Enum",
            Self::Constant(_) => "Constant",
            Self::Endpoint(_) => "Endpoint",
            Self::Schema(_) => "Schema",
            Self::Operation(_) => "Operation",
            Self::File(_) => "File",
            Self::Module(_) => "Module",
            Self::Package(_) => "Package",
            Self::TestCase(_) => "TestCase",
            Self::TestSuite(_) => "TestSuite",
        }
    }

    /// Get the unique identifier for this node.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Function(n) => n.id.as_deref(),
            Self::Struct(n) => n.id.as_deref(),
            Self::Trait(n) => n.id.as_deref(),
            Self::Impl(n) => n.id.as_deref(),
            Self::Enum(n) => n.id.as_deref(),
            Self::Constant(n) => n.id.as_deref(),
            Self::Endpoint(n) => n.id.as_deref(),
            Self::Schema(n) => n.id.as_deref(),
            Self::Operation(n) => n.id.as_deref(),
            Self::File(n) => n.id.as_deref(),
            Self::Module(n) => n.id.as_deref(),
            Self::Package(n) => n.id.as_deref(),
            Self::TestCase(n) => n.id.as_deref(),
            Self::TestSuite(n) => n.id.as_deref(),
        }
    }

    /// Get the display name for this node.
    pub fn name(&self) -> &str {
        match self {
            Self::Function(n) => &n.name,
            Self::Struct(n) => &n.name,
            Self::Trait(n) => &n.name,
            Self::Impl(n) => &n.target_type,
            Self::Enum(n) => &n.name,
            Self::Constant(n) => &n.name,
            Self::Endpoint(n) => &n.path,
            Self::Schema(n) => &n.name,
            Self::Operation(n) => &n.name,
            Self::File(n) => &n.path,
            Self::Module(n) => &n.name,
            Self::Package(n) => &n.name,
            Self::TestCase(n) => &n.name,
            Self::TestSuite(n) => &n.name,
        }
    }
}
