//! API entity nodes: Endpoints, Schemas, Operations.
//!
//! These represent API contracts and data transfer objects.

use serde::{Deserialize, Serialize};

// =============================================================================
// ENDPOINT ENTITY
// =============================================================================

/// An API endpoint (REST, GraphQL, gRPC).
///
/// Represents a single endpoint that can be called externally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointEntity {
    /// Unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Endpoint path (e.g., "/api/v1/users/{id}")
    pub path: String,

    /// HTTP method or equivalent (GET, POST, QUERY, MUTATION)
    pub method: HttpMethod,

    /// Handler function that implements this endpoint
    pub handler: String,

    /// File containing the endpoint definition
    pub file_path: String,

    /// Line number where endpoint is defined
    pub line: u32,

    /// API type
    pub api_type: ApiType,

    /// Request body schema
    pub request_schema: Option<String>,

    /// Response schema
    pub response_schema: Option<String>,

    /// Path parameters
    pub path_params: Vec<ApiParam>,

    /// Query parameters
    pub query_params: Vec<ApiParam>,

    /// Required headers
    pub headers: Vec<ApiParam>,

    /// Authentication requirements
    pub auth: Option<AuthRequirement>,

    /// Tags/categories for grouping
    pub tags: Vec<String>,

    /// Documentation/description
    pub description: Option<String>,

    /// Deprecation info
    pub deprecated: Option<DeprecationInfo>,
}

/// HTTP method or API operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    // GraphQL
    Query,
    Mutation,
    Subscription,
    // gRPC
    Unary,
    ServerStream,
    ClientStream,
    BidiStream,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
            Self::Query => "QUERY",
            Self::Mutation => "MUTATION",
            Self::Subscription => "SUBSCRIPTION",
            Self::Unary => "UNARY",
            Self::ServerStream => "SERVER_STREAM",
            Self::ClientStream => "CLIENT_STREAM",
            Self::BidiStream => "BIDI_STREAM",
        }
    }
}

/// Type of API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiType {
    Rest,
    GraphQL,
    Grpc,
    WebSocket,
    Custom,
}

/// An API parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiParam {
    pub name: String,
    pub type_name: String,
    pub required: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
    pub validation: Option<String>,
}

/// Authentication requirement for an endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequirement {
    pub auth_type: AuthType,
    pub scopes: Vec<String>,
    pub optional: bool,
}

/// Type of authentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    None,
    Bearer,
    Basic,
    ApiKey,
    OAuth2,
    Custom,
}

/// Deprecation information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecationInfo {
    pub since: Option<String>,
    pub replacement: Option<String>,
    pub message: Option<String>,
}

// =============================================================================
// SCHEMA ENTITY
// =============================================================================

/// A data schema (DTO, request/response body, etc.).
///
/// Represents structured data that flows through APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEntity {
    /// Unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Schema name
    pub name: String,

    /// Full qualified name
    pub qualified_name: String,

    /// File containing this schema
    pub file_path: String,

    /// Start line number
    pub start_line: u32,

    /// End line number
    pub end_line: u32,

    /// Schema type
    pub schema_type: SchemaType,

    /// Fields/properties
    pub fields: Vec<SchemaField>,

    /// Source struct/type this maps to
    pub source_type: Option<String>,

    /// Validation rules
    pub validations: Vec<String>,

    /// Serialization format (JSON, XML, Protobuf)
    pub format: SerializationFormat,

    /// Documentation
    pub description: Option<String>,

    /// Example value
    pub example: Option<String>,
}

/// Type of schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaType {
    /// Request body
    Request,
    /// Response body
    Response,
    /// Shared DTO
    Dto,
    /// Event payload
    Event,
    /// Database entity
    Entity,
}

/// A field in a schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaField {
    pub name: String,
    pub type_name: String,
    pub required: bool,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
    pub validation: Option<String>,
    /// JSON/XML property name if different
    pub serialized_name: Option<String>,
}

/// Serialization format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SerializationFormat {
    #[default]
    Json,
    Xml,
    Protobuf,
    MessagePack,
    Yaml,
    Custom,
}

// =============================================================================
// OPERATION ENTITY
// =============================================================================

/// An API operation (OpenAPI operation, GraphQL resolver).
///
/// Higher-level grouping of related endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationEntity {
    /// Unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Operation name (e.g., "createUser", "getOrderById")
    pub name: String,

    /// Operation ID (OpenAPI operationId)
    pub operation_id: String,

    /// File containing this operation
    pub file_path: String,

    /// Line number
    pub line: u32,

    /// HTTP method
    pub method: HttpMethod,

    /// Path
    pub path: String,

    /// Summary
    pub summary: Option<String>,

    /// Full description
    pub description: Option<String>,

    /// Tags
    pub tags: Vec<String>,

    /// Request body reference
    pub request_body: Option<String>,

    /// Response references by status code
    pub responses: Vec<OperationResponse>,
}

/// A response for an operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResponse {
    pub status_code: u16,
    pub description: Option<String>,
    pub schema: Option<String>,
}
