use thiserror::Error;

use crate::context::{Context, ContextBuilder, ContextError};
use crate::llm::{LLMError, LLM};
use crate::research::document::{Dependency, Finding, ResearchDoc, Source, SourceType};
use crate::research::prompts::{build_research_prompt, RESEARCH_SYSTEM_PROMPT};
use crate::Task;

/// Runs the research phase for a task.
pub struct ResearchRunner<L: LLM> {
    llm: L,
    context_builder: ContextBuilder,
}

impl<L: LLM> ResearchRunner<L> {
    /// Creates a new research runner.
    pub fn new(llm: L, context_builder: ContextBuilder) -> Self {
        Self {
            llm,
            context_builder,
        }
    }

    /// Runs research for the given task.
    pub async fn run(&self, task: &Task) -> Result<ResearchDoc, ResearchError> {
        // 1. Gather context from codebase
        let context = self.context_builder.gather()?;

        // 2. Build prompt
        let context_str = context.to_prompt_string();
        let prompt = build_research_prompt(&task.prompt, &context_str);

        // 3. Call LLM
        let response = self
            .llm
            .complete_with_system(RESEARCH_SYSTEM_PROMPT, &prompt)
            .await?;

        // 4. Parse response into ResearchDoc
        let doc = self.parse_response(&task.name, &response, &context)?;

        Ok(doc)
    }

    /// Parses the LLM response into a ResearchDoc.
    fn parse_response(
        &self,
        task_name: &str,
        response: &str,
        context: &Context,
    ) -> Result<ResearchDoc, ResearchError> {
        // Strip markdown code block if present
        let json_str = extract_json(response);

        // Try to parse as JSON
        let parsed: ResearchResponse = serde_json::from_str(json_str).map_err(|e| {
            ResearchError::ParseError(format!(
                "Failed to parse LLM response as JSON: {}. Response: {}",
                e,
                &json_str[..json_str.len().min(500)]
            ))
        })?;

        // Convert to ResearchDoc
        let mut doc = ResearchDoc::new(task_name);
        doc.summary = parsed.summary;
        doc.suggested_approach = parsed.suggested_approach;

        // Convert findings
        doc.codebase_analysis = parsed
            .findings
            .into_iter()
            .map(|f| Finding {
                title: f.title,
                description: f.description,
                related_files: f.related_files,
            })
            .collect();

        // Convert dependencies
        doc.dependencies = parsed
            .dependencies
            .into_iter()
            .map(|d| Dependency {
                name: d.name,
                description: d.description,
                is_external: d.is_external,
            })
            .collect();

        // Add sources from context files
        doc.sources = context
            .files
            .iter()
            .map(|f| Source {
                source_type: SourceType::File,
                location: f.path.clone(),
            })
            .collect();

        Ok(doc)
    }
}

/// Response structure from LLM.
#[derive(Debug, serde::Deserialize)]
struct ResearchResponse {
    summary: String,
    findings: Vec<FindingResponse>,
    dependencies: Vec<DependencyResponse>,
    suggested_approach: String,
}

#[derive(Debug, serde::Deserialize)]
struct FindingResponse {
    title: String,
    description: String,
    #[serde(default)]
    related_files: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct DependencyResponse {
    name: String,
    description: String,
    is_external: bool,
}

/// Extracts JSON from a response that might be wrapped in markdown code blocks.
fn extract_json(response: &str) -> &str {
    let trimmed = response.trim();

    // Check for ```json ... ``` or ``` ... ```
    if trimmed.starts_with("```") {
        // Find the end of the first line (after ```json or ```)
        if let Some(start) = trimmed.find('\n') {
            let rest = &trimmed[start + 1..];
            // Find the closing ```
            if let Some(end) = rest.rfind("```") {
                return rest[..end].trim();
            }
        }
    }

    trimmed
}

/// Errors that can occur during research.
#[derive(Debug, Error)]
pub enum ResearchError {
    #[error("Context error: {0}")]
    Context(#[from] ContextError),

    #[error("LLM error: {0}")]
    LLM(#[from] LLMError),

    #[error("Parse error: {0}")]
    ParseError(String),
}

