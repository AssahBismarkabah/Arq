use serde::{Deserialize, Serialize};

/// The output of the Planning phase.
///
/// A specification that defines exactly what the Agent phase will implement.
/// This serves as a contract between user approval and code generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Name of the task this plan is for
    pub task_name: String,
    /// The selected approach (from options presented to user)
    pub approach: String,
    /// Complexity rating of this plan
    pub complexity: Complexity,
    /// Files to create
    pub files_to_create: Vec<FileSpec>,
    /// Files to modify
    pub files_to_modify: Vec<FileModification>,
    /// Dependencies to add (package names)
    pub dependencies_to_add: Vec<String>,
}

impl Plan {
    /// Creates a new empty plan for a task.
    pub fn new(task_name: impl Into<String>, approach: impl Into<String>) -> Self {
        Self {
            task_name: task_name.into(),
            approach: approach.into(),
            complexity: Complexity::Medium,
            files_to_create: Vec::new(),
            files_to_modify: Vec::new(),
            dependencies_to_add: Vec::new(),
        }
    }

    /// Converts the plan to YAML format.
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Parses a plan from YAML format.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Returns the total number of files affected by this plan.
    pub fn total_files_affected(&self) -> usize {
        self.files_to_create.len() + self.files_to_modify.len()
    }
}

/// Complexity rating for a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Complexity {
    Low,
    Medium,
    High,
}

impl Complexity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Complexity::Low => "Low",
            Complexity::Medium => "Medium",
            Complexity::High => "High",
        }
    }
}

/// Specification for a file to create.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSpec {
    /// Path where the file should be created
    pub path: String,
    /// Description of the file's purpose
    pub description: String,
    /// Functions/exports this file will contain
    pub exports: Vec<FunctionSignature>,
}

/// Specification for a file modification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileModification {
    /// Path of the file to modify
    pub path: String,
    /// Approximate line number for the change
    pub line: Option<u32>,
    /// Description of what changes will be made
    pub description: String,
    /// Code to add (imports, function calls, etc.)
    pub additions: Vec<String>,
    /// Code to remove (if any)
    pub removals: Vec<String>,
}

/// A function signature specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// Function name
    pub name: String,
    /// Full signature (parameters and return type)
    pub signature: String,
    /// Description of behavior
    pub behavior: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_to_yaml() {
        let plan = Plan::new("rate-limiting", "Global middleware");
        let yaml = plan.to_yaml().unwrap();
        assert!(yaml.contains("rate-limiting"));
        assert!(yaml.contains("Global middleware"));
    }

    #[test]
    fn test_plan_roundtrip() {
        let mut plan = Plan::new("rate-limiting", "Global middleware");
        plan.complexity = Complexity::Low;
        plan.files_to_create.push(FileSpec {
            path: "src/middleware/rateLimit.ts".to_string(),
            description: "Rate limiting middleware".to_string(),
            exports: vec![],
        });

        let yaml = plan.to_yaml().unwrap();
        let parsed = Plan::from_yaml(&yaml).unwrap();

        assert_eq!(parsed.task_name, plan.task_name);
        assert_eq!(parsed.approach, plan.approach);
        assert_eq!(parsed.complexity, plan.complexity);
        assert_eq!(parsed.files_to_create.len(), 1);
    }
}
