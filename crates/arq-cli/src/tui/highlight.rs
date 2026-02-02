//! Syntax highlighting using syntect.
//!
//! Provides professional syntax highlighting for code blocks in the TUI chat.
//! Uses lazy-loaded syntax and theme sets for efficient startup.

use once_cell::sync::Lazy;
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, Style as SyntectStyle, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};

/// Lazy-loaded syntax set with all default syntaxes.
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);

/// Lazy-loaded theme set with default themes.
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Get the theme name to use for highlighting.
const THEME_NAME: &str = "base16-ocean.dark";

/// Highlighter state for tracking code blocks across lines.
pub struct Highlighter<'a> {
    in_code_block: bool,
    current_lang: Option<String>,
    highlight_lines: Option<HighlightLines<'a>>,
}

impl<'a> Default for Highlighter<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Highlighter<'a> {
    /// Create a new highlighter.
    pub fn new() -> Self {
        Self {
            in_code_block: false,
            current_lang: None,
            highlight_lines: None,
        }
    }

    /// Highlight a single line of text.
    ///
    /// Tracks code block state (``` markers) and applies appropriate syntax highlighting.
    /// Returns a vector of styled spans for ratatui rendering.
    pub fn highlight_line(&mut self, line: &str) -> Vec<Span<'static>> {
        let trimmed = line.trim();

        // Check for code block markers
        if trimmed.starts_with("```") {
            if self.in_code_block {
                // Closing code block
                self.in_code_block = false;
                self.current_lang = None;
                self.highlight_lines = None;
            } else {
                // Opening code block
                self.in_code_block = true;
                let lang = trimmed.strip_prefix("```").map(|s| s.trim().to_string());
                self.current_lang = lang.clone();

                // Initialize highlighter for the detected language
                if let Some(syntax) = self.detect_syntax(lang.as_deref()) {
                    let theme = &THEME_SET.themes[THEME_NAME];
                    self.highlight_lines = Some(HighlightLines::new(syntax, theme));
                }
            }

            // Return the marker line in dim style
            return vec![Span::styled(
                line.to_string(),
                Style::default().fg(Color::DarkGray),
            )];
        }

        // Inside code block - use syntax highlighting
        if self.in_code_block {
            return self.highlight_code_line(line);
        }

        // Outside code block - check for structured content
        if Self::looks_like_structured(trimmed) {
            return self.highlight_structured_line(line);
        }

        // Default - plain text
        vec![Span::raw(line.to_string())]
    }

    /// Detect syntax from language identifier.
    fn detect_syntax(&self, lang: Option<&str>) -> Option<&'static SyntaxReference> {
        let lang = lang?;
        let lang = lang.to_lowercase();

        // Try by extension first
        if let Some(syntax) = SYNTAX_SET.find_syntax_by_extension(&lang) {
            return Some(syntax);
        }

        // Try by name
        if let Some(syntax) = SYNTAX_SET.find_syntax_by_name(&lang) {
            return Some(syntax);
        }

        // Common aliases
        let alias = match lang.as_str() {
            "yml" => "yaml",
            "rs" => "rust",
            "py" => "python",
            "js" => "javascript",
            "ts" => "typescript",
            "rb" => "ruby",
            "sh" | "bash" | "zsh" => "shell",
            "md" => "markdown",
            "dockerfile" => "docker",
            _ => return None,
        };

        SYNTAX_SET.find_syntax_by_name(alias)
    }

    /// Highlight a line inside a code block using syntect.
    fn highlight_code_line(&mut self, line: &str) -> Vec<Span<'static>> {
        if let Some(ref mut hl) = self.highlight_lines {
            match hl.highlight_line(line, &SYNTAX_SET) {
                Ok(ranges) => {
                    return ranges
                        .into_iter()
                        .map(|(style, text)| {
                            Span::styled(text.to_string(), syntect_to_ratatui(style))
                        })
                        .collect();
                }
                Err(_) => {
                    // Fall back to plain text on error
                    return vec![Span::raw(line.to_string())];
                }
            }
        }

        // No highlighter available - return plain text
        vec![Span::raw(line.to_string())]
    }

    /// Check if a line looks like structured content (YAML-like).
    fn looks_like_structured(line: &str) -> bool {
        // Has a key: value pattern (but not URL)
        if let Some(colon_pos) = line.find(':') {
            let before_colon = &line[..colon_pos];
            if !before_colon.ends_with("http")
                && !before_colon.ends_with("https")
                && !before_colon.ends_with("file")
            {
                return true;
            }
        }
        // List items
        line.trim_start().starts_with("- ") || line.trim_start().starts_with("* ")
    }

    /// Highlight structured content (YAML-like) using syntect.
    fn highlight_structured_line(&mut self, line: &str) -> Vec<Span<'static>> {
        // Use YAML syntax for structured content
        let syntax = SYNTAX_SET
            .find_syntax_by_extension("yaml")
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
        let theme = &THEME_SET.themes[THEME_NAME];
        let mut hl = HighlightLines::new(syntax, theme);

        match hl.highlight_line(line, &SYNTAX_SET) {
            Ok(ranges) => ranges
                .into_iter()
                .map(|(style, text)| Span::styled(text.to_string(), syntect_to_ratatui(style)))
                .collect(),
            Err(_) => vec![Span::raw(line.to_string())],
        }
    }
}

/// Convert syntect Style to ratatui Style.
/// Note: We intentionally skip background colors for cleaner TUI appearance.
fn syntect_to_ratatui(style: SyntectStyle) -> Style {
    let mut ratatui_style = Style::default();

    // Foreground color only (skip background for cleaner TUI look)
    let fg = style.foreground;
    ratatui_style = ratatui_style.fg(Color::Rgb(fg.r, fg.g, fg.b));

    // Font style modifiers
    let font_style = style.font_style;
    if font_style.contains(FontStyle::BOLD) {
        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
    }
    if font_style.contains(FontStyle::ITALIC) {
        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
    }
    if font_style.contains(FontStyle::UNDERLINE) {
        ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
    }

    ratatui_style
}

/// Highlight markdown headers and bold text.
pub fn highlight_markdown(line: &str) -> Option<Vec<Span<'static>>> {
    let trimmed = line.trim();

    // Headers
    if trimmed.starts_with("## ") || trimmed.starts_with("# ") {
        return Some(vec![Span::styled(
            line.to_string(),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )]);
    }

    // Bold text **text**
    if trimmed.starts_with("**") && trimmed.ends_with("**") {
        return Some(vec![Span::styled(
            line.to_string(),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )]);
    }

    None
}
