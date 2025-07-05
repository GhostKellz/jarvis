use super::*;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use chrono::{DateTime, Utc};

/// GhostChain network implementation using real ghostd/walletd services
pub struct GhostChainNetwork {
    pub rpc_client: Client,
    pub ghostd_url: String,
    pub walletd_url: Option<String>,
    pub ghostbridge_url: Option<String>,
    pub zvm_url: Option<String>,
    pub zns_url: Option<String>,
    pub chain_id: u64,
}

#[async_trait]
impl BlockchainNetwork for GhostChainNetwork {
    fn network_info(&self) -> NetworkInfo {
        NetworkInfo {
            name: "GhostChain".to_string(),
            chain_id: self.chain_id,
            network_type: NetworkType::GhostChain,
            rpc_endpoints: vec![self.ghostd_url.clone()],
            explorer_urls: vec!["https://ghostscan.io".to_string()],
            native_currency: CurrencyInfo {
                name: "Ghost".to_string(),
                symbol: "GHOST".to_string(),
                decimals: 18,
            },
        }
    }
    
    async fn get_latest_block(&self) -> Result<BlockInfo> {
        let response = self.rpc_client
            .post(&self.ghostd_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_getBlockByNumber",
                "params": ["latest", false],
                "id": 1
            }))
            .send()
            .await?;
            
        let block: Value = response.json().await?;
        
        if let Some(error) = block.get("error") {
            return Err(anyhow::anyhow!("RPC Error: {}", error));
        }
        
        let result = block["result"].as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid block response"))?;
        
        Ok(BlockInfo {
            number: u64::from_str_radix(
                result["number"].as_str().unwrap_or("0x0").trim_start_matches("0x"), 
                16
            )?,
            hash: result["hash"].as_str().unwrap_or("").to_string(),
            parent_hash: result["parentHash"].as_str().unwrap_or("").to_string(),
            timestamp: DateTime::from_timestamp(
                i64::from_str_radix(
                    result["timestamp"].as_str().unwrap_or("0x0").trim_start_matches("0x"), 
                    16
                )?,
                0
            ).unwrap_or_else(|| Utc::now()),
            transaction_count: result["transactions"].as_array()
                .map(|txs| txs.len() as u32)
                .unwrap_or(0),
            gas_used: u64::from_str_radix(
                result["gasUsed"].as_str().unwrap_or("0x0").trim_start_matches("0x"), 
                16
            ).unwrap_or(0),
            gas_limit: u64::from_str_radix(
                result["gasLimit"].as_str().unwrap_or("0x0").trim_start_matches("0x"), 
                16
            ).unwrap_or(0),
            miner: result["miner"].as_str().map(|s| s.to_string()),
            size: u64::from_str_radix(
                result["size"].as_str().unwrap_or("0x0").trim_start_matches("0x"), 
                16
            ).unwrap_or(0),
        })
    }
    
    async fn get_gas_info(&self) -> Result<GasInfo> {
        let response = self.rpc_client
            .post(&self.ghostd_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_gasPrice",
                "params": [],
                "id": 1
            }))
            .send()
            .await?;
            
        let gas_data: Value = response.json().await?;
        
        if let Some(error) = gas_data.get("error") {
            return Err(anyhow::anyhow!("Gas price RPC Error: {}", error));
        }
        
        let gas_price = u64::from_str_radix(
            gas_data["result"].as_str().unwrap_or("0x0").trim_start_matches("0x"),
            16
        ).unwrap_or(0);
        
        // Get network congestion by checking pending transactions
        let pending_response = self.rpc_client
            .post(&self.ghostd_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_getBlockTransactionCountByNumber",
                "params": ["pending"],
                "id": 2
            }))
            .send()
            .await?;
            
        let pending_data: Value = pending_response.json().await?;
        let pending_count = u64::from_str_radix(
            pending_data["result"].as_str().unwrap_or("0x0").trim_start_matches("0x"),
            16
        ).unwrap_or(0);
        
        let congestion = match pending_count {
            0..=10 => CongestionLevel::Low,
            11..=50 => CongestionLevel::Medium,
            51..=200 => CongestionLevel::High,
            _ => CongestionLevel::Critical,
        };
        
        Ok(GasInfo {
            base_fee: gas_price / 2, // Estimate base fee
            priority_fee: gas_price / 4, // Estimate priority fee
            max_fee: gas_price,
            gas_price,
            estimated_confirmation_time: match congestion {
                CongestionLevel::Low => Duration::from_secs(15),
                CongestionLevel::Medium => Duration::from_secs(30),
                CongestionLevel::High => Duration::from_secs(60),
                CongestionLevel::Critical => Duration::from_secs(120),
            },
            network_congestion: congestion,
        })
    }
    
    async fn submit_transaction(&self, tx: Transaction) -> Result<String> {
        // If walletd is available, use it for signing
        if let Some(walletd_url) = &self.walletd_url {
            return self.submit_transaction_via_walletd(tx).await;
        }
        
        // Otherwise submit raw transaction
        let response = self.rpc_client
            .post(&self.ghostd_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_sendRawTransaction",
                "params": [tx.raw_data],
                "id": 1
            }))
            .send()
            .await?;
            
        let result: Value = response.json().await?;
        
        if let Some(error) = result.get("error") {
            return Err(anyhow::anyhow!("Transaction submission error: {}", error));
        }
        
        Ok(result["result"].as_str().unwrap_or("").to_string())
    }
    
    async fn get_transaction(&self, tx_hash: &str) -> Result<TransactionInfo> {
        let response = self.rpc_client
            .post(&self.ghostd_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_getTransactionByHash",
                "params": [tx_hash],
                "id": 1
            }))
            .send()
            .await?;
            
        let tx_data: Value = response.json().await?;
        
        if let Some(error) = tx_data.get("error") {
            return Err(anyhow::anyhow!("Transaction lookup error: {}", error));
        }
        
        let tx = tx_data["result"].as_object()
            .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
        
        // Get transaction receipt for status
        let receipt_response = self.rpc_client
            .post(&self.ghostd_url)
            .json(&json!({
                "jsonrpc": "2.0", 
                "method": "eth_getTransactionReceipt",
                "params": [tx_hash],
                "id": 2
            }))
            .send()
            .await?;
            
        let receipt_data: Value = receipt_response.json().await?;
        let receipt = receipt_data["result"].as_object();
        
        Ok(TransactionInfo {
            hash: tx_hash.to_string(),
            from: tx["from"].as_str().unwrap_or("").to_string(),
            to: tx["to"].as_str().map(|s| s.to_string()),
            value: tx["value"].as_str().unwrap_or("0x0").to_string(),
            gas_limit: u64::from_str_radix(
                tx["gas"].as_str().unwrap_or("0x0").trim_start_matches("0x"),
                16
            ).unwrap_or(0),
            gas_price: u64::from_str_radix(
                tx["gasPrice"].as_str().unwrap_or("0x0").trim_start_matches("0x"),
                16
            ).unwrap_or(0),
            gas_used: receipt.and_then(|r| r["gasUsed"].as_str())
                .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok()),
            block_number: tx["blockNumber"].as_str()
                .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok()),
            block_hash: tx["blockHash"].as_str().map(|s| s.to_string()),
            transaction_index: tx["transactionIndex"].as_str()
                .and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok()),
            status: receipt.and_then(|r| r["status"].as_str())
                .map(|s| if s == "0x1" { 
                    TransactionStatus::Success 
                } else { 
                    TransactionStatus::Failed 
                })
                .unwrap_or(TransactionStatus::Pending),
            confirmations: 0, // TODO: Calculate confirmations
        })
    }
    
    async fn get_network_health(&self) -> Result<NetworkHealth> {
        // Check ghostd health
        let block_result = self.get_latest_block().await;
        let gas_result = self.get_gas_info().await;
        
        let mut health_score = 100.0;
        let mut issues = Vec::new();
        
        // Check if ghostd is responding
        if block_result.is_err() {
            health_score -= 50.0;
            issues.push("ghostd not responding".to_string());
        }
        
        if gas_result.is_err() {
            health_score -= 20.0;
            issues.push("gas price API unavailable".to_string());
        }
        
        // Check walletd health if configured
        if let Some(walletd_url) = &self.walletd_url {
            let walletd_health = self.check_walletd_health(walletd_url).await;
            if walletd_health.is_err() {
                health_score -= 15.0;
                issues.push("walletd unavailable".to_string());
            }
        }
        
        // Check network congestion
        if let Ok(gas_info) = gas_result {
            match gas_info.network_congestion {
                CongestionLevel::Critical => {
                    health_score -= 10.0;
                    issues.push("critical network congestion".to_string());
                }
                CongestionLevel::High => {
                    health_score -= 5.0;
                    issues.push("high network congestion".to_string());
                }
                _ => {}
            }
        }
        
        Ok(NetworkHealth {
            overall_health: health_score,
            block_height: block_result.map(|b| b.number).unwrap_or(0),
            peer_count: 0, // TODO: Get peer count from ghostd
            sync_status: if health_score > 80.0 { 
                SyncStatus::Synced 
            } else { 
                SyncStatus::Syncing 
            },
            last_block_time: block_result.map(|b| b.timestamp).ok(),
            issues,
        })
    }
    
    async fn audit_contract(&self, contract_address: &str) -> Result<SecurityReport> {
        // Use ZVM for contract analysis if available
        if let Some(zvm_url) = &self.zvm_url {
            return self.audit_contract_via_zvm(contract_address).await;
        }
        
        // Basic contract analysis using ghostd
        let code_response = self.rpc_client
            .post(&self.ghostd_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_getCode",
                "params": [contract_address, "latest"],
                "id": 1
            }))
            .send()
            .await?;
            
        let code_data: Value = code_response.json().await?;
        
        if let Some(error) = code_data.get("error") {
            return Err(anyhow::anyhow!("Contract code lookup error: {}", error));
        }
        
        let code = code_data["result"].as_str().unwrap_or("0x");
        
        let mut vulnerabilities = Vec::new();
        let mut gas_optimizations = Vec::new();
        let mut compliance_score = 100.0;
        
        // Basic static analysis
        if code == "0x" {
            return Err(anyhow::anyhow!("Contract not found or not deployed"));
        }
        
        // Simple heuristic checks (replace with real analysis)
        if code.len() > 50000 {
            vulnerabilities.push(Vulnerability {
                vulnerability_type: VulnerabilityType::Custom("Large contract size".to_string()),
                severity: Severity::Medium,
                description: "Contract bytecode is unusually large".to_string(),
                location: "Entire contract".to_string(),
                recommendation: "Consider splitting into multiple contracts".to_string(),
            });
            compliance_score -= 10.0;
        }
        
        // Check for common patterns that might indicate issues
        if code.contains("delegatecall") {
            vulnerabilities.push(Vulnerability {
                vulnerability_type: VulnerabilityType::UnauthorizedAccess,
                severity: Severity::High,
                description: "Contract uses delegatecall which can be dangerous".to_string(),
                location: "Contract bytecode".to_string(),
                recommendation: "Ensure delegatecall targets are trusted".to_string(),
            });
            compliance_score -= 20.0;
        }
        
        gas_optimizations.push(GasOptimization {
            description: "Consider using packed structs to reduce storage costs".to_string(),
            estimated_savings: 2000,
            difficulty: OptimizationDifficulty::Medium,
        });
        
        let risk_level = if compliance_score >= 90.0 {
            RiskLevel::Low
        } else if compliance_score >= 70.0 {
            RiskLevel::Medium
        } else if compliance_score >= 50.0 {
            RiskLevel::High
        } else {
            RiskLevel::Critical
        };
        
        Ok(SecurityReport {
            contract_address: contract_address.to_string(),
            scan_date: Utc::now(),
            risk_level,
            vulnerabilities,
            gas_optimizations,
            compliance_score,
        })
    }
    
    async fn get_network_stats(&self) -> Result<NetworkStats> {
        let latest_block = self.get_latest_block().await?;
        let gas_info = self.get_gas_info().await?;
        
        Ok(NetworkStats {
            chain_id: self.chain_id,
            block_height: latest_block.number,
            total_transactions: 0, // TODO: Get from ghostd
            active_validators: 0, // TODO: Get validator count
            network_hashrate: 0.0, // TODO: Get hashrate if applicable
            average_block_time: 15.0, // TODO: Calculate from recent blocks
            pending_transactions: 0, // TODO: Get pending tx count
            gas_price_gwei: (gas_info.gas_price as f64) / 1e9,
            total_value_locked: 0.0, // TODO: Calculate TVL
        })
    }
}

impl GhostChainNetwork {
    /// Create a new GhostChain network connection
    pub async fn new(rpc_url: &str, chain_id: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        
        Ok(Self {
            rpc_client: client,
            ghostd_url: rpc_url.to_string(),
            walletd_url: None,
            ghostbridge_url: None,
            zvm_url: None,
            zns_url: None,
            chain_id,
        })
    }
    
    /// Create a new GhostChain network with all service URLs
    pub async fn new_with_services(
        rpc_url: &str,
        chain_id: u64,
        walletd_url: Option<String>,
        ghostbridge_url: Option<String>,
        zvm_url: Option<String>,
        zns_url: Option<String>,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        
        Ok(Self {
            rpc_client: client,
            ghostd_url: rpc_url.to_string(),
            walletd_url,
            ghostbridge_url,
            zvm_url,
            zns_url,
            chain_id,
        })
    }
}
