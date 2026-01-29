use clap::{Parser, Subcommand};
use arq_core::{
    Config, ContextBuilder, FileStorage, Phase, Provider, ResearchRunner, TaskManager,
};

const ARQ_DIR: &str = ".arq";

#[derive(Parser)]
#[command(name = "arq")]
#[command(about = "Spec-driven AI coding tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a new task
    New {
        /// Description of what you want to build
        #[arg(required = true)]
        prompt: Vec<String>,
    },
    /// Show current task status
    Status,
    /// List all tasks
    List,
    /// Delete a task
    Delete {
        /// Task ID to delete
        id: String,
    },
    /// Switch to a different task
    Switch {
        /// Task ID to switch to
        id: String,
    },
    /// Run research phase for current task
    Research,
    /// Advance to the next phase
    Advance,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let storage = FileStorage::new(ARQ_DIR);
    let mut manager = TaskManager::new(storage);

    match cli.command {
        Commands::New { prompt } => {
            let prompt_str = prompt.join(" ");
            let task = manager.create_task(&prompt_str)?;
            println!("Created new task: {}", task.name);
            println!("  ID: {}", task.id);
            println!("  Phase: {}", task.phase.display_name());
            println!("  Prompt: {}", task.prompt);
            println!("\nTask saved to {}/tasks/{}/", ARQ_DIR, task.id);
            println!("\nNext: Run 'arq research' to analyze the codebase.");
        }
        Commands::Status => {
            match manager.get_current_task()? {
                Some(task) => {
                    println!("Current task: {}", task.name);
                    println!("  ID: {}", task.id);
                    println!("  Phase: {}", task.phase.display_name());
                    println!("  Prompt: {}", task.prompt);
                    println!("  Created: {}", task.created_at.format("%Y-%m-%d %H:%M"));
                    println!("  Updated: {}", task.updated_at.format("%Y-%m-%d %H:%M"));

                    if task.research_doc.is_some() {
                        println!("  Research: Complete");
                    }
                    if task.plan.is_some() {
                        println!("  Plan: Complete");
                    }

                    // Show next action
                    match task.phase {
                        Phase::Research => {
                            if task.research_doc.is_some() {
                                println!("\nNext: Run 'arq advance' to move to Planning phase.");
                            } else {
                                println!("\nNext: Run 'arq research' to analyze the codebase.");
                            }
                        }
                        Phase::Planning => {
                            println!("\nNext: Planning phase (coming soon).");
                        }
                        Phase::Agent => {
                            println!("\nNext: Agent phase (coming soon).");
                        }
                        Phase::Complete => {
                            println!("\nTask complete!");
                        }
                    }
                }
                None => {
                    println!("No active task. Use 'arq new <prompt>' to start.");
                }
            }
        }
        Commands::List => {
            let tasks = manager.list_tasks()?;
            if tasks.is_empty() {
                println!("No tasks found. Use 'arq new <prompt>' to create one.");
            } else {
                println!("Tasks:\n");
                for task in tasks {
                    println!(
                        "  {} - {} ({})",
                        &task.id[..8],
                        task.name,
                        task.phase.display_name()
                    );
                }
            }
        }
        Commands::Delete { id } => {
            let tasks = manager.list_tasks()?;
            let matching: Vec<_> = tasks.iter().filter(|t| t.id.starts_with(&id)).collect();

            match matching.len() {
                0 => {
                    println!("No task found with ID starting with '{}'", id);
                }
                1 => {
                    let task_id = &matching[0].id;
                    manager.delete_task(task_id)?;
                    println!("Deleted task: {}", matching[0].name);
                }
                _ => {
                    println!("Multiple tasks match '{}'. Be more specific:", id);
                    for task in matching {
                        println!("  {} - {}", &task.id[..8], task.name);
                    }
                }
            }
        }
        Commands::Switch { id } => {
            let tasks = manager.list_tasks()?;
            let matching: Vec<_> = tasks.iter().filter(|t| t.id.starts_with(&id)).collect();

            match matching.len() {
                0 => {
                    println!("No task found with ID starting with '{}'", id);
                }
                1 => {
                    let task_id = &matching[0].id;
                    manager.set_current_task(task_id)?;
                    println!("Switched to task: {}", matching[0].name);
                }
                _ => {
                    println!("Multiple tasks match '{}'. Be more specific:", id);
                    for task in matching {
                        println!("  {} - {}", &task.id[..8], task.name);
                    }
                }
            }
        }
        Commands::Research => {
            let task = manager
                .get_current_task()?
                .ok_or("No current task. Use 'arq new <prompt>' first.")?;

            if task.phase != Phase::Research {
                return Err(format!(
                    "Task is in {} phase, not Research phase.",
                    task.phase.display_name()
                )
                .into());
            }

            if task.research_doc.is_some() {
                println!("Research already complete for this task.");
                println!("Run 'arq advance' to move to Planning phase.");
                return Ok(());
            }

            println!("Starting research for: {}", task.prompt);
            println!();

            // Load config (from arq.toml or defaults)
            let config = Config::load().unwrap_or_default();

            // Create LLM client from config
            let llm = Provider::from_config(&config.llm).build().map_err(|e| {
                format!(
                    "{}. Configure [llm] in arq.toml or set OPENAI_API_KEY or ANTHROPIC_API_KEY.",
                    e
                )
            })?;

            // Create context builder with config
            let context_builder = ContextBuilder::with_config(".", config.context);

            // Create runner
            let runner = ResearchRunner::new(llm, context_builder);

            println!("Scanning codebase...");

            // Run research
            let doc = runner.run(&task).await?;

            println!("Research complete!\n");
            println!("## Summary\n");
            println!("{}\n", doc.summary);
            println!("## Suggested Approach\n");
            println!("{}\n", doc.suggested_approach);

            // Save research doc
            manager.set_research_doc(&task.id, doc)?;

            println!("Research saved to {}/tasks/{}/research-doc.md", ARQ_DIR, task.id);
            println!("\nNext: Run 'arq advance' to move to Planning phase.");
        }
        Commands::Advance => {
            let task = manager
                .get_current_task()?
                .ok_or("No current task. Use 'arq new <prompt>' first.")?;

            if !task.can_advance() {
                let hint = match task.phase {
                    Phase::Research => "Run 'arq research' first.",
                    Phase::Planning => "Complete planning first.",
                    Phase::Agent => "Complete implementation first.",
                    Phase::Complete => "Task is already complete.",
                };
                return Err(format!(
                    "Cannot advance from {} phase. {}",
                    task.phase.display_name(),
                    hint
                )
                .into());
            }

            let new_phase = manager.advance_phase(&task.id)?;
            println!(
                "Advanced to {} phase.",
                new_phase.display_name()
            );
        }
    }

    Ok(())
}
