/// System prompt for the research phase.
pub const RESEARCH_SYSTEM_PROMPT: &str = r#"You are a code analyst helping a developer understand a codebase before making changes.

Your task is to analyze the provided codebase and create a research document that will help the developer understand:
1. The relevant parts of the codebase for their task
2. Dependencies and relationships between components
3. Existing patterns and conventions used
4. A suggested approach for implementing the task

Be thorough but concise. Focus on what's relevant to the task at hand.

IMPORTANT: Output your analysis as valid JSON matching this exact structure:
{
  "summary": "A 2-3 sentence summary of your findings",
  "findings": [
    {
      "title": "Finding title",
      "description": "Detailed description of the finding",
      "related_files": ["path/to/file1.rs", "path/to/file2.rs"]
    }
  ],
  "dependencies": [
    {
      "name": "Dependency name",
      "description": "What it does and why it's relevant",
      "is_external": true
    }
  ],
  "suggested_approach": "A clear, actionable description of how to implement the task"
}

Only output the JSON, no additional text."#;

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
