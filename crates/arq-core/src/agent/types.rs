//! Core types for the agent phase.

use serde::{Deserialize, Serialize};

/// Operation to perform on a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOperation {
    /// Create a new file.
    Create,
    /// Modify an existing file.
    Modify {
        /// Original file content.
        original: String,
    },
    /// Delete a file.
    Delete,
}

/// Result of code generation for a single plan item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCode {
    /// Index of the plan item.
    pub item_index: usize,
    /// Path to the file.
    pub file_path: String,
    /// Operation to perform.
    pub operation: FileOperation,
    /// Generated content.
    pub content: String,
    /// Conformance check result.
    pub conformance: ConformanceResult,
}

/// Result of conformance checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceResult {
    /// Whether all checks passed.
    pub passed: bool,
    /// Individual check results.
    pub checks: Vec<ConformanceCheck>,
    /// Any deviations from the spec.
    pub deviations: Vec<Deviation>,
}

impl Default for ConformanceResult {
    fn default() -> Self {
        Self {
            passed: true,
            checks: Vec::new(),
            deviations: Vec::new(),
        }
    }
}

impl ConformanceResult {
    /// Create a passing result with the given checks.
    pub fn passed(checks: Vec<ConformanceCheck>) -> Self {
        Self {
            passed: true,
            checks,
            deviations: Vec::new(),
        }
    }

    /// Create a failing result with deviations.
    pub fn failed(checks: Vec<ConformanceCheck>, deviations: Vec<Deviation>) -> Self {
        Self {
            passed: false,
            checks,
            deviations,
        }
    }
}

/// A single conformance check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceCheck {
    /// Name of the check.
    pub name: String,
    /// Whether it passed.
    pub passed: bool,
    /// Additional details.
    pub details: Option<String>,
}

impl ConformanceCheck {
    /// Create a passing check.
    pub fn pass(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            details: None,
        }
    }

    /// Create a passing check with details.
    pub fn pass_with_details(name: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            details: Some(details.into()),
        }
    }

    /// Create a failing check.
    pub fn fail(name: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            details: Some(details.into()),
        }
    }
}

/// A deviation from the specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deviation {
    /// Description of the deviation.
    pub description: String,
    /// What the spec expected.
    pub expected: String,
    /// What was actually generated.
    pub actual: String,
    /// Severity of the deviation.
    pub severity: DeviationSeverity,
}

impl Deviation {
    /// Create a warning deviation.
    pub fn warning(
        description: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self {
            description: description.into(),
            expected: expected.into(),
            actual: actual.into(),
            severity: DeviationSeverity::Warning,
        }
    }

    /// Create an error deviation.
    pub fn error(
        description: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self {
            description: description.into(),
            expected: expected.into(),
            actual: actual.into(),
            severity: DeviationSeverity::Error,
        }
    }
}

/// Severity of a deviation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviationSeverity {
    /// Minor difference, can be accepted.
    Warning,
    /// Significant deviation, should be addressed.
    Error,
}

/// Summary of agent execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionSummary {
    /// Files that were created.
    pub files_created: Vec<String>,
    /// Files that were modified.
    pub files_modified: Vec<String>,
    /// Files that were deleted.
    pub files_deleted: Vec<String>,
    /// Dependencies that were added.
    pub dependencies_added: Vec<String>,
    /// Whether all conformance checks passed.
    pub all_conformance_passed: bool,
    /// Test results if tests were run.
    pub test_results: Option<TestResults>,
}

impl ExecutionSummary {
    /// Create an empty summary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a created file.
    pub fn add_created(&mut self, path: impl Into<String>) {
        self.files_created.push(path.into());
    }

    /// Add a modified file.
    pub fn add_modified(&mut self, path: impl Into<String>) {
        self.files_modified.push(path.into());
    }

    /// Get total number of changes.
    pub fn total_changes(&self) -> usize {
        self.files_created.len() + self.files_modified.len() + self.files_deleted.len()
    }
}

/// Results from running tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    /// Number of tests that passed.
    pub passed: usize,
    /// Number of tests that failed.
    pub failed: usize,
    /// Error messages from failed tests.
    pub errors: Vec<String>,
}

impl TestResults {
    /// Check if all tests passed.
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }
}

/// Status of a plan item during execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemStatus {
    /// Not yet processed.
    Pending,
    /// Currently being generated.
    Generating,
    /// Awaiting user review.
    AwaitingReview,
    /// Accepted by user.
    Accepted,
    /// Skipped by user.
    Skipped,
    /// Failed to generate.
    Failed,
}
