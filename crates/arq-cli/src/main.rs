use arq_core::{
    Config, ContextBuilder, FileStorage, IndexStats, KnowledgeGraph, KnowledgeStore, Phase,
    Provider, ResearchRunner, SearchResult, TaskManager,
};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

mod serve;
mod tui;

#[derive(Parser)]
#[command(name = "arq")]
#[command(version)]
#[command(
    about = "AI coding engine for deep codebase understanding and high-precision code generation"
)]
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
    /// Index codebase into knowledge graph
    Init {
        /// Force re-indexing even if already indexed
        #[arg(short, long)]
        force: bool,
    },
    /// Search code using semantic search
    Search {
        /// Search query
        #[arg(required = true)]
        query: Vec<String>,
        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Show knowledge graph statistics
    KgStatus,
    /// Clear the knowledge graph database
    KgClear,
    /// Query graph relationships (dependencies and impact)
    Graph {
        #[command(subcommand)]
        action: GraphAction,
    },
    /// Launch interactive TUI chat interface
    #[command(alias = "ui")]
    Tui,
    /// Start visualization server for knowledge graph
    Serve {
        /// Port to run the server on
        #[arg(short, long, default_value = "3333")]
        port: u16,
        /// Don't automatically open browser
        #[arg(long)]
        no_open: bool,
    },
}

#[derive(Subcommand)]
enum GraphAction {
    /// Show what a function depends on (calls)
    Deps {
        /// Function name to look up
        name: String,
    },
    /// Show what depends on a function (callers / impact)
    Impact {
        /// Function name to look up
        name: String,
    },
    /// List all indexed functions
    Functions {
        /// Maximum number to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
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
    let config = Config::load().unwrap_or_default();
    let storage = FileStorage::with_config(config.storage.clone());
    let mut manager = TaskManager::new(storage);

    match cli.command {
        Commands::New { prompt } => {
            let prompt_str = prompt.join(" ");
            let task = manager.create_task(&prompt_str)?;
            println!("Created new task: {}", task.name);
            println!("  ID: {}", task.id);
            println!("  Phase: {}", task.phase.display_name());
            println!("  Prompt: {}", task.prompt);
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

            // Create LLM client from config
            let llm = Provider::from_config(&config.llm).build().map_err(|e| {
                format!(
                    "{}. Configure [llm] in arq.toml or set OPENAI_API_KEY or ANTHROPIC_API_KEY.",
                    e
                )
            })?;

            // Create context builder with config
            let context_builder = ContextBuilder::with_config(".", config.context.clone());

            // Check if knowledge graph is available
            let db_path = config.knowledge.db_full_path(&config.storage);
            let runner = if db_path.exists() {
                println!("Using knowledge graph for smart context...");
                let kg = KnowledgeGraph::open(&db_path).await?;
                ResearchRunner::with_knowledge_store(llm, context_builder, std::sync::Arc::new(kg))
            } else {
                println!("Scanning codebase (run 'arq init' for faster semantic search)...");
                ResearchRunner::new(llm, context_builder)
            };

            // Run research
            let doc = runner.run(&task).await?;

            println!("Research complete!\n");
            println!("## Summary\n");
            println!("{}\n", doc.summary);
            println!("## Suggested Approach\n");
            println!("{}\n", doc.suggested_approach);

            // Save research doc
            manager.set_research_doc(&task.id, doc)?;

            let research_path = config.storage.local_research_path();
            println!("Research saved to {}", research_path.display());
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
            println!("Advanced to {} phase.", new_phase.display_name());
        }
        Commands::Init { force } => {
            let db_path = config.knowledge.db_full_path(&config.storage);
            let project_dir = config.storage.project_dir();

            // Create project directory if it doesn't exist
            std::fs::create_dir_all(&project_dir)?;

            // Check if already initialized
            if db_path.exists() && !force {
                println!("Knowledge graph already initialized.");
                println!("Use --force to re-index.");
                return Ok(());
            }

            // Remove existing database if force re-indexing
            if force && db_path.exists() {
                let pb = ProgressBar::new_spinner();
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.cyan} {msg}")
                        .unwrap(),
                );
                pb.set_message("Clearing existing knowledge graph...");
                std::fs::remove_dir_all(&db_path)?;
                pb.finish_with_message("Done");
            }

            // Step 1: Load embedding model
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            pb.set_message("Loading embedding model (first run downloads ~50MB)...");

            let kg = KnowledgeGraph::open(&db_path).await?;
            kg.initialize().await?;
            pb.finish_with_message("Done");

            // Step 2: Index codebase
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            pb.set_message("Indexing codebase...");

            let stats: IndexStats = kg.index_directory(Path::new(".")).await?;
            pb.finish_with_message("Done");

            println!("\nKnowledge graph initialized!");
            println!("  Files indexed: {}", stats.files);
            println!("  Code chunks: {}", stats.chunks);
            println!("  Total size: {} KB", stats.total_size / 1024);
            println!("\nDatabase: {}", db_path.display());
        }
        Commands::Search { query, limit } => {
            let db_path = config.knowledge.db_full_path(&config.storage);

            if !db_path.exists() {
                return Err("Knowledge graph not initialized. Run 'arq init' first.".into());
            }

            let kg = KnowledgeGraph::open(&db_path).await?;

            let query_str = query.join(" ");
            println!("Searching for: {}\n", query_str);

            let results: Vec<SearchResult> = kg.search_code(&query_str, limit).await?;

            if results.is_empty() {
                println!("No results found.");
            } else {
                println!("Found {} results:\n", results.len());
                for (i, result) in results.iter().enumerate() {
                    println!(
                        "{}. {} (lines {}-{}) - score: {:.2}",
                        i + 1,
                        result.path,
                        result.start_line,
                        result.end_line,
                        result.score
                    );
                    if let Some(ref preview) = result.preview {
                        for line in preview.lines().take(3) {
                            println!("   {}", line);
                        }
                    }
                    println!();
                }
            }
        }
        Commands::KgStatus => {
            let db_path = config.knowledge.db_full_path(&config.storage);

            if !db_path.exists() {
                println!("Knowledge graph not initialized.");
                println!("Run 'arq init' to index your codebase.");
                return Ok(());
            }

            let kg = KnowledgeGraph::open(&db_path).await?;
            let stats = kg.get_extended_stats().await?;

            println!("Knowledge Graph Status\n");
            println!("  Files indexed: {}", stats.files);
            println!("  Code chunks: {}", stats.chunks);
            println!();
            println!("  Rich Ontology:");
            println!("    Functions: {}", stats.functions);
            println!("    Structs: {}", stats.structs);
            println!("    Traits: {}", stats.traits);
            println!("    Impls: {}", stats.impls);
            println!("    Enums: {}", stats.enums);
            println!("    Constants: {}", stats.constants);
            println!();
            println!("  Relations:");
            println!("    Calls: {}", stats.calls);
            println!("    Implements: {}", stats.implements);
            println!("\nDatabase path: {}", db_path.display());
        }
        Commands::KgClear => {
            let db_path = config.knowledge.db_full_path(&config.storage);

            if !db_path.exists() {
                println!("Knowledge graph not initialized. Nothing to clear.");
                return Ok(());
            }

            std::fs::remove_dir_all(&db_path)?;
            println!("Knowledge graph cleared.");
            println!("Run 'arq init' to re-index your codebase.");
        }
        Commands::Graph { action } => {
            let db_path = config.knowledge.db_full_path(&config.storage);

            if !db_path.exists() {
                return Err("Knowledge graph not initialized. Run 'arq init' first.".into());
            }

            let kg = KnowledgeGraph::open(&db_path).await?;

            match action {
                GraphAction::Deps { name } => {
                    println!("Dependencies for '{}'\n", name);

                    // Find function by name first
                    let func = kg.find_function_by_name(&name).await?;

                    match func {
                        Some(f) => {
                            // Use function name directly for the call lookup
                            let deps = kg.get_dependencies(&name).await?;

                            if deps.is_empty() {
                                println!("'{}' has no outgoing calls recorded.", name);
                                println!("  Location: {}:{}", f.file_path, f.start_line);
                            } else {
                                println!("'{}' calls:", name);
                                for dep in &deps {
                                    println!("  → {}", dep);
                                }
                            }
                        }
                        None => {
                            println!("Function '{}' not found in the index.", name);
                            println!("\nTip: Use 'arq graph functions' to list indexed functions.");
                        }
                    }
                }
                GraphAction::Impact { name } => {
                    println!("Impact analysis for '{}'\n", name);

                    // Find function by name first
                    let func = kg.find_function_by_name(&name).await?;

                    match func {
                        Some(f) => {
                            // Use function name directly for the call lookup
                            let callers = kg.get_impact(&name).await?;

                            if callers.is_empty() {
                                println!("'{}' has no incoming calls recorded.", name);
                                println!("  Location: {}:{}", f.file_path, f.start_line);
                            } else {
                                println!("'{}' is called by:", name);
                                for caller in &callers {
                                    println!("  ← {}", caller);
                                }
                            }
                        }
                        None => {
                            println!("Function '{}' not found in the index.", name);
                            println!("\nTip: Use 'arq graph functions' to list indexed functions.");
                        }
                    }
                }
                GraphAction::Functions { limit } => {
                    println!("Indexed functions (showing up to {}):\n", limit);

                    let all_functions = kg.list_functions(limit).await?;
                    let functions: Vec<_> = all_functions.into_iter().take(limit).collect();

                    if functions.is_empty() {
                        println!("  No functions indexed yet.");
                    } else {
                        for f in &functions {
                            let visibility = if f.visibility == "public" { "pub " } else { "" };
                            let async_marker = if f.is_async { "async " } else { "" };
                            println!(
                                "  {}{}fn {} ({}:{})",
                                visibility, async_marker, f.name, f.file_path, f.start_line
                            );
                        }
                        println!("\n  Total: {} functions", functions.len());
                    }
                }
            }
        }
        Commands::Tui => {
            tui::run(config, manager).await?;
        }
        Commands::Serve { port, no_open } => {
            let db_path = config.knowledge.db_full_path(&config.storage);

            if !db_path.exists() {
                println!("Knowledge graph not initialized.");
                println!("Run 'arq init' to index your codebase first.");
                return Ok(());
            }

            let serve_config = serve::ServeConfig {
                port,
                open_browser: !no_open,
                project_path: std::env::current_dir()?,
                db_path: db_path.clone(),
            };

            serve::start_server(serve_config).await?;
        }
    }

    Ok(())
}
