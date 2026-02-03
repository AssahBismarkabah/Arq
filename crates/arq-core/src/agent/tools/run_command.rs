//! RunCommand tool - executes shell commands.

use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use tokio::process::Command;

use super::{Tool, ToolContext, ToolDefinition, ToolResult};

/// Tool for executing shell commands.
pub struct RunCommandTool;

/// Default timeout for commands (60 seconds).
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Maximum timeout for commands (5 minutes).
const MAX_TIMEOUT_SECS: u64 = 300;

#[async_trait]
impl Tool for RunCommandTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "run_command".to_string(),
            description: "Execute a shell command in the project directory. \
                         Use this to run build commands, tests, linters, formatters, etc. \
                         Commands are run with a timeout and some dangerous patterns are blocked."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to run"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 60, max: 300)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, context: &ToolContext) -> ToolResult {
        let command = match args.get("command").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ToolResult::failure("Missing required parameter: command"),
        };

        let timeout_secs = args
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .map(|t| t.min(MAX_TIMEOUT_SECS))
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        // Check for blocked command patterns
        for blocked in &context.blocked_commands {
            if command.contains(blocked) {
                return ToolResult::failure(format!(
                    "Command blocked: contains dangerous pattern '{}'",
                    blocked
                ));
            }
        }

        if context.dry_run {
            return ToolResult::success(format!(
                "[DRY RUN] Would run: {}\nTimeout: {}s",
                command, timeout_secs
            ));
        }

        // Execute the command
        let result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            execute_command(command, &context.root),
        )
        .await;

        match result {
            Ok(Ok((stdout, stderr, exit_code))) => {
                let output = format_output(&stdout, &stderr, exit_code);

                if exit_code == 0 {
                    ToolResult::success(output)
                } else {
                    // Command failed but executed - return output so LLM can see errors
                    ToolResult {
                        success: false,
                        output,
                        error: Some(format!("Command exited with code {}", exit_code)),
                        modified_files: Vec::new(),
                    }
                }
            }
            Ok(Err(e)) => ToolResult::failure(format!("Failed to execute command: {}", e)),
            Err(_) => ToolResult::failure(format!(
                "Command timed out after {} seconds. Consider increasing timeout_secs or breaking the command into smaller steps.",
                timeout_secs
            )),
        }
    }

    fn requires_confirmation(&self) -> bool {
        true // Running commands needs confirmation
    }
}

/// Execute a shell command and return stdout, stderr, and exit code.
async fn execute_command(
    command: &str,
    cwd: &std::path::Path,
) -> Result<(String, String, i32), std::io::Error> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok((stdout, stderr, exit_code))
}

/// Format command output for display.
fn format_output(stdout: &str, stderr: &str, exit_code: i32) -> String {
    let mut output = format!("Exit code: {}\n", exit_code);

    if !stdout.is_empty() {
        output.push_str(&format!(
            "\n--- STDOUT ---\n{}",
            truncate_output(stdout, 5000)
        ));
    }

    if !stderr.is_empty() {
        output.push_str(&format!(
            "\n--- STDERR ---\n{}",
            truncate_output(stderr, 2000)
        ));
    }

    if stdout.is_empty() && stderr.is_empty() {
        output.push_str("\n(no output)");
    }

    output
}

/// Truncate output if too long.
fn truncate_output(output: &str, max_len: usize) -> &str {
    if output.len() <= max_len {
        output
    } else {
        &output[..max_len]
    }
}
