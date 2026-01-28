use thiserror::Error;

/// Errors that can occur during LLM operations.
#[derive(Debug, Error)]
pub enum LLMError {
    #[error("Missing API key. Set the appropriate environment variable for your provider.")]
    MissingApiKey,

    #[error("Missing configuration: {0}")]
    MissingConfig(String),

    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("API returned error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Rate limited. Try again later.")]
    RateLimited,

    #[error("Network error: {0}")]
    Network(String),

    #[error("Unknown provider: {0}")]
    UnknownProvider(String),
}

impl From<reqwest::Error> for LLMError {
    fn from(err: reqwest::Error) -> Self {
        LLMError::Network(err.to_string())
    }
}
