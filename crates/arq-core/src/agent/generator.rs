//! Code generator for agent phase.
//!
//! Uses LLM to generate code that conforms to the plan specification.

use tokio::sync::mpsc;

use crate::llm::{StreamChunk, LLM};
use crate::planning::{FileModification, FileSpec, Plan};

use super::error::AgentError;
use super::executor::ExecutionItem;

/// System prompt for code generation.
const SYSTEM_PROMPT: &str = r#"You are an AI code generator that implements code based on approved specifications.

IMPORTANT RULES:
1. Implement EXACTLY what the specification defines
2. Do NOT add features, comments, or code not in the spec
3. Do NOT explain the code - just output the raw code
4. Use the EXACT types, signatures, and structure specified
5. Follow the language's best practices and idioms
6. Output ONLY the file content - no markdown code blocks, no explanations"#;

/// Code generator that produces code from plan specifications.
pub struct CodeGenerator {
    llm: Box<dyn LLM>,
}

impl CodeGenerator {
    /// Create a new code generator with the given LLM.
    pub fn new(llm: Box<dyn LLM>) -> Self {
        Self { llm }
    }

    /// Generate code for a file to create.
    pub async fn generate_create(
        &self,
        file: &FileSpec,
        plan: &Plan,
        context: Option<&str>,
    ) -> Result<String, AgentError> {
        let prompt = self.build_create_prompt(file, plan, context);
        self.llm
            .complete_with_system(SYSTEM_PROMPT, &prompt)
            .await
            .map_err(AgentError::from)
    }

    /// Generate code for a file to create with streaming.
    pub async fn generate_create_streaming(
        &self,
        file: &FileSpec,
        plan: &Plan,
        context: Option<&str>,
        tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<(), AgentError> {
        let prompt = self.build_create_prompt(file, plan, context);
        self.llm
            .stream_complete(SYSTEM_PROMPT, &prompt, tx)
            .await
            .map_err(AgentError::from)
    }

    /// Generate code for a file to modify.
    pub async fn generate_modify(
        &self,
        file: &FileModification,
        plan: &Plan,
        existing_content: &str,
    ) -> Result<String, AgentError> {
        let prompt = self.build_modify_prompt(file, plan, existing_content);
        self.llm
            .complete_with_system(SYSTEM_PROMPT, &prompt)
            .await
            .map_err(AgentError::from)
    }

    /// Generate code for a file to modify with streaming.
    pub async fn generate_modify_streaming(
        &self,
        file: &FileModification,
        plan: &Plan,
        existing_content: &str,
        tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<(), AgentError> {
        let prompt = self.build_modify_prompt(file, plan, existing_content);
        self.llm
            .stream_complete(SYSTEM_PROMPT, &prompt, tx)
            .await
            .map_err(AgentError::from)
    }

    /// Generate code for an execution item.
    pub async fn generate_for_item(
        &self,
        item: &ExecutionItem,
        plan: &Plan,
        existing_content: Option<&str>,
    ) -> Result<String, AgentError> {
        match item {
            ExecutionItem::Create { path, .. } => {
                // Find the corresponding FileSpec in the plan
                let file = plan
                    .files_to_create
                    .iter()
                    .find(|f| f.path == *path)
                    .ok_or_else(|| AgentError::InvalidPlanItem {
                        index: 0,
                        message: format!("File to create not found in plan: {}", path),
                    })?;
                self.generate_create(file, plan, existing_content).await
            }
            ExecutionItem::Modify { path, .. } => {
                // Find the corresponding FileModification in the plan
                let file = plan
                    .files_to_modify
                    .iter()
                    .find(|f| f.path == *path)
                    .ok_or_else(|| AgentError::InvalidPlanItem {
                        index: 0,
                        message: format!("File to modify not found in plan: {}", path),
                    })?;
                let content = existing_content.unwrap_or("");
                self.generate_modify(file, plan, content).await
            }
        }
    }

    /// Generate code for an execution item with streaming.
    pub async fn generate_for_item_streaming(
        &self,
        item: &ExecutionItem,
        plan: &Plan,
        existing_content: Option<&str>,
        tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<(), AgentError> {
        match item {
            ExecutionItem::Create { path, .. } => {
                let file = plan
                    .files_to_create
                    .iter()
                    .find(|f| f.path == *path)
                    .ok_or_else(|| AgentError::InvalidPlanItem {
                        index: 0,
                        message: format!("File to create not found in plan: {}", path),
                    })?;
                self.generate_create_streaming(file, plan, existing_content, tx)
                    .await
            }
            ExecutionItem::Modify { path, .. } => {
                let file = plan
                    .files_to_modify
                    .iter()
                    .find(|f| f.path == *path)
                    .ok_or_else(|| AgentError::InvalidPlanItem {
                        index: 0,
                        message: format!("File to modify not found in plan: {}", path),
                    })?;
                let content = existing_content.unwrap_or("");
                self.generate_modify_streaming(file, plan, content, tx)
                    .await
            }
        }
    }

    /// Build the prompt for creating a new file.
    fn build_create_prompt(&self, file: &FileSpec, plan: &Plan, context: Option<&str>) -> String {
        let mut prompt = format!(
            r#"TASK: {}
APPROACH: {}

CREATE NEW FILE: {}
DESCRIPTION: {}

SPECIFICATION:
```yaml
path: {}
description: {}
"#,
            plan.task_name, plan.approach, file.path, file.description, file.path, file.description
        );

        // Add exports/functions if present
        if !file.exports.is_empty() {
            prompt.push_str("exports:\n");
            for export in &file.exports {
                prompt.push_str(&format!(
                    "  - name: {}\n    signature: {}\n",
                    export.name, export.signature
                ));
                if !export.behavior.is_empty() {
                    prompt.push_str("    behavior:\n");
                    for behavior in &export.behavior {
                        prompt.push_str(&format!("      - {}\n", behavior));
                    }
                }
            }
        }

        prompt.push_str("```\n");

        // Add context if provided
        if let Some(ctx) = context {
            prompt.push_str(&format!("\nCONTEXT:\n{}\n", ctx));
        }

        prompt.push_str("\nGenerate the complete file content now:");

        prompt
    }

    /// Build the prompt for modifying an existing file.
    fn build_modify_prompt(
        &self,
        file: &FileModification,
        plan: &Plan,
        existing_content: &str,
    ) -> String {
        let line_info = file
            .line
            .map(|l| format!("LINE: ~{}", l))
            .unwrap_or_default();

        let mut prompt = format!(
            r#"TASK: {}
APPROACH: {}

MODIFY FILE: {}
{}
DESCRIPTION: {}

SPECIFICATION:
```yaml
path: {}
description: {}
"#,
            plan.task_name,
            plan.approach,
            file.path,
            line_info,
            file.description,
            file.path,
            file.description
        );

        // Add additions if present
        if !file.additions.is_empty() {
            prompt.push_str("additions:\n");
            for addition in &file.additions {
                prompt.push_str(&format!("  - '{}'\n", addition));
            }
        }

        // Add removals if present
        if !file.removals.is_empty() {
            prompt.push_str("removals:\n");
            for removal in &file.removals {
                prompt.push_str(&format!("  - '{}'\n", removal));
            }
        }

        prompt.push_str("```\n");

        // Add existing content
        prompt.push_str(&format!(
            "\nEXISTING FILE CONTENT:\n```\n{}\n```\n",
            existing_content
        ));

        prompt.push_str(
            "\nApply EXACTLY the changes specified. Output the COMPLETE modified file content:",
        );

        prompt
    }
}
