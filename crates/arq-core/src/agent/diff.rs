//! Diff generator for agent phase.
//!
//! Generates unified diffs for file modifications and provides
//! structured diff output for TUI display.

use similar::{ChangeTag, TextDiff};

/// A line in a diff with its change type.
#[derive(Debug, Clone)]
pub struct DiffLine {
    /// The type of change for this line.
    pub change_type: DiffLineType,
    /// The content of the line.
    pub content: String,
}

/// Type of change for a diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineType {
    /// Line is unchanged (context).
    Context,
    /// Line was added.
    Added,
    /// Line was removed.
    Removed,
    /// Header line (file info, hunk header).
    Header,
}

/// A complete diff between two versions of a file.
#[derive(Debug, Clone)]
pub struct FileDiff {
    /// Path to the file.
    pub path: String,
    /// Whether this is a new file.
    pub is_new: bool,
    /// Whether this is a deleted file.
    pub is_deleted: bool,
    /// All diff lines.
    pub lines: Vec<DiffLine>,
    /// Statistics.
    pub stats: DiffStats,
}

/// Statistics about a diff.
#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    /// Number of lines added.
    pub additions: usize,
    /// Number of lines removed.
    pub deletions: usize,
}

/// Diff generator that creates diffs between file versions.
pub struct DiffGenerator;

impl DiffGenerator {
    /// Create a new diff generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate a diff between two versions of a file.
    pub fn generate(&self, path: &str, old: &str, new: &str) -> FileDiff {
        let diff = TextDiff::from_lines(old, new);
        let mut lines = Vec::new();
        let mut stats = DiffStats::default();

        for change in diff.iter_all_changes() {
            let (line_type, count_add, count_del) = match change.tag() {
                ChangeTag::Equal => (DiffLineType::Context, false, false),
                ChangeTag::Insert => (DiffLineType::Added, true, false),
                ChangeTag::Delete => (DiffLineType::Removed, false, true),
            };

            if count_add {
                stats.additions += 1;
            }
            if count_del {
                stats.deletions += 1;
            }

            lines.push(DiffLine {
                change_type: line_type,
                content: change.value().to_string(),
            });
        }

        FileDiff {
            path: path.to_string(),
            is_new: old.is_empty(),
            is_deleted: new.is_empty(),
            lines,
            stats,
        }
    }

    /// Generate a diff for a new file (all additions).
    pub fn generate_create(&self, path: &str, content: &str) -> FileDiff {
        self.generate(path, "", content)
    }

    /// Generate a diff for a deleted file (all deletions).
    pub fn generate_delete(&self, path: &str, content: &str) -> FileDiff {
        self.generate(path, content, "")
    }

    /// Format a diff as a unified diff string.
    pub fn format_unified(&self, path: &str, old: &str, new: &str) -> String {
        let diff = TextDiff::from_lines(old, new);

        let old_header = if old.is_empty() {
            "/dev/null".to_string()
        } else {
            format!("a/{}", path)
        };

        let new_header = if new.is_empty() {
            "/dev/null".to_string()
        } else {
            format!("b/{}", path)
        };

        diff.unified_diff()
            .context_radius(3)
            .header(&old_header, &new_header)
            .to_string()
    }

    /// Format diff with ANSI colors for terminal display.
    pub fn format_colored(&self, diff: &FileDiff) -> String {
        const RED: &str = "\x1b[31m";
        const GREEN: &str = "\x1b[32m";
        const CYAN: &str = "\x1b[36m";
        const RESET: &str = "\x1b[0m";

        let mut output = String::new();

        // File header (cyan)
        if diff.is_new {
            output.push_str(&format!("{}--- /dev/null{}\n", CYAN, RESET));
            output.push_str(&format!("{}+++ b/{}{}\n", CYAN, diff.path, RESET));
        } else if diff.is_deleted {
            output.push_str(&format!("{}--- a/{}{}\n", CYAN, diff.path, RESET));
            output.push_str(&format!("{}+++ /dev/null{}\n", CYAN, RESET));
        } else {
            output.push_str(&format!("{}--- a/{}{}\n", CYAN, diff.path, RESET));
            output.push_str(&format!("{}+++ b/{}{}\n", CYAN, diff.path, RESET));
        }

        // Lines
        for line in &diff.lines {
            let content = &line.content;
            let content_display = if content.ends_with('\n') {
                content.to_string()
            } else {
                format!("{}\n", content)
            };

            match line.change_type {
                DiffLineType::Context => {
                    output.push_str(&format!(" {}", content_display));
                }
                DiffLineType::Added => {
                    output.push_str(&format!("{}+{}{}", GREEN, content_display, RESET));
                }
                DiffLineType::Removed => {
                    output.push_str(&format!("{}-{}{}", RED, content_display, RESET));
                }
                DiffLineType::Header => {
                    output.push_str(&format!("{}{}{}", CYAN, content_display, RESET));
                }
            }
        }

        output
    }

    /// Get a summary string for the diff.
    pub fn summary(&self, diff: &FileDiff) -> String {
        if diff.is_new {
            format!("{} (new file, +{} lines)", diff.path, diff.stats.additions)
        } else if diff.is_deleted {
            format!("{} (deleted, -{} lines)", diff.path, diff.stats.deletions)
        } else {
            format!(
                "{} (+{}, -{})",
                diff.path, diff.stats.additions, diff.stats.deletions
            )
        }
    }

    /// Check if there are any changes in the diff.
    pub fn has_changes(&self, diff: &FileDiff) -> bool {
        diff.stats.additions > 0 || diff.stats.deletions > 0
    }
}

impl Default for DiffGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_modify() {
        let generator = DiffGenerator::new();
        let old = "line 1\nline 2\nline 3\n";
        let new = "line 1\nmodified line 2\nline 3\n";

        let diff = generator.generate("test.rs", old, new);

        assert!(!diff.is_new);
        assert!(!diff.is_deleted);
        assert_eq!(diff.stats.additions, 1);
        assert_eq!(diff.stats.deletions, 1);
    }

    #[test]
    fn test_generate_create() {
        let generator = DiffGenerator::new();
        let content = "new content\n";

        let diff = generator.generate_create("new_file.rs", content);

        assert!(diff.is_new);
        assert_eq!(diff.stats.additions, 1);
        assert_eq!(diff.stats.deletions, 0);
    }

    #[test]
    fn test_generate_delete() {
        let generator = DiffGenerator::new();
        let content = "old content\n";

        let diff = generator.generate_delete("old_file.rs", content);

        assert!(diff.is_deleted);
        assert_eq!(diff.stats.additions, 0);
        assert_eq!(diff.stats.deletions, 1);
    }

    #[test]
    fn test_format_unified() {
        let generator = DiffGenerator::new();
        let old = "hello\n";
        let new = "hello\nworld\n";

        let formatted = generator.format_unified("test.txt", old, new);

        assert!(formatted.contains("--- a/test.txt"));
        assert!(formatted.contains("+++ b/test.txt"));
        assert!(formatted.contains("+world"));
    }

    #[test]
    fn test_summary() {
        let generator = DiffGenerator::new();

        let new_file = generator.generate_create("new.rs", "content\n");
        assert!(generator.summary(&new_file).contains("new file"));

        let deleted = generator.generate_delete("old.rs", "content\n");
        assert!(generator.summary(&deleted).contains("deleted"));

        let modified = generator.generate("mod.rs", "old\n", "new\n");
        assert!(generator.summary(&modified).contains("+1"));
        assert!(generator.summary(&modified).contains("-1"));
    }

    #[test]
    fn test_has_changes() {
        let generator = DiffGenerator::new();

        let same = generator.generate("same.rs", "content\n", "content\n");
        assert!(!generator.has_changes(&same));

        let changed = generator.generate("changed.rs", "old\n", "new\n");
        assert!(generator.has_changes(&changed));
    }
}
