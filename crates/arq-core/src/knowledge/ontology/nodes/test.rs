//! Test entity nodes: TestCase, TestSuite.
//!
//! These represent test infrastructure in the codebase.

use serde::{Deserialize, Serialize};

// =============================================================================
// TEST CASE ENTITY
// =============================================================================

/// A single test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseEntity {
    /// Unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Test name
    pub name: String,

    /// Full path (module::test_name)
    pub full_path: String,

    /// File containing this test
    pub file_path: String,

    /// Start line
    pub start_line: u32,

    /// End line
    pub end_line: u32,

    /// Test type
    pub test_type: TestType,

    /// Test suite this belongs to
    pub suite: Option<String>,

    /// Function being tested (if identifiable)
    pub tests_function: Option<String>,

    /// Whether the test is ignored
    pub ignored: bool,

    /// Ignore reason
    pub ignore_reason: Option<String>,

    /// Whether the test should panic
    pub should_panic: bool,

    /// Timeout in milliseconds
    pub timeout_ms: Option<u64>,

    /// Tags/categories
    pub tags: Vec<String>,

    /// Description
    pub description: Option<String>,
}

/// Type of test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestType {
    /// Unit test
    Unit,
    /// Integration test
    Integration,
    /// End-to-end test
    E2e,
    /// Performance/benchmark test
    Benchmark,
    /// Property-based test
    Property,
    /// Snapshot test
    Snapshot,
    /// Doc test
    Doc,
}

// =============================================================================
// TEST SUITE ENTITY
// =============================================================================

/// A test suite (group of related tests).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuiteEntity {
    /// Unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Suite name
    pub name: String,

    /// Full path
    pub full_path: String,

    /// File containing this suite
    pub file_path: String,

    /// Start line
    pub start_line: u32,

    /// End line
    pub end_line: u32,

    /// Tests in this suite
    pub tests: Vec<String>,

    /// Nested suites
    pub nested_suites: Vec<String>,

    /// Setup function
    pub setup: Option<String>,

    /// Teardown function
    pub teardown: Option<String>,

    /// Fixtures used
    pub fixtures: Vec<String>,

    /// Tags
    pub tags: Vec<String>,

    /// Description
    pub description: Option<String>,
}
