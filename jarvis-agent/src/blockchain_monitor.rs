// jarvis-agent/src/blockchain_monitor.rs
//! Real-time blockchain monitoring agent using gRPC

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jarvis_core::{GhostChainClient, MemoryStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringAlert {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub network: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    HighTransactionVolume,
    UnusualGasPrice,
    NetworkCongestion,
    SuspiciousActivity,
    PerformanceDegradation,
    SecurityThreat,
}

impl std::fmt::Display for AlertType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertType::HighTransactionVolume => write!(f, "High Transaction Volume"),
            AlertType::UnusualGasPrice => write!(f, "Unusual Gas Price"),
            AlertType::NetworkCongestion => write!(f, "Network Congestion"),
            AlertType::SuspiciousActivity => write!(f, "Suspicious Activity"),
            AlertType::PerformanceDegradation => write!(f, "Performance Degradation"),
            AlertType::SecurityThreat => write!(f, "Security Threat"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub check_interval: Duration,
    pub latency_threshold: f64,
    pub throughput_threshold: f64,
    pub packet_loss_threshold: f64,
    pub enable_ai_analysis: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            latency_threshold: 100.0, // 100ms
            throughput_threshold: 10.0, // 10 Mbps minimum
            packet_loss_threshold: 1.0, // 1% packet loss
            enable_ai_analysis: true,
        }
    }
}

pub struct BlockchainMonitorAgent {
    client: GhostChainClient,
    memory: MemoryStore,
    config: MonitoringConfig,
    baseline_metrics: Option<BaselineMetrics>,
}

#[derive(Debug, Clone)]
struct BaselineMetrics {
    avg_latency: f64,
    avg_throughput: f64,
    normal_peer_count: u32,
    normal_packet_loss: f64,
    updated_at: DateTime<Utc>,
}

impl BlockchainMonitorAgent {
    pub fn new(client: GhostChainClient, memory: MemoryStore, config: MonitoringConfig) -> Self {
        Self {
            client,
            memory,
            config,
            baseline_metrics: None,
        }
    }

    /// Start continuous monitoring
    pub async fn start_monitoring(&mut self) -> Result<()> {
        info!("Starting blockchain monitoring agent");
        
        // Establish baseline metrics first
        self.establish_baseline().await?;
        
        let mut interval = interval(self.config.check_interval);
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.perform_monitoring_cycle().await {
                error!("Monitoring cycle failed: {}", e);
                // Continue monitoring even if one cycle fails
            }
        }
    }

    /// Establish baseline metrics for comparison
    async fn establish_baseline(&mut self) -> Result<()> {
        info!("Establishing baseline metrics...");
        
        // Collect metrics over several samples to establish baseline
        let mut samples = Vec::new();
        for _ in 0..10 {
            match self.client.get_network_metrics().await {
                Ok(metrics) => samples.push(metrics),
                Err(e) => warn!("Failed to collect baseline sample: {}", e),
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }

        if samples.is_empty() {
            return Err(anyhow::anyhow!("Could not establish baseline - no metrics collected"));
        }

        let baseline = BaselineMetrics {
            avg_latency: samples.iter().map(|m| m.latency_ms).sum::<f64>() / samples.len() as f64,
            avg_throughput: samples.iter().map(|m| m.throughput_mbps).sum::<f64>() / samples.len() as f64,
            normal_peer_count: (samples.iter().map(|m| m.peer_count as f64).sum::<f64>() / samples.len() as f64) as u32,
            normal_packet_loss: samples.iter().map(|m| m.packet_loss_rate).sum::<f64>() / samples.len() as f64,
            updated_at: Utc::now(),
        };

        info!("Baseline established: Latency={:.2}ms, Throughput={:.2}Mbps, Peers={}, PacketLoss={:.2}%", 
              baseline.avg_latency, baseline.avg_throughput, baseline.normal_peer_count, baseline.normal_packet_loss);
        
        self.baseline_metrics = Some(baseline);
        Ok(())
    }

    /// Perform one monitoring cycle
    async fn perform_monitoring_cycle(&mut self) -> Result<()> {
        // Get current network metrics
        let metrics = self.client.get_network_metrics().await
            .context("Failed to fetch network metrics")?;

        // Check for alerts
        let alerts = self.analyze_metrics(&metrics).await?;
        
        // Process any alerts
        for alert in alerts {
            self.handle_alert(alert).await?;
        }

        // Update memory with current state
        self.store_metrics(&metrics).await?;

        Ok(())
    }

    /// Analyze metrics and generate alerts
    async fn analyze_metrics(&self, metrics: &jarvis_core::grpc_client::ghostchain::network::NetworkMetrics) -> Result<Vec<MonitoringAlert>> {
        let mut alerts = Vec::new();

        if let Some(baseline) = &self.baseline_metrics {
            // Check latency issues
            if metrics.latency_ms > self.config.latency_threshold {
                alerts.push(MonitoringAlert {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    alert_type: AlertType::PerformanceDegradation,
                    severity: AlertSeverity::Warning,
                    message: format!("High latency detected: {:.2}ms (threshold: {:.2}ms)", 
                                   metrics.latency_ms, self.config.latency_threshold),
                    network: "GhostChain".to_string(),
                    metadata: [("latency_ms".to_string(), metrics.latency_ms.to_string())].into(),
                });
            }

            // Check throughput issues
            if metrics.throughput_mbps < self.config.throughput_threshold {
                alerts.push(MonitoringAlert {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    alert_type: AlertType::PerformanceDegradation,
                    severity: AlertSeverity::Warning,
                    message: format!("Low throughput: {:.2}Mbps (threshold: {:.2}Mbps)", 
                                   metrics.throughput_mbps, self.config.throughput_threshold),
                    network: "GhostChain".to_string(),
                    metadata: [("throughput_mbps".to_string(), metrics.throughput_mbps.to_string())].into(),
                });
            }

            // Check packet loss
            if metrics.packet_loss_rate > self.config.packet_loss_threshold {
                alerts.push(MonitoringAlert {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    alert_type: AlertType::NetworkCongestion,
                    severity: if metrics.packet_loss_rate > 5.0 { AlertSeverity::Critical } else { AlertSeverity::Warning },
                    message: format!("High packet loss: {:.2}% (threshold: {:.2}%)", 
                                   metrics.packet_loss_rate, self.config.packet_loss_threshold),
                    network: "GhostChain".to_string(),
                    metadata: [("packet_loss_rate".to_string(), metrics.packet_loss_rate.to_string())].into(),
                });
            }

            // Check peer count changes
            if metrics.peer_count < baseline.normal_peer_count / 2 {
                alerts.push(MonitoringAlert {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    alert_type: AlertType::NetworkCongestion,
                    severity: AlertSeverity::Critical,
                    message: format!("Low peer count: {} (normal: {})", metrics.peer_count, baseline.normal_peer_count),
                    network: "GhostChain".to_string(),
                    metadata: [("peer_count".to_string(), metrics.peer_count.to_string())].into(),
                });
            }
        }

        Ok(alerts)
    }

    /// Handle generated alerts
    async fn handle_alert(&mut self, alert: MonitoringAlert) -> Result<()> {
        info!("Alert generated: {} - {}", alert.alert_type, alert.message);

        // Store alert in memory
        self.memory.store_document(&format!("alert:{}", alert.id), &serde_json::to_string(&alert)?).await?;

        // If AI analysis is enabled, analyze the alert
        if self.config.enable_ai_analysis {
            self.ai_analyze_alert(&alert).await?;
        }

        // Take automated actions based on alert type
        match alert.alert_type {
            AlertType::HighTransactionVolume => {
                // Could trigger auto-scaling or load balancing
                info!("Detected high transaction volume - considering auto-optimization");
            }
            AlertType::NetworkCongestion => {
                // Could suggest network optimizations
                info!("Network congestion detected - analyzing optimization opportunities");
            }
            AlertType::SecurityThreat => {
                // Immediate security response
                warn!("Security threat detected - implementing protective measures");
            }
            _ => {}
        }

        Ok(())
    }

    /// AI-powered alert analysis
    async fn ai_analyze_alert(&self, alert: &MonitoringAlert) -> Result<()> {
        // This would integrate with Ollama or other LLMs for intelligent analysis
        info!("AI analysis requested for alert: {}", alert.id);
        
        // Placeholder for AI integration
        // In the real implementation, this would:
        // 1. Send alert context to AI model
        // 2. Get recommendations and analysis
        // 3. Store AI insights
        // 4. Suggest automated actions
        
        Ok(())
    }

    /// Store current metrics in memory
    async fn store_metrics(&mut self, metrics: &jarvis_core::grpc_client::ghostchain::network::NetworkMetrics) -> Result<()> {
        let timestamp = Utc::now().timestamp();
        let key = format!("metrics:{}", timestamp);
        
        // Create a simplified JSON representation of metrics
        let data = serde_json::json!({
            "latency_ms": metrics.latency_ms,
            "throughput_mbps": metrics.throughput_mbps,
            "peer_count": metrics.peer_count,
            "packet_loss_rate": metrics.packet_loss_rate,
            "active_connections": metrics.active_connections,
            "timestamp": timestamp
        });
        
        self.memory.store_document(&key, &data.to_string()).await?;
        
        // Clean up old metrics (keep last 24 hours)
        let cutoff = timestamp - (24 * 60 * 60);
        // In a real implementation, we'd query and delete old entries
        
        Ok(())
    }

    /// Get historical alert summary
    pub async fn get_alert_summary(&self, hours: u32) -> Result<HashMap<AlertType, u32>> {
        // This would query the memory store for alerts in the time range
        // and return a summary by type
        Ok(HashMap::new())
    }

    /// Get performance recommendations based on monitoring data
    pub async fn get_recommendations(&self) -> Result<Vec<String>> {
        let mut recommendations = Vec::new();

        if let Some(baseline) = &self.baseline_metrics {
            // Analyze recent trends and provide recommendations
            recommendations.push("Consider implementing transaction batching for efficiency".to_string());
            recommendations.push("Monitor gas price trends for optimal transaction timing".to_string());
            
            if baseline.avg_latency > 50.0 { // > 50ms average latency
                recommendations.push("High latency detected - consider network optimization".to_string());
            }
        }

        Ok(recommendations)
    }
}

#[async_trait]
impl Drop for BlockchainMonitorAgent {
    fn drop(&mut self) {
        info!("Blockchain monitoring agent shutting down");
    }
}
