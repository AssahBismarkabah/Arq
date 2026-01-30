//! API edges: relationships that define API contracts and mappings.
//!
//! These edges represent how APIs are defined and connected:
//! - EXPOSES: A exposes B (function exposes endpoint)
//! - MAPS_TO: A maps to B (endpoint maps to handler)
//! - CONSUMES: A consumes B (endpoint consumes schema)
//! - PRODUCES: A produces B (endpoint produces schema)

use serde::{Deserialize, Serialize};

// =============================================================================
// EXPOSES EDGE
// =============================================================================

/// A EXPOSES B: Function/handler A exposes endpoint B.
///
/// Links implementation to API surface.
///
/// Examples:
/// - Handler EXPOSES Endpoint
/// - Resolver EXPOSES GraphQLField
/// - Controller EXPOSES Route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposesEdge {
    /// Source node ID (handler function)
    pub from: String,
    /// Target node ID (endpoint)
    pub to: String,
    /// Framework used (axum, actix, express, etc.)
    pub framework: Option<String>,
    /// Route annotation/decorator
    pub annotation: Option<String>,
    /// Middleware applied
    pub middleware: Vec<String>,
}

impl ExposesEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            framework: None,
            annotation: None,
            middleware: Vec::new(),
        }
    }

    pub fn with_framework(mut self, framework: impl Into<String>) -> Self {
        self.framework = Some(framework.into());
        self
    }

    pub fn with_middleware(mut self, middleware: Vec<String>) -> Self {
        self.middleware = middleware;
        self
    }
}

// =============================================================================
// MAPS_TO EDGE
// =============================================================================

/// A MAPS_TO B: API entity A maps to implementation B.
///
/// Inverse of EXPOSES, for traversing from API to implementation.
///
/// Examples:
/// - Endpoint MAPS_TO Handler
/// - Schema MAPS_TO Struct
/// - Operation MAPS_TO Function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapsToEdge {
    /// Source node ID (API entity)
    pub from: String,
    /// Target node ID (implementation)
    pub to: String,
    /// Mapping type
    pub mapping_type: MappingType,
    /// Transformation applied (if any)
    pub transformation: Option<String>,
}

/// Type of API-to-implementation mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MappingType {
    /// Direct 1:1 mapping
    #[default]
    Direct,
    /// Adapter/wrapper pattern
    Adapter,
    /// Aggregated from multiple sources
    Aggregated,
    /// Generated (e.g., from OpenAPI spec)
    Generated,
}

impl MapsToEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            mapping_type: MappingType::Direct,
            transformation: None,
        }
    }

    pub fn adapter(mut self) -> Self {
        self.mapping_type = MappingType::Adapter;
        self
    }

    pub fn with_transformation(mut self, transform: impl Into<String>) -> Self {
        self.transformation = Some(transform.into());
        self
    }
}

// =============================================================================
// CONSUMES EDGE
// =============================================================================

/// A CONSUMES B: Endpoint A consumes schema B as input.
///
/// Tracks request body contracts.
///
/// Examples:
/// - Endpoint CONSUMES RequestSchema
/// - Operation CONSUMES InputType
/// - Handler CONSUMES DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumesEdge {
    /// Source node ID (endpoint)
    pub from: String,
    /// Target node ID (schema)
    pub to: String,
    /// Content type (application/json, etc.)
    pub content_type: String,
    /// Whether the body is required
    pub required: bool,
    /// Validation applied
    pub validation: Option<String>,
}

impl ConsumesEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            content_type: "application/json".to_string(),
            required: true,
            validation: None,
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = content_type.into();
        self
    }
}

// =============================================================================
// PRODUCES EDGE
// =============================================================================

/// A PRODUCES B: Endpoint A produces schema B as output.
///
/// Tracks response body contracts.
///
/// Examples:
/// - Endpoint PRODUCES ResponseSchema
/// - Operation PRODUCES OutputType
/// - Handler PRODUCES DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProducesEdge {
    /// Source node ID (endpoint)
    pub from: String,
    /// Target node ID (schema)
    pub to: String,
    /// HTTP status code for this response
    pub status_code: u16,
    /// Content type
    pub content_type: String,
    /// Whether this is the primary/success response
    pub is_primary: bool,
    /// Description of when this response is returned
    pub description: Option<String>,
}

impl ProducesEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            status_code: 200,
            content_type: "application/json".to_string(),
            is_primary: true,
            description: None,
        }
    }

    pub fn with_status(mut self, status: u16) -> Self {
        self.status_code = status;
        self
    }

    pub fn error_response(mut self, status: u16) -> Self {
        self.status_code = status;
        self.is_primary = false;
        self
    }
}
