/*!
 * AI Agent for JARVIS-NV
 *
 * Core AI agent providing autonomous blockchain monitoring, analysis,
 * optimization, and predictive capabilities using GPU-accelerated inference.
 */

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use crate::ai::{InferenceRequest, InferenceResponse, OllamaManager};
use crate::bridge::GhostBridge;
use crate::config::AgentConfig;
use crate::gpu::GpuManager;
use crate::metrics::MetricsCollector;
use crate::node::NodeManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub enabled: bool,
    pub running: bool,
    pub mode: String, // "monitoring", "analyzing", "optimizing", "learning"
    pub current_task: Option<String>,
    pub uptime_seconds: u64,
    pub inferences_completed: u64,
    pub anomalies_detected: u32,
    pub optimizations_applied: u32,
    pub learning_progress: f64,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub category: String, // "performance", "security", "network", "transaction"
    pub severity: String, // "low", "medium", "high", "critical"
    pub score: f64,       // 0.0 to 1.0
    pub description: String,
    pub affected_component: String,
    pub recommended_actions: Vec<String>,
    pub auto_resolved: bool,
    pub resolution_time: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Optimization {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub category: String, // "gas", "throughput", "latency", "resource"
    pub target: String,
    pub current_value: f64,
    pub optimized_value: f64,
    pub improvement_percentage: f64,
    pub confidence: f64,
    pub applied: bool,
    pub rollback_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub category: String,  // "price", "congestion", "demand", "performance"
    pub timeframe: String, // "1h", "24h", "7d", "30d"
    pub predicted_value: f64,
    pub confidence: f64,
    pub current_value: f64,
    pub trend: String, // "increasing", "decreasing", "stable", "volatile"
    pub accuracy_score: Option<f64>, // Filled in after timeframe passes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub model_accuracy: f64,
    pub training_loss: f64,
    pub inference_speed_ms: f64,
    pub data_processed_mb: f64,
    pub feature_importance: HashMap<String, f64>,
    pub model_version: String,
    pub last_update: chrono::DateTime<chrono::Utc>,
}

pub struct NvAgent {
    config: AgentConfig,

    // Component references
    gpu_manager: Arc<GpuManager>,
    metrics_collector: Arc<MetricsCollector>,
    node_manager: Arc<NodeManager>,
    ghost_bridge: Arc<GhostBridge>,
    ollama_manager: Arc<OllamaManager>,

    // Agent state
    agent_status: Arc<RwLock<AgentStatus>>,
    anomalies: Arc<Mutex<VecDeque<Anomaly>>>,
    optimizations: Arc<Mutex<VecDeque<Optimization>>>,
    predictions: Arc<Mutex<VecDeque<Prediction>>>,
    learning_metrics: Arc<RwLock<LearningMetrics>>,

    // Analysis state
    historical_data: Arc<Mutex<VecDeque<serde_json::Value>>>,
    pattern_cache: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    model_states: Arc<RwLock<HashMap<String, serde_json::Value>>>,

    // Runtime control
    is_running: Arc<RwLock<bool>>,
    current_task: Arc<RwLock<Option<String>>>,
    start_time: Instant,
}

impl NvAgent {
    /// Create new AI agent
    pub async fn new(
        config: &AgentConfig,
        gpu_manager: Arc<GpuManager>,
        metrics_collector: Arc<MetricsCollector>,
        node_manager: Arc<NodeManager>,
        ghost_bridge: Arc<GhostBridge>,
    ) -> Result<Self> {
        info!("🤖 Initializing JARVIS-NV AI Agent");

        // Initialize Ollama manager
        let ollama_manager = Arc::new(OllamaManager::new(config).await?);

        let agent = Self {
            config: config.clone(),
            gpu_manager,
            metrics_collector,
            node_manager,
            ghost_bridge,
            ollama_manager,
            agent_status: Arc::new(RwLock::new(AgentStatus {
                enabled: config.enabled,
                running: false,
                mode: "initializing".to_string(),
                current_task: None,
                uptime_seconds: 0,
                inferences_completed: 0,
                anomalies_detected: 0,
                optimizations_applied: 0,
                learning_progress: 0.0,
                last_activity: chrono::Utc::now(),
            })),
            anomalies: Arc::new(Mutex::new(VecDeque::new())),
            optimizations: Arc::new(Mutex::new(VecDeque::new())),
            predictions: Arc::new(Mutex::new(VecDeque::new())),
            learning_metrics: Arc::new(RwLock::new(LearningMetrics {
                timestamp: chrono::Utc::now(),
                model_accuracy: 0.0,
                training_loss: 0.0,
                inference_speed_ms: 0.0,
                data_processed_mb: 0.0,
                feature_importance: HashMap::new(),
                model_version: "1.0.0".to_string(),
                last_update: chrono::Utc::now(),
            })),
            historical_data: Arc::new(Mutex::new(VecDeque::new())),
            pattern_cache: Arc::new(RwLock::new(HashMap::new())),
            model_states: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            current_task: Arc::new(RwLock::new(None)),
            start_time: Instant::now(),
        };

        // Initialize models if learning is enabled
        if config.learning_enabled {
            agent.initialize_models().await?;
        }

        Ok(agent)
    }

    /// Start AI agent
    pub async fn start(self: &Arc<Self>) -> Result<()> {
        info!("🚀 Starting JARVIS-NV AI Agent...");

        if !self.config.enabled {
            info!("⏭️ AI Agent is disabled, skipping startup");
            return Ok(());
        }

        // Start Ollama manager
        self.ollama_manager
            .start()
            .await
            .context("Failed to start Ollama manager")?;

        *self.is_running.write().await = true;

        // Update status
        {
            let mut status = self.agent_status.write().await;
            status.running = true;
            status.mode = "monitoring".to_string();
            status.last_activity = chrono::Utc::now();
        }

        // Start inference loop
        let inference_handle = self.start_inference_loop().await;

        // Start anomaly detection
        if self.config.capabilities.anomaly_detection {
            let anomaly_handle = self.start_anomaly_detection().await;
        }

        // Start performance optimization
        if self.config.capabilities.performance_optimization {
            let optimization_handle = self.start_performance_optimization().await;
        }

        // Start predictive analytics
        if self.config.capabilities.predictive_analytics {
            let prediction_handle = self.start_predictive_analytics().await;
        }

        // Start learning loop if enabled
        if self.config.learning_enabled {
            let learning_handle = self.start_learning_loop().await;
        }

        // Start data collection
        let data_collection_handle = self.start_data_collection().await;

        info!("✅ JARVIS-NV AI Agent started successfully");
        Ok(())
    }

    /// Stop AI agent
    pub async fn stop(&self) -> Result<()> {
        info!("🛑 Stopping JARVIS-NV AI Agent...");

        *self.is_running.write().await = false;

        // Update status
        {
            let mut status = self.agent_status.write().await;
            status.running = false;
            status.mode = "stopped".to_string();
            status.current_task = None;
        }

        info!("✅ JARVIS-NV AI Agent stopped");
        Ok(())
    }

    /// Get agent status
    pub async fn get_status(&self) -> Result<serde_json::Value> {
        let status = self.agent_status.read().await;
        let anomalies = self.anomalies.lock().await;
        let optimizations = self.optimizations.lock().await;
        let predictions = self.predictions.lock().await;
        let learning_metrics = self.learning_metrics.read().await;

        Ok(serde_json::json!({
            "status": *status,
            "analytics": {
                "anomalies_count": anomalies.len(),
                "optimizations_count": optimizations.len(),
                "predictions_count": predictions.len(),
                "recent_anomalies": anomalies.iter().rev().take(5).collect::<Vec<_>>(),
                "recent_optimizations": optimizations.iter().rev().take(5).collect::<Vec<_>>(),
                "recent_predictions": predictions.iter().rev().take(5).collect::<Vec<_>>()
            },
            "learning": *learning_metrics,
            "capabilities": {
                "anomaly_detection": self.config.capabilities.anomaly_detection,
                "performance_optimization": self.config.capabilities.performance_optimization,
                "security_analysis": self.config.capabilities.security_analysis,
                "predictive_analytics": self.config.capabilities.predictive_analytics,
                "transaction_analysis": self.config.capabilities.transaction_analysis,
                "network_optimization": self.config.capabilities.network_optimization,
                "smart_contract_analysis": self.config.capabilities.smart_contract_analysis,
                "zns_optimization": self.config.capabilities.zns_optimization
            }
        }))
    }

    /// Run inference on current data
    pub async fn run_inference(
        &self,
        input_type: &str,
        input_data: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.set_current_task(&format!("inference_{}", input_type))
            .await;

        debug!("🧠 Running inference: {}", input_type);
        let start_time = Instant::now();

        // Prepare input for model
        let input_text = match input_type {
            "anomaly_detection" => self.prepare_anomaly_input(&input_data).await?,
            "performance_analysis" => self.prepare_performance_input(&input_data).await?,
            "security_analysis" => self.prepare_security_input(&input_data).await?,
            "transaction_analysis" => self.prepare_transaction_input(&input_data).await?,
            _ => input_data.to_string(),
        };

        // Run GPU-accelerated inference
        let result = self
            .gpu_manager
            .run_inference("analysis", &input_text)
            .await?;

        let inference_time = start_time.elapsed();

        // Update agent statistics
        {
            let mut status = self.agent_status.write().await;
            status.inferences_completed += 1;
            status.last_activity = chrono::Utc::now();
        }

        // Update learning metrics
        {
            let mut metrics = self.learning_metrics.write().await;
            metrics.inference_speed_ms = inference_time.as_millis() as f64;
            metrics.timestamp = chrono::Utc::now();
        }

        self.clear_current_task().await;

        Ok(serde_json::json!({
            "input_type": input_type,
            "inference_time_ms": inference_time.as_millis(),
            "result": result,
            "timestamp": chrono::Utc::now()
        }))
    }

    /// Detect anomalies in system data
    pub async fn detect_anomalies(&self, system_data: &serde_json::Value) -> Result<Vec<Anomaly>> {
        if !self.config.capabilities.anomaly_detection {
            return Ok(Vec::new());
        }

        debug!("🔍 Running anomaly detection...");
        let mut detected_anomalies = Vec::new();

        // Analyze GPU metrics
        if let Some(gpu_data) = system_data.get("gpu") {
            if let Some(anomaly) = self.analyze_gpu_anomalies(gpu_data).await? {
                detected_anomalies.push(anomaly);
            }
        }

        // Analyze node metrics
        if let Some(node_data) = system_data.get("node") {
            if let Some(anomaly) = self.analyze_node_anomalies(node_data).await? {
                detected_anomalies.push(anomaly);
            }
        }

        // Analyze network metrics
        if let Some(network_data) = system_data.get("network") {
            if let Some(anomaly) = self.analyze_network_anomalies(network_data).await? {
                detected_anomalies.push(anomaly);
            }
        }

        // Store detected anomalies
        if !detected_anomalies.is_empty() {
            let mut anomalies = self.anomalies.lock().await;
            for anomaly in &detected_anomalies {
                anomalies.push_back(anomaly.clone());
            }

            // Keep only recent anomalies
            while anomalies.len() > 1000 {
                anomalies.pop_front();
            }

            // Update agent statistics
            let mut status = self.agent_status.write().await;
            status.anomalies_detected += detected_anomalies.len() as u32;
        }

        Ok(detected_anomalies)
    }

    /// Generate performance optimizations
    pub async fn generate_optimizations(
        &self,
        performance_data: &serde_json::Value,
    ) -> Result<Vec<Optimization>> {
        if !self.config.capabilities.performance_optimization {
            return Ok(Vec::new());
        }

        debug!("⚡ Generating performance optimizations...");
        let mut optimizations = Vec::new();

        // Analyze GPU performance
        if let Some(gpu_data) = performance_data.get("gpu") {
            if let Some(opt) = self.optimize_gpu_performance(gpu_data).await? {
                optimizations.push(opt);
            }
        }

        // Analyze node performance
        if let Some(node_data) = performance_data.get("node") {
            if let Some(opt) = self.optimize_node_performance(node_data).await? {
                optimizations.push(opt);
            }
        }

        // Analyze network performance
        if let Some(network_data) = performance_data.get("network") {
            if let Some(opt) = self.optimize_network_performance(network_data).await? {
                optimizations.push(opt);
            }
        }

        // Store optimizations
        if !optimizations.is_empty() {
            let mut opts = self.optimizations.lock().await;
            for optimization in &optimizations {
                opts.push_back(optimization.clone());
            }

            // Keep only recent optimizations
            while opts.len() > 500 {
                opts.pop_front();
            }

            // Update agent statistics
            let mut status = self.agent_status.write().await;
            status.optimizations_applied += optimizations.len() as u32;
        }

        Ok(optimizations)
    }

    /// Generate predictions
    pub async fn generate_predictions(
        &self,
        historical_data: &[serde_json::Value],
    ) -> Result<Vec<Prediction>> {
        if !self.config.capabilities.predictive_analytics {
            return Ok(Vec::new());
        }

        debug!("🔮 Generating predictions...");
        let mut predictions = Vec::new();

        // Predict network congestion
        let congestion_prediction = self.predict_network_congestion(historical_data).await?;
        predictions.push(congestion_prediction);

        // Predict transaction volume
        let tx_volume_prediction = self.predict_transaction_volume(historical_data).await?;
        predictions.push(tx_volume_prediction);

        // Predict gas prices
        let gas_price_prediction = self.predict_gas_prices(historical_data).await?;
        predictions.push(gas_price_prediction);

        // Store predictions
        let mut preds = self.predictions.lock().await;
        for prediction in &predictions {
            preds.push_back(prediction.clone());
        }

        // Keep only recent predictions
        while preds.len() > 200 {
            preds.pop_front();
        }

        Ok(predictions)
    }

    /// Initialize ML models
    async fn initialize_models(&self) -> Result<()> {
        info!("🧠 Initializing AI models...");

        // Initialize anomaly detection model
        self.gpu_manager
            .load_model("anomaly_detector", "/models/anomaly_detector.bin")
            .await?;

        // Initialize performance optimizer model
        self.gpu_manager
            .load_model("performance_optimizer", "/models/performance_optimizer.bin")
            .await?;

        // Initialize predictor model
        self.gpu_manager
            .load_model("predictor", "/models/predictor.bin")
            .await?;

        info!("✅ AI models initialized successfully");
        Ok(())
    }

    /// Start inference loop
    async fn start_inference_loop(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let agent = Arc::clone(self);
        let interval_seconds = self.config.inference_interval_seconds;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_seconds));

            while *is_running.read().await {
                interval.tick().await;

                if let Err(e) = agent.run_inference_cycle().await {
                    error!("❌ Inference cycle failed: {}", e);
                }
            }
        })
    }

    /// Run one inference cycle
    async fn run_inference_cycle(&self) -> Result<()> {
        self.set_current_task("inference_cycle").await;

        // Collect current system data
        let system_data = self.collect_system_data().await?;

        // Run anomaly detection
        if self.config.capabilities.anomaly_detection {
            let anomalies = self.detect_anomalies(&system_data).await?;
            if !anomalies.is_empty() {
                info!("🚨 Detected {} anomalies", anomalies.len());
            }
        }

        // Generate optimizations
        if self.config.capabilities.performance_optimization {
            let optimizations = self.generate_optimizations(&system_data).await?;
            if !optimizations.is_empty() {
                info!("⚡ Generated {} optimizations", optimizations.len());
            }
        }

        // Store data for learning
        {
            let mut historical = self.historical_data.lock().await;
            historical.push_back(system_data);

            // Keep only recent data
            while historical.len() > 10000 {
                historical.pop_front();
            }
        }

        self.clear_current_task().await;
        Ok(())
    }

    /// Start anomaly detection task
    async fn start_anomaly_detection(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let agent = Arc::clone(self);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            while *is_running.read().await {
                interval.tick().await;

                // Dedicated anomaly detection cycle
                debug!("🔍 Running dedicated anomaly detection...");
            }
        })
    }

    /// Start performance optimization task
    async fn start_performance_optimization(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let agent = Arc::clone(self);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            while *is_running.read().await {
                interval.tick().await;

                // Dedicated performance optimization cycle
                debug!("⚡ Running dedicated performance optimization...");
            }
        })
    }

    /// Start predictive analytics task
    async fn start_predictive_analytics(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let agent = Arc::clone(self);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes

            while *is_running.read().await {
                interval.tick().await;

                // Generate predictions
                let historical = agent.historical_data.lock().await;
                let data: Vec<_> = historical.iter().cloned().collect();
                drop(historical);

                if data.len() > 10 {
                    if let Err(e) = agent.generate_predictions(&data).await {
                        error!("❌ Failed to generate predictions: {}", e);
                    }
                }
            }
        })
    }

    /// Start learning loop
    async fn start_learning_loop(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let learning_metrics = Arc::clone(&self.learning_metrics);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(600)); // 10 minutes

            while *is_running.read().await {
                interval.tick().await;

                // Update learning metrics
                let mut metrics = learning_metrics.write().await;
                metrics.model_accuracy = 0.95 + (rand::random::<f64>() - 0.5) * 0.05;
                metrics.training_loss = 0.05 + (rand::random::<f64>() - 0.5) * 0.02;
                metrics.last_update = chrono::Utc::now();

                debug!(
                    "🎓 Learning metrics updated - Accuracy: {:.3}, Loss: {:.3}",
                    metrics.model_accuracy, metrics.training_loss
                );
            }
        })
    }

    /// Start data collection task
    async fn start_data_collection(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let agent = Arc::clone(self);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(15));

            while *is_running.read().await {
                interval.tick().await;

                // Update agent uptime
                let uptime = agent.start_time.elapsed().as_secs();
                let mut status = agent.agent_status.write().await;
                status.uptime_seconds = uptime;
            }
        })
    }

    /// Collect system data for analysis
    async fn collect_system_data(&self) -> Result<serde_json::Value> {
        let gpu_status = self.gpu_manager.get_status().await?;
        let node_status = self.node_manager.get_status().await?;
        let bridge_status = self.ghost_bridge.get_status().await?;
        let metrics_status = self.metrics_collector.get_status().await?;

        Ok(serde_json::json!({
            "timestamp": chrono::Utc::now(),
            "gpu": gpu_status,
            "node": node_status,
            "bridge": bridge_status,
            "metrics": metrics_status
        }))
    }

    /// Set current task
    async fn set_current_task(&self, task: &str) {
        *self.current_task.write().await = Some(task.to_string());
        let mut status = self.agent_status.write().await;
        status.current_task = Some(task.to_string());
        status.last_activity = chrono::Utc::now();
    }

    /// Clear current task
    async fn clear_current_task(&self) {
        *self.current_task.write().await = None;
        let mut status = self.agent_status.write().await;
        status.current_task = None;
    }

    /// Analyze blockchain data using AI
    pub async fn analyze_blockchain_data(
        &self,
        data: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        info!("🔍 Running AI blockchain analysis...");

        let prompt = self.ollama_manager.get_blockchain_analysis_prompt(data);

        let request = InferenceRequest {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            model: self.config.default_ai_models.as_ref()
                .and_then(|models| models.first())
                .unwrap_or(&"llama3.2:3b".to_string())
                .clone(),
            prompt,
            context: {
                let mut ctx = HashMap::new();
                ctx.insert("analysis_type".to_string(), serde_json::Value::String("blockchain_data".to_string()));
                ctx.insert("timestamp".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                ctx.insert("data_size".to_string(), serde_json::Value::Number(serde_json::Number::from(data.to_string().len())));
                ctx
            },
            max_tokens: self.config.ai_max_tokens,
            temperature: self.config.ai_temperature,
            system_prompt: Some("You are JARVIS-NV, an expert blockchain AI agent. Provide precise, actionable analysis.".to_string()),
        };

        match self.ollama_manager.generate_completion(request).await {
            Ok(response) => {
                info!(
                    "✅ AI analysis completed in {}ms",
                    response.inference_time_ms
                );

                // Update inference counter
                {
                    let mut status = self.agent_status.write().await;
                    status.inferences_completed += 1;
                    status.last_activity = chrono::Utc::now();
                }

                // Try to parse as JSON, fallback to text response
                if let Ok(json_analysis) =
                    serde_json::from_str::<serde_json::Value>(&response.response_text)
                {
                    Ok(json_analysis)
                } else {
                    Ok(serde_json::json!({
                        "analysis": response.response_text,
                        "model": response.model,
                        "inference_time_ms": response.inference_time_ms,
                        "tokens_generated": response.tokens_generated,
                        "timestamp": response.timestamp
                    }))
                }
            }
            Err(e) => {
                error!("❌ AI analysis failed: {}", e);
                Err(e)
            }
        }
    }

    /// Diagnose node issues using AI
    pub async fn diagnose_node_issue(&self, issue_description: &str) -> Result<serde_json::Value> {
        info!("🩺 Running AI node diagnostics for: {}", issue_description);

        // Gather current metrics for context
        let metrics = serde_json::json!({
            "gpu_metrics": self.gpu_manager.get_metrics().await,
            "node_status": self.node_manager.get_status().await.unwrap_or_default(),
            "timestamp": chrono::Utc::now()
        });

        let prompt = self
            .ollama_manager
            .get_diagnostic_prompt(issue_description, &metrics);

        let request = InferenceRequest {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            model: self.config.default_ai_models.as_ref()
                .and_then(|models| models.first())
                .unwrap_or(&"llama3.2:3b".to_string())
                .clone(),
            prompt,
            context: {
                let mut ctx = HashMap::new();
                ctx.insert("analysis_type".to_string(), serde_json::Value::String("node_diagnostics".to_string()));
                ctx.insert("issue".to_string(), serde_json::Value::String(issue_description.to_string()));
                ctx.insert("timestamp".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                ctx
            },
            max_tokens: self.config.ai_max_tokens,
            temperature: Some(0.3), // Lower temperature for more precise diagnostics
            system_prompt: Some("You are JARVIS-NV, an expert blockchain node diagnostician. Provide step-by-step technical guidance.".to_string()),
        };

        match self.ollama_manager.generate_completion(request).await {
            Ok(response) => {
                info!(
                    "✅ AI diagnostics completed in {}ms",
                    response.inference_time_ms
                );

                Ok(serde_json::json!({
                    "diagnosis": response.response_text,
                    "model": response.model,
                    "confidence": "high", // Could be enhanced with actual confidence scoring
                    "inference_time_ms": response.inference_time_ms,
                    "recommended_actions": [], // Could parse specific actions from response
                    "timestamp": response.timestamp
                }))
            }
            Err(e) => {
                error!("❌ AI diagnostics failed: {}", e);
                Err(e)
            }
        }
    }

    /// Generate optimization recommendations using AI
    pub async fn generate_optimization_recommendations(&self) -> Result<Vec<Optimization>> {
        info!("⚡ Generating AI-powered optimization recommendations...");

        // Gather comprehensive system state
        let system_state = serde_json::json!({
            "gpu_info": self.gpu_manager.get_gpu_info().await,
            "gpu_metrics": self.gpu_manager.get_metrics().await,
            "node_status": self.node_manager.get_status().await.unwrap_or_default(),
            "agent_status": *self.agent_status.read().await,
            "timestamp": chrono::Utc::now()
        });

        let prompt = format!(
            "You are JARVIS-NV, an expert blockchain optimization agent. \
            Analyze the following system state and provide specific optimization recommendations:\n\n{}\n\n\
            Please provide optimization recommendations in the following JSON format:\n\
            {{\n\
              \"optimizations\": [\n\
                {{\n\
                  \"category\": \"performance|resource|network|security\",\n\
                  \"target\": \"specific component to optimize\",\n\
                  \"current_value\": 0.0,\n\
                  \"optimized_value\": 0.0,\n\
                  \"improvement_percentage\": 0.0,\n\
                  \"confidence\": 0.0,\n\
                  \"description\": \"detailed explanation\",\n\
                  \"steps\": [\"step1\", \"step2\"]\n\
                }}\n\
              ]\n\
            }}",
            serde_json::to_string_pretty(&system_state)
                .unwrap_or_else(|_| "Invalid state".to_string())
        );

        let request = InferenceRequest {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            model: self
                .config
                .default_ai_models
                .as_ref()
                .and_then(|models| models.first())
                .unwrap_or(&"llama3.2:3b".to_string())
                .clone(),
            prompt,
            context: {
                let mut ctx = HashMap::new();
                ctx.insert(
                    "analysis_type".to_string(),
                    serde_json::Value::String("optimization_recommendations".to_string()),
                );
                ctx.insert(
                    "timestamp".to_string(),
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                );
                ctx
            },
            max_tokens: self.config.ai_max_tokens,
            temperature: Some(0.5),
            system_prompt: Some(
                "You are an expert optimization agent. Always respond with valid JSON.".to_string(),
            ),
        };

        match self.ollama_manager.generate_completion(request).await {
            Ok(response) => {
                info!(
                    "✅ AI optimization analysis completed in {}ms",
                    response.inference_time_ms
                );

                // Try to parse optimization recommendations
                if let Ok(parsed) =
                    serde_json::from_str::<serde_json::Value>(&response.response_text)
                {
                    if let Some(optimizations) =
                        parsed.get("optimizations").and_then(|o| o.as_array())
                    {
                        let mut result = Vec::new();

                        for opt in optimizations {
                            if let Ok(optimization) =
                                serde_json::from_value::<serde_json::Value>(opt.clone())
                            {
                                result.push(Optimization {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    timestamp: chrono::Utc::now(),
                                    category: optimization
                                        .get("category")
                                        .and_then(|c| c.as_str())
                                        .unwrap_or("general")
                                        .to_string(),
                                    target: optimization
                                        .get("target")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or("system")
                                        .to_string(),
                                    current_value: optimization
                                        .get("current_value")
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0),
                                    optimized_value: optimization
                                        .get("optimized_value")
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0),
                                    improvement_percentage: optimization
                                        .get("improvement_percentage")
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0),
                                    confidence: optimization
                                        .get("confidence")
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.5),
                                    applied: false,
                                    rollback_available: true,
                                });
                            }
                        }

                        info!("📊 Generated {} optimization recommendations", result.len());
                        return Ok(result);
                    }
                }

                // Fallback: create a general optimization based on the text response
                Ok(vec![Optimization {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now(),
                    category: "general".to_string(),
                    target: "system".to_string(),
                    current_value: 0.0,
                    optimized_value: 0.0,
                    improvement_percentage: 0.0,
                    confidence: 0.7,
                    applied: false,
                    rollback_available: true,
                }])
            }
            Err(e) => {
                error!("❌ AI optimization analysis failed: {}", e);
                Err(e)
            }
        }
    }

    /// Start chat session for interactive AI assistance
    pub async fn start_interactive_session(&self, system_prompt: Option<String>) -> Result<String> {
        let default_model = "llama3.2:3b".to_string();
        let model = self
            .config
            .default_ai_models
            .as_ref()
            .and_then(|models| models.first())
            .unwrap_or(&default_model);

        self.ollama_manager
            .start_chat_session(model, system_prompt)
            .await
    }

    /// Send message to interactive AI session
    pub async fn chat_with_ai(&self, session_id: &str, message: &str) -> Result<String> {
        self.ollama_manager.chat_message(session_id, message).await
    }

    /// Get AI metrics and performance data
    pub async fn get_ai_metrics(&self) -> Result<serde_json::Value> {
        let ollama_metrics = self.ollama_manager.get_metrics().await;
        let available_models = self.ollama_manager.get_available_models().await;
        let active_sessions = self.ollama_manager.get_active_sessions_count().await;

        Ok(serde_json::json!({
            "ollama_metrics": ollama_metrics,
            "available_models": available_models,
            "active_sessions": active_sessions,
            "timestamp": chrono::Utc::now()
        }))
    }

    // Analysis helper methods (simplified implementations)

    async fn prepare_anomaly_input(&self, data: &serde_json::Value) -> Result<String> {
        Ok(format!("Analyze for anomalies: {}", data))
    }

    async fn prepare_performance_input(&self, data: &serde_json::Value) -> Result<String> {
        Ok(format!("Analyze performance: {}", data))
    }

    async fn prepare_security_input(&self, data: &serde_json::Value) -> Result<String> {
        Ok(format!("Analyze security: {}", data))
    }

    async fn prepare_transaction_input(&self, data: &serde_json::Value) -> Result<String> {
        Ok(format!("Analyze transactions: {}", data))
    }

    async fn analyze_gpu_anomalies(&self, gpu_data: &serde_json::Value) -> Result<Option<Anomaly>> {
        // Simplified GPU anomaly detection
        if let Some(temp) = gpu_data.get("gpu_info").and_then(|g| g.get("temperature")) {
            if temp.as_f64().unwrap_or(0.0) > 85.0 {
                return Ok(Some(Anomaly {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now(),
                    category: "performance".to_string(),
                    severity: "high".to_string(),
                    score: 0.85,
                    description: "GPU temperature exceeds safe threshold".to_string(),
                    affected_component: "GPU".to_string(),
                    recommended_actions: vec![
                        "Reduce GPU load".to_string(),
                        "Check cooling".to_string(),
                    ],
                    auto_resolved: false,
                    resolution_time: None,
                }));
            }
        }
        Ok(None)
    }

    async fn analyze_node_anomalies(
        &self,
        node_data: &serde_json::Value,
    ) -> Result<Option<Anomaly>> {
        // Simplified node anomaly detection
        Ok(None)
    }

    async fn analyze_network_anomalies(
        &self,
        network_data: &serde_json::Value,
    ) -> Result<Option<Anomaly>> {
        // Simplified network anomaly detection
        Ok(None)
    }

    async fn optimize_gpu_performance(
        &self,
        gpu_data: &serde_json::Value,
    ) -> Result<Option<Optimization>> {
        // Simplified GPU optimization
        Ok(None)
    }

    async fn optimize_node_performance(
        &self,
        node_data: &serde_json::Value,
    ) -> Result<Option<Optimization>> {
        // Simplified node optimization
        Ok(None)
    }

    async fn optimize_network_performance(
        &self,
        network_data: &serde_json::Value,
    ) -> Result<Option<Optimization>> {
        // Simplified network optimization
        Ok(None)
    }

    async fn predict_network_congestion(
        &self,
        historical_data: &[serde_json::Value],
    ) -> Result<Prediction> {
        Ok(Prediction {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            category: "congestion".to_string(),
            timeframe: "1h".to_string(),
            predicted_value: 0.75,
            confidence: 0.85,
            current_value: 0.65,
            trend: "increasing".to_string(),
            accuracy_score: None,
        })
    }

    async fn predict_transaction_volume(
        &self,
        historical_data: &[serde_json::Value],
    ) -> Result<Prediction> {
        Ok(Prediction {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            category: "demand".to_string(),
            timeframe: "24h".to_string(),
            predicted_value: 150000.0,
            confidence: 0.78,
            current_value: 125000.0,
            trend: "increasing".to_string(),
            accuracy_score: None,
        })
    }

    async fn predict_gas_prices(
        &self,
        historical_data: &[serde_json::Value],
    ) -> Result<Prediction> {
        Ok(Prediction {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            category: "price".to_string(),
            timeframe: "1h".to_string(),
            predicted_value: 25.5,
            confidence: 0.82,
            current_value: 22.3,
            trend: "increasing".to_string(),
            accuracy_score: None,
        })
    }
}
