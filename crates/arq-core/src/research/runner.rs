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
        // Try to parse as JSON
        let parsed: ResearchResponse = serde_json::from_str(response).map_err(|e| {
            ResearchError::ParseError(format!(
                "Failed to parse LLM response as JSON: {}. Response: {}",
                e,
                &response[..response.len().min(500)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_response() {
        let response = r#"{
            "summary": "Test summary",
            "findings": [
                {
                    "title": "Finding 1",
                    "description": "Description 1",
                    "related_files": ["file1.rs"]
                }
            ],
            "dependencies": [
                {
                    "name": "tokio",
                    "description": "Async runtime",
                    "is_external": true
                }
            ],
            "suggested_approach": "Do this thing"
        }"#;

        let context = Context {
            structure: String::new(),
            files: vec![],
        };

        // Create a mock runner just to test parsing
        struct MockLLM;

        #[async_trait::async_trait]
        impl LLM for MockLLM {
            async fn complete(&self, _: &str) -> Result<String, LLMError> {
                Ok(String::new())
            }
            async fn complete_with_system(&self, _: &str, _: &str) -> Result<String, LLMError> {
                Ok(String::new())
            }
        }

        let runner = ResearchRunner::new(MockLLM, ContextBuilder::new("."));
        let doc = runner.parse_response("test-task", response, &context).unwrap();

        assert_eq!(doc.summary, "Test summary");
        assert_eq!(doc.codebase_analysis.len(), 1);
        assert_eq!(doc.dependencies.len(), 1);
        assert_eq!(doc.suggested_approach, "Do this thing");
    }
}
