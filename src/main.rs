use anyhow::Result;
use clap::{Parser, Subcommand};
use jarvis_core::{config::Config, llm::LLMRouter, memory::MemoryStore};
use jarvis_agent::AgentRunner;
use jarvis_shell::Environment;
use tracing::{info, Level};
use tracing_subscriber;

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

#[derive(Subcommand)]
enum BlockchainCommands {
    /// Analyze blockchain network performance
    Analyze {
        /// Network to analyze (ghostchain, zig-blockchain, or auto-detect)
        #[arg(short, long, default_value = "auto")]
        network: String,
    },
    /// Optimize network settings for IPv6 and QUIC
    Optimize {
        /// Optimization target (ipv6, quic, all)
        #[arg(short, long, default_value = "all")]
        target: String,
        /// Enable dry-run mode (show recommendations without executing)
        #[arg(long)]
        dry_run: bool,
    },
    /// Audit smart contracts for security and gas optimization
    Audit {
        /// Contract address or path to contract code
        contract: String,
        /// Security level (basic, standard, comprehensive, paranoid)
        #[arg(short, long, default_value = "comprehensive")]
        security_level: String,
    },
    /// Monitor blockchain performance in real-time
    Monitor {
        /// Monitoring duration in seconds (0 for continuous)
        #[arg(short, long, default_value = "0")]
        duration: u64,
        /// Output format (json, table, dashboard)
        #[arg(short, long, default_value = "dashboard")]
        format: String,
    },
    /// Schedule or execute maintenance tasks
    Maintenance {
        #[command(subcommand)]
        action: MaintenanceActions,
    },
    /// Configure blockchain agent settings
    Configure {
        /// Agent type to configure
        agent: String,
        /// Configuration key-value pairs
        settings: Vec<String>,
    },
    /// Show status of all blockchain agents
    Status,
}

#[derive(Subcommand)]
enum MaintenanceActions {
    /// Schedule a maintenance task
    Schedule {
        /// Task type (cleanup, update, optimization, backup)
        task_type: String,
        /// Scheduled time (e.g., "2024-01-15 02:00" or "in 1 hour")
        when: String,
    },
    /// List scheduled maintenance tasks
    List,
    /// Cancel a scheduled task
    Cancel {
        /// Task ID to cancel
        task_id: String,
    },
    /// Execute emergency maintenance
    Emergency {
        /// Emergency task type
        task_type: String,
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
            match blockchain_command {
                BlockchainCommands::Analyze { network } => {
                    info!("🔍 Analyzing blockchain network: {}", network);
                    agent_runner.analyze_blockchain(&network).await?;
                }
                BlockchainCommands::Optimize { target, dry_run } => {
                    info!("⚙️ Optimizing network settings: {} (dry run: {})", target, dry_run);
                    agent_runner.optimize_network(&target, dry_run).await?;
                }
                BlockchainCommands::Audit { contract, security_level } => {
                    info!("🔒 Auditing smart contract: {} (security level: {})", contract, security_level);
                    agent_runner.audit_contract(&contract, &security_level).await?;
                }
                BlockchainCommands::Monitor { duration, format } => {
                    info!("📊 Monitoring blockchain performance: {} seconds, format: {}", duration, format);
                    agent_runner.monitor_blockchain(duration, &format).await?;
                }
                BlockchainCommands::Maintenance { action } => {
                    match action {
                        MaintenanceActions::Schedule { task_type, when } => {
                            info!("🗓️ Scheduling maintenance task: {} at {}", task_type, when);
                            agent_runner.schedule_maintenance(&task_type, &when).await?;
                        }
                        MaintenanceActions::List => {
                            agent_runner.list_maintenance_tasks().await?;
                        }
                        MaintenanceActions::Cancel { task_id } => {
                            info!("❌ Cancelling maintenance task: {}", task_id);
                            agent_runner.cancel_maintenance(&task_id).await?;
                        }
                        MaintenanceActions::Emergency { task_type } => {
                            info!("🚨 Executing emergency maintenance: {}", task_type);
                            agent_runner.emergency_maintenance(&task_type).await?;
                        }
                    }
                }
                BlockchainCommands::Configure { agent, settings } => {
                    info!("⚙️ Configuring blockchain agent: {} with settings: {:?}", agent, settings);
                    agent_runner.configure_blockchain_agent(&agent, &settings).await?;
                }
                BlockchainCommands::Status => {
                    agent_runner.show_blockchain_agent_status().await?;
                }
            }
        }
    }
    
    Ok(())
}
