//! Behavioral edges: relationships that define code execution flow.
//!
//! These edges represent how code executes and interacts:
//! - CALLS: A calls B (function invocation)
//! - RETURNS: A returns B (return value)
//! - THROWS: A throws B (error/exception)
//! - AWAITS: A awaits B (async operation)
//! - READS: A reads B (data access)
//! - WRITES: A writes B (data mutation)

use serde::{Deserialize, Serialize};

// =============================================================================
// CALLS EDGE
// =============================================================================

/// A CALLS B: Function A invokes function B.
///
/// The most important edge for understanding code flow.
///
/// Examples:
/// - Function CALLS Function
/// - Method CALLS Method
/// - Handler CALLS Service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallsEdge {
    /// Source node ID (caller)
    pub from: String,
    /// Target node ID (callee)
    pub to: String,
    /// Line number of the call site
    pub line: Option<u32>,
    /// Column number of the call site
    pub column: Option<u32>,
    /// Call type
    pub call_type: CallType,
    /// Whether this is a conditional call (in if/match)
    pub is_conditional: bool,
    /// Whether this is inside a loop
    pub is_in_loop: bool,
    /// Number of times called (static analysis estimate)
    pub call_count: Option<u32>,
}

/// Type of function call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CallType {
    /// Direct function call
    #[default]
    Direct,
    /// Method call on self
    Method,
    /// Static method call
    Static,
    /// Trait method call (dynamic dispatch)
    Dynamic,
    /// Closure/callback invocation
    Closure,
    /// Macro invocation
    Macro,
    /// Constructor call
    Constructor,
}

impl CallsEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            line: None,
            column: None,
            call_type: CallType::Direct,
            is_conditional: false,
            is_in_loop: false,
            call_count: None,
        }
    }

    pub fn at_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    pub fn method(mut self) -> Self {
        self.call_type = CallType::Method;
        self
    }

    pub fn conditional(mut self) -> Self {
        self.is_conditional = true;
        self
    }
}

// =============================================================================
// RETURNS EDGE
// =============================================================================

/// A RETURNS B: Function A returns value B.
///
/// Used for tracking data flow through return values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnsEdge {
    /// Source node ID (function)
    pub from: String,
    /// Target node ID (returned value/type)
    pub to: String,
    /// Line number of return statement
    pub line: Option<u32>,
    /// Return type
    pub return_type: ReturnType,
    /// Whether this is an early return
    pub is_early_return: bool,
}

/// Type of return.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReturnType {
    /// Normal return value
    #[default]
    Value,
    /// Return Result::Ok
    Ok,
    /// Return Result::Err
    Err,
    /// Return Option::Some
    Some,
    /// Return Option::None
    None,
    /// Implicit return (last expression)
    Implicit,
}

impl ReturnsEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            line: None,
            return_type: ReturnType::Value,
            is_early_return: false,
        }
    }
}

// =============================================================================
// THROWS EDGE
// =============================================================================

/// A THROWS B: Function A can throw/return error B.
///
/// Important for error handling analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrowsEdge {
    /// Source node ID (function)
    pub from: String,
    /// Target node ID (error type)
    pub to: String,
    /// Line numbers where error can originate
    pub lines: Vec<u32>,
    /// Whether error is propagated (? operator) or created
    pub is_propagated: bool,
    /// Error handling mechanism
    pub mechanism: ErrorMechanism,
}

/// Error handling mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ErrorMechanism {
    /// Rust Result type
    #[default]
    Result,
    /// Exception (throw/catch)
    Exception,
    /// Panic
    Panic,
    /// Error return code
    ErrorCode,
}

impl ThrowsEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            lines: Vec::new(),
            is_propagated: false,
            mechanism: ErrorMechanism::Result,
        }
    }

    pub fn propagated(mut self) -> Self {
        self.is_propagated = true;
        self
    }
}

// =============================================================================
// AWAITS EDGE
// =============================================================================

/// A AWAITS B: Async function A awaits async operation B.
///
/// Tracks async control flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwaitsEdge {
    /// Source node ID (awaiting function)
    pub from: String,
    /// Target node ID (awaited future)
    pub to: String,
    /// Line number of await
    pub line: Option<u32>,
    /// Whether this is in a select/join
    pub is_concurrent: bool,
    /// Await context
    pub context: AwaitContext,
}

/// Context of an await.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AwaitContext {
    /// Simple sequential await
    #[default]
    Sequential,
    /// In tokio::select! or similar
    Select,
    /// In tokio::join! or similar
    Join,
    /// In a loop
    Loop,
    /// In a spawn
    Spawned,
}

impl AwaitsEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            line: None,
            is_concurrent: false,
            context: AwaitContext::Sequential,
        }
    }
}

// =============================================================================
// READS EDGE
// =============================================================================

/// A READS B: Function A reads field/variable B.
///
/// Tracks data access patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadsEdge {
    /// Source node ID (reader)
    pub from: String,
    /// Target node ID (data being read)
    pub to: String,
    /// Line numbers of read accesses
    pub lines: Vec<u32>,
    /// Access type
    pub access_type: AccessType,
}

/// Type of data access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccessType {
    /// Direct field access
    #[default]
    Field,
    /// Through getter method
    Getter,
    /// Array/map index access
    Index,
    /// Dereference
    Deref,
}

impl ReadsEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            lines: Vec::new(),
            access_type: AccessType::Field,
        }
    }
}

// =============================================================================
// WRITES EDGE
// =============================================================================

/// A WRITES B: Function A writes/mutates field/variable B.
///
/// Tracks data mutation for impact analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritesEdge {
    /// Source node ID (writer)
    pub from: String,
    /// Target node ID (data being written)
    pub to: String,
    /// Line numbers of write accesses
    pub lines: Vec<u32>,
    /// Write type
    pub write_type: WriteType,
}

/// Type of write operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WriteType {
    /// Direct assignment
    #[default]
    Assign,
    /// Through setter method
    Setter,
    /// Mutable borrow modification
    MutBorrow,
    /// Compound assignment (+=, etc.)
    Compound,
}

impl WritesEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            lines: Vec::new(),
            write_type: WriteType::Assign,
        }
    }
}
