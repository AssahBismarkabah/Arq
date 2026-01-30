//! HTTP route handlers for the visualization server.
//!
//! This module contains the request handlers for each API endpoint.
//! Handlers are kept thin, delegating business logic to other modules.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::Html,
    Json,
};

use super::graph::GraphBuilder;
use super::models::{GraphData, NodeDetails, SearchQuery, SearchResult};
use super::templates;
use super::AppState;

use arq_core::knowledge::KnowledgeStore; // For search_code method

// =============================================================================
// Page Handlers
// =============================================================================

/// GET `/` - Main page with Sigma.js graph visualization.
pub async fn index(State(state): State<Arc<AppState>>) -> Html<String> {
    Html(templates::render_graph_page(&state.project_path))
}

// =============================================================================
// API Handlers
// =============================================================================

/// GET `/api/graph` - Returns full knowledge graph for Sigma.js/Graphology.
///
/// Response format:
/// ```json
/// {
///   "nodes": [{"key": "fn:...", "attributes": {...}}],
///   "edges": [{"source": "fn:...", "target": "fn:..."}]
/// }
/// ```
pub async fn api_graph(State(state): State<Arc<AppState>>) -> Json<GraphData> {
    let kg = state.kg.read().await;
    let graph_data = GraphBuilder::new().build_from_kg(&kg).await;
    Json(graph_data)
}

/// GET `/api/node/{id}` - Get details for a specific node.
///
/// The node ID format is `type:file:line:name` (e.g., `fn:src/main.rs:42:process`).
pub async fn api_node(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<Option<NodeDetails>> {
    // Parse the node ID to extract type and remaining parts
    let (node_type, rest) = if let Some(rest) = id.strip_prefix("fn:") {
        ("function", rest)
    } else if let Some(rest) = id.strip_prefix("struct:") {
        ("struct", rest)
    } else if let Some(rest) = id.strip_prefix("trait:") {
        ("trait", rest)
    } else if let Some(rest) = id.strip_prefix("enum:") {
        ("enum", rest)
    } else {
        return Json(None);
    };

    // Extract label from the rest (file:line:name -> name)
    let label = rest.rsplit(':').next().unwrap_or(rest);

    Json(Some(NodeDetails {
        key: id.clone(),
        label: label.to_string(),
        node_type: node_type.to_string(),
        file: None,       // TODO: Parse from ID
        start_line: None, // TODO: Parse from ID
        end_line: None,
        dependencies: vec![],
        dependents: vec![],
    }))
}

/// GET `/api/search` - Search for nodes by name or content.
///
/// Query parameters:
/// - `q`: Search query string (required)
/// - `limit`: Maximum results (default: 20)
pub async fn api_search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Json<Vec<SearchResult>> {
    let kg = state.kg.read().await;

    let results = kg
        .search_code(&params.q, params.limit)
        .await
        .unwrap_or_default();

    let search_results: Vec<SearchResult> = results
        .into_iter()
        .map(|r| {
            let node_type = if r.path.ends_with(".rs") {
                "function"
            } else {
                "unknown"
            };

            SearchResult {
                key: format!("chunk:{}:{}", r.path, r.start_line),
                label: r.preview.unwrap_or_else(|| r.path.clone()),
                node_type: node_type.to_string(),
                file: Some(r.path),
                score: r.score,
            }
        })
        .collect();

    Json(search_results)
}
