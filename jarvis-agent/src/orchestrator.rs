// jarvis-agent/src/orchestrator.rs
//! Orchestrates all blockchain agents and provides unified management

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use jarvis_core::{Config, GhostChainClient, LLMRouter, MemoryStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::ai_analyzer::{AIAnalysisResult, AIAnalyzerConfig, AIBlockchainAnalyzer};
use crate::blockchain_monitor::{BlockchainMonitorAgent, MonitoringAlert, MonitoringConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub agent_name: String,
    pub status: AgentState,
    pub last_activity: DateTime<Utc>,
    pub metrics: AgentMetrics,
    pub error_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentState {
    Starting,
    Running,
    Paused,
    Error,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub uptime_seconds: u64,
    pub tasks_completed: u64,
    pub alerts_processed: u32,
    pub ai_analyses_performed: u32,
}

#[derive(Debug)]
pub enum AgentMessage {
    Alert(MonitoringAlert),
    AnalysisRequest(String),
    StatusUpdate(String, AgentState),
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub enable_monitoring: bool,
    pub enable_ai_analysis: bool,
    pub auto_restart_failed_agents: bool,
    pub max_error_count: u32,
    pub status_report_interval_minutes: u32,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            enable_monitoring: true,
            enable_ai_analysis: true,
            auto_restart_failed_agents: true,
            max_error_count: 10,
            status_report_interval_minutes: 15,
        }
    }
}

pub struct BlockchainAgentOrchestrator {
    config: OrchestratorConfig,
    grpc_client: GhostChainClient,
    memory: MemoryStore,
    llm_router: LLMRouter,

    // Agent instances
    monitor_agent: Option<BlockchainMonitorAgent>,
    ai_analyzer: Option<AIBlockchainAnalyzer>,

    // Communication channels
    message_sender: mpsc::UnboundedSender<AgentMessage>,
    message_receiver: Option<mpsc::UnboundedReceiver<AgentMessage>>,

    // Agent status tracking
    agent_status: Arc<RwLock<HashMap<String, AgentStatus>>>,

    // Task handles
    running_tasks: Vec<JoinHandle<()>>,
}

impl BlockchainAgentOrchestrator {
    pub fn new(
        config: OrchestratorConfig,
        grpc_client: GhostChainClient,
        memory: MemoryStore,
        llm_router: LLMRouter,
    ) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            config,
            grpc_client,
            memory,
            llm_router,
            monitor_agent: None,
            ai_analyzer: None,
            message_sender: sender,
            message_receiver: Some(receiver),
            agent_status: Arc::new(RwLock::new(HashMap::new())),
            running_tasks: Vec::new(),
        }
    }

    /// Start all configured agents
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Blockchain Agent Orchestrator");

        // Initialize monitoring agent
        if self.config.enable_monitoring {
            self.start_monitoring_agent().await?;
        }

        // Initialize AI analyzer
        if self.config.enable_ai_analysis {
            self.start_ai_analyzer().await?;
        }

        // Start message processing loop
        self.start_message_processor().await?;

        // Start status monitoring
        self.start_status_monitor().await?;

        info!("All agents started successfully");
        Ok(())
    }

    /// Start the blockchain monitoring agent
    async fn start_monitoring_agent(&mut self) -> Result<()> {
        info!("Starting blockchain monitoring agent");

        let monitor_config = MonitoringConfig::default();
        let monitor = BlockchainMonitorAgent::new(
            self.grpc_client.clone(),
            self.memory.clone(),
            monitor_config,
        );

        // Update status
        self.update_agent_status("monitor", AgentState::Starting)
            .await;

        // Store the agent
        self.monitor_agent = Some(monitor);

        // Start monitoring in background task
        let mut agent = self.monitor_agent.take().unwrap();
        let sender = self.message_sender.clone();
        let status_tracker = self.agent_status.clone();

        let task = tokio::spawn(async move {
            // Update status to running
            {
                let mut status = status_tracker.write().await;
                if let Some(agent_status) = status.get_mut("monitor") {
                    agent_status.status = AgentState::Running;
                    agent_status.last_activity = Utc::now();
                }
            }

            // Run monitoring with error handling
            if let Err(e) = agent.start_monitoring().await {
                error!("Monitoring agent failed: {}", e);

                // Update status to error
                {
                    let mut status = status_tracker.write().await;
                    if let Some(agent_status) = status.get_mut("monitor") {
                        agent_status.status = AgentState::Error;
                        agent_status.error_count += 1;
                    }
                }

                // Send error message
                let _ = sender.send(AgentMessage::StatusUpdate(
                    "monitor".to_string(),
                    AgentState::Error,
                ));
            }
        });

        self.running_tasks.push(task);
        Ok(())
    }

    /// Start the AI analyzer
    async fn start_ai_analyzer(&mut self) -> Result<()> {
        info!("Starting AI blockchain analyzer");

        let ai_config = AIAnalyzerConfig::default();
        let analyzer =
            AIBlockchainAnalyzer::new(self.llm_router.clone(), self.memory.clone(), ai_config);

        self.update_agent_status("ai_analyzer", AgentState::Running)
            .await;
        self.ai_analyzer = Some(analyzer);

        Ok(())
    }

    /// Start the message processing loop
    async fn start_message_processor(&mut self) -> Result<()> {
        let mut receiver = self
            .message_receiver
            .take()
            .context("Message receiver already taken")?;

        let analyzer = self.ai_analyzer.take();
        let status_tracker = self.agent_status.clone();
        let config = self.config.clone();

        let task = tokio::spawn(async move {
            let mut ai_analyzer = analyzer;

            while let Some(message) = receiver.recv().await {
                match message {
                    AgentMessage::Alert(alert) => {
                        info!("Processing alert: {}", alert.id);

                        // Process with AI analyzer if available
                        if let Some(ref mut analyzer) = ai_analyzer {
                            match analyzer.analyze_alert(&alert).await {
                                Ok(analysis) => {
                                    info!(
                                        "AI analysis completed for alert {}: Risk Score {}",
                                        alert.id, analysis.risk_score
                                    );

                                    // Update metrics
                                    {
                                        let mut status = status_tracker.write().await;
                                        if let Some(agent_status) = status.get_mut("ai_analyzer") {
                                            agent_status.metrics.ai_analyses_performed += 1;
                                            agent_status.last_activity = Utc::now();
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("AI analysis failed for alert {}: {}", alert.id, e);
                                }
                            }
                        }
                    }
                    AgentMessage::AnalysisRequest(request_type) => {
                        info!("Processing analysis request: {}", request_type);

                        if let Some(ref mut analyzer) = ai_analyzer {
                            let result = match request_type.as_str() {
                                "patterns" => analyzer.analyze_patterns(24).await,
                                "predictive" => analyzer.predictive_analysis().await,
                                _ => {
                                    warn!("Unknown analysis request type: {}", request_type);
                                    continue;
                                }
                            };

                            match result {
                                Ok(analysis) => {
                                    info!("Analysis completed: {}", analysis.summary);
                                }
                                Err(e) => {
                                    error!("Analysis failed: {}", e);
                                }
                            }
                        }
                    }
                    AgentMessage::StatusUpdate(agent_name, new_status) => {
                        info!("Status update for {}: {:?}", agent_name, new_status);

                        // Handle failed agents
                        if matches!(new_status, AgentState::Error)
                            && config.auto_restart_failed_agents
                        {
                            warn!("Agent {} failed, considering restart", agent_name);
                            // Restart logic would go here
                        }
                    }
                    AgentMessage::Shutdown => {
                        info!("Received shutdown message, stopping message processor");
                        break;
                    }
                }
            }
        });

        self.running_tasks.push(task);
        Ok(())
    }

    /// Start status monitoring
    async fn start_status_monitor(&mut self) -> Result<()> {
        let status_tracker = self.agent_status.clone();
        let interval_minutes = self.config.status_report_interval_minutes;

        let task = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(interval_minutes as u64 * 60));

            loop {
                interval.tick().await;

                let status = status_tracker.read().await;
                let active_agents = status.len();
                let running_agents = status
                    .values()
                    .filter(|s| matches!(s.status, AgentState::Running))
                    .count();
                let error_agents = status
                    .values()
                    .filter(|s| matches!(s.status, AgentState::Error))
                    .count();

                info!(
                    "Agent Status Report: {} total, {} running, {} errors",
                    active_agents, running_agents, error_agents
                );

                // Log individual agent status
                for (name, agent_status) in status.iter() {
                    debug!(
                        "Agent {}: {:?}, last activity: {}, errors: {}",
                        name,
                        agent_status.status,
                        agent_status.last_activity,
                        agent_status.error_count
                    );
                }
            }
        });

        self.running_tasks.push(task);
        Ok(())
    }

    /// Update agent status
    async fn update_agent_status(&self, agent_name: &str, state: AgentState) {
        let mut status = self.agent_status.write().await;

        let agent_status = status
            .entry(agent_name.to_string())
            .or_insert_with(|| AgentStatus {
                agent_name: agent_name.to_string(),
                status: state.clone(),
                last_activity: Utc::now(),
                metrics: AgentMetrics {
                    uptime_seconds: 0,
                    tasks_completed: 0,
                    alerts_processed: 0,
                    ai_analyses_performed: 0,
                },
                error_count: 0,
            });

        agent_status.status = state;
        agent_status.last_activity = Utc::now();
    }

    /// Send a message to the agent system
    pub async fn send_message(&self, message: AgentMessage) -> Result<()> {
        self.message_sender
            .send(message)
            .context("Failed to send message to agent system")?;
        Ok(())
    }

    /// Get current status of all agents
    pub async fn get_agent_status(&self) -> HashMap<String, AgentStatus> {
        self.agent_status.read().await.clone()
    }

    /// Request AI analysis
    pub async fn request_analysis(&self, analysis_type: &str) -> Result<()> {
        self.send_message(AgentMessage::AnalysisRequest(analysis_type.to_string()))
            .await
    }

    /// Get comprehensive system health report
    pub async fn get_system_health(&self) -> Result<serde_json::Value> {
        let agent_status = self.get_agent_status().await;

        let total_agents = agent_status.len();
        let healthy_agents = agent_status
            .values()
            .filter(|s| matches!(s.status, AgentState::Running))
            .count();

        let total_errors = agent_status.values().map(|s| s.error_count).sum::<u32>();

        let total_alerts_processed = agent_status
            .values()
            .map(|s| s.metrics.alerts_processed)
            .sum::<u32>();

        let total_ai_analyses = agent_status
            .values()
            .map(|s| s.metrics.ai_analyses_performed)
            .sum::<u32>();

        Ok(serde_json::json!({
            "system_health": {
                "overall_status": if healthy_agents == total_agents { "healthy" } else { "degraded" },
                "total_agents": total_agents,
                "healthy_agents": healthy_agents,
                "total_errors": total_errors,
                "uptime": "calculated_uptime", // Would calculate actual uptime
            },
            "performance_metrics": {
                "alerts_processed": total_alerts_processed,
                "ai_analyses_performed": total_ai_analyses,
                "average_response_time": "calculated_response_time"
            },
            "agent_details": agent_status
        }))
    }

    /// Graceful shutdown
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down Blockchain Agent Orchestrator");

        // Send shutdown message
        self.send_message(AgentMessage::Shutdown).await?;

        // Wait for all tasks to complete
        for task in self.running_tasks.drain(..) {
            if let Err(e) = task.await {
                warn!("Task failed to shut down gracefully: {}", e);
            }
        }

        info!("All agents shut down successfully");
        Ok(())
    }

    /// Create orchestrator from Jarvis config
    pub async fn from_config(config: &Config) -> Result<Self> {
        // Create gRPC client
        let ghost_config = config
            .blockchain
            .as_ref()
            .and_then(|bc| bc.ghostchain.as_ref())
            .cloned()
            .unwrap_or_default();

        let grpc_client = GhostChainClient::new(ghost_config.into()).await?;

        // Initialize memory store
        let memory = MemoryStore::new(&config.database_path).await?;

        // Initialize LLM router
        let llm_router = LLMRouter::new(config).await?;

        // Create orchestrator config from main config
        let orchestrator_config = OrchestratorConfig {
            enable_monitoring: config.agents.transaction_monitor.enabled,
            enable_ai_analysis: true, // Enable by default for now
            auto_restart_failed_agents: true,
            max_error_count: 10,
            status_report_interval_minutes: 15,
        };

        Ok(Self::new(
            orchestrator_config,
            grpc_client,
            memory,
            llm_router,
        ))
    }
}
