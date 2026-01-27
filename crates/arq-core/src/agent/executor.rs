use serde::{Deserialize, Serialize};

use crate::planning::Plan;

/// Executor for the Agent phase.
///
/// Takes an approved plan and executes it step by step,
/// checking conformance at each step.
#[derive(Debug)]
pub struct AgentExecutor {
    /// The plan being executed
    plan: Plan,
    /// Current item index being processed
    current_index: usize,
    /// Results of executed items
    results: Vec<ExecutionResult>,
}

impl AgentExecutor {
    /// Creates a new executor for the given plan.
    pub fn new(plan: Plan) -> Self {
        Self {
            plan,
            current_index: 0,
            results: Vec::new(),
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

    /// Returns the total number of items.
    pub fn total_items(&self) -> usize {
        self.plan.files_to_create.len() + self.plan.files_to_modify.len()
    }

    /// Returns the number of completed items.
    pub fn completed_items(&self) -> usize {
        self.results.len()
    }

    /// Records a result for the current item and advances.
    pub fn record_result(&mut self, result: ExecutionResult) {
        self.results.push(result);
        self.current_index += 1;
    }

    /// Returns true if all items have been executed.
    pub fn is_complete(&self) -> bool {
        self.current_index >= self.total_items()
    }

    /// Returns all recorded results.
    pub fn results(&self) -> &[ExecutionResult] {
        &self.results
    }

    /// Returns true if all results passed conformance.
    pub fn all_conformant(&self) -> bool {
        self.results
            .iter()
            .all(|r| r.conformance == ConformanceStatus::Passed)
    }
}

/// An item to be executed by the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionItem {
    /// Create a new file
    Create { path: String, description: String },
    /// Modify an existing file
    Modify { path: String, description: String },
}

impl ExecutionItem {
    pub fn path(&self) -> &str {
        match self {
            ExecutionItem::Create { path, .. } => path,
            ExecutionItem::Modify { path, .. } => path,
        }
    }
}

/// Result of executing one item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// The item that was executed
    pub item: ExecutionItem,
    /// Generated code
    pub generated_code: String,
    /// Conformance check result
    pub conformance: ConformanceStatus,
    /// Deviations from spec (if any)
    pub deviations: Vec<String>,
}

/// Status of conformance checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConformanceStatus {
    /// Code matches spec exactly
    Passed,
    /// Code has minor deviations
    Warning,
    /// Code significantly deviates from spec
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planning::{Plan, FileSpec};

    #[test]
    fn test_executor_items() {
        let mut plan = Plan::new("test", "test approach");
        plan.files_to_create.push(FileSpec {
            path: "src/foo.rs".to_string(),
            description: "Foo module".to_string(),
            exports: vec![],
        });

        let executor = AgentExecutor::new(plan);
        assert_eq!(executor.total_items(), 1);
        assert_eq!(executor.completed_items(), 0);
        assert!(!executor.is_complete());
    }

    #[test]
    fn test_executor_progress() {
        let mut plan = Plan::new("test", "test approach");
        plan.files_to_create.push(FileSpec {
            path: "src/foo.rs".to_string(),
            description: "Foo module".to_string(),
            exports: vec![],
        });

        let mut executor = AgentExecutor::new(plan);

        let result = ExecutionResult {
            item: executor.current_item().unwrap(),
            generated_code: "// code".to_string(),
            conformance: ConformanceStatus::Passed,
            deviations: vec![],
        };

        executor.record_result(result);

        assert_eq!(executor.completed_items(), 1);
        assert!(executor.is_complete());
        assert!(executor.all_conformant());
    }
}
