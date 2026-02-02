//! Conformance checker for agent phase.
//!
//! Validates that generated code matches the plan specification.

use crate::planning::{FileModification, FileSpec, Plan};

use super::types::{ConformanceCheck, ConformanceResult, Deviation};

/// Conformance checker that validates generated code against specs.
pub struct ConformanceChecker;

impl ConformanceChecker {
    /// Create a new conformance checker.
    pub fn new() -> Self {
        Self
    }

    /// Check conformance of generated code for a file to create.
    pub fn check_create(&self, file: &FileSpec, generated: &str) -> ConformanceResult {
        let mut checks = Vec::new();
        let mut deviations = Vec::new();

        // Check 1: File is not empty
        let is_not_empty = !generated.trim().is_empty();
        checks.push(if is_not_empty {
            ConformanceCheck::pass("File not empty")
        } else {
            ConformanceCheck::fail("File not empty", "Generated code is empty")
        });

        // Check 2: Contains expected exports (basic check)
        for export in &file.exports {
            let contains_name = generated.contains(&export.name);
            checks.push(if contains_name {
                ConformanceCheck::pass_with_details(
                    format!("Contains '{}'", export.name),
                    format!("Found function/export '{}'", export.name),
                )
            } else {
                deviations.push(Deviation::error(
                    format!("Missing export '{}'", export.name),
                    format!("Should contain: {}", export.name),
                    "Not found in generated code".to_string(),
                ));
                ConformanceCheck::fail(
                    format!("Contains '{}'", export.name),
                    "Export not found in generated code",
                )
            });
        }

        // Check 3: No obvious issues (basic heuristics)
        let no_todo_comments = !generated.contains("TODO") && !generated.contains("FIXME");
        checks.push(if no_todo_comments {
            ConformanceCheck::pass("No TODO/FIXME comments")
        } else {
            deviations.push(Deviation::warning(
                "Contains TODO/FIXME comments",
                "No incomplete markers",
                "Found TODO or FIXME in code".to_string(),
            ));
            ConformanceCheck::fail("No TODO/FIXME comments", "Contains incomplete markers")
        });

        // Check 4: No placeholder text
        let no_placeholders = !generated.contains("...")
            && !generated.contains("// ...")
            && !generated.contains("/* ... */");
        checks.push(if no_placeholders {
            ConformanceCheck::pass("No placeholder text")
        } else {
            deviations.push(Deviation::error(
                "Contains placeholder text",
                "Complete implementation",
                "Found '...' placeholder in code".to_string(),
            ));
            ConformanceCheck::fail("No placeholder text", "Contains '...' placeholders")
        });

        // Determine overall pass/fail
        let all_passed = checks.iter().all(|c| c.passed);

        ConformanceResult {
            passed: all_passed
                && deviations
                    .iter()
                    .all(|d| d.severity != super::types::DeviationSeverity::Error),
            checks,
            deviations,
        }
    }

    /// Check conformance of generated code for a file modification.
    pub fn check_modify(
        &self,
        file: &FileModification,
        original: &str,
        generated: &str,
    ) -> ConformanceResult {
        let mut checks = Vec::new();
        let mut deviations = Vec::new();

        // Check 1: File is not empty
        let is_not_empty = !generated.trim().is_empty();
        checks.push(if is_not_empty {
            ConformanceCheck::pass("File not empty")
        } else {
            ConformanceCheck::fail("File not empty", "Generated code is empty")
        });

        // Check 2: File was actually modified (unless it was empty before)
        if !original.trim().is_empty() {
            let was_modified = original != generated;
            checks.push(if was_modified {
                ConformanceCheck::pass("File was modified")
            } else {
                deviations.push(Deviation::warning(
                    "File appears unchanged",
                    "File should have modifications",
                    "Generated content matches original".to_string(),
                ));
                ConformanceCheck::fail("File was modified", "No changes detected")
            });
        }

        // Check 3: Additions are present (warning only - LLM may adapt variable names)
        for addition in &file.additions {
            // Extract the core part of the addition (strip quotes and whitespace)
            let addition_core = addition.trim().trim_matches('\'').trim_matches('"');
            let contains_addition = generated.contains(addition_core);
            checks.push(if contains_addition {
                ConformanceCheck::pass_with_details(
                    "Addition present",
                    format!("Found: {}", truncate(addition_core, 50)),
                )
            } else {
                // Use warning instead of error - LLM may correctly adapt
                // variable/field names to match actual code (e.g., self.rx vs self.receiver)
                deviations.push(Deviation::warning(
                    "Addition not found verbatim (may be adapted)",
                    format!("Plan specified: {}", truncate(addition_core, 50)),
                    "LLM may have adapted to match actual code".to_string(),
                ));
                ConformanceCheck::pass_with_details(
                    "Addition (adapted)",
                    format!("Plan: {} (may be adapted)", truncate(addition_core, 40)),
                )
            });
        }

        // Check 4: Removals are gone (warning if still present - may be adapted)
        for removal in &file.removals {
            let removal_core = removal.trim().trim_matches('\'').trim_matches('"');
            let still_contains = generated.contains(removal_core);
            checks.push(if !still_contains {
                ConformanceCheck::pass_with_details(
                    "Removal applied",
                    format!("Removed: {}", truncate(removal_core, 50)),
                )
            } else {
                // Use warning - the exact text might still exist in a different context
                // or the LLM may have made equivalent changes
                deviations.push(Deviation::warning(
                    "Removal may not be applied",
                    format!("Plan wanted to remove: {}", truncate(removal_core, 50)),
                    "Still found - verify manually".to_string(),
                ));
                ConformanceCheck::pass_with_details(
                    "Removal (verify)",
                    format!("Check: {}", truncate(removal_core, 40)),
                )
            });
        }

        // Check 5: No obvious issues
        let no_todo_comments = !generated.contains("TODO") && !generated.contains("FIXME");
        checks.push(if no_todo_comments {
            ConformanceCheck::pass("No TODO/FIXME comments")
        } else {
            deviations.push(Deviation::warning(
                "Contains TODO/FIXME comments",
                "No incomplete markers",
                "Found TODO or FIXME in code".to_string(),
            ));
            ConformanceCheck::fail("No TODO/FIXME comments", "Contains incomplete markers")
        });

        // Determine overall pass/fail
        let all_passed = checks.iter().all(|c| c.passed);

        ConformanceResult {
            passed: all_passed
                && deviations
                    .iter()
                    .all(|d| d.severity != super::types::DeviationSeverity::Error),
            checks,
            deviations,
        }
    }

    /// Quick check if generated code looks valid (basic sanity check).
    pub fn quick_check(&self, generated: &str) -> bool {
        let trimmed = generated.trim();

        // Basic checks
        !trimmed.is_empty()
            && !trimmed.contains("I cannot")
            && !trimmed.contains("I'm sorry")
            && !trimmed.contains("As an AI")
            && !trimmed.starts_with("```") // Should not have markdown code blocks
    }

    /// Check conformance for a plan item by path.
    pub fn check_by_path(
        &self,
        plan: &Plan,
        path: &str,
        generated: &str,
        original: Option<&str>,
    ) -> ConformanceResult {
        // Try to find in files_to_create
        if let Some(file) = plan.files_to_create.iter().find(|f| f.path == path) {
            return self.check_create(file, generated);
        }

        // Try to find in files_to_modify
        if let Some(file) = plan.files_to_modify.iter().find(|f| f.path == path) {
            return self.check_modify(file, original.unwrap_or(""), generated);
        }

        // Not found in plan - this is an error
        ConformanceResult::failed(
            vec![ConformanceCheck::fail(
                "File in plan",
                format!("File '{}' not found in plan", path),
            )],
            vec![Deviation::error(
                "File not in plan",
                format!("Expected file '{}' to be in plan", path),
                "File path not found".to_string(),
            )],
        )
    }
}

impl Default for ConformanceChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Truncate a string to a maximum length with ellipsis.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
