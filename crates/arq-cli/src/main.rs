use clap::{Parser, Subcommand};
use arq_core::Task;

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
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { prompt } => {
            let prompt_str = prompt.join(" ");
            let task = Task::new(&prompt_str);
            println!("Created new task: {}", task.name);
            println!("  ID: {}", task.id);
            println!("  Phase: {}", task.phase.display_name());
            println!("  Prompt: {}", task.prompt);
        }
        Commands::Status => {
            println!("No active task. Use 'arq new <prompt>' to start.");
        }
        Commands::List => {
            println!("No tasks found. Use 'arq new <prompt>' to create one.");
        }
    }
}
