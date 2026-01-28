use crate::config::DEFAULT_RESEARCH_SYSTEM_PROMPT;

/// Gets the system prompt for the research phase.
///
/// If a custom prompt is provided, uses that. Otherwise uses the default.
pub fn get_research_system_prompt(custom: Option<&str>) -> &str {
    custom.unwrap_or(DEFAULT_RESEARCH_SYSTEM_PROMPT)
}

/// System prompt for the research phase (default).
/// Kept for backwards compatibility - prefer using get_research_system_prompt().
pub const RESEARCH_SYSTEM_PROMPT: &str = DEFAULT_RESEARCH_SYSTEM_PROMPT;

/// Builds the user prompt for research.
pub fn build_research_prompt(task_prompt: &str, context: &str) -> String {
    format!(
        r#"## Task

{task_prompt}

## Codebase

{context}

Analyze this codebase for the given task. Identify relevant files, dependencies, and suggest an approach."#
    )
}
