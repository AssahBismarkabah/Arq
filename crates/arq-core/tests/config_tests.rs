use arq_core::config::{
    DEFAULT_ANTHROPIC_MODEL, DEFAULT_DATA_DIR, DEFAULT_LLM_PROVIDER, DEFAULT_MAX_FILE_SIZE,
    DEFAULT_OLLAMA_MODEL, DEFAULT_OPENAI_MODEL,
};
use arq_core::{Config, LLMConfig};

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
    let mut config = LLMConfig {
        provider: "anthropic".to_string(),
        ..Default::default()
    };
    assert_eq!(config.model_or_default(), DEFAULT_ANTHROPIC_MODEL);

    config.provider = "ollama".to_string();
    assert_eq!(config.model_or_default(), DEFAULT_OLLAMA_MODEL);

    config.provider = "openai".to_string();
    assert_eq!(config.model_or_default(), DEFAULT_OPENAI_MODEL);

    config.model = Some("custom-model".to_string());
    assert_eq!(config.model_or_default(), "custom-model");
}
