use super::{ClaudeClient, LLMError, OpenAIClient, LLM};
use crate::config::{
    LLMConfig, DEFAULT_ANTHROPIC_MODEL, DEFAULT_OLLAMA_MODEL, DEFAULT_OLLAMA_URL,
    DEFAULT_OPENAI_MODEL, DEFAULT_OPENAI_URL,
};

/// LLM Provider configuration.
#[derive(Debug, Clone)]
pub enum Provider {
    /// OpenAI-compatible endpoint (default, most universal)
    OpenAI {
        base_url: Option<String>,
        api_key: Option<String>,
        model: Option<String>,
    },
    /// Anthropic Claude
    Anthropic {
        api_key: Option<String>,
        model: Option<String>,
    },
    /// Local Ollama instance
    Ollama {
        base_url: Option<String>,
        model: String,
    },
}

impl Default for Provider {
    fn default() -> Self {
        Provider::OpenAI {
            base_url: None,
            api_key: None,
            model: None,
        }
    }
}

impl Provider {
    /// Creates a provider from LLMConfig.
    pub fn from_config(config: &LLMConfig) -> Self {
        match config.provider.as_str() {
            "anthropic" | "claude" => Provider::Anthropic {
                api_key: config.api_key.clone(),
                model: config.model.clone(),
            },
            "ollama" => Provider::Ollama {
                base_url: config.base_url.clone(),
                model: config
                    .model
                    .clone()
                    .unwrap_or_else(|| DEFAULT_OLLAMA_MODEL.to_string()),
            },
            _ => Provider::OpenAI {
                base_url: config.base_url.clone(),
                api_key: config.api_key.clone(),
                model: config.model.clone(),
            },
        }
    }

    /// Creates an LLM client from the provider configuration.
    pub fn build(self) -> Result<Box<dyn LLM>, LLMError> {
        match self {
            Provider::OpenAI {
                base_url,
                api_key,
                model,
            } => {
                let base = base_url
                    .or_else(|| std::env::var("ARQ_LLM_BASE_URL").ok())
                    .or_else(|| std::env::var("OPENAI_BASE_URL").ok())
                    .unwrap_or_else(|| DEFAULT_OPENAI_URL.to_string());

                let key = api_key
                    .or_else(|| std::env::var("ARQ_LLM_API_KEY").ok())
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                    .unwrap_or_default();

                let mdl = model
                    .or_else(|| std::env::var("ARQ_LLM_MODEL").ok())
                    .or_else(|| std::env::var("OPENAI_MODEL").ok())
                    .unwrap_or_else(|| DEFAULT_OPENAI_MODEL.to_string());

                Ok(Box::new(OpenAIClient::new(base, key, mdl)))
            }

            Provider::Anthropic { api_key, model } => {
                let key = api_key
                    .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                    .ok_or(LLMError::MissingApiKey)?;

                let mdl = model
                    .or_else(|| std::env::var("ANTHROPIC_MODEL").ok())
                    .unwrap_or_else(|| DEFAULT_ANTHROPIC_MODEL.to_string());

                Ok(Box::new(ClaudeClient::new(key).with_model(mdl)))
            }

            Provider::Ollama { base_url, model } => {
                let base = base_url
                    .or_else(|| std::env::var("OLLAMA_HOST").ok())
                    .map(|h| format!("{}/v1", h.trim_end_matches('/')))
                    .unwrap_or_else(|| DEFAULT_OLLAMA_URL.to_string());

                Ok(Box::new(OpenAIClient::new(base, "", model)))
            }
        }
    }

    /// Auto-detect provider from environment variables.
    ///
    /// Detection order:
    /// 1. ARQ_LLM_PROVIDER explicitly set
    /// 2. ARQ_LLM_BASE_URL set → OpenAI-compatible
    /// 3. ANTHROPIC_API_KEY set → Anthropic
    /// 4. OPENAI_API_KEY set → OpenAI
    /// 5. OLLAMA_HOST set → Ollama
    /// 6. Default to OpenAI-compatible (works with local servers too)
    pub fn from_env() -> Result<Box<dyn LLM>, LLMError> {
        // Check for explicit provider setting
        if let Ok(provider) = std::env::var("ARQ_LLM_PROVIDER") {
            return match provider.to_lowercase().as_str() {
                "openai" => Provider::OpenAI {
                    base_url: None,
                    api_key: None,
                    model: None,
                }
                .build(),
                "anthropic" | "claude" => Provider::Anthropic {
                    api_key: None,
                    model: None,
                }
                .build(),
                "ollama" => {
                    let model = std::env::var("ARQ_LLM_MODEL")
                        .or_else(|_| std::env::var("OLLAMA_MODEL"))
                        .unwrap_or_else(|_| DEFAULT_OLLAMA_MODEL.to_string());
                    Provider::Ollama {
                        base_url: None,
                        model,
                    }
                    .build()
                }
                other => Err(LLMError::UnknownProvider(other.to_string())),
            };
        }

        // Auto-detect based on available env vars
        if std::env::var("ARQ_LLM_BASE_URL").is_ok() {
            return Provider::OpenAI {
                base_url: None,
                api_key: None,
                model: None,
            }
            .build();
        }

        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            return Provider::Anthropic {
                api_key: None,
                model: None,
            }
            .build();
        }

        if std::env::var("OPENAI_API_KEY").is_ok() {
            return Provider::OpenAI {
                base_url: None,
                api_key: None,
                model: None,
            }
            .build();
        }

        if std::env::var("OLLAMA_HOST").is_ok() {
            let model = std::env::var("ARQ_LLM_MODEL")
                .or_else(|_| std::env::var("OLLAMA_MODEL"))
                .unwrap_or_else(|_| DEFAULT_OLLAMA_MODEL.to_string());
            return Provider::Ollama {
                base_url: None,
                model,
            }
            .build();
        }

        // Default to OpenAI-compatible (might work with local server)
        Provider::OpenAI {
            base_url: None,
            api_key: None,
            model: None,
        }
        .build()
    }
}
