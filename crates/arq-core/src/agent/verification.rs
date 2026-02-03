//! Language-agnostic project verification.
//!
//! Detects project type and provides verification command hints.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Detected project type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectType {
    /// Rust project (Cargo.toml)
    Rust,
    /// Node.js project (package.json)
    Node,
    /// Python project (pyproject.toml, setup.py, requirements.txt)
    Python,
    /// Go project (go.mod)
    Go,
    /// Java project (pom.xml, build.gradle)
    Java,
    /// C# project (.csproj)
    CSharp,
    /// Unknown project type
    Unknown,
}

impl ProjectType {
    /// Get a string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectType::Rust => "Rust",
            ProjectType::Node => "Node.js",
            ProjectType::Python => "Python",
            ProjectType::Go => "Go",
            ProjectType::Java => "Java",
            ProjectType::CSharp => "C#",
            ProjectType::Unknown => "Unknown",
        }
    }
}

/// Verification commands for a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCommands {
    /// The detected project type.
    pub project_type: ProjectType,
    /// Build/compile command (if applicable).
    pub build_command: Option<String>,
    /// Test command (if applicable).
    pub test_command: Option<String>,
    /// Lint command (if applicable).
    pub lint_command: Option<String>,
    /// Type check command (if applicable).
    pub type_check_command: Option<String>,
}

impl VerificationCommands {
    /// Get all non-empty commands as a list.
    pub fn all_commands(&self) -> Vec<&str> {
        let mut commands = Vec::new();
        if let Some(ref cmd) = self.build_command {
            commands.push(cmd.as_str());
        }
        if let Some(ref cmd) = self.type_check_command {
            commands.push(cmd.as_str());
        }
        if let Some(ref cmd) = self.lint_command {
            commands.push(cmd.as_str());
        }
        if let Some(ref cmd) = self.test_command {
            commands.push(cmd.as_str());
        }
        commands
    }

    /// Get a human-readable hint for the agent.
    pub fn hint(&self) -> String {
        match self.project_type {
            ProjectType::Rust => {
                "This is a Rust project. Use `cargo check` to verify compilation and `cargo test` to run tests.".to_string()
            }
            ProjectType::Node => {
                "This is a Node.js project. Use `npm run build` to build and `npm test` to run tests.".to_string()
            }
            ProjectType::Python => {
                "This is a Python project. Use `python -m pytest` to run tests.".to_string()
            }
            ProjectType::Go => {
                "This is a Go project. Use `go build ./...` to build and `go test ./...` to run tests.".to_string()
            }
            ProjectType::Java => {
                "This is a Java project. Use `mvn compile` to build and `mvn test` to run tests.".to_string()
            }
            ProjectType::CSharp => {
                "This is a C# project. Use `dotnet build` to build and `dotnet test` to run tests.".to_string()
            }
            ProjectType::Unknown => {
                "Unable to detect project type. Check for config files to determine build commands.".to_string()
            }
        }
    }
}

/// Detect the project type from files in the directory.
pub fn detect_project_type(root: &Path) -> ProjectType {
    // Check for Rust
    if root.join("Cargo.toml").exists() {
        return ProjectType::Rust;
    }

    // Check for Node.js
    if root.join("package.json").exists() {
        return ProjectType::Node;
    }

    // Check for Python
    if root.join("pyproject.toml").exists()
        || root.join("setup.py").exists()
        || root.join("requirements.txt").exists()
    {
        return ProjectType::Python;
    }

    // Check for Go
    if root.join("go.mod").exists() {
        return ProjectType::Go;
    }

    // Check for Java
    if root.join("pom.xml").exists()
        || root.join("build.gradle").exists()
        || root.join("build.gradle.kts").exists()
    {
        return ProjectType::Java;
    }

    // Check for C#
    if has_csproj(root) {
        return ProjectType::CSharp;
    }

    ProjectType::Unknown
}

/// Get verification commands for a project type.
pub fn get_verification_commands(project_type: ProjectType) -> VerificationCommands {
    match project_type {
        ProjectType::Rust => VerificationCommands {
            project_type,
            build_command: Some("cargo check".to_string()),
            test_command: Some("cargo test".to_string()),
            lint_command: Some("cargo clippy".to_string()),
            type_check_command: None, // Built into cargo check
        },
        ProjectType::Node => VerificationCommands {
            project_type,
            build_command: Some("npm run build".to_string()),
            test_command: Some("npm test".to_string()),
            lint_command: Some("npm run lint".to_string()),
            type_check_command: Some("npx tsc --noEmit".to_string()),
        },
        ProjectType::Python => VerificationCommands {
            project_type,
            build_command: None,
            test_command: Some("python -m pytest".to_string()),
            lint_command: Some("ruff check .".to_string()),
            type_check_command: Some("mypy .".to_string()),
        },
        ProjectType::Go => VerificationCommands {
            project_type,
            build_command: Some("go build ./...".to_string()),
            test_command: Some("go test ./...".to_string()),
            lint_command: Some("golangci-lint run".to_string()),
            type_check_command: None,
        },
        ProjectType::Java => VerificationCommands {
            project_type,
            build_command: Some("mvn compile".to_string()),
            test_command: Some("mvn test".to_string()),
            lint_command: None,
            type_check_command: None,
        },
        ProjectType::CSharp => VerificationCommands {
            project_type,
            build_command: Some("dotnet build".to_string()),
            test_command: Some("dotnet test".to_string()),
            lint_command: None,
            type_check_command: None,
        },
        ProjectType::Unknown => VerificationCommands {
            project_type,
            build_command: None,
            test_command: None,
            lint_command: None,
            type_check_command: None,
        },
    }
}

/// Detect project type and get verification commands in one call.
pub fn detect_and_get_commands(root: &Path) -> VerificationCommands {
    let project_type = detect_project_type(root);
    get_verification_commands(project_type)
}

/// Check if directory contains any .csproj files.
fn has_csproj(root: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.filter_map(Result::ok) {
            if entry
                .path()
                .extension()
                .map(|e| e == "csproj")
                .unwrap_or(false)
            {
                return true;
            }
        }
    }
    false
}
