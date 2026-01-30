//! API response models for the graph visualization server.
//!
//! These are Data Transfer Objects (DTOs) that define the shape of
//! JSON responses sent to the frontend.

use serde::{Deserialize, Serialize};

// =============================================================================
// Graph Data Models (for Sigma.js/Graphology)
// =============================================================================

/// Full graph data response for `/api/graph`.
#[derive(Debug, Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// A node in the graph visualization.
#[derive(Debug, Serialize)]
pub struct GraphNode {
    /// Unique identifier for the node.
    pub key: String,
    /// Visual and metadata attributes.
    pub attributes: NodeAttributes,
}

/// Node attributes for rendering and display.
#[derive(Debug, Serialize)]
pub struct NodeAttributes {
    /// Display label for the node.
    pub label: String,
    /// Semantic category (function, struct, trait, enum, impl).
    /// Note: This is NOT Sigma's render type - it's our semantic type.
    pub category: String,
    /// Hex color for rendering.
    pub color: String,
    /// Node size in pixels.
    pub size: u32,
    /// Source file path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Start line number in source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    /// End line number in source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
}

/// An edge (relationship) in the graph visualization.
#[derive(Debug, Serialize)]
pub struct GraphEdge {
    /// Source node key.
    pub source: String,
    /// Target node key.
    pub target: String,
    /// Optional edge attributes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<EdgeAttributes>,
}

/// Edge attributes for rendering.
#[derive(Debug, Serialize)]
pub struct EdgeAttributes {
    /// Type of relationship (e.g., "Direct", "Method").
    /// Note: Using "relationship" instead of "type" to avoid conflict with Sigma's edge type.
    pub relationship: String,
}

// =============================================================================
// Node Details Model (for `/api/node/{id}`)
// =============================================================================

/// Detailed information about a single node.
#[derive(Debug, Serialize)]
pub struct NodeDetails {
    /// Unique identifier.
    pub key: String,
    /// Display name.
    pub label: String,
    /// Entity type (function, struct, etc.).
    pub node_type: String,
    /// Source file path.
    pub file: Option<String>,
    /// Start line number.
    pub start_line: Option<u32>,
    /// End line number.
    pub end_line: Option<u32>,
    /// Outgoing dependencies (nodes this node calls/uses).
    pub dependencies: Vec<String>,
    /// Incoming dependents (nodes that call/use this node).
    pub dependents: Vec<String>,
}

// =============================================================================
// Search Models (for `/api/search`)
// =============================================================================

/// Query parameters for search endpoint.
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// Search query string.
    pub q: String,
    /// Maximum number of results.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

/// A single search result.
#[derive(Debug, Serialize)]
pub struct SearchResult {
    /// Node key for graph reference.
    pub key: String,
    /// Display label (preview text).
    pub label: String,
    /// Entity type.
    pub node_type: String,
    /// Source file path.
    pub file: Option<String>,
    /// Relevance score.
    pub score: f32,
}
