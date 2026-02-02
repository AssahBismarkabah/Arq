use tokio::sync::mpsc;

use crate::llm::{StreamChunk, LLM};
use crate::research::ResearchDoc;

use super::approach::{Approach, ApproachOptions};
use super::error::PlanningError;
use super::plan::{Complexity, FileModification, FileSpec, FunctionSignature, Plan};
use super::progress::PlanningProgress;
use super::prompts::{
    build_approaches_prompt, build_spec_prompt, APPROACHES_SYSTEM_PROMPT, SPEC_SYSTEM_PROMPT,
};

/// Runs the planning phase for a task.
///
/// The planning phase takes a ResearchDoc as input and produces:
/// 1. Multiple implementation approaches (via `generate_approaches`)
/// 2. A detailed specification/plan (via `generate_plan`)
pub struct PlanningRunner<L: LLM> {
    llm: L,
}

impl<L: LLM> PlanningRunner<L> {
    /// Creates a new planning runner.
    pub fn new(llm: L) -> Self {
        Self { llm }
    }

    /// Generates implementation approaches from research.
    pub async fn generate_approaches(
        &self,
        research: &ResearchDoc,
    ) -> Result<ApproachOptions, PlanningError> {
        let prompt = build_approaches_prompt(research);

        let response = self
            .llm
            .complete_with_system(APPROACHES_SYSTEM_PROMPT, &prompt)
            .await?;

        self.parse_approaches(&response)
    }

    /// Generates approaches with progress callbacks.
    pub async fn generate_approaches_with_progress(
        &self,
        research: &ResearchDoc,
        progress_tx: mpsc::UnboundedSender<PlanningProgress>,
    ) -> Result<ApproachOptions, PlanningError> {
        let _ = progress_tx.send(PlanningProgress::Started);
        let _ = progress_tx.send(PlanningProgress::LoadingResearch);

        let prompt = build_approaches_prompt(research);

        let _ = progress_tx.send(PlanningProgress::GeneratingApproaches);

        let response = self
            .llm
            .complete_with_system(APPROACHES_SYSTEM_PROMPT, &prompt)
            .await?;

        let options = self.parse_approaches(&response)?;
        let count = options.len();
        let _ = progress_tx.send(PlanningProgress::ApproachesReady(count));

        Ok(options)
    }

    /// Generates approaches with streaming LLM output and progress callbacks.
    pub async fn generate_approaches_streaming(
        &self,
        research: &ResearchDoc,
        progress_tx: mpsc::UnboundedSender<PlanningProgress>,
        stream_tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<ApproachOptions, PlanningError> {
        let _ = progress_tx.send(PlanningProgress::Started);
        let _ = progress_tx.send(PlanningProgress::LoadingResearch);

        let prompt = build_approaches_prompt(research);

        let _ = progress_tx.send(PlanningProgress::GeneratingApproaches);

        let response = if self.llm.supports_streaming() {
            let (collector_tx, mut collector_rx) = mpsc::unbounded_channel::<StreamChunk>();

            let stream_tx_clone = stream_tx.clone();
            let collect_handle = tokio::spawn(async move {
                let mut full_response = String::new();
                while let Some(chunk) = collector_rx.recv().await {
                    if !chunk.is_final {
                        full_response.push_str(&chunk.text);
                    }
                    let _ = stream_tx_clone.send(chunk);
                }
                full_response
            });

            self.llm
                .stream_complete(APPROACHES_SYSTEM_PROMPT, &prompt, collector_tx)
                .await?;

            collect_handle.await.unwrap_or_default()
        } else {
            let response = self
                .llm
                .complete_with_system(APPROACHES_SYSTEM_PROMPT, &prompt)
                .await?;
            let _ = stream_tx.send(StreamChunk::text(response.clone()));
            let _ = stream_tx.send(StreamChunk::done());
            response
        };

        let options = self.parse_approaches(&response)?;
        let count = options.len();
        let _ = progress_tx.send(PlanningProgress::ApproachesReady(count));

        Ok(options)
    }

    /// Generates a detailed plan from a selected approach.
    pub async fn generate_plan(
        &self,
        research: &ResearchDoc,
        approach: &Approach,
    ) -> Result<Plan, PlanningError> {
        let prompt = build_spec_prompt(research, approach);

        let response = self
            .llm
            .complete_with_system(SPEC_SYSTEM_PROMPT, &prompt)
            .await?;

        self.parse_plan(&response)
    }

    /// Generates a plan with progress callbacks.
    pub async fn generate_plan_with_progress(
        &self,
        research: &ResearchDoc,
        approach: &Approach,
        progress_tx: mpsc::UnboundedSender<PlanningProgress>,
    ) -> Result<Plan, PlanningError> {
        let _ = progress_tx.send(PlanningProgress::GeneratingSpec);

        let prompt = build_spec_prompt(research, approach);

        let response = self
            .llm
            .complete_with_system(SPEC_SYSTEM_PROMPT, &prompt)
            .await?;

        let _ = progress_tx.send(PlanningProgress::CheckingComplexity);

        let plan = self.parse_plan(&response)?;

        let _ = progress_tx.send(PlanningProgress::Complete);

        Ok(plan)
    }

    /// Generates a plan with streaming LLM output and progress callbacks.
    pub async fn generate_plan_streaming(
        &self,
        research: &ResearchDoc,
        approach: &Approach,
        progress_tx: mpsc::UnboundedSender<PlanningProgress>,
        stream_tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<Plan, PlanningError> {
        let _ = progress_tx.send(PlanningProgress::GeneratingSpec);

        let prompt = build_spec_prompt(research, approach);

        let response = if self.llm.supports_streaming() {
            let (collector_tx, mut collector_rx) = mpsc::unbounded_channel::<StreamChunk>();

            let stream_tx_clone = stream_tx.clone();
            let collect_handle = tokio::spawn(async move {
                let mut full_response = String::new();
                while let Some(chunk) = collector_rx.recv().await {
                    if !chunk.is_final {
                        full_response.push_str(&chunk.text);
                    }
                    let _ = stream_tx_clone.send(chunk);
                }
                full_response
            });

            self.llm
                .stream_complete(SPEC_SYSTEM_PROMPT, &prompt, collector_tx)
                .await?;

            collect_handle.await.unwrap_or_default()
        } else {
            let response = self
                .llm
                .complete_with_system(SPEC_SYSTEM_PROMPT, &prompt)
                .await?;
            let _ = stream_tx.send(StreamChunk::text(response.clone()));
            let _ = stream_tx.send(StreamChunk::done());
            response
        };

        let _ = progress_tx.send(PlanningProgress::CheckingComplexity);

        let plan = self.parse_plan(&response)?;

        let _ = progress_tx.send(PlanningProgress::Complete);

        Ok(plan)
    }

    /// Parses approaches from LLM response.
    fn parse_approaches(&self, response: &str) -> Result<ApproachOptions, PlanningError> {
        let json_str = extract_json(response);

        let parsed: ApproachesResponse = serde_json::from_str(json_str).map_err(|e| {
            PlanningError::ParseError(format!(
                "Failed to parse approaches: {}. Response: {}",
                e,
                &json_str[..json_str.len().min(500)]
            ))
        })?;

        let mut options = ApproachOptions::new();

        for approach_data in parsed.approaches {
            let complexity = match approach_data.complexity.to_lowercase().as_str() {
                "low" => Complexity::Low,
                "high" => Complexity::High,
                _ => Complexity::Medium,
            };

            let approach = Approach {
                id: approach_data.id,
                name: approach_data.name,
                description: approach_data.description,
                pros: approach_data.pros,
                cons: approach_data.cons,
                complexity,
                recommended: approach_data.recommended,
            };

            options.add(approach);
        }

        Ok(options)
    }

    /// Parses a plan from LLM response.
    fn parse_plan(&self, response: &str) -> Result<Plan, PlanningError> {
        // Check for empty response
        if response.trim().is_empty() {
            return Err(PlanningError::ParseError(
                "LLM returned an empty response. This may indicate an API issue or rate limiting.".to_string()
            ));
        }

        let json_str = extract_json(response);

        // Check if we found valid JSON
        if json_str.is_empty() || !json_str.starts_with('{') {
            return Err(PlanningError::ParseError(format!(
                "No valid JSON found in LLM response. Raw response (first 500 chars): {}",
                &response[..response.len().min(500)]
            )));
        }

        let parsed: PlanResponse = serde_json::from_str(json_str).map_err(|e| {
            PlanningError::ParseError(format!(
                "Failed to parse plan JSON: {}. JSON content: {}",
                e,
                &json_str[..json_str.len().min(500)]
            ))
        })?;

        let complexity = match parsed.complexity.to_lowercase().as_str() {
            "low" => Complexity::Low,
            "high" => Complexity::High,
            _ => Complexity::Medium,
        };

        let files_to_create = parsed
            .files_to_create
            .into_iter()
            .map(|f| FileSpec {
                path: f.path,
                description: f.description,
                exports: f
                    .exports
                    .into_iter()
                    .map(|e| FunctionSignature {
                        name: e.name,
                        signature: e.signature,
                        behavior: e.behavior,
                    })
                    .collect(),
            })
            .collect();

        let files_to_modify = parsed
            .files_to_modify
            .into_iter()
            .map(|f| FileModification {
                path: f.path,
                line: f.line,
                description: f.description,
                additions: f.additions,
                removals: f.removals,
            })
            .collect();

        Ok(Plan {
            task_name: parsed.task_name,
            approach: parsed.approach,
            complexity,
            files_to_create,
            files_to_modify,
            dependencies_to_add: parsed.dependencies_to_add,
        })
    }
}

// Response structures for JSON parsing

#[derive(Debug, serde::Deserialize)]
struct ApproachesResponse {
    approaches: Vec<ApproachData>,
}

#[derive(Debug, serde::Deserialize)]
struct ApproachData {
    id: String,
    name: String,
    description: String,
    #[serde(default)]
    pros: Vec<String>,
    #[serde(default)]
    cons: Vec<String>,
    #[serde(default = "default_complexity")]
    complexity: String,
    #[serde(default)]
    recommended: bool,
}

fn default_complexity() -> String {
    "medium".to_string()
}

#[derive(Debug, serde::Deserialize)]
struct PlanResponse {
    task_name: String,
    approach: String,
    #[serde(default = "default_complexity")]
    complexity: String,
    #[serde(default)]
    files_to_create: Vec<FileSpecData>,
    #[serde(default)]
    files_to_modify: Vec<FileModificationData>,
    #[serde(default)]
    dependencies_to_add: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct FileSpecData {
    path: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    exports: Vec<ExportData>,
}

#[derive(Debug, serde::Deserialize)]
struct ExportData {
    name: String,
    #[serde(default)]
    signature: String,
    #[serde(default)]
    behavior: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct FileModificationData {
    path: String,
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    additions: Vec<String>,
    #[serde(default)]
    removals: Vec<String>,
}

/// Extracts JSON from a response that might be wrapped in markdown code blocks.
fn extract_json(response: &str) -> &str {
    let trimmed = response.trim();

    // Check for ```json ... ``` or ``` ... ```
    if trimmed.starts_with("```") {
        if let Some(start) = trimmed.find('\n') {
            let rest = &trimmed[start + 1..];
            if let Some(end) = rest.rfind("```") {
                return rest[..end].trim();
            }
        }
    }

    // Look for JSON object by finding first { and last }
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                return &trimmed[start..=end];
            }
        }
    }

    trimmed
}
