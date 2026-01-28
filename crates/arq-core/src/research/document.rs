use serde::{Deserialize, Serialize};

/// The output of the Research phase.
///
/// Contains validated understanding of the codebase and context
/// relevant to the user's task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchDoc {
    /// Name of the task this research is for
    pub task_name: String,
    /// High-level summary of findings
    pub summary: String,
    /// Findings from codebase analysis
    pub codebase_analysis: Vec<Finding>,
    /// Dependencies identified (internal and external)
    pub dependencies: Vec<Dependency>,
    /// AI's suggested approach based on research
    pub suggested_approach: String,
    /// Sources referenced during research
    pub sources: Vec<Source>,
}

impl ResearchDoc {
    /// Creates a new empty research document for a task.
    pub fn new(task_name: impl Into<String>) -> Self {
        Self {
            task_name: task_name.into(),
            summary: String::new(),
            codebase_analysis: Vec::new(),
            dependencies: Vec::new(),
            suggested_approach: String::new(),
            sources: Vec::new(),
        }
    }

    /// Converts the research document to markdown format.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str(&format!("# Research: {}\n\n", self.task_name));

        md.push_str("## Summary\n\n");
        md.push_str(&self.summary);
        md.push_str("\n\n");

        md.push_str("## Codebase Analysis\n\n");
        for finding in &self.codebase_analysis {
            md.push_str(&format!("### {}\n\n", finding.title));
            md.push_str(&finding.description);
            md.push_str("\n\n");
        }

        md.push_str("## Dependencies\n\n");
        for dep in &self.dependencies {
            let dep_type = if dep.is_external { "external" } else { "internal" };
            md.push_str(&format!("- **{}** ({}): {}\n", dep.name, dep_type, dep.description));
        }
        md.push_str("\n");

        md.push_str("## Suggested Approach\n\n");
        md.push_str(&self.suggested_approach);
        md.push_str("\n\n");

        md.push_str("## Sources\n\n");
        for source in &self.sources {
            md.push_str(&format!("- {}: {}\n", source.source_type.as_str(), source.location));
        }

        md
    }
}

/// A finding from codebase analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Title of the finding
    pub title: String,
    /// Detailed description
    pub description: String,
    /// File paths related to this finding
    pub related_files: Vec<String>,
}

/// A dependency identified during research.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Name of the dependency
    pub name: String,
    /// Description of what it does
    pub description: String,
    /// Whether this is an external package or internal module
    pub is_external: bool,
}

/// A source referenced during research.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// Type of source
    pub source_type: SourceType,
    /// Location (file path, URL, etc.)
    pub location: String,
}

/// Type of source referenced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    /// A file in the codebase
    File,
    /// A URL (documentation, Stack Overflow, etc.)
    Web,
    /// A Slack message or thread
    Slack,
    /// A Confluence page
    Confluence,
    /// Git history (commit, blame, etc.)
    Git,
}

impl SourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceType::File => "File",
            SourceType::Web => "Web",
            SourceType::Slack => "Slack",
            SourceType::Confluence => "Confluence",
            SourceType::Git => "Git",
        }
    }
}

