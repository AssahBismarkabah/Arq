//! HTML template rendering for the graph visualization.
//!
//! Templates are stored as separate files for maintainability:
//! - `templates/index.html` - HTML structure
//! - `templates/styles.css` - CSS styles
//! - `templates/app.js` - JavaScript application code
//!
//! Files are embedded at compile time using `include_str!`.

use std::path::Path;

// Embed template files at compile time
const HTML_TEMPLATE: &str = include_str!("templates/index.html");
const STYLES: &str = include_str!("templates/styles.css");
const SCRIPT: &str = include_str!("templates/app.js");

/// Render the graph visualization page.
///
/// Assembles the final HTML by substituting placeholders in the template:
/// - `{{PROJECT_NAME}}` - Display name for the project
/// - `{{STYLES}}` - CSS styles
/// - `{{SCRIPT}}` - JavaScript code
pub fn render_graph_page(project_path: &Path) -> String {
    let project_name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Project");

    HTML_TEMPLATE
        .replace("{{PROJECT_NAME}}", &html_escape(project_name))
        .replace("{{STYLES}}", STYLES)
        .replace("{{SCRIPT}}", SCRIPT)
}

/// Escape HTML special characters to prevent XSS.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
