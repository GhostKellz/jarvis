// jarvis-core/src/grpc_client.rs
//! GhostChain gRPC client with IPv6 and modern network optimizations

use anyhow::{Context, Result};
use std::time::Duration;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};
use tonic::{Request, Streaming};
use tracing::{debug, info, warn};

// Generated gRPC types
pub mod ghostchain {
    pub mod blockchain {
        tonic::include_proto!("ghostchain.blockchain");
    }
    pub mod transaction {
        tonic::include_proto!("ghostchain.transaction");
    }
    pub mod network {
        tonic::include_proto!("ghostchain.network");
    }
}

use ghostchain::blockchain::{blockchain_service_client::BlockchainServiceClient, Block, NetworkInfo, GasPrice};
use ghostchain::transaction::{transaction_service_client::TransactionServiceClient, TransactionAnalysis, MempoolStatus};
use ghostchain::network::{network_service_client::NetworkServiceClient, NetworkMetrics, IPv6Status, QuicStatus};

/// Configuration for GhostChain gRPC connection
#[derive(Debug, Clone)]
pub struct GhostChainConfig {
    pub endpoint: String,
    pub use_tls: bool,
    pub ipv6_preferred: bool,
    pub connection_timeout: Duration,
    pub request_timeout: Duration,
    pub max_concurrent_streams: u32,
}

impl Default for GhostChainConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://[::1]:9090".to_string(), // IPv6 localhost
            use_tls: true,
            ipv6_preferred: true,
            connection_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            max_concurrent_streams: 100,
        }
    }
}

impl From<crate::config::GhostChainConfig> for GhostChainConfig {
    fn from(config: crate::config::GhostChainConfig) -> Self {
        Self {
            endpoint: config.grpc_url,
            use_tls: config.use_tls,
            ipv6_preferred: config.ipv6_preferred,
            connection_timeout: Duration::from_secs(config.connection_timeout_secs),
            request_timeout: Duration::from_secs(config.request_timeout_secs),
            max_concurrent_streams: config.max_concurrent_streams,
        }
    }
}

/// GhostChain gRPC client with modern network optimizations
#[derive(Clone)]
pub struct GhostChainClient {
    blockchain_client: BlockchainServiceClient<Channel>,
    transaction_client: TransactionServiceClient<Channel>,
    network_client: NetworkServiceClient<Channel>,
    config: GhostChainConfig,
}

impl GhostChainClient {
    /// Create a new GhostChain client with optimized network configuration
    pub async fn new(config: GhostChainConfig) -> Result<Self> {
        info!("Connecting to GhostChain node at {}", config.endpoint);
        
        let channel = Self::create_optimized_channel(&config).await
            .context("Failed to create gRPC channel")?;

        let blockchain_client = BlockchainServiceClient::new(channel.clone())
            .max_decoding_message_size(64 * 1024 * 1024) // 64MB
            .max_encoding_message_size(64 * 1024 * 1024);

        let transaction_client = TransactionServiceClient::new(channel.clone())
            .max_decoding_message_size(32 * 1024 * 1024) // 32MB
            .max_encoding_message_size(32 * 1024 * 1024);

        let network_client = NetworkServiceClient::new(channel)
            .max_decoding_message_size(16 * 1024 * 1024) // 16MB
            .max_encoding_message_size(16 * 1024 * 1024);

        Ok(Self {
            blockchain_client,
            transaction_client,
            network_client,
            config,
        })
    }

    /// Create an optimized gRPC channel with IPv6 support
    async fn create_optimized_channel(config: &GhostChainConfig) -> Result<Channel> {
        let mut endpoint = Endpoint::from_shared(config.endpoint.clone())
            .context("Invalid endpoint URL")?
            .timeout(config.connection_timeout)
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .http2_keep_alive_interval(Duration::from_secs(30))
            .keep_alive_timeout(Duration::from_secs(5))
            .concurrency_limit(config.max_concurrent_streams as usize);

        // Configure TLS if enabled
        if config.use_tls {
            let tls_config = ClientTlsConfig::new();
            
            endpoint = endpoint.tls_config(tls_config)
                .context("Failed to configure TLS")?;
        }

        // Create the channel
        let channel = endpoint.connect().await
            .context("Failed to connect to GhostChain node")?;

        debug!("Successfully connected to GhostChain node");
        Ok(channel)
    }

    /// Test connection to GhostChain node
    pub async fn test_connection(&mut self) -> Result<bool> {
        debug!("Testing connection to GhostChain node");
        
        match self.get_network_info().await {
            Ok(info) => {
                info!("Connected to GhostChain network: {} (chain_id: {})", 
                     info.network_name, info.chain_id);
                Ok(true)
            }
            Err(e) => {
                warn!("Connection test failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get latest block from the blockchain
    pub async fn get_latest_block(&mut self) -> Result<Block> {
        debug!("Fetching latest block");
        
        let request = Request::new(ghostchain::blockchain::Empty {});
        let response = self.blockchain_client.get_latest_block(request).await
            .context("Failed to get latest block")?;

        Ok(response.into_inner())
    }

    /// Get network information
    pub async fn get_network_info(&mut self) -> Result<NetworkInfo> {
        debug!("Fetching network information");
        
        let request = Request::new(ghostchain::blockchain::Empty {});
        let response = self.blockchain_client.get_network_info(request).await
            .context("Failed to get network info")?;

        Ok(response.into_inner())
    }

    /// Get current gas prices
    pub async fn get_gas_price(&mut self) -> Result<GasPrice> {
        debug!("Fetching gas price information");
        
        let request = Request::new(ghostchain::blockchain::Empty {});
        let response = self.blockchain_client.get_gas_price(request).await
            .context("Failed to get gas price")?;

        Ok(response.into_inner())
    }

    /// Stream new blocks as they are mined
    pub async fn stream_blocks(&mut self) -> Result<Streaming<Block>> {
        info!("Starting block stream");
        
        let request = Request::new(ghostchain::blockchain::Empty {});
        let response = self.blockchain_client.stream_blocks(request).await
            .context("Failed to start block stream")?;

        Ok(response.into_inner())
    }

    /// Get mempool status
    pub async fn get_mempool_status(&mut self) -> Result<MempoolStatus> {
        debug!("Fetching mempool status");
        
        let request = Request::new(ghostchain::transaction::Empty {});
        let response = self.transaction_client.get_mempool_status(request).await
            .context("Failed to get mempool status")?;

        Ok(response.into_inner())
    }

    /// Analyze a transaction for security and optimization
    pub async fn analyze_transaction(&mut self, tx_hash: &str, deep_analysis: bool) -> Result<TransactionAnalysis> {
        debug!("Analyzing transaction: {}", tx_hash);
        
        let request = Request::new(ghostchain::transaction::AnalyzeTransactionRequest {
            transaction_hash: tx_hash.to_string(),
            deep_analysis,
            analysis_types: vec![
                "security".to_string(),
                "gas".to_string(), 
                "mev".to_string(),
            ],
        });

        let response = self.transaction_client.analyze_transaction(request).await
            .context("Failed to analyze transaction")?;

        Ok(response.into_inner())
    }

    /// Get current network performance metrics
    pub async fn get_network_metrics(&mut self) -> Result<NetworkMetrics> {
        debug!("Fetching network metrics");
        
        let request = Request::new(ghostchain::network::Empty {});
        let response = self.network_client.get_network_metrics(request).await
            .context("Failed to get network metrics")?;

        Ok(response.into_inner())
    }

    /// Get IPv6 status and configuration
    pub async fn get_ipv6_status(&mut self) -> Result<IPv6Status> {
        debug!("Fetching IPv6 status");
        
        let request = Request::new(ghostchain::network::Empty {});
        let response = self.network_client.get_i_pv6_status(request).await
            .context("Failed to get IPv6 status")?;

        Ok(response.into_inner())
    }

    /// Get QUIC status and configuration  
    pub async fn get_quic_status(&mut self) -> Result<QuicStatus> {
        debug!("Fetching QUIC status");
        
        let request = Request::new(ghostchain::network::Empty {});
        let response = self.network_client.get_quic_status(request).await
            .context("Failed to get QUIC status")?;

        Ok(response.into_inner())
    }

    /// Stream network metrics for real-time monitoring
    pub async fn stream_network_metrics(&mut self) -> Result<Streaming<NetworkMetrics>> {
        info!("Starting network metrics stream");
        
        let request = Request::new(ghostchain::network::Empty {});
        let response = self.network_client.stream_network_metrics(request).await
            .context("Failed to start network metrics stream")?;

        Ok(response.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let config = GhostChainConfig::default();
        
        // This will fail without a running GhostChain node, but tests the client creation
        let result = GhostChainClient::new(config).await;
        
        // We expect this to fail in tests without a node
        assert!(result.is_err());
    }

    #[test]
    fn test_default_config() {
        let config = GhostChainConfig::default();
        
        assert!(config.ipv6_preferred);
        assert_eq!(config.endpoint, "https://[::1]:9090");
    }
}
