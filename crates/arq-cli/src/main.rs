use clap::{Parser, Subcommand};
use arq_core::{FileStorage, TaskManager};

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
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
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
            // Try to find task by partial ID
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
            // Try to find task by partial ID
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
    }

    Ok(())
}
