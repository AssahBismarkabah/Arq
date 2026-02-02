//! Agent executor - orchestrates the full agent phase.
//!
//! Brings together code generation, conformance checking, diff generation,
//! and change application into a unified execution flow.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::llm::{StreamChunk, LLM};
use crate::planning::Plan;

use super::applier::{ApplyOptions, ChangeApplier, ChangeOperation, PendingChange};
use super::conformance::ConformanceChecker;
use super::diff::DiffGenerator;
use super::error::AgentError;
use super::generator::CodeGenerator;
use super::progress::AgentProgress;
use super::types::{ExecutionSummary, FileOperation, GeneratedCode, ItemStatus};

/// Executor for the Agent phase.
///
/// Takes an approved plan and executes it step by step,
/// generating code, checking conformance, and applying changes.
pub struct AgentExecutor {
    /// The plan being executed.
    plan: Plan,
    /// Current item index being processed.
    current_index: usize,
    /// Results of executed items.
    results: Vec<ExecutionResult>,
    /// Code generator.
    generator: CodeGenerator,
    /// Conformance checker.
    conformance: ConformanceChecker,
    /// Diff generator.
    diff_gen: DiffGenerator,
    /// Project root directory.
    root: PathBuf,
    /// Item statuses.
    statuses: Vec<ItemStatus>,
}

impl AgentExecutor {
    /// Creates a new executor for the given plan.
    pub fn new(plan: Plan, llm: Box<dyn LLM>, root: impl Into<PathBuf>) -> Self {
        let total_items = plan.files_to_create.len() + plan.files_to_modify.len();
        Self {
            plan,
            current_index: 0,
            results: Vec::new(),
            generator: CodeGenerator::new(llm),
            conformance: ConformanceChecker::new(),
            diff_gen: DiffGenerator::new(),
            root: root.into(),
            statuses: vec![ItemStatus::Pending; total_items],
        }
    }

    /// Returns the plan being executed.
    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    /// Returns all execution items (files to create + files to modify).
    pub fn items(&self) -> Vec<ExecutionItem> {
        let mut items = Vec::new();

        for file in &self.plan.files_to_create {
            items.push(ExecutionItem::Create {
                path: file.path.clone(),
                description: file.description.clone(),
            });
        }

        for file in &self.plan.files_to_modify {
            items.push(ExecutionItem::Modify {
                path: file.path.clone(),
                description: file.description.clone(),
            });
        }

        items
    }

    /// Returns the current item being processed.
    pub fn current_item(&self) -> Option<ExecutionItem> {
        self.items().get(self.current_index).cloned()
    }

    /// Returns the current item index.
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Returns the total number of items.
    pub fn total_items(&self) -> usize {
        self.plan.files_to_create.len() + self.plan.files_to_modify.len()
    }

    /// Returns the number of completed items.
    pub fn completed_items(&self) -> usize {
        self.results.len()
    }

    /// Returns true if all items have been executed.
    pub fn is_complete(&self) -> bool {
        self.current_index >= self.total_items()
    }

    /// Returns all recorded results.
    pub fn results(&self) -> &[ExecutionResult] {
        &self.results
    }

    /// Returns the status of an item.
    pub fn get_status(&self, index: usize) -> Option<ItemStatus> {
        self.statuses.get(index).copied()
    }

    /// Returns true if all results passed conformance.
    pub fn all_conformant(&self) -> bool {
        self.results
            .iter()
            .all(|r| r.conformance == ConformanceStatus::Passed)
    }

    /// Generate code for the current item.
    pub async fn generate_current(&mut self) -> Result<GeneratedCode, AgentError> {
        let item = self.current_item().ok_or(AgentError::InvalidPlanItem {
            index: self.current_index,
            message: "No item at current index".to_string(),
        })?;

        // Update status
        if let Some(status) = self.statuses.get_mut(self.current_index) {
            *status = ItemStatus::Generating;
        }

        // Read existing content if modifying
        let existing_content = match &item {
            ExecutionItem::Modify { path, .. } => {
                let full_path = self.root.join(path);
                if full_path.exists() {
                    Some(std::fs::read_to_string(&full_path).map_err(|e| {
                        AgentError::FileReadError {
                            path: path.clone(),
                            message: e.to_string(),
                        }
                    })?)
                } else {
                    None
                }
            }
            ExecutionItem::Create { .. } => None,
        };

        // Generate code
        let generated = self
            .generator
            .generate_for_item(&item, &self.plan, existing_content.as_deref())
            .await?;

        // Check conformance
        let conformance = self.conformance.check_by_path(
            &self.plan,
            item.path(),
            &generated,
            existing_content.as_deref(),
        );

        // Determine operation
        let operation = match &item {
            ExecutionItem::Create { .. } => FileOperation::Create,
            ExecutionItem::Modify { .. } => FileOperation::Modify {
                original: existing_content.unwrap_or_default(),
            },
        };

        // Update status
        if let Some(status) = self.statuses.get_mut(self.current_index) {
            *status = ItemStatus::AwaitingReview;
        }

        Ok(GeneratedCode {
            item_index: self.current_index,
            file_path: item.path().to_string(),
            operation,
            content: generated,
            conformance,
        })
    }

    /// Generate code for the current item with streaming.
    pub async fn generate_current_streaming(
        &mut self,
        tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<GeneratedCode, AgentError> {
        let item = self.current_item().ok_or(AgentError::InvalidPlanItem {
            index: self.current_index,
            message: "No item at current index".to_string(),
        })?;

        // Update status
        if let Some(status) = self.statuses.get_mut(self.current_index) {
            *status = ItemStatus::Generating;
        }

        // Read existing content if modifying
        let existing_content = match &item {
            ExecutionItem::Modify { path, .. } => {
                let full_path = self.root.join(path);
                if full_path.exists() {
                    Some(std::fs::read_to_string(&full_path).map_err(|e| {
                        AgentError::FileReadError {
                            path: path.clone(),
                            message: e.to_string(),
                        }
                    })?)
                } else {
                    None
                }
            }
            ExecutionItem::Create { .. } => None,
        };

        // Generate code with streaming - collect the content
        let (content_tx, mut content_rx) = mpsc::unbounded_channel();
        let mut collected_content = String::new();

        // Clone for the forwarding task
        let forward_tx = tx;

        // Generate with streaming
        self.generator
            .generate_for_item_streaming(&item, &self.plan, existing_content.as_deref(), content_tx)
            .await?;

        // Collect all chunks
        while let Some(chunk) = content_rx.recv().await {
            if !chunk.is_final {
                collected_content.push_str(&chunk.text);
            }
            // Forward to the provided channel
            let _ = forward_tx.send(chunk);
        }

        // Check conformance
        let conformance = self.conformance.check_by_path(
            &self.plan,
            item.path(),
            &collected_content,
            existing_content.as_deref(),
        );

        // Determine operation
        let operation = match &item {
            ExecutionItem::Create { .. } => FileOperation::Create,
            ExecutionItem::Modify { .. } => FileOperation::Modify {
                original: existing_content.unwrap_or_default(),
            },
        };

        // Update status
        if let Some(status) = self.statuses.get_mut(self.current_index) {
            *status = ItemStatus::AwaitingReview;
        }

        Ok(GeneratedCode {
            item_index: self.current_index,
            file_path: item.path().to_string(),
            operation,
            content: collected_content,
            conformance,
        })
    }

    /// Accept generated code for the current item.
    pub fn accept_current(&mut self, generated: GeneratedCode) {
        // Update status
        if let Some(status) = self.statuses.get_mut(self.current_index) {
            *status = ItemStatus::Accepted;
        }

        // Record result
        let item = self.current_item().unwrap();
        let conformance_status = if generated.conformance.passed {
            ConformanceStatus::Passed
        } else if generated
            .conformance
            .deviations
            .iter()
            .any(|d| d.severity == super::types::DeviationSeverity::Error)
        {
            ConformanceStatus::Failed
        } else {
            ConformanceStatus::Warning
        };

        self.results.push(ExecutionResult {
            item,
            generated_code: generated.content,
            conformance: conformance_status,
            deviations: generated
                .conformance
                .deviations
                .iter()
                .map(|d| d.description.clone())
                .collect(),
        });

        self.current_index += 1;
    }

    /// Skip the current item.
    pub fn skip_current(&mut self) {
        if let Some(status) = self.statuses.get_mut(self.current_index) {
            *status = ItemStatus::Skipped;
        }
        self.current_index += 1;
    }

    /// Apply all accepted changes to the filesystem.
    pub fn apply_changes(&self, dry_run: bool) -> Result<ExecutionSummary, AgentError> {
        let options = ApplyOptions {
            create_backups: true,
            dry_run,
            ..Default::default()
        };

        let mut applier = ChangeApplier::with_options(&self.root, options);
        let mut summary = ExecutionSummary::new();

        for result in &self.results {
            let change = PendingChange {
                path: result.item.path().to_string(),
                operation: match &result.item {
                    ExecutionItem::Create { .. } => ChangeOperation::Create {
                        content: result.generated_code.clone(),
                    },
                    ExecutionItem::Modify { .. } => {
                        // We need the original content - get from the file
                        let original = std::fs::read_to_string(self.root.join(result.item.path()))
                            .unwrap_or_default();
                        ChangeOperation::Modify {
                            original,
                            content: result.generated_code.clone(),
                        }
                    }
                },
            };

            let apply_result = applier.apply(&change)?;

            if apply_result.success {
                match &result.item {
                    ExecutionItem::Create { path, .. } => {
                        summary.add_created(path);
                    }
                    ExecutionItem::Modify { path, .. } => {
                        summary.add_modified(path);
                    }
                }
            }
        }

        summary.all_conformance_passed = self.all_conformant();
        Ok(summary)
    }

    /// Generate a diff for a generated code result.
    pub fn generate_diff(&self, generated: &GeneratedCode) -> String {
        match &generated.operation {
            FileOperation::Create => {
                self.diff_gen
                    .format_unified(&generated.file_path, "", &generated.content)
            }
            FileOperation::Modify { original } => {
                self.diff_gen
                    .format_unified(&generated.file_path, original, &generated.content)
            }
            FileOperation::Delete => {
                let original = std::fs::read_to_string(self.root.join(&generated.file_path))
                    .unwrap_or_default();
                self.diff_gen
                    .format_unified(&generated.file_path, &original, "")
            }
        }
    }

    /// Get progress for the current state.
    pub fn progress(&self) -> AgentProgress {
        if self.is_complete() {
            let files_created = self
                .results
                .iter()
                .filter(|r| matches!(r.item, ExecutionItem::Create { .. }))
                .count();
            let files_modified = self
                .results
                .iter()
                .filter(|r| matches!(r.item, ExecutionItem::Modify { .. }))
                .count();
            AgentProgress::Complete {
                files_created,
                files_modified,
            }
        } else if let Some(item) = self.current_item() {
            let status = self
                .get_status(self.current_index)
                .unwrap_or(ItemStatus::Pending);
            match status {
                ItemStatus::Pending | ItemStatus::Generating => AgentProgress::GeneratingCode {
                    item_index: self.current_index,
                    total_items: self.total_items(),
                    item_description: match &item {
                        ExecutionItem::Create { description, .. } => description.clone(),
                        ExecutionItem::Modify { description, .. } => description.clone(),
                    },
                },
                ItemStatus::AwaitingReview => AgentProgress::AwaitingReview {
                    item_index: self.current_index,
                },
                ItemStatus::Accepted | ItemStatus::Skipped | ItemStatus::Failed => {
                    AgentProgress::GeneratingCode {
                        item_index: self.current_index,
                        total_items: self.total_items(),
                        item_description: match &item {
                            ExecutionItem::Create { description, .. } => description.clone(),
                            ExecutionItem::Modify { description, .. } => description.clone(),
                        },
                    }
                }
            }
        } else {
            AgentProgress::LoadingPlan
        }
    }
}

/// An item to be executed by the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionItem {
    /// Create a new file.
    Create {
        /// Path to the file.
        path: String,
        /// Description of the file.
        description: String,
    },
    /// Modify an existing file.
    Modify {
        /// Path to the file.
        path: String,
        /// Description of the modification.
        description: String,
    },
}

impl ExecutionItem {
    /// Get the path of the item.
    pub fn path(&self) -> &str {
        match self {
            ExecutionItem::Create { path, .. } => path,
            ExecutionItem::Modify { path, .. } => path,
        }
    }

    /// Get the description of the item.
    pub fn description(&self) -> &str {
        match self {
            ExecutionItem::Create { description, .. } => description,
            ExecutionItem::Modify { description, .. } => description,
        }
    }
}

/// Result of executing one item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// The item that was executed.
    pub item: ExecutionItem,
    /// Generated code.
    pub generated_code: String,
    /// Conformance check result.
    pub conformance: ConformanceStatus,
    /// Deviations from spec (if any).
    pub deviations: Vec<String>,
}

/// Status of conformance checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConformanceStatus {
    /// Code matches spec exactly.
    Passed,
    /// Code has minor deviations.
    Warning,
    /// Code significantly deviates from spec.
    Failed,
}
