use anyhow::Result;
use clap::{Parser, Subcommand};
use jarvis_core::{config::Config, llm::LLMRouter, memory::MemoryStore};
use jarvis_agent::AgentRunner;
use jarvis_shell::Environment;
use tracing::{info, Level};
use tracing_subscriber;

mod commands;
use commands::{BlockchainCommands, handle_blockchain_command};

#[derive(Parser)]
#[command(name = "jarvis")]
#[command(about = "Your local AI assistant for Rust, Linux, and Homelab operations")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(short, long, global = true)]
    verbose: bool,
    
    #[arg(short, long, global = true)]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Explain system components or files
    Explain {
        /// What to explain (e.g., "my snapper timeline", "this error log")
        query: Vec<String>,
    },
    /// Diagnose system issues
    Diagnose {
        /// Service or component to diagnose
        target: Vec<String>,
    },
    /// Write code or scripts
    Write {
        /// What to write (e.g., "a Rust CLI with clap")
        description: Vec<String>,
    },
    /// Check system status
    Check {
        /// What to check (e.g., "btrfs mount status")
        target: Vec<String>,
    },
    /// Fix issues automatically
    Fix {
        /// Issue description or error message
        issue: Vec<String>,
    },
    /// Blockchain operations and optimization
    Blockchain {
        #[command(subcommand)]
        blockchain_command: BlockchainCommands,
    },
    /// Train or manage local LLMs
    Train {
        #[command(subcommand)]
        action: TrainCommands,
    },
    /// Interactive chat mode
    Chat,
    /// Configure Jarvis
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum TrainCommands {
    /// Start training a custom model
    Start {
        /// Training data path
        data_path: String,
        /// Model name
        model_name: String,
    },
    /// List available models
    List,
    /// Load a specific model
    Load {
        model_name: String,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Initialize configuration
    Init,
    /// Set configuration values
    Set {
        key: String,
        value: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let level = if cli.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .init();
        
    info!("🤖 Jarvis starting up...");
    
    // Handle config commands before initializing other components
    match &cli.command {
        Commands::Config { action } => {
            match action {
                ConfigCommands::Show => {
                    let config = Config::load(cli.config.as_deref()).await?;
                    println!("{:#?}", config);
                }
                ConfigCommands::Init => {
                    Config::init().await?;
                    println!("✅ Configuration initialized at ~/.config/jarvis/jarvis.toml");
                }
                ConfigCommands::Set { key, value } => {
                    Config::set(&key, &value).await?;
                    println!("✅ Set {} = {}", key, value);
                }
            }
            return Ok(());
        }
        _ => {}
    }
    
    // Load configuration for other commands
    let config = Config::load(cli.config.as_deref()).await?;
    
    // Initialize core components
    let memory = MemoryStore::new(&config.database_path).await?;
    let llm_router = LLMRouter::new(&config).await?;
    let environment = Environment::detect().await?;
    let agent_runner = AgentRunner::new(memory.clone(), llm_router.clone()).await?;
    
    // Route commands
    match cli.command {
        Commands::Explain { query } => {
            let query_str = query.join(" ");
            info!("📚 Explaining: {}", query_str);
            agent_runner.explain(&query_str, &environment).await?;
        }
        Commands::Diagnose { target } => {
            let target_str = target.join(" ");
            info!("🔍 Diagnosing: {}", target_str);
            agent_runner.diagnose(&target_str, &environment).await?;
        }
        Commands::Write { description } => {
            let desc_str = description.join(" ");
            info!("✍️ Writing: {}", desc_str);
            agent_runner.write_code(&desc_str, &environment).await?;
        }
        Commands::Check { target } => {
            let target_str = target.join(" ");
            info!("✅ Checking: {}", target_str);
            agent_runner.check_status(&target_str, &environment).await?;
        }
        Commands::Fix { issue } => {
            let issue_str = issue.join(" ");
            info!("🔧 Fixing: {}", issue_str);
            agent_runner.fix_issue(&issue_str, &environment).await?;
        }
        Commands::Train { action } => {
            match action {
                TrainCommands::Start { data_path, model_name } => {
                    info!("🧠 Starting training: {} with data from {}", model_name, data_path);
                    agent_runner.train_model(&model_name, &data_path).await?;
                }
                TrainCommands::List => {
                    agent_runner.list_models().await?;
                }
                TrainCommands::Load { model_name } => {
                    info!("📥 Loading model: {}", model_name);
                    agent_runner.load_model(&model_name).await?;
                }
            }
        }
        Commands::Chat => {
            info!("💬 Entering interactive chat mode...");
            agent_runner.interactive_chat(&environment).await?;
        }
        Commands::Config { .. } => {
            // Config commands are handled earlier, this should never be reached
            unreachable!("Config commands should be handled earlier")
        }
        Commands::Blockchain { blockchain_command } => {
            handle_blockchain_command(blockchain_command, &config).await?;
        }
    }
    
    Ok(())
}
