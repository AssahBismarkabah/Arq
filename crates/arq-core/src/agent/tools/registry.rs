//! Tool registry for managing available tools.

use std::collections::HashMap;
use std::sync::Arc;

use super::{
    EditFileTool, GitAddTool, GitCommitTool, GitDiffTool, GitLogTool, GitStatusTool, ListFilesTool,
    ReadFileTool, RollbackFileTool, RunCommandTool, SearchFilesTool, TaskCompleteTool, Tool,
    ToolDefinition, WriteFileTool,
};

/// Registry of available tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Create a registry with all default tools.
    pub fn with_default_tools() -> Self {
        let mut registry = Self::new();
        // File tools
        registry.register(Arc::new(ReadFileTool));
        registry.register(Arc::new(WriteFileTool));
        registry.register(Arc::new(EditFileTool));
        registry.register(Arc::new(ListFilesTool));
        registry.register(Arc::new(SearchFilesTool));
        registry.register(Arc::new(RollbackFileTool));
        // Command execution
        registry.register(Arc::new(RunCommandTool));
        // Git tools
        registry.register(Arc::new(GitStatusTool));
        registry.register(Arc::new(GitDiffTool));
        registry.register(Arc::new(GitLogTool));
        registry.register(Arc::new(GitAddTool));
        registry.register(Arc::new(GitCommitTool));
        // Task management
        registry.register(Arc::new(TaskCompleteTool));
        registry
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let def = tool.definition();
        self.tools.insert(def.name, tool);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Get all tool definitions.
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// List all tool definitions (alias for definitions).
    pub fn list(&self) -> Vec<ToolDefinition> {
        self.definitions()
    }

    /// Get tool definitions in OpenAI function calling format.
    pub fn to_openai_format(&self) -> Vec<serde_json::Value> {
        self.definitions()
            .iter()
            .map(|def| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": def.name,
                        "description": def.description,
                        "parameters": def.parameters
                    }
                })
            })
            .collect()
    }

    /// Get tool definitions in Anthropic tool use format.
    pub fn to_anthropic_format(&self) -> Vec<serde_json::Value> {
        self.definitions()
            .iter()
            .map(|def| {
                serde_json::json!({
                    "name": def.name,
                    "description": def.description,
                    "input_schema": def.parameters
                })
            })
            .collect()
    }

    /// Get the number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::with_default_tools()
    }
}
