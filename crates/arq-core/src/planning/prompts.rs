use crate::research::ResearchDoc;

use super::Approach;

/// System prompt for generating implementation approaches.
pub const APPROACHES_SYSTEM_PROMPT: &str = r#"You are a senior software architect. Your task is to analyze research findings and propose 2-3 implementation approaches with clear trade-offs.

Guidelines:
- Each approach should be distinct with different complexity levels
- Focus on minimal, targeted changes that respect the existing codebase
- Consider maintainability, testability, and performance
- Be specific about which files and patterns will be affected
- One approach should be marked as recommended

Output your response as valid JSON with this exact structure:
{
  "approaches": [
    {
      "id": "approach_1",
      "name": "Human-readable name",
      "description": "Detailed description of what this approach entails",
      "pros": ["advantage 1", "advantage 2"],
      "cons": ["disadvantage 1", "disadvantage 2"],
      "complexity": "low|medium|high",
      "recommended": true|false
    }
  ]
}

IMPORTANT: Return ONLY valid JSON, no markdown code blocks or extra text."#;

/// System prompt for generating a detailed specification from a selected approach.
pub const SPEC_SYSTEM_PROMPT: &str = r#"You are a senior software architect. Your task is to create a detailed implementation specification based on the selected approach.

Guidelines:
- Be specific about file paths and function signatures
- Focus on minimal changes needed
- Consider existing patterns in the codebase
- Include any necessary dependencies
- Specify exact code additions and modifications

Output your response as valid JSON with this exact structure:
{
  "task_name": "Name of the task",
  "approach": "Selected approach name",
  "complexity": "low|medium|high",
  "files_to_create": [
    {
      "path": "src/path/to/file.rs",
      "description": "What this file does",
      "exports": [
        {
          "name": "function_name",
          "signature": "fn function_name(arg: Type) -> Result",
          "behavior": ["bullet point 1", "bullet point 2"]
        }
      ]
    }
  ],
  "files_to_modify": [
    {
      "path": "src/existing/file.rs",
      "line": 42,
      "description": "What changes are needed",
      "additions": ["code to add"],
      "removals": ["code to remove"]
    }
  ],
  "dependencies_to_add": ["package-name"]
}

IMPORTANT: Return ONLY valid JSON, no markdown code blocks or extra text."#;

/// Builds a prompt for generating implementation approaches.
pub fn build_approaches_prompt(research: &ResearchDoc) -> String {
    let mut prompt = String::new();

    prompt.push_str("# Task\n\n");
    prompt.push_str(&research.task_name);
    prompt.push_str("\n\n");

    prompt.push_str("# Research Summary\n\n");
    prompt.push_str(&research.summary);
    prompt.push_str("\n\n");

    prompt.push_str("# Codebase Analysis\n\n");
    for finding in &research.codebase_analysis {
        prompt.push_str(&format!("## {}\n\n", finding.title));
        prompt.push_str(&finding.description);
        prompt.push_str("\n\n");
        if !finding.related_files.is_empty() {
            prompt.push_str("Related files:\n");
            for file in &finding.related_files {
                prompt.push_str(&format!("- {}\n", file));
            }
            prompt.push('\n');
        }
    }

    prompt.push_str("# Dependencies\n\n");
    for dep in &research.dependencies {
        let dep_type = if dep.is_external {
            "external"
        } else {
            "internal"
        };
        prompt.push_str(&format!(
            "- {} ({}): {}\n",
            dep.name, dep_type, dep.description
        ));
    }
    prompt.push('\n');

    prompt.push_str("# AI's Suggested Approach\n\n");
    prompt.push_str(&research.suggested_approach);
    prompt.push_str("\n\n");

    prompt.push_str(
        "Based on this research, propose 2-3 implementation approaches with trade-offs.\n",
    );

    prompt
}

/// Builds a prompt for generating a detailed specification from a selected approach.
/// This prompt includes instructions inline (some LLM providers don't handle system prompts well).
pub fn build_spec_prompt(research: &ResearchDoc, approach: &Approach) -> String {
    let mut prompt = String::new();

    // Include instructions inline for better compatibility
    prompt
        .push_str("You are creating an implementation specification. Output ONLY valid JSON.\n\n");

    prompt.push_str("# Task\n\n");
    prompt.push_str(&research.task_name);
    prompt.push_str("\n\n");

    prompt.push_str("# Research Summary\n\n");
    prompt.push_str(&research.summary);
    prompt.push_str("\n\n");

    prompt.push_str("# Selected Approach\n\n");
    prompt.push_str(&format!("**{}**\n\n", approach.name));
    prompt.push_str(&approach.description);
    prompt.push_str("\n\n");

    prompt.push_str("## Trade-offs\n\n");
    prompt.push_str("Pros:\n");
    for pro in &approach.pros {
        prompt.push_str(&format!("- {}\n", pro));
    }
    prompt.push_str("\nCons:\n");
    for con in &approach.cons {
        prompt.push_str(&format!("- {}\n", con));
    }
    prompt.push('\n');

    prompt.push_str("# Relevant Files from Research\n\n");
    for finding in &research.codebase_analysis {
        if !finding.related_files.is_empty() {
            for file in &finding.related_files {
                prompt.push_str(&format!("- {}\n", file));
            }
        }
    }
    prompt.push('\n');

    prompt.push_str("# Dependencies Identified\n\n");
    for dep in &research.dependencies {
        let dep_type = if dep.is_external {
            "external"
        } else {
            "internal"
        };
        prompt.push_str(&format!(
            "- {} ({}): {}\n",
            dep.name, dep_type, dep.description
        ));
    }
    prompt.push('\n');

    prompt
        .push_str("Generate a detailed implementation specification for the selected approach.\n");
    prompt
        .push_str("Be specific about exact file paths, function signatures, and code changes.\n\n");

    // Add JSON format inline for better compatibility with various LLM providers
    prompt.push_str(
        r#"Output ONLY valid JSON with this structure:
{
  "task_name": "Name of the task",
  "approach": "Selected approach name",
  "complexity": "low|medium|high",
  "files_to_create": [
    {"path": "src/path.rs", "description": "What it does", "exports": []}
  ],
  "files_to_modify": [
    {"path": "src/existing.rs", "description": "Changes needed", "additions": [], "removals": []}
  ],
  "dependencies_to_add": []
}

Return ONLY the JSON, no markdown or explanation."#,
    );

    prompt
}
