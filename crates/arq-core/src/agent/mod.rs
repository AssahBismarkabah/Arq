//! Agent phase - executes the approved plan.
//!
//! The Agent is the third and final phase in Arq's workflow.
//! It generates code that strictly conforms to the approved specification.
//!
//! # Core Principle
//!
//! "Build exactly what was approved."
//!
//! The Agent is a disciplined executor, not a creative partner. It implements
//! the specification without adding, removing, or changing what was approved.
//!
//! # Flow
//!
//! 1. Load plan.yaml as the contract
//! 2. Iterate through each plan item
//! 3. Generate code for each item
//! 4. Check conformance against spec
//! 5. User reviews and accepts/regenerates
//! 6. Apply changes to filesystem

mod applier;
mod conformance;
mod diff;
mod error;
mod executor;
mod generator;
mod progress;
mod types;

pub use applier::{ApplyOptions, ApplyResult, ChangeApplier, ChangeOperation, PendingChange};
pub use conformance::ConformanceChecker;
pub use diff::{DiffGenerator, DiffLine, DiffLineType, DiffStats, FileDiff};
pub use error::AgentError;
pub use executor::{AgentExecutor, ConformanceStatus, ExecutionItem, ExecutionResult};
pub use generator::CodeGenerator;
pub use progress::AgentProgress;
pub use types::{
    ConformanceCheck, ConformanceResult, Deviation, DeviationSeverity, ExecutionSummary,
    FileOperation, GeneratedCode, ItemStatus, TestResults,
};
