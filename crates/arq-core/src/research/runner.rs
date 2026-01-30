use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;

use crate::context::{ContextBuilder, ContextError};
use crate::knowledge::{KnowledgeError, KnowledgeStore, SearchResult};
use crate::llm::{LLMError, LLM, StreamChunk};
use crate::research::document::{Dependency, Finding, ResearchDoc, Source, SourceType};
use crate::research::prompts::{build_research_prompt, RESEARCH_SYSTEM_PROMPT};
use crate::Task;

/// Progress events during research.
#[derive(Debug, Clone)]
pub enum ResearchProgress {
    /// Research has started
    Started,
    /// Gathering context from codebase
    GatheringContext,
    /// Searching knowledge graph
    SearchingKnowledgeGraph,
    /// Found results from knowledge graph
    KnowledgeGraphResults { count: usize },
    /// Calling LLM for analysis
    CallingLLM,
    /// Parsing the LLM response
    ParsingResponse,
    /// Research completed successfully
    Complete,
    /// An error occurred
    Error(String),
}

/// Runs the research phase for a task.
pub struct ResearchRunner<L: LLM> {
    llm: L,
    context_builder: ContextBuilder,
    knowledge_store: Option<Arc<dyn KnowledgeStore>>,
}

impl<L: LLM> ResearchRunner<L> {
    /// Creates a new research runner.
    pub fn new(llm: L, context_builder: ContextBuilder) -> Self {
        Self {
            llm,
            context_builder,
            knowledge_store: None,
        }
    }

    /// Creates a new research runner with a knowledge store for semantic search.
    pub fn with_knowledge_store(
        llm: L,
        context_builder: ContextBuilder,
        knowledge_store: Arc<dyn KnowledgeStore>,
    ) -> Self {
        Self {
            llm,
            context_builder,
            knowledge_store: Some(knowledge_store),
        }
    }

    /// Runs research for the given task.
    pub async fn run(&self, task: &Task) -> Result<ResearchDoc, ResearchError> {
        // 1. Gather context - use knowledge graph if available, otherwise fall back to file scan
        let (context_str, sources) = if let Some(ref kg) = self.knowledge_store {
            self.gather_smart_context(kg, &task.prompt).await?
        } else {
            let context = self.context_builder.gather()?;
            let sources: Vec<Source> = context
                .files
                .iter()
                .map(|f| Source {
                    source_type: SourceType::File,
                    location: f.path.clone(),
                })
                .collect();
            (context.to_prompt_string(), sources)
        };

        // 2. Build prompt
        let prompt = build_research_prompt(&task.prompt, &context_str);

        // 3. Call LLM
        let response = self
            .llm
            .complete_with_system(RESEARCH_SYSTEM_PROMPT, &prompt)
            .await?;

        // 4. Parse response into ResearchDoc
        let doc = self.parse_response(&task.name, &response, sources)?;

        Ok(doc)
    }

    /// Runs research with progress callbacks.
    ///
    /// Sends progress updates through the provided channel as research proceeds.
    pub async fn run_with_progress(
        &self,
        task: &Task,
        progress_tx: mpsc::UnboundedSender<ResearchProgress>,
    ) -> Result<ResearchDoc, ResearchError> {
        let _ = progress_tx.send(ResearchProgress::Started);

        // 1. Gather context
        let (context_str, sources) = if let Some(ref kg) = self.knowledge_store {
            let _ = progress_tx.send(ResearchProgress::SearchingKnowledgeGraph);
            let result = self.gather_smart_context(kg, &task.prompt).await?;
            // Count sources for progress
            let count = result.1.len();
            let _ = progress_tx.send(ResearchProgress::KnowledgeGraphResults { count });
            result
        } else {
            let _ = progress_tx.send(ResearchProgress::GatheringContext);
            let context = self.context_builder.gather()?;
            let sources: Vec<Source> = context
                .files
                .iter()
                .map(|f| Source {
                    source_type: SourceType::File,
                    location: f.path.clone(),
                })
                .collect();
            (context.to_prompt_string(), sources)
        };

        // 2. Build prompt
        let prompt = build_research_prompt(&task.prompt, &context_str);

        // 3. Call LLM
        let _ = progress_tx.send(ResearchProgress::CallingLLM);
        let response = self
            .llm
            .complete_with_system(RESEARCH_SYSTEM_PROMPT, &prompt)
            .await?;

        // 4. Parse response
        let _ = progress_tx.send(ResearchProgress::ParsingResponse);
        let doc = self.parse_response(&task.name, &response, sources)?;

        let _ = progress_tx.send(ResearchProgress::Complete);
        Ok(doc)
    }

    /// Runs research with streaming LLM output and progress callbacks.
    ///
    /// This method streams LLM tokens as they arrive while also sending progress updates.
    pub async fn run_streaming(
        &self,
        task: &Task,
        progress_tx: mpsc::UnboundedSender<ResearchProgress>,
        stream_tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<ResearchDoc, ResearchError> {
        let _ = progress_tx.send(ResearchProgress::Started);

        // 1. Gather context
        let (context_str, sources) = if let Some(ref kg) = self.knowledge_store {
            let _ = progress_tx.send(ResearchProgress::SearchingKnowledgeGraph);
            let result = self.gather_smart_context(kg, &task.prompt).await?;
            let count = result.1.len();
            let _ = progress_tx.send(ResearchProgress::KnowledgeGraphResults { count });
            result
        } else {
            let _ = progress_tx.send(ResearchProgress::GatheringContext);
            let context = self.context_builder.gather()?;
            let sources: Vec<Source> = context
                .files
                .iter()
                .map(|f| Source {
                    source_type: SourceType::File,
                    location: f.path.clone(),
                })
                .collect();
            (context.to_prompt_string(), sources)
        };

        // 2. Build prompt
        let prompt = build_research_prompt(&task.prompt, &context_str);

        // 3. Stream LLM response
        let _ = progress_tx.send(ResearchProgress::CallingLLM);

        // Collect streamed response
        let response = if self.llm.supports_streaming() {
            // Use streaming - collect chunks while forwarding to stream_tx
            let (collector_tx, mut collector_rx) = mpsc::unbounded_channel::<StreamChunk>();

            // Spawn task to forward chunks and collect full response
            let stream_tx_clone = stream_tx.clone();
            let collect_handle = tokio::spawn(async move {
                let mut full_response = String::new();
                while let Some(chunk) = collector_rx.recv().await {
                    if !chunk.is_final {
                        full_response.push_str(&chunk.text);
                    }
                    // Forward to TUI
                    let _ = stream_tx_clone.send(chunk);
                }
                full_response
            });

            // Start streaming
            self.llm
                .stream_complete(RESEARCH_SYSTEM_PROMPT, &prompt, collector_tx)
                .await?;

            // Wait for collection to complete
            collect_handle.await.unwrap_or_default()
        } else {
            // Non-streaming fallback
            let response = self
                .llm
                .complete_with_system(RESEARCH_SYSTEM_PROMPT, &prompt)
                .await?;
            // Send as single chunk
            let _ = stream_tx.send(StreamChunk::text(response.clone()));
            let _ = stream_tx.send(StreamChunk::done());
            response
        };

        // 4. Parse response
        let _ = progress_tx.send(ResearchProgress::ParsingResponse);
        let doc = self.parse_response(&task.name, &response, sources)?;

        let _ = progress_tx.send(ResearchProgress::Complete);
        Ok(doc)
    }

    /// Gathers smart context using the knowledge graph.
    ///
    /// This method:
    /// 1. Performs semantic search to find relevant code
    /// 2. Expands results using graph traversal (dependencies & impact)
    /// 3. Builds rich context showing code AND its connections
    async fn gather_smart_context(
        &self,
        kg: &Arc<dyn KnowledgeStore>,
        query: &str,
    ) -> Result<(String, Vec<Source>), ResearchError> {
        // 1. Semantic search to find relevant code chunks
        let results: Vec<SearchResult> = kg.search_code(query, 15).await?;

        if results.is_empty() {
            // Fall back to regular context gathering if no results
            let context = self.context_builder.gather()?;
            let sources: Vec<Source> = context
                .files
                .iter()
                .map(|f| Source {
                    source_type: SourceType::File,
                    location: f.path.clone(),
                })
                .collect();
            return Ok((context.to_prompt_string(), sources));
        }

        let mut context_parts = Vec::new();
        let mut sources = Vec::new();
        let mut seen_files = std::collections::HashSet::new();
        let mut graph_context = Vec::new();

        // 2. Process search results and gather graph connections
        for result in &results {
            // Track source files
            if !seen_files.contains(&result.path) {
                seen_files.insert(result.path.clone());
                sources.push(Source {
                    source_type: SourceType::KnowledgeGraph,
                    location: format!(
                        "{}:{}-{} (score: {:.2})",
                        result.path, result.start_line, result.end_line, result.score
                    ),
                });
            }

            // Add code preview
            if let Some(ref preview) = result.preview {
                context_parts.push(format!(
                    "### {} (lines {}-{})\n```\n{}\n```",
                    result.path, result.start_line, result.end_line, preview
                ));
            }

            // 3. Graph expansion - get dependencies and impact for entities
            if let Some(ref entity_id) = result.entity_id {
                let entity_name = &result.entity_type;

                // Get what this entity depends on (calls)
                if let Ok(deps) = kg.get_dependencies(entity_id).await {
                    if !deps.is_empty() {
                        graph_context.push(format!(
                            "- **{}** `{}` calls: {}",
                            entity_name,
                            entity_id,
                            deps.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
                        ));
                    }
                }

                // Get what depends on this entity (callers / impact)
                if let Ok(impact) = kg.get_impact(entity_id).await {
                    if !impact.is_empty() {
                        graph_context.push(format!(
                            "- **{}** `{}` is called by: {}",
                            entity_name,
                            entity_id,
                            impact.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
                        ));
                    }
                }
            }
        }

        // 4. Build final context string
        let mut context_str = format!(
            "## Relevant Code (semantic search)\n\n{}\n",
            context_parts.join("\n\n")
        );

        // Add graph relationships if found
        if !graph_context.is_empty() {
            context_str.push_str(&format!(
                "\n## Code Relationships (graph analysis)\n\n{}\n",
                graph_context.join("\n")
            ));
        }

        Ok((context_str, sources))
    }

    /// Parses the LLM response into a ResearchDoc.
    fn parse_response(
        &self,
        task_name: &str,
        response: &str,
        sources: Vec<Source>,
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

        // Use provided sources
        doc.sources = sources;

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

/// Extracts JSON from a response that might be wrapped in markdown code blocks or have extra text.
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

    // Look for JSON object by finding first { and last }
    // This handles cases where LLM adds text before/after JSON
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                return &trimmed[start..=end];
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

    #[error("Knowledge graph error: {0}")]
    Knowledge(#[from] KnowledgeError),

    #[error("Parse error: {0}")]
    ParseError(String),
}

