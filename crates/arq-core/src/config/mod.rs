//! Configuration management for Arq.
//!
//! Configuration is loaded from multiple sources with the following priority:
//! 1. Environment variables (highest priority)
//! 2. Project-local `arq.toml` file
//! 3. User config `~/.config/arq/config.toml`
//! 4. Built-in defaults (lowest priority)

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

mod defaults;

pub use defaults::*;

/// Configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

/// Main configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Context gathering configuration.
    pub context: ContextConfig,

    /// LLM provider configuration.
    pub llm: LLMConfig,

    /// Storage configuration.
    pub storage: StorageConfig,

    /// Research phase configuration.
    pub research: ResearchConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            context: ContextConfig::default(),
            llm: LLMConfig::default(),
            storage: StorageConfig::default(),
            research: ResearchConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from default locations.
    ///
    /// Searches for config in order:
    /// 1. `./arq.toml` (project local)
    /// 2. `~/.config/arq/config.toml` (user config)
    /// 3. Falls back to defaults
    pub fn load() -> Result<Self, ConfigError> {
        // Try project-local config first
        if Path::new("arq.toml").exists() {
            return Self::from_file("arq.toml");
        }

        // Try user config
        if let Some(config_dir) = dirs::config_dir() {
            let user_config = config_dir.join("arq").join("config.toml");
            if user_config.exists() {
                return Self::from_file(&user_config);
            }
        }

        // Use defaults
        Ok(Self::default())
    }

    /// Load configuration from a specific file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let mut config: Config = toml::from_str(&content)?;

        // Apply environment variable overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply environment variable overrides.
    fn apply_env_overrides(&mut self) {
        // LLM overrides
        if let Ok(provider) = std::env::var("ARQ_LLM_PROVIDER") {
            self.llm.provider = provider;
        }
        if let Ok(model) = std::env::var("ARQ_LLM_MODEL") {
            self.llm.model = Some(model);
        }
        if let Ok(url) = std::env::var("ARQ_LLM_BASE_URL") {
            self.llm.base_url = Some(url);
        }
        if let Ok(key) = std::env::var("ARQ_LLM_API_KEY") {
            self.llm.api_key = Some(key);
        }
        if let Ok(tokens) = std::env::var("ARQ_LLM_MAX_TOKENS") {
            if let Ok(n) = tokens.parse() {
                self.llm.max_tokens = n;
            }
        }

        // Context overrides
        if let Ok(size) = std::env::var("ARQ_MAX_FILE_SIZE") {
            if let Ok(n) = size.parse() {
                self.context.max_file_size = n;
            }
        }
        if let Ok(size) = std::env::var("ARQ_MAX_TOTAL_SIZE") {
            if let Ok(n) = size.parse() {
                self.context.max_total_size = n;
            }
        }

        // Storage overrides
        if let Ok(dir) = std::env::var("ARQ_DATA_DIR") {
            self.storage.data_dir = dir;
        }
    }

    /// Create a default config file content as a string.
    pub fn default_config_string() -> String {
        let config = Config::default();
        toml::to_string_pretty(&config).unwrap_or_default()
    }
}

/// Context gathering configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ContextConfig {
    /// Maximum size of a single file to include (in bytes).
    pub max_file_size: u64,

    /// Maximum total context size (in bytes).
    pub max_total_size: u64,

    /// File extensions to include (without leading dot).
    pub include_extensions: Vec<String>,

    /// Directories to exclude from scanning.
    pub exclude_dirs: Vec<String>,

    /// Additional file patterns to exclude (glob patterns).
    pub exclude_patterns: Vec<String>,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            max_total_size: DEFAULT_MAX_TOTAL_SIZE,
            include_extensions: DEFAULT_EXTENSIONS.iter().map(|s| s.to_string()).collect(),
            exclude_dirs: DEFAULT_EXCLUDE_DIRS.iter().map(|s| s.to_string()).collect(),
            exclude_patterns: DEFAULT_EXCLUDE_PATTERNS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LLMConfig {
    /// Provider name: "openai", "anthropic", "ollama", or "openai-compatible".
    pub provider: String,

    /// Model name (provider-specific).
    pub model: Option<String>,

    /// Base URL for API (for openai-compatible providers).
    pub base_url: Option<String>,

    /// API key (can also be set via environment variable).
    #[serde(skip_serializing)]
    pub api_key: Option<String>,

    /// Maximum tokens for response.
    pub max_tokens: u32,

    /// API version (for Anthropic).
    pub api_version: Option<String>,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: DEFAULT_LLM_PROVIDER.to_string(),
            model: None, // Use provider default
            base_url: None, // Use provider default
            api_key: None, // Load from env
            max_tokens: DEFAULT_MAX_TOKENS,
            api_version: Some(DEFAULT_ANTHROPIC_API_VERSION.to_string()),
        }
    }
}

impl LLMConfig {
    /// Get the model name, falling back to provider defaults.
    pub fn model_or_default(&self) -> String {
        self.model.clone().unwrap_or_else(|| {
            match self.provider.as_str() {
                "anthropic" | "claude" => DEFAULT_ANTHROPIC_MODEL.to_string(),
                "ollama" => DEFAULT_OLLAMA_MODEL.to_string(),
                _ => DEFAULT_OPENAI_MODEL.to_string(),
            }
        })
    }

    /// Get the base URL, falling back to provider defaults.
    pub fn base_url_or_default(&self) -> String {
        self.base_url.clone().unwrap_or_else(|| {
            match self.provider.as_str() {
                "anthropic" | "claude" => DEFAULT_ANTHROPIC_URL.to_string(),
                "ollama" => DEFAULT_OLLAMA_URL.to_string(),
                "openrouter" => DEFAULT_OPENROUTER_URL.to_string(),
                _ => DEFAULT_OPENAI_URL.to_string(),
            }
        })
    }

    /// Get API key from config or environment.
    pub fn api_key_or_env(&self) -> Option<String> {
        self.api_key.clone()
            .or_else(|| std::env::var("ARQ_LLM_API_KEY").ok())
            .or_else(|| match self.provider.as_str() {
                "anthropic" | "claude" => std::env::var("ANTHROPIC_API_KEY").ok(),
                "openai" => std::env::var("OPENAI_API_KEY").ok(),
                "openrouter" => std::env::var("OPENROUTER_API_KEY").ok(),
                _ => std::env::var("OPENAI_API_KEY").ok(),
            })
    }
}

/// Storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    /// Base directory for arq data (default: ".arq").
    pub data_dir: String,

    /// Task subdirectory name.
    pub tasks_dir: String,

    /// Task file name.
    pub task_file: String,

    /// Research document file name.
    pub research_file: String,

    /// Plan file name.
    pub plan_file: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: DEFAULT_DATA_DIR.to_string(),
            tasks_dir: DEFAULT_TASKS_DIR.to_string(),
            task_file: DEFAULT_TASK_FILE.to_string(),
            research_file: DEFAULT_RESEARCH_FILE.to_string(),
            plan_file: DEFAULT_PLAN_FILE.to_string(),
        }
    }
}

impl StorageConfig {
    /// Get the full path to the tasks directory.
    pub fn tasks_path(&self) -> PathBuf {
        PathBuf::from(&self.data_dir).join(&self.tasks_dir)
    }

    /// Get the full path to a task directory.
    pub fn task_path(&self, task_id: &str) -> PathBuf {
        self.tasks_path().join(task_id)
    }
}

/// Research phase configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ResearchConfig {
    /// System prompt for research phase.
    /// If not set, uses the built-in default.
    pub system_prompt: Option<String>,

    /// Maximum length of error context in messages.
    pub error_context_length: usize,
}

impl Default for ResearchConfig {
    fn default() -> Self {
        Self {
            system_prompt: None, // Use built-in default
            error_context_length: DEFAULT_ERROR_CONTEXT_LENGTH,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.context.max_file_size, DEFAULT_MAX_FILE_SIZE);
        assert_eq!(config.llm.provider, DEFAULT_LLM_PROVIDER);
        assert_eq!(config.storage.data_dir, DEFAULT_DATA_DIR);
    }

    #[test]
    fn test_config_to_toml() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[context]"));
        assert!(toml_str.contains("[llm]"));
        assert!(toml_str.contains("[storage]"));
    }

    #[test]
    fn test_config_from_toml() {
        let toml_str = r#"
[context]
max_file_size = 200000

[llm]
provider = "ollama"
model = "llama3"

[storage]
data_dir = ".custom-arq"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.context.max_file_size, 200000);
        assert_eq!(config.llm.provider, "ollama");
        assert_eq!(config.llm.model, Some("llama3".to_string()));
        assert_eq!(config.storage.data_dir, ".custom-arq");
    }

    #[test]
    fn test_model_or_default() {
        let mut config = LLMConfig::default();

        config.provider = "anthropic".to_string();
        assert_eq!(config.model_or_default(), DEFAULT_ANTHROPIC_MODEL);

        config.provider = "ollama".to_string();
        assert_eq!(config.model_or_default(), DEFAULT_OLLAMA_MODEL);

        config.provider = "openai".to_string();
        assert_eq!(config.model_or_default(), DEFAULT_OPENAI_MODEL);

        config.model = Some("custom-model".to_string());
        assert_eq!(config.model_or_default(), "custom-model");
    }
}
