//! Edge types (relationships) for the knowledge graph.
//!
//! Edges represent relationships between nodes. They are organized by semantic meaning:
//!
//! - **Structural**: CONTAINS, BELONGS_TO, IMPORTS, EXPORTS
//! - **Behavioral**: CALLS, RETURNS, THROWS, AWAITS
//! - **Type System**: IMPLEMENTS, EXTENDS, USES_TYPE, RETURNS_TYPE
//! - **API**: EXPOSES, MAPS_TO, CONSUMES, PRODUCES

mod structural;
mod behavioral;
mod type_system;
mod api;

pub use structural::*;
pub use behavioral::*;
pub use type_system::*;
pub use api::*;

use serde::{Deserialize, Serialize};
use super::EdgeCategory;

/// A unified edge type that can represent any relationship in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "edge_type")]
pub enum GraphEdge {
    // === Structural Edges ===
    /// A contains B (file contains function, module contains struct)
    Contains(ContainsEdge),
    /// A belongs to B (function belongs to file)
    BelongsTo(BelongsToEdge),
    /// A imports B (file imports module)
    Imports(ImportsEdge),
    /// A exports B (module exports function)
    Exports(ExportsEdge),
    /// A depends on B (package depends on package)
    DependsOn(DependsOnEdge),

    // === Behavioral Edges ===
    /// A calls B (function calls function)
    Calls(CallsEdge),
    /// A returns B (function returns type)
    Returns(ReturnsEdge),
    /// A throws B (function throws error)
    Throws(ThrowsEdge),
    /// A awaits B (async function awaits another)
    Awaits(AwaitsEdge),
    /// A reads B (function reads field/variable)
    Reads(ReadsEdge),
    /// A writes B (function writes field/variable)
    Writes(WritesEdge),

    // === Type System Edges ===
    /// A implements B (struct implements trait)
    Implements(ImplementsEdge),
    /// A extends B (trait extends trait, class extends class)
    Extends(ExtendsEdge),
    /// A uses type B (function parameter uses type)
    UsesType(UsesTypeEdge),
    /// A returns type B (function return type)
    ReturnsType(ReturnsTypeEdge),
    /// A has field of type B
    HasField(HasFieldEdge),

    // === API Edges ===
    /// A exposes B (function exposes endpoint)
    Exposes(ExposesEdge),
    /// A maps to B (endpoint maps to handler)
    MapsTo(MapsToEdge),
    /// A consumes B (endpoint consumes schema)
    Consumes(ConsumesEdge),
    /// A produces B (endpoint produces schema)
    Produces(ProducesEdge),

    // === Test Edges ===
    /// A tests B (test case tests function)
    Tests(TestsEdge),
}

impl GraphEdge {
    /// Get the category of this edge.
    pub fn category(&self) -> EdgeCategory {
        match self {
            Self::Contains(_) | Self::BelongsTo(_) | Self::Imports(_) |
            Self::Exports(_) | Self::DependsOn(_) => EdgeCategory::Structural,

            Self::Calls(_) | Self::Returns(_) | Self::Throws(_) |
            Self::Awaits(_) | Self::Reads(_) | Self::Writes(_) => EdgeCategory::Behavioral,

            Self::Implements(_) | Self::Extends(_) | Self::UsesType(_) |
            Self::ReturnsType(_) | Self::HasField(_) => EdgeCategory::TypeSystem,

            Self::Exposes(_) | Self::MapsTo(_) | Self::Consumes(_) |
            Self::Produces(_) | Self::Tests(_) => EdgeCategory::Api,
        }
    }

    /// Get the relationship name for display.
    pub fn relation_name(&self) -> &'static str {
        match self {
            Self::Contains(_) => "CONTAINS",
            Self::BelongsTo(_) => "BELONGS_TO",
            Self::Imports(_) => "IMPORTS",
            Self::Exports(_) => "EXPORTS",
            Self::DependsOn(_) => "DEPENDS_ON",
            Self::Calls(_) => "CALLS",
            Self::Returns(_) => "RETURNS",
            Self::Throws(_) => "THROWS",
            Self::Awaits(_) => "AWAITS",
            Self::Reads(_) => "READS",
            Self::Writes(_) => "WRITES",
            Self::Implements(_) => "IMPLEMENTS",
            Self::Extends(_) => "EXTENDS",
            Self::UsesType(_) => "USES_TYPE",
            Self::ReturnsType(_) => "RETURNS_TYPE",
            Self::HasField(_) => "HAS_FIELD",
            Self::Exposes(_) => "EXPOSES",
            Self::MapsTo(_) => "MAPS_TO",
            Self::Consumes(_) => "CONSUMES",
            Self::Produces(_) => "PRODUCES",
            Self::Tests(_) => "TESTS",
        }
    }

    /// Get the source node ID.
    pub fn from_id(&self) -> &str {
        match self {
            Self::Contains(e) => &e.from,
            Self::BelongsTo(e) => &e.from,
            Self::Imports(e) => &e.from,
            Self::Exports(e) => &e.from,
            Self::DependsOn(e) => &e.from,
            Self::Calls(e) => &e.from,
            Self::Returns(e) => &e.from,
            Self::Throws(e) => &e.from,
            Self::Awaits(e) => &e.from,
            Self::Reads(e) => &e.from,
            Self::Writes(e) => &e.from,
            Self::Implements(e) => &e.from,
            Self::Extends(e) => &e.from,
            Self::UsesType(e) => &e.from,
            Self::ReturnsType(e) => &e.from,
            Self::HasField(e) => &e.from,
            Self::Exposes(e) => &e.from,
            Self::MapsTo(e) => &e.from,
            Self::Consumes(e) => &e.from,
            Self::Produces(e) => &e.from,
            Self::Tests(e) => &e.from,
        }
    }

    /// Get the target node ID.
    pub fn to_id(&self) -> &str {
        match self {
            Self::Contains(e) => &e.to,
            Self::BelongsTo(e) => &e.to,
            Self::Imports(e) => &e.to,
            Self::Exports(e) => &e.to,
            Self::DependsOn(e) => &e.to,
            Self::Calls(e) => &e.to,
            Self::Returns(e) => &e.to,
            Self::Throws(e) => &e.to,
            Self::Awaits(e) => &e.to,
            Self::Reads(e) => &e.to,
            Self::Writes(e) => &e.to,
            Self::Implements(e) => &e.to,
            Self::Extends(e) => &e.to,
            Self::UsesType(e) => &e.to,
            Self::ReturnsType(e) => &e.to,
            Self::HasField(e) => &e.to,
            Self::Exposes(e) => &e.to,
            Self::MapsTo(e) => &e.to,
            Self::Consumes(e) => &e.to,
            Self::Produces(e) => &e.to,
            Self::Tests(e) => &e.to,
        }
    }
}

/// Test edge for test coverage tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestsEdge {
    /// Source node (test case)
    pub from: String,
    /// Target node (function/struct being tested)
    pub to: String,
    /// Test coverage type
    pub coverage_type: TestCoverageType,
}

/// Type of test coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestCoverageType {
    /// Direct unit test
    Unit,
    /// Integration test
    Integration,
    /// Indirect coverage through other tests
    Indirect,
}
