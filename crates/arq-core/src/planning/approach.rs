use serde::{Deserialize, Serialize};

use super::Complexity;

/// A possible implementation approach for a task.
///
/// The planning phase generates multiple approaches for the user to choose from,
/// each with trade-offs in terms of complexity, flexibility, and scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approach {
    /// Unique identifier for this approach (e.g., "approach_1")
    pub id: String,
    /// Human-readable name (e.g., "Minimal Implementation")
    pub name: String,
    /// Detailed description of what this approach entails
    pub description: String,
    /// Advantages of this approach
    pub pros: Vec<String>,
    /// Disadvantages or trade-offs
    pub cons: Vec<String>,
    /// Estimated complexity
    pub complexity: Complexity,
    /// Whether this is the recommended approach
    pub recommended: bool,
}

impl Approach {
    /// Creates a new approach with the given id and name.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            pros: Vec::new(),
            cons: Vec::new(),
            complexity: Complexity::Medium,
            recommended: false,
        }
    }

    /// Sets this approach as recommended.
    pub fn set_recommended(mut self) -> Self {
        self.recommended = true;
        self
    }

    /// Sets the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Sets the complexity.
    pub fn with_complexity(mut self, complexity: Complexity) -> Self {
        self.complexity = complexity;
        self
    }

    /// Formats the approach for display in the TUI.
    pub fn to_display_string(&self, index: usize) -> String {
        let mut s = String::new();

        // Header with index and name
        let rec = if self.recommended { " (Recommended)" } else { "" };
        s.push_str(&format!("[{}] {}{}\n", index + 1, self.name, rec));

        // Description
        s.push_str(&format!("    {}\n", self.description));

        // Pros
        for pro in &self.pros {
            s.push_str(&format!("    + {}\n", pro));
        }

        // Cons
        for con in &self.cons {
            s.push_str(&format!("    - {}\n", con));
        }

        // Complexity
        s.push_str(&format!("    Complexity: {}\n", self.complexity.as_str()));

        s
    }
}

/// A collection of possible approaches for a task.
#[derive(Debug, Clone, Default)]
pub struct ApproachOptions {
    /// The available approaches (typically 2-3)
    pub approaches: Vec<Approach>,
    /// ID of the recommended approach (if any)
    pub recommended_id: Option<String>,
}

impl ApproachOptions {
    /// Creates a new empty options set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an approach.
    pub fn add(&mut self, approach: Approach) {
        if approach.recommended {
            self.recommended_id = Some(approach.id.clone());
        }
        self.approaches.push(approach);
    }

    /// Gets an approach by index.
    pub fn get(&self, index: usize) -> Option<&Approach> {
        self.approaches.get(index)
    }

    /// Gets the recommended approach if one exists.
    pub fn recommended(&self) -> Option<&Approach> {
        self.recommended_id
            .as_ref()
            .and_then(|id| self.approaches.iter().find(|a| &a.id == id))
    }

    /// Returns the number of approaches.
    pub fn len(&self) -> usize {
        self.approaches.len()
    }

    /// Returns true if there are no approaches.
    pub fn is_empty(&self) -> bool {
        self.approaches.is_empty()
    }

    /// Formats all approaches for display.
    pub fn to_display_string(&self) -> String {
        let mut s = String::new();
        s.push_str("Based on your research, here are possible approaches:\n\n");

        for (i, approach) in self.approaches.iter().enumerate() {
            s.push_str(&approach.to_display_string(i));
            s.push('\n');
        }

        s.push_str("Type 1-");
        s.push_str(&self.approaches.len().to_string());
        s.push_str(" to select, or describe your own approach:");

        s
    }
}
