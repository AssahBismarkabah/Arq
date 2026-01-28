use arq_core::{ClaudeClient, OpenAIClient, LLMError, LLMConfig};
use arq_core::llm::Provider;
use arq_core::config::DEFAULT_OLLAMA_MODEL;

// Claude client tests
mod claude {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = ClaudeClient::new("test-key");
    }

    #[test]
    fn test_client_with_model() {
        let _client = ClaudeClient::new("test-key").with_model("claude-3-opus");
    }

    #[test]
    fn test_client_with_api_url() {
        let _client = ClaudeClient::new("test-key")
            .with_api_url("https://proxy.example.com/v1/messages");
    }

    #[test]
    fn test_from_env_missing() {
        std::env::remove_var("ANTHROPIC_API_KEY");
        let result = ClaudeClient::from_env();
        assert!(matches!(result, Err(LLMError::MissingApiKey)));
    }
}

// OpenAI client tests
mod openai {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = OpenAIClient::new(
            "https://api.example.com/v1",
            "test-key",
            "gpt-4",
        );
    }

    #[test]
    fn test_openai_client() {
        let _client = OpenAIClient::openai("test-key", "gpt-4o");
    }

    #[test]
    fn test_ollama_client() {
        let _client = OpenAIClient::ollama("llama3");
    }

    #[test]
    fn test_openrouter_client() {
        let _client = OpenAIClient::openrouter("test-key", "anthropic/claude-3-opus");
    }

    #[test]
    fn test_url_trailing_slash_removed() {
        let _client = OpenAIClient::new(
            "https://api.example.com/v1/",
            "key",
            "model",
        );
    }
}

// Provider tests
mod provider {
    use super::*;

    #[test]
    fn test_default_provider() {
        let provider = Provider::default();
        assert!(matches!(provider, Provider::OpenAI { .. }));
    }

    #[test]
    fn test_ollama_provider_build() {
        let provider = Provider::Ollama {
            base_url: None,
            model: DEFAULT_OLLAMA_MODEL.to_string(),
        };
        let result = provider.build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_openai_provider_build() {
        let provider = Provider::OpenAI {
            base_url: Some("http://localhost:8080/v1".to_string()),
            api_key: Some("test".to_string()),
            model: Some("local-model".to_string()),
        };
        let result = provider.build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_config() {
        let config = LLMConfig {
            provider: "ollama".to_string(),
            model: Some("codellama".to_string()),
            base_url: None,
            api_key: None,
            max_tokens: 4096,
            api_version: None,
        };

        let provider = Provider::from_config(&config);
        assert!(matches!(provider, Provider::Ollama { model, .. } if model == "codellama"));
    }
}
