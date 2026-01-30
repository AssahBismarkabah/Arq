//! Local web server for knowledge graph visualization.
//!
//! Provides a browser-based single-page UI using Sigma.js (WebGL renderer)
//! and Graphology (graph data structure) with ForceAtlas2 layout algorithm
//! to explore the indexed codebase structure and relationships.
//!
//! # Module Structure
//!
//! - `handlers` - HTTP route handlers
//! - `models` - API request/response types (DTOs)
//! - `graph` - Graph building logic
//! - `templates` - HTML/CSS/JS template rendering

mod graph;
mod handlers;
mod models;
mod templates;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{routing::get, Router};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

use arq_core::knowledge::KnowledgeGraph;

// =============================================================================
// Application State
// =============================================================================

/// Shared application state for the server.
pub struct AppState {
    /// Knowledge graph instance.
    pub kg: Arc<RwLock<KnowledgeGraph>>,
    /// Path to the project being visualized.
    pub project_path: PathBuf,
}

// =============================================================================
// Server Configuration
// =============================================================================

/// Configuration for the visualization server.
pub struct ServeConfig {
    /// Port to listen on.
    pub port: u16,
    /// Whether to open the browser automatically.
    pub open_browser: bool,
    /// Path to the project directory.
    pub project_path: PathBuf,
    /// Path to the knowledge graph database.
    pub db_path: PathBuf,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            port: 3333,
            open_browser: true,
            project_path: PathBuf::from("."),
            db_path: PathBuf::from(".arq/knowledge"),
        }
    }
}

// =============================================================================
// Server Entry Point
// =============================================================================

/// Start the visualization server.
pub async fn start_server(config: ServeConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize knowledge graph from the database
    let kg = KnowledgeGraph::new(&config.db_path).await?;

    let state = Arc::new(AppState {
        kg: Arc::new(RwLock::new(kg)),
        project_path: config.project_path.clone(),
    });

    // Build router with API endpoints
    let app = Router::new()
        // Main page - Sigma.js graph visualization
        .route("/", get(handlers::index))
        // API endpoints
        .route("/api/graph", get(handlers::api_graph))
        .route("/api/node/{id}", get(handlers::api_node))
        .route("/api/search", get(handlers::api_search))
        // CORS for API access
        .layer(CorsLayer::new().allow_origin(Any))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let url = format!("http://localhost:{}", config.port);

    println!("Starting Arq visualization server...");
    println!("Dashboard: {}", url);
    println!("Press Ctrl+C to stop\n");

    // Open browser if requested
    if config.open_browser {
        if let Err(e) = open::that(&url) {
            eprintln!("Could not open browser: {}", e);
        }
    }

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
