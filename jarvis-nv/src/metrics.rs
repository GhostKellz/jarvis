/*!
 * Metrics Collection and Export for JARVIS-NV
 *
 * Handles Prometheus metrics, performance monitoring, and health checks
 * for GPU, node, network, and agent operations.
 */

use anyhow::{Context, Result};
use prometheus::{
    Counter, CounterVec, Encoder, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec, Opts,
    Registry, TextEncoder,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use warp::{Filter, Reply};

use crate::config::MetricsConfig;
use crate::gpu::GpuManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub uptime_seconds: u64,
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_total_bytes: u64,
    pub disk_usage_bytes: u64,
    pub disk_total_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub inferences_total: u64,
    pub inferences_successful: u64,
    pub inferences_failed: u64,
    pub avg_inference_time_ms: f64,
    pub models_loaded: u32,
    pub anomalies_detected: u32,
    pub optimizations_applied: u32,
    pub security_alerts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub block_height: u64,
    pub peer_count: u32,
    pub mempool_size: u32,
    pub sync_progress: f64,
    pub transaction_count_24h: u64,
    pub avg_block_time_seconds: f64,
    pub node_status: String,
    pub chain_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub active_connections: u32,
    pub quic_connections: u32,
    pub http3_requests: u64,
    pub grpc_requests: u64,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub connection_errors: u32,
    pub avg_response_time_ms: f64,
}

pub struct MetricsCollector {
    config: MetricsConfig,
    registry: Registry,
    gpu_manager: Arc<GpuManager>,

    // Prometheus metrics
    system_metrics: SystemPrometheusMetrics,
    gpu_metrics: GpuPrometheusMetrics,
    agent_metrics: AgentPrometheusMetrics,
    node_metrics: NodePrometheusMetrics,
    network_metrics: NetworkPrometheusMetrics,

    // Internal state
    start_time: Instant,
    metrics_history: Arc<Mutex<Vec<serde_json::Value>>>,
    is_running: Arc<RwLock<bool>>,
}

struct SystemPrometheusMetrics {
    uptime: Gauge,
    cpu_usage: Gauge,
    memory_usage: Gauge,
    memory_total: Gauge,
    disk_usage: Gauge,
    disk_total: Gauge,
    network_rx: Counter,
    network_tx: Counter,
}

struct GpuPrometheusMetrics {
    utilization_gpu: Gauge,
    utilization_memory: Gauge,
    memory_used: Gauge,
    memory_free: Gauge,
    temperature: Gauge,
    power_draw: Gauge,
    inference_rate: Gauge,
    models_loaded: Gauge,
}

struct AgentPrometheusMetrics {
    inferences_total: Counter,
    inferences_success: Counter,
    inferences_failed: Counter,
    inference_duration: Histogram,
    models_loaded: Gauge,
    anomalies_detected: Counter,
    optimizations_applied: Counter,
    security_alerts: Counter,
}

struct NodePrometheusMetrics {
    block_height: Gauge,
    peer_count: Gauge,
    mempool_size: Gauge,
    sync_progress: Gauge,
    transaction_count: Counter,
    block_time: Histogram,
    node_up: Gauge,
}

struct NetworkPrometheusMetrics {
    active_connections: Gauge,
    quic_connections: Gauge,
    http3_requests: Counter,
    grpc_requests: Counter,
    bytes_received: Counter,
    bytes_sent: Counter,
    connection_errors: Counter,
    response_time: Histogram,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub async fn new(config: &MetricsConfig, gpu_manager: Arc<GpuManager>) -> Result<Self> {
        info!("📊 Initializing Metrics Collector");

        let registry = Registry::new();

        // Initialize Prometheus metrics
        let system_metrics = SystemPrometheusMetrics::new(&registry)?;
        let gpu_metrics = GpuPrometheusMetrics::new(&registry)?;
        let agent_metrics = AgentPrometheusMetrics::new(&registry)?;
        let node_metrics = NodePrometheusMetrics::new(&registry)?;
        let network_metrics = NetworkPrometheusMetrics::new(&registry)?;

        Ok(Self {
            config: config.clone(),
            registry,
            gpu_manager,
            system_metrics,
            gpu_metrics,
            agent_metrics,
            node_metrics,
            network_metrics,
            start_time: Instant::now(),
            metrics_history: Arc::new(Mutex::new(Vec::new())),
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start metrics collection
    pub async fn start(self: &Arc<Self>) -> Result<()> {
        info!("🚀 Starting Metrics Collector...");

        *self.is_running.write().await = true;

        // Start metrics collection task
        let _collection_handle = self.start_collection_task().await;

        // Start Prometheus HTTP server if enabled
        if self.config.enabled {
            let server_handle = self.start_prometheus_server().await?;
        }

        // Start metrics export task if configured
        if self.config.export.enabled {
            let export_handle = self.start_export_task().await;
        }

        info!("✅ Metrics Collector started successfully");
        Ok(())
    }

    /// Stop metrics collection
    pub async fn stop(&self) -> Result<()> {
        info!("🛑 Stopping Metrics Collector...");

        *self.is_running.write().await = false;

        info!("✅ Metrics Collector stopped");
        Ok(())
    }

    /// Get current metrics status
    pub async fn get_status(&self) -> Result<serde_json::Value> {
        let uptime = self.start_time.elapsed();
        let is_running = *self.is_running.read().await;
        let history_count = self.metrics_history.lock().await.len();

        Ok(serde_json::json!({
            "enabled": self.config.enabled,
            "running": is_running,
            "uptime": uptime.as_secs(),
            "collection_interval_seconds": self.config.collection_interval_seconds,
            "metrics_collected": history_count,
            "prometheus_endpoint": self.config.prometheus_endpoint,
            "export": {
                "enabled": self.config.export.enabled,
                "format": self.config.export.format,
                "endpoint": self.config.export.endpoint
            }
        }))
    }

    /// Get current system metrics
    pub async fn get_system_metrics(&self) -> Result<SystemMetrics> {
        let uptime = self.start_time.elapsed().as_secs();

        // In a real implementation, these would query actual system stats
        Ok(SystemMetrics {
            timestamp: chrono::Utc::now(),
            uptime_seconds: uptime,
            cpu_usage_percent: 45.2,
            memory_usage_bytes: 8 * 1024 * 1024 * 1024, // 8GB
            memory_total_bytes: 32 * 1024 * 1024 * 1024, // 32GB
            disk_usage_bytes: 50 * 1024 * 1024 * 1024,  // 50GB
            disk_total_bytes: 1024 * 1024 * 1024 * 1024, // 1TB
            network_rx_bytes: 1024 * 1024 * 500,        // 500MB
            network_tx_bytes: 1024 * 1024 * 200,        // 200MB
        })
    }

    /// Get current agent metrics
    pub async fn get_agent_metrics(&self) -> Result<AgentMetrics> {
        // In a real implementation, this would aggregate from the AI agent
        Ok(AgentMetrics {
            timestamp: chrono::Utc::now(),
            inferences_total: 1250,
            inferences_successful: 1200,
            inferences_failed: 50,
            avg_inference_time_ms: 125.5,
            models_loaded: 3,
            anomalies_detected: 5,
            optimizations_applied: 15,
            security_alerts: 2,
        })
    }

    /// Get current node metrics
    pub async fn get_node_metrics(&self) -> Result<NodeMetrics> {
        // In a real implementation, this would query the blockchain node
        Ok(NodeMetrics {
            timestamp: chrono::Utc::now(),
            block_height: 2_500_000,
            peer_count: 25,
            mempool_size: 150,
            sync_progress: 100.0,
            transaction_count_24h: 50_000,
            avg_block_time_seconds: 12.5,
            node_status: "synced".to_string(),
            chain_id: 1337,
        })
    }

    /// Get current network metrics
    pub async fn get_network_metrics(&self) -> Result<NetworkMetrics> {
        // In a real implementation, this would aggregate network stats
        Ok(NetworkMetrics {
            timestamp: chrono::Utc::now(),
            active_connections: 50,
            quic_connections: 25,
            http3_requests: 5000,
            grpc_requests: 2500,
            bytes_received: 1024 * 1024 * 100, // 100MB
            bytes_sent: 1024 * 1024 * 50,      // 50MB
            connection_errors: 5,
            avg_response_time_ms: 85.5,
        })
    }

    /// Get Prometheus metrics
    pub async fn get_prometheus_metrics(&self) -> Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    /// Update all metrics
    async fn update_metrics(&self) -> Result<()> {
        debug!("📊 Updating metrics...");

        // Update system metrics
        let system_metrics = self.get_system_metrics().await?;
        self.update_system_prometheus_metrics(&system_metrics)
            .await?;

        // Update GPU metrics if available
        if self.config.gpu_metrics {
            if let Ok(gpu_status) = self.gpu_manager.get_status().await {
                self.update_gpu_prometheus_metrics(&gpu_status).await?;
            }
        }

        // Update agent metrics
        if self.config.agent_metrics {
            let agent_metrics = self.get_agent_metrics().await?;
            self.update_agent_prometheus_metrics(&agent_metrics).await?;
        }

        // Update node metrics
        if self.config.node_metrics {
            let node_metrics = self.get_node_metrics().await?;
            self.update_node_prometheus_metrics(&node_metrics).await?;
        }

        // Update network metrics
        if self.config.network_metrics {
            let network_metrics = self.get_network_metrics().await?;
            self.update_network_prometheus_metrics(&network_metrics)
                .await?;
        }

        // Store metrics history
        let combined_metrics = serde_json::json!({
            "timestamp": chrono::Utc::now(),
            "system": system_metrics,
            "uptime": self.start_time.elapsed().as_secs()
        });

        let mut history = self.metrics_history.lock().await;
        history.push(combined_metrics);

        // Keep only recent metrics based on retention policy
        let max_entries = (self.config.retention_days as u64 * 24 * 60 * 60)
            / self.config.collection_interval_seconds;
        if history.len() > max_entries as usize {
            let excess = history.len() - max_entries as usize;
            history.drain(0..excess);
        }

        Ok(())
    }

    /// Start metrics collection task
    async fn start_collection_task(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let interval_seconds = self.config.collection_interval_seconds;
        let collector = Arc::clone(self);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_seconds));

            while *is_running.read().await {
                interval.tick().await;

                if let Err(e) = collector.update_metrics().await {
                    error!("❌ Failed to update metrics: {}", e);
                }
            }
        })
    }

    /// Start Prometheus HTTP server
    async fn start_prometheus_server(&self) -> Result<tokio::task::JoinHandle<()>> {
        let registry = self.registry.clone();
        let endpoint = self.config.prometheus_endpoint.clone();
        let is_running = Arc::clone(&self.is_running);

        // Parse endpoint to get bind address and port
        let url = url::Url::parse(&endpoint).context("Invalid Prometheus endpoint URL")?;

        let host = url.host_str().unwrap_or("127.0.0.1");
        let port = url.port().unwrap_or(9090);

        // Use IPv4 addresses only for now
        let bind_addr = if host == "0.0.0.0" {
            ([0, 0, 0, 0], port)
        } else if host == "127.0.0.1" || host == "localhost" {
            ([127, 0, 0, 1], port)
        } else {
            // Default to localhost
            ([127, 0, 0, 1], port)
        };

        let metrics_route = warp::path("metrics").and(warp::get()).map(move || {
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = Vec::new();
            if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
                return warp::reply::with_status(
                    format!("Error encoding metrics: {}", e),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                )
                .into_response();
            }

            warp::reply::with_header(
                String::from_utf8_lossy(&buffer).to_string(),
                "content-type",
                "text/plain; charset=utf-8",
            )
            .into_response()
        });

        let health_route = warp::path("health").and(warp::get()).map(|| {
            warp::reply::json(&serde_json::json!({
                "status": "healthy",
                "timestamp": chrono::Utc::now()
            }))
        });

        let routes = metrics_route.or(health_route);

        info!("🌐 Starting Prometheus server on {}:{}", host, port);

        let handle = tokio::spawn(async move {
            warp::serve(routes).run(bind_addr).await;
        });

        Ok(handle)
    }

    /// Start metrics export task
    async fn start_export_task(&self) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let export_config = self.config.export.clone();
        let metrics_history = Arc::clone(&self.metrics_history);

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(export_config.flush_interval_seconds));

            while *is_running.read().await {
                interval.tick().await;

                let history = metrics_history.lock().await;
                if history.len() >= export_config.batch_size as usize {
                    // Export metrics batch
                    let batch: Vec<_> = history
                        .iter()
                        .rev()
                        .take(export_config.batch_size as usize)
                        .cloned()
                        .collect();

                    match export_config.format.as_str() {
                        "prometheus" => {
                            debug!("📤 Exporting {} metrics to Prometheus", batch.len());
                            // In real implementation, would send to Prometheus pushgateway
                        }
                        "influxdb" => {
                            debug!("📤 Exporting {} metrics to InfluxDB", batch.len());
                            // In real implementation, would send to InfluxDB
                        }
                        "elasticsearch" => {
                            debug!("📤 Exporting {} metrics to Elasticsearch", batch.len());
                            // In real implementation, would send to Elasticsearch
                        }
                        _ => {
                            warn!("⚠️ Unknown export format: {}", export_config.format);
                        }
                    }
                }
            }
        })
    }

    /// Update system Prometheus metrics
    async fn update_system_prometheus_metrics(&self, metrics: &SystemMetrics) -> Result<()> {
        self.system_metrics
            .uptime
            .set(metrics.uptime_seconds as f64);
        self.system_metrics.cpu_usage.set(metrics.cpu_usage_percent);
        self.system_metrics
            .memory_usage
            .set(metrics.memory_usage_bytes as f64);
        self.system_metrics
            .memory_total
            .set(metrics.memory_total_bytes as f64);
        self.system_metrics
            .disk_usage
            .set(metrics.disk_usage_bytes as f64);
        self.system_metrics
            .disk_total
            .set(metrics.disk_total_bytes as f64);
        Ok(())
    }

    /// Update GPU Prometheus metrics
    async fn update_gpu_prometheus_metrics(&self, gpu_status: &serde_json::Value) -> Result<()> {
        if let Some(gpu_info) = gpu_status.get("gpu_info") {
            if let Some(utilization_gpu) = gpu_info.get("utilization_gpu") {
                self.gpu_metrics
                    .utilization_gpu
                    .set(utilization_gpu.as_f64().unwrap_or(0.0));
            }
            if let Some(utilization_memory) = gpu_info.get("utilization_memory") {
                self.gpu_metrics
                    .utilization_memory
                    .set(utilization_memory.as_f64().unwrap_or(0.0));
            }
            if let Some(memory_used) = gpu_info.get("memory_used") {
                self.gpu_metrics
                    .memory_used
                    .set(memory_used.as_f64().unwrap_or(0.0));
            }
            if let Some(memory_free) = gpu_info.get("memory_free") {
                self.gpu_metrics
                    .memory_free
                    .set(memory_free.as_f64().unwrap_or(0.0));
            }
            if let Some(temperature) = gpu_info.get("temperature") {
                self.gpu_metrics
                    .temperature
                    .set(temperature.as_f64().unwrap_or(0.0));
            }
            if let Some(power_draw) = gpu_info.get("power_draw") {
                self.gpu_metrics
                    .power_draw
                    .set(power_draw.as_f64().unwrap_or(0.0));
            }
        }
        Ok(())
    }

    /// Update agent Prometheus metrics
    async fn update_agent_prometheus_metrics(&self, metrics: &AgentMetrics) -> Result<()> {
        self.agent_metrics
            .models_loaded
            .set(metrics.models_loaded as f64);
        self.agent_metrics
            .inference_duration
            .observe(metrics.avg_inference_time_ms / 1000.0);
        Ok(())
    }

    /// Update node Prometheus metrics
    async fn update_node_prometheus_metrics(&self, metrics: &NodeMetrics) -> Result<()> {
        self.node_metrics
            .block_height
            .set(metrics.block_height as f64);
        self.node_metrics.peer_count.set(metrics.peer_count as f64);
        self.node_metrics
            .mempool_size
            .set(metrics.mempool_size as f64);
        self.node_metrics.sync_progress.set(metrics.sync_progress);
        self.node_metrics
            .block_time
            .observe(metrics.avg_block_time_seconds);
        self.node_metrics
            .node_up
            .set(if metrics.node_status == "synced" {
                1.0
            } else {
                0.0
            });
        Ok(())
    }

    /// Update network Prometheus metrics
    async fn update_network_prometheus_metrics(&self, metrics: &NetworkMetrics) -> Result<()> {
        self.network_metrics
            .active_connections
            .set(metrics.active_connections as f64);
        self.network_metrics
            .quic_connections
            .set(metrics.quic_connections as f64);
        self.network_metrics
            .response_time
            .observe(metrics.avg_response_time_ms / 1000.0);
        Ok(())
    }
}

// Prometheus metrics implementations
impl SystemPrometheusMetrics {
    fn new(registry: &Registry) -> Result<Self> {
        let uptime = Gauge::new("system_uptime_seconds", "System uptime in seconds")?;
        let cpu_usage = Gauge::new("system_cpu_usage_percent", "CPU usage percentage")?;
        let memory_usage = Gauge::new("system_memory_usage_bytes", "Memory usage in bytes")?;
        let memory_total = Gauge::new("system_memory_total_bytes", "Total memory in bytes")?;
        let disk_usage = Gauge::new("system_disk_usage_bytes", "Disk usage in bytes")?;
        let disk_total = Gauge::new("system_disk_total_bytes", "Total disk space in bytes")?;
        let network_rx = Counter::new("system_network_received_bytes", "Network bytes received")?;
        let network_tx = Counter::new(
            "system_network_transmitted_bytes",
            "Network bytes transmitted",
        )?;

        registry.register(Box::new(uptime.clone()))?;
        registry.register(Box::new(cpu_usage.clone()))?;
        registry.register(Box::new(memory_usage.clone()))?;
        registry.register(Box::new(memory_total.clone()))?;
        registry.register(Box::new(disk_usage.clone()))?;
        registry.register(Box::new(disk_total.clone()))?;
        registry.register(Box::new(network_rx.clone()))?;
        registry.register(Box::new(network_tx.clone()))?;

        Ok(Self {
            uptime,
            cpu_usage,
            memory_usage,
            memory_total,
            disk_usage,
            disk_total,
            network_rx,
            network_tx,
        })
    }
}

impl GpuPrometheusMetrics {
    fn new(registry: &Registry) -> Result<Self> {
        let utilization_gpu = Gauge::new("gpu_utilization_percent", "GPU utilization percentage")?;
        let utilization_memory = Gauge::new(
            "gpu_memory_utilization_percent",
            "GPU memory utilization percentage",
        )?;
        let memory_used = Gauge::new("gpu_memory_used_bytes", "GPU memory used in bytes")?;
        let memory_free = Gauge::new("gpu_memory_free_bytes", "GPU memory free in bytes")?;
        let temperature = Gauge::new("gpu_temperature_celsius", "GPU temperature in Celsius")?;
        let power_draw = Gauge::new("gpu_power_draw_watts", "GPU power draw in watts")?;
        let inference_rate = Gauge::new(
            "gpu_inference_rate_per_second",
            "GPU inference rate per second",
        )?;
        let models_loaded = Gauge::new("gpu_models_loaded", "Number of models loaded on GPU")?;

        registry.register(Box::new(utilization_gpu.clone()))?;
        registry.register(Box::new(utilization_memory.clone()))?;
        registry.register(Box::new(memory_used.clone()))?;
        registry.register(Box::new(memory_free.clone()))?;
        registry.register(Box::new(temperature.clone()))?;
        registry.register(Box::new(power_draw.clone()))?;
        registry.register(Box::new(inference_rate.clone()))?;
        registry.register(Box::new(models_loaded.clone()))?;

        Ok(Self {
            utilization_gpu,
            utilization_memory,
            memory_used,
            memory_free,
            temperature,
            power_draw,
            inference_rate,
            models_loaded,
        })
    }
}

impl AgentPrometheusMetrics {
    fn new(registry: &Registry) -> Result<Self> {
        let inferences_total =
            Counter::new("agent_inferences_total", "Total number of inferences")?;
        let inferences_success = Counter::new(
            "agent_inferences_success_total",
            "Total successful inferences",
        )?;
        let inferences_failed =
            Counter::new("agent_inferences_failed_total", "Total failed inferences")?;
        let inference_duration = Histogram::with_opts(HistogramOpts::new(
            "agent_inference_duration_seconds",
            "Inference duration in seconds",
        ))?;
        let models_loaded = Gauge::new("agent_models_loaded", "Number of models loaded")?;
        let anomalies_detected =
            Counter::new("agent_anomalies_detected_total", "Total anomalies detected")?;
        let optimizations_applied = Counter::new(
            "agent_optimizations_applied_total",
            "Total optimizations applied",
        )?;
        let security_alerts = Counter::new("agent_security_alerts_total", "Total security alerts")?;

        registry.register(Box::new(inferences_total.clone()))?;
        registry.register(Box::new(inferences_success.clone()))?;
        registry.register(Box::new(inferences_failed.clone()))?;
        registry.register(Box::new(inference_duration.clone()))?;
        registry.register(Box::new(models_loaded.clone()))?;
        registry.register(Box::new(anomalies_detected.clone()))?;
        registry.register(Box::new(optimizations_applied.clone()))?;
        registry.register(Box::new(security_alerts.clone()))?;

        Ok(Self {
            inferences_total,
            inferences_success,
            inferences_failed,
            inference_duration,
            models_loaded,
            anomalies_detected,
            optimizations_applied,
            security_alerts,
        })
    }
}

impl NodePrometheusMetrics {
    fn new(registry: &Registry) -> Result<Self> {
        let block_height = Gauge::new("node_block_height", "Current block height")?;
        let peer_count = Gauge::new("node_peer_count", "Number of connected peers")?;
        let mempool_size = Gauge::new("node_mempool_size", "Number of transactions in mempool")?;
        let sync_progress = Gauge::new(
            "node_sync_progress_percent",
            "Node sync progress percentage",
        )?;
        let transaction_count =
            Counter::new("node_transactions_total", "Total number of transactions")?;
        let block_time = Histogram::with_opts(HistogramOpts::new(
            "node_block_time_seconds",
            "Block time in seconds",
        ))?;
        let node_up = Gauge::new("node_up", "Node status (1 = up, 0 = down)")?;

        registry.register(Box::new(block_height.clone()))?;
        registry.register(Box::new(peer_count.clone()))?;
        registry.register(Box::new(mempool_size.clone()))?;
        registry.register(Box::new(sync_progress.clone()))?;
        registry.register(Box::new(transaction_count.clone()))?;
        registry.register(Box::new(block_time.clone()))?;
        registry.register(Box::new(node_up.clone()))?;

        Ok(Self {
            block_height,
            peer_count,
            mempool_size,
            sync_progress,
            transaction_count,
            block_time,
            node_up,
        })
    }
}

impl NetworkPrometheusMetrics {
    fn new(registry: &Registry) -> Result<Self> {
        let active_connections =
            Gauge::new("network_active_connections", "Number of active connections")?;
        let quic_connections =
            Gauge::new("network_quic_connections", "Number of QUIC connections")?;
        let http3_requests = Counter::new("network_http3_requests_total", "Total HTTP/3 requests")?;
        let grpc_requests = Counter::new("network_grpc_requests_total", "Total gRPC requests")?;
        let bytes_received = Counter::new("network_bytes_received_total", "Total bytes received")?;
        let bytes_sent = Counter::new("network_bytes_sent_total", "Total bytes sent")?;
        let connection_errors =
            Counter::new("network_connection_errors_total", "Total connection errors")?;
        let response_time = Histogram::with_opts(HistogramOpts::new(
            "network_response_time_seconds",
            "Response time in seconds",
        ))?;

        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(quic_connections.clone()))?;
        registry.register(Box::new(http3_requests.clone()))?;
        registry.register(Box::new(grpc_requests.clone()))?;
        registry.register(Box::new(bytes_received.clone()))?;
        registry.register(Box::new(bytes_sent.clone()))?;
        registry.register(Box::new(connection_errors.clone()))?;
        registry.register(Box::new(response_time.clone()))?;

        Ok(Self {
            active_connections,
            quic_connections,
            http3_requests,
            grpc_requests,
            bytes_received,
            bytes_sent,
            connection_errors,
            response_time,
        })
    }
}
