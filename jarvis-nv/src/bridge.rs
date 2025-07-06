/*!
 * GhostBridge for JARVIS-NV
 * 
 * Handles gRPC and QUIC communication with GhostChain network,
 * providing high-performance, low-latency blockchain operations.
 */

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, Instant};
use tracing::{info, warn, error, debug};
use tonic::{transport::Server, Request, Response, Status};
use quinn::{Endpoint, ServerConfig, ClientConfig};
use rustls::{Certificate, PrivateKey, ServerConfig as RustlsServerConfig};

use crate::config::{BridgeConfig, Web5Config};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeStatus {
    pub enabled: bool,
    pub grpc_endpoint: String,
    pub quic_endpoint: Option<String>,
    pub active_connections: u32,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time_ms: f64,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub connection_id: String,
    pub protocol: String, // "grpc", "quic", "http3"
    pub client_address: String,
    pub request_count: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub avg_latency_ms: f64,
    pub errors: u32,
    pub established_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeRequest {
    pub id: String,
    pub method: String,
    pub path: String,
    pub client_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub payload_size: usize,
    pub response_time_ms: Option<u64>,
    pub status_code: Option<u16>,
    pub error: Option<String>,
}

pub struct GhostBridge {
    config: BridgeConfig,
    web5_config: Web5Config,
    
    // Server components
    grpc_server: Option<Arc<tokio::task::JoinHandle<()>>>,
    quic_endpoint: Option<Arc<Endpoint>>,
    
    // Connection tracking
    active_connections: Arc<RwLock<HashMap<String, ConnectionMetrics>>>,
    request_history: Arc<Mutex<Vec<BridgeRequest>>>,
    
    // Metrics
    bridge_status: Arc<RwLock<BridgeStatus>>,
    connection_count: Arc<RwLock<u32>>,
    
    // Runtime state
    is_running: Arc<RwLock<bool>>,
    start_time: Instant,
}

// gRPC service definitions
pub mod ghostbridge_proto {
    tonic::include_proto!("ghostbridge");
}

use ghostbridge_proto::ghost_bridge_server::{GhostBridge as GhostBridgeService, GhostBridgeServer};
use ghostbridge_proto::{
    BlockRequest, BlockResponse, TransactionRequest, TransactionResponse,
    StatusRequest, StatusResponse, MetricsRequest, MetricsResponse,
};

#[derive(Default)]
pub struct GhostBridgeServiceImpl {
    connection_metrics: Arc<RwLock<HashMap<String, ConnectionMetrics>>>,
}

#[tonic::async_trait]
impl GhostBridgeService for GhostBridgeServiceImpl {
    async fn get_block(
        &self,
        request: Request<BlockRequest>,
    ) -> Result<Response<BlockResponse>, Status> {
        let req = request.into_inner();
        debug!("📦 Received block request: {:?}", req);

        // Simulate block retrieval
        tokio::time::sleep(Duration::from_millis(10)).await;

        let response = BlockResponse {
            block_hash: format!("0x{:064x}", req.block_number),
            block_number: req.block_number,
            timestamp: chrono::Utc::now().timestamp() as u64,
            transaction_count: 150,
            gas_used: 8500000,
            gas_limit: 30000000,
            miner: "0x742d35Cc6675C05C6d0e3E2CD6e0FB92F9D84D52".to_string(),
            difficulty: "0x1bc16d674ec80000".to_string(),
            total_difficulty: "0xa4a470781c00000000000000000000".to_string(),
            size: 50000,
            parent_hash: format!("0x{:064x}", req.block_number.saturating_sub(1)),
            state_root: "0x123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0".to_string(),
            transactions_root: "0xabcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn get_transaction(
        &self,
        request: Request<TransactionRequest>,
    ) -> Result<Response<TransactionResponse>, Status> {
        let req = request.into_inner();
        debug!("💳 Received transaction request: {}", req.tx_hash);

        // Simulate transaction retrieval
        tokio::time::sleep(Duration::from_millis(5)).await;

        let response = TransactionResponse {
            tx_hash: req.tx_hash.clone(),
            block_hash: format!("0x{:064x}", 12345),
            block_number: 12345,
            transaction_index: 42,
            from_address: "0x742d35Cc6675C05C6d0e3E2CD6e0FB92F9D84D52".to_string(),
            to_address: Some("0x8ba1f109551bD432803012645Hac136c4e43e".to_string()),
            value: "1000000000000000000".to_string(), // 1 ETH in wei
            gas: 21000,
            gas_price: "20000000000".to_string(), // 20 gwei
            gas_used: Some(21000),
            nonce: 42,
            input: "0x".to_string(),
            status: Some(1), // Success
            cumulative_gas_used: Some(21000),
            effective_gas_price: Some("20000000000".to_string()),
            r#type: Some(2), // EIP-1559
        };

        Ok(Response::new(response))
    }

    async fn get_status(
        &self,
        _request: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        debug!("📊 Received status request");

        let response = StatusResponse {
            node_version: "jarvis-nv-0.1.0".to_string(),
            chain_id: 1337,
            network_id: "ghostchain-mainnet".to_string(),
            block_height: 2500000,
            peer_count: 25,
            sync_status: "synced".to_string(),
            uptime_seconds: 86400, // 24 hours
            memory_usage_mb: 2048,
            cpu_usage_percent: 45.2,
            disk_usage_gb: 125.5,
        };

        Ok(Response::new(response))
    }

    async fn get_metrics(
        &self,
        _request: Request<MetricsRequest>,
    ) -> Result<Response<MetricsResponse>, Status> {
        debug!("📈 Received metrics request");

        let connections = self.connection_metrics.read().await;
        let active_connections = connections.len() as u32;
        let total_requests: u64 = connections.values().map(|c| c.request_count).sum();

        let response = MetricsResponse {
            active_connections,
            total_requests,
            requests_per_second: 125.5,
            avg_response_time_ms: 25.3,
            error_rate_percent: 0.05,
            throughput_mbps: 85.2,
            cpu_usage_percent: 35.8,
            memory_usage_mb: 1024,
            network_in_mbps: 42.1,
            network_out_mbps: 38.7,
        };

        Ok(Response::new(response))
    }
}

impl GhostBridge {
    /// Create new GhostBridge
    pub async fn new(config: &BridgeConfig, web5_config: &Web5Config) -> Result<Self> {
        info!("🌉 Initializing GhostBridge");

        Ok(Self {
            config: config.clone(),
            web5_config: web5_config.clone(),
            grpc_server: None,
            quic_endpoint: None,
            active_connections: Arc::new(RwLock::new(HashMap::new())),
            request_history: Arc::new(Mutex::new(Vec::new())),
            bridge_status: Arc::new(RwLock::new(BridgeStatus {
                enabled: config.enabled,
                grpc_endpoint: config.grpc_endpoint.clone(),
                quic_endpoint: config.quic_endpoint.clone(),
                active_connections: 0,
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                avg_response_time_ms: 0.0,
                last_activity: chrono::Utc::now(),
            })),
            connection_count: Arc::new(RwLock::new(0)),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        })
    }

    /// Start GhostBridge services
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting GhostBridge...");
        
        *self.is_running.write().await = true;

        if !self.config.enabled {
            info!("⏭️ GhostBridge is disabled, skipping startup");
            return Ok(());
        }

        // Start gRPC server
        self.start_grpc_server().await?;

        // Start QUIC server if enabled
        if self.config.quic_endpoint.is_some() {
            self.start_quic_server().await?;
        }

        // Start connection monitoring
        let monitoring_handle = self.start_connection_monitoring().await;

        // Start metrics collection
        let metrics_handle = self.start_metrics_collection().await;

        info!("✅ GhostBridge started successfully");
        Ok(())
    }

    /// Stop GhostBridge services
    pub async fn stop(&self) -> Result<()> {
        info!("🛑 Stopping GhostBridge...");
        
        *self.is_running.write().await = false;

        // Stop gRPC server
        if let Some(server_handle) = &self.grpc_server {
            server_handle.abort();
        }

        // Close QUIC endpoint
        if let Some(endpoint) = &self.quic_endpoint {
            endpoint.close(0u32.into(), b"shutdown");
        }

        info!("✅ GhostBridge stopped");
        Ok(())
    }

    /// Get bridge status
    pub async fn get_status(&self) -> Result<serde_json::Value> {
        let status = self.bridge_status.read().await;
        let connections = self.active_connections.read().await;
        let is_running = *self.is_running.read().await;

        Ok(serde_json::json!({
            "enabled": status.enabled,
            "running": is_running,
            "grpc_endpoint": status.grpc_endpoint,
            "quic_endpoint": status.quic_endpoint,
            "active_connections": connections.len(),
            "total_requests": status.total_requests,
            "successful_requests": status.successful_requests,
            "failed_requests": status.failed_requests,
            "avg_response_time_ms": status.avg_response_time_ms,
            "last_activity": status.last_activity,
            "uptime_seconds": self.start_time.elapsed().as_secs()
        }))
    }

    /// Start gRPC server
    async fn start_grpc_server(&self) -> Result<()> {
        let endpoint_url = url::Url::parse(&self.config.grpc_endpoint)
            .context("Invalid gRPC endpoint URL")?;
        
        let host = endpoint_url.host_str().unwrap_or("127.0.0.1");
        let port = endpoint_url.port().unwrap_or(9090);
        
        // Convert IPv6 addresses
        let addr = if host == "::" || host == "[::]" {
            format!("[::1]:{}", port).parse()
                .context("Failed to parse IPv6 address")?
        } else {
            format!("{}:{}", host, port).parse()
                .context("Failed to parse address")?
        };

        info!("🌐 Starting gRPC server on {}", addr);

        let service_impl = GhostBridgeServiceImpl {
            connection_metrics: Arc::clone(&self.active_connections),
        };

        let service = GhostBridgeServer::new(service_impl);

        let server_future = Server::builder()
            .add_service(service)
            .serve(addr);

        let handle = tokio::spawn(async move {
            if let Err(e) = server_future.await {
                error!("❌ gRPC server error: {}", e);
            }
        });

        // Store the handle for cleanup
        // Note: In a real implementation, we'd store this properly
        // self.grpc_server = Some(Arc::new(handle));

        info!("✅ gRPC server started on {}", addr);
        Ok(())
    }

    /// Start QUIC server
    async fn start_quic_server(&self) -> Result<()> {
        if let Some(quic_endpoint_str) = &self.config.quic_endpoint {
            info!("🚀 Starting QUIC server on {}", quic_endpoint_str);

            // Parse QUIC endpoint
            let endpoint_url = url::Url::parse(quic_endpoint_str)
                .context("Invalid QUIC endpoint URL")?;
            
            let host = endpoint_url.host_str().unwrap_or("127.0.0.1");
            let port = endpoint_url.port().unwrap_or(9091);
            
            let addr = if host == "::" || host == "[::]" {
                format!("[::1]:{}", port).parse()
                    .context("Failed to parse IPv6 address")?
            } else {
                format!("{}:{}", host, port).parse()
                    .context("Failed to parse address")?
            };

            // Create QUIC server configuration
            let server_config = self.create_quic_server_config().await?;
            let endpoint = Endpoint::server(server_config, addr)
                .context("Failed to create QUIC endpoint")?;

            // Start accepting connections
            let connections = Arc::clone(&self.active_connections);
            let is_running = Arc::clone(&self.is_running);

            tokio::spawn(async move {
                while *is_running.read().await {
                    if let Some(connecting) = endpoint.accept().await {
                        let connections = Arc::clone(&connections);
                        
                        tokio::spawn(async move {
                            match connecting.await {
                                Ok(connection) => {
                                    let connection_id = format!("quic_{}", uuid::Uuid::new_v4());
                                    info!("🔗 New QUIC connection: {}", connection_id);
                                    
                                    let metrics = ConnectionMetrics {
                                        timestamp: chrono::Utc::now(),
                                        connection_id: connection_id.clone(),
                                        protocol: "quic".to_string(),
                                        client_address: connection.remote_address().to_string(),
                                        request_count: 0,
                                        bytes_sent: 0,
                                        bytes_received: 0,
                                        avg_latency_ms: 0.0,
                                        errors: 0,
                                        established_at: chrono::Utc::now(),
                                    };
                                    
                                    connections.write().await.insert(connection_id.clone(), metrics);
                                    
                                    // Handle QUIC streams
                                    Self::handle_quic_connection(connection, connection_id, connections).await;
                                }
                                Err(e) => {
                                    error!("❌ Failed to establish QUIC connection: {}", e);
                                }
                            }
                        });
                    }
                }
            });

            info!("✅ QUIC server started on {}", addr);
        }

        Ok(())
    }

    /// Create QUIC server configuration
    async fn create_quic_server_config(&self) -> Result<ServerConfig> {
        // In a real implementation, this would load actual certificates
        let cert = include_bytes!("../certs/cert.pem");
        let key = include_bytes!("../certs/key.pem");

        // For now, create a self-signed certificate
        let cert = Certificate(cert.to_vec());
        let key = PrivateKey(key.to_vec());

        let mut rustls_config = RustlsServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .map_err(|e| anyhow::anyhow!("Failed to create TLS config: {}", e))?;

        rustls_config.alpn_protocols = vec![b"h3".to_vec(), b"hq-29".to_vec()];

        let mut server_config = ServerConfig::with_crypto(Arc::new(rustls_config));
        
        // Configure QUIC transport parameters
        let mut transport_config = quinn::TransportConfig::default();
        transport_config.max_concurrent_uni_streams(100_u32.into());
        transport_config.max_concurrent_bidi_streams(100_u32.into());
        transport_config.max_idle_timeout(Some(Duration::from_secs(60).try_into().unwrap()));
        
        server_config.transport = Arc::new(transport_config);

        Ok(server_config)
    }

    /// Handle QUIC connection
    async fn handle_quic_connection(
        connection: quinn::Connection,
        connection_id: String,
        connections: Arc<RwLock<HashMap<String, ConnectionMetrics>>>,
    ) {
        while let Ok((send, recv)) = connection.accept_bi().await {
            let connection_id = connection_id.clone();
            let connections = Arc::clone(&connections);
            
            tokio::spawn(async move {
                // Handle bidirectional stream
                match Self::handle_quic_stream(send, recv, &connection_id, &connections).await {
                    Ok(_) => {
                        debug!("✅ QUIC stream handled successfully");
                    }
                    Err(e) => {
                        error!("❌ Error handling QUIC stream: {}", e);
                        
                        // Update error count
                        if let Some(metrics) = connections.write().await.get_mut(&connection_id) {
                            metrics.errors += 1;
                        }
                    }
                }
            });
        }
        
        // Remove connection when done
        connections.write().await.remove(&connection_id);
        info!("🔌 QUIC connection closed: {}", connection_id);
    }

    /// Handle QUIC stream
    async fn handle_quic_stream(
        mut send: quinn::SendStream,
        mut recv: quinn::RecvStream,
        connection_id: &str,
        connections: &Arc<RwLock<HashMap<String, ConnectionMetrics>>>,
    ) -> Result<()> {
        // Read request data
        let mut buffer = Vec::new();
        while let Some(chunk) = recv.read_chunk(1024, false).await? {
            buffer.extend_from_slice(&chunk.bytes);
        }

        let request_size = buffer.len();
        
        // Process request (simplified)
        let response = serde_json::json!({
            "status": "success",
            "data": "QUIC response",
            "timestamp": chrono::Utc::now(),
            "connection_id": connection_id
        });

        let response_data = serde_json::to_vec(&response)?;
        
        // Send response
        send.write_all(&response_data).await?;
        send.finish().await?;

        // Update connection metrics
        if let Some(metrics) = connections.write().await.get_mut(connection_id) {
            metrics.request_count += 1;
            metrics.bytes_received += request_size as u64;
            metrics.bytes_sent += response_data.len() as u64;
        }

        Ok(())
    }

    /// Start connection monitoring
    async fn start_connection_monitoring(&self) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let connections = Arc::clone(&self.active_connections);
        let bridge_status = Arc::clone(&self.bridge_status);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            
            while *is_running.read().await {
                interval.tick().await;

                let connection_map = connections.read().await;
                let active_count = connection_map.len() as u32;
                
                // Update bridge status
                let mut status = bridge_status.write().await;
                status.active_connections = active_count;
                status.last_activity = chrono::Utc::now();

                // Calculate metrics
                let total_requests: u64 = connection_map.values().map(|c| c.request_count).sum();
                let total_latency: f64 = connection_map.values().map(|c| c.avg_latency_ms).sum();
                let avg_latency = if active_count > 0 {
                    total_latency / active_count as f64
                } else {
                    0.0
                };

                status.total_requests = total_requests;
                status.avg_response_time_ms = avg_latency;

                debug!("📊 Bridge monitoring: {} active connections, {} total requests", 
                       active_count, total_requests);
            }
        })
    }

    /// Start metrics collection
    async fn start_metrics_collection(&self) -> tokio::task::JoinHandle<()> {
        let is_running = Arc::clone(&self.is_running);
        let request_history = Arc::clone(&self.request_history);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            while *is_running.read().await {
                interval.tick().await;

                // Clean up old request history
                let mut history = request_history.lock().await;
                let cutoff = chrono::Utc::now() - chrono::Duration::hours(24);
                history.retain(|req| req.timestamp > cutoff);

                debug!("🧹 Cleaned up request history, {} requests retained", history.len());
            }
        })
    }
}
