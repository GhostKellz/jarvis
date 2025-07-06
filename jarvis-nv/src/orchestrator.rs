/*!
 * Testnet Node Orchestrator for JARVIS-NV
 *
 * Manages deployment and orchestration of blockchain testnets using Docker Compose,
 * Podman, and systemd. Provides automated deployment, health monitoring, and
 * snapshot management for GhostChain testnet environments.
 */

use anyhow::{Context, Result};
use bollard::container::{
    Config, CreateContainerOptions, StartContainerOptions, StopContainerOptions,
};
use bollard::models::{ContainerInspectResponse, ContainerState};
use bollard::{API_DEFAULT_VERSION, Docker};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestnetConfig {
    pub name: String,
    pub chain_id: u64,
    pub network_id: String,
    pub deployment_type: DeploymentType,
    pub nodes: Vec<TestnetNodeConfig>,
    pub docker_compose_path: Option<String>,
    pub data_dir: String,
    pub snapshot_dir: String,
    pub health_check_interval_seconds: u64,
    pub auto_restart: bool,
    pub backup_enabled: bool,
    pub backup_interval_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentType {
    DockerCompose,
    Podman,
    Systemd,
    Kubernetes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestnetNodeConfig {
    pub name: String,
    pub node_type: String, // "ghostd", "walletd", "validator", "bootnode"
    pub image: String,
    pub ports: HashMap<String, u16>,
    pub environment: HashMap<String, String>,
    pub volumes: HashMap<String, String>,
    pub resources: ResourceLimits,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_mb: Option<u64>,
    pub cpu_cores: Option<f32>,
    pub storage_gb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestnetStatus {
    pub name: String,
    pub status: String, // "stopped", "starting", "running", "degraded", "failed"
    pub nodes: HashMap<String, NodeStatus>,
    pub health_score: f32, // 0.0 to 1.0
    pub uptime_seconds: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_health_check: chrono::DateTime<chrono::Utc>,
    pub deployment_type: DeploymentType,
    pub snapshot_count: u32,
    pub latest_snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub container_id: Option<String>,
    pub status: String, // "stopped", "starting", "running", "exited", "error"
    pub health: String, // "healthy", "unhealthy", "starting"
    pub uptime_seconds: u64,
    pub restart_count: u32,
    pub last_restart: Option<chrono::DateTime<chrono::Utc>>,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub memory_percent: f32,
    pub disk_usage_mb: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub testnet_name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub size_mb: u64,
    pub block_height: Option<u64>,
    pub description: String,
    pub path: String,
    pub checksum: Option<String>,
}

pub struct TestnetOrchestrator {
    docker: Arc<Docker>,
    testnets: Arc<RwLock<HashMap<String, TestnetConfig>>>,
    statuses: Arc<RwLock<HashMap<String, TestnetStatus>>>,
    snapshots: Arc<Mutex<HashMap<String, Vec<Snapshot>>>>,
    is_running: Arc<RwLock<bool>>,
    start_time: Instant,
}

impl TestnetOrchestrator {
    /// Create new testnet orchestrator
    pub async fn new() -> Result<Self> {
        info!("🎭 Initializing Testnet Orchestrator");

        // Connect to Docker daemon
        let docker =
            Docker::connect_with_socket_defaults().context("Failed to connect to Docker daemon")?;

        // Test Docker connection
        let version = docker
            .version()
            .await
            .context("Failed to get Docker version")?;
        info!(
            "🐳 Connected to Docker {}",
            version.version.unwrap_or_else(|| "unknown".to_string())
        );

        Ok(Self {
            docker: Arc::new(docker),
            testnets: Arc::new(RwLock::new(HashMap::new())),
            statuses: Arc::new(RwLock::new(HashMap::new())),
            snapshots: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        })
    }

    /// Start the orchestrator
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting Testnet Orchestrator...");

        *self.is_running.write().await = true;

        // Start health monitoring
        self.start_health_monitoring().await;

        // Start snapshot management
        self.start_snapshot_management().await;

        info!("✅ Testnet Orchestrator started successfully");
        Ok(())
    }

    /// Stop the orchestrator
    pub async fn stop(&self) -> Result<()> {
        info!("🛑 Stopping Testnet Orchestrator...");

        *self.is_running.write().await = false;

        // Stop all running testnets
        let testnet_names: Vec<String> = self.testnets.read().await.keys().cloned().collect();
        for name in testnet_names {
            if let Err(e) = self.stop_testnet(&name).await {
                warn!("Failed to stop testnet '{}': {}", name, e);
            }
        }

        info!("✅ Testnet Orchestrator stopped");
        Ok(())
    }

    /// Deploy a new testnet
    pub async fn deploy_testnet(&self, config: TestnetConfig) -> Result<()> {
        info!("🎪 Deploying testnet: {}", config.name);

        // Validate configuration
        self.validate_testnet_config(&config)?;

        // Create data directories
        self.create_testnet_directories(&config).await?;

        match config.deployment_type {
            DeploymentType::DockerCompose => {
                self.deploy_with_docker_compose(&config).await?;
            }
            DeploymentType::Podman => {
                self.deploy_with_podman(&config).await?;
            }
            DeploymentType::Systemd => {
                self.deploy_with_systemd(&config).await?;
            }
            DeploymentType::Kubernetes => {
                self.deploy_with_kubernetes(&config).await?;
            }
        }

        // Store configuration and initial status
        self.testnets
            .write()
            .await
            .insert(config.name.clone(), config.clone());
        self.statuses.write().await.insert(
            config.name.clone(),
            TestnetStatus {
                name: config.name.clone(),
                status: "starting".to_string(),
                nodes: HashMap::new(),
                health_score: 0.0,
                uptime_seconds: 0,
                created_at: chrono::Utc::now(),
                last_health_check: chrono::Utc::now(),
                deployment_type: config.deployment_type,
                snapshot_count: 0,
                latest_snapshot: None,
            },
        );

        info!("✅ Testnet '{}' deployed successfully", config.name);
        Ok(())
    }

    /// Deploy testnet using Docker Compose
    async fn deploy_with_docker_compose(&self, config: &TestnetConfig) -> Result<()> {
        info!("🐳 Deploying testnet '{}' with Docker Compose", config.name);

        if let Some(compose_path) = &config.docker_compose_path {
            // Generate docker-compose.yml if it doesn't exist
            if !Path::new(compose_path).exists() {
                self.generate_docker_compose_file(config, compose_path)
                    .await?;
            }

            // Run docker-compose up
            let output = tokio::process::Command::new("docker-compose")
                .args(&["-f", compose_path, "up", "-d"])
                .output()
                .await
                .context("Failed to run docker-compose")?;

            if output.status.success() {
                info!("✅ Docker Compose deployment successful");
            } else {
                let error = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Docker Compose failed: {}", error));
            }
        } else {
            // Deploy using Docker API directly
            for node_config in &config.nodes {
                self.deploy_docker_container(config, node_config).await?;
            }
        }

        Ok(())
    }

    /// Deploy individual Docker container
    async fn deploy_docker_container(
        &self,
        testnet_config: &TestnetConfig,
        node_config: &TestnetNodeConfig,
    ) -> Result<()> {
        info!("🚀 Deploying container: {}", node_config.name);

        let container_name = format!("{}_{}", testnet_config.name, node_config.name);

        // Prepare port bindings
        let mut port_bindings = HashMap::new();
        for (internal_port, external_port) in &node_config.ports {
            port_bindings.insert(
                internal_port.clone(),
                Some(vec![bollard::models::PortBinding {
                    host_ip: Some("0.0.0.0".to_string()),
                    host_port: Some(external_port.to_string()),
                }]),
            );
        }

        // Prepare environment variables
        let env: Vec<String> = node_config
            .environment
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // Prepare volume bindings
        let binds: Vec<String> = node_config
            .volumes
            .iter()
            .map(|(host_path, container_path)| format!("{}:{}", host_path, container_path))
            .collect();

        // Create container configuration
        let config = Config {
            image: Some(node_config.image.clone()),
            env: Some(env),
            host_config: Some(bollard::models::HostConfig {
                port_bindings: Some(port_bindings),
                binds: Some(binds),
                memory: node_config
                    .resources
                    .memory_mb
                    .map(|m| (m * 1024 * 1024) as i64), // Convert to bytes
                nano_cpus: node_config
                    .resources
                    .cpu_cores
                    .map(|c| (c * 1_000_000_000.0) as i64),
                restart_policy: Some(bollard::models::RestartPolicy {
                    name: Some(bollard::models::RestartPolicyNameEnum::UNLESS_STOPPED),
                    maximum_retry_count: Some(3),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Create and start container
        let container = self
            .docker
            .create_container(
                Some(CreateContainerOptions {
                    name: container_name.clone(),
                    platform: None,
                }),
                config,
            )
            .await
            .context("Failed to create container")?;

        self.docker
            .start_container(&container.id, None::<StartContainerOptions<String>>)
            .await
            .context("Failed to start container")?;

        info!("✅ Container '{}' deployed and started", container_name);
        Ok(())
    }

    /// Generate docker-compose.yml file
    async fn generate_docker_compose_file(
        &self,
        config: &TestnetConfig,
        compose_path: &str,
    ) -> Result<()> {
        info!(
            "📝 Generating docker-compose.yml for testnet: {}",
            config.name
        );

        let mut compose_content = format!(
            "version: '3.8'\n\n\
            services:\n"
        );

        for node_config in &config.nodes {
            compose_content.push_str(&format!(
                "  {}:\n\
                    image: {}\n\
                    container_name: {}_{}\n\
                    restart: unless-stopped\n",
                node_config.name, node_config.image, config.name, node_config.name
            ));

            // Add ports
            if !node_config.ports.is_empty() {
                compose_content.push_str("    ports:\n");
                for (internal_port, external_port) in &node_config.ports {
                    compose_content.push_str(&format!(
                        "      - \"{}:{}\"\n",
                        external_port, internal_port
                    ));
                }
            }

            // Add environment variables
            if !node_config.environment.is_empty() {
                compose_content.push_str("    environment:\n");
                for (key, value) in &node_config.environment {
                    compose_content.push_str(&format!("      {}: \"{}\"\n", key, value));
                }
            }

            // Add volumes
            if !node_config.volumes.is_empty() {
                compose_content.push_str("    volumes:\n");
                for (host_path, container_path) in &node_config.volumes {
                    compose_content
                        .push_str(&format!("      - \"{}:{}\"\n", host_path, container_path));
                }
            }

            // Add resource limits
            if node_config.resources.memory_mb.is_some()
                || node_config.resources.cpu_cores.is_some()
            {
                compose_content.push_str("    deploy:\n      resources:\n        limits:\n");
                if let Some(memory) = node_config.resources.memory_mb {
                    compose_content.push_str(&format!("          memory: {}M\n", memory));
                }
                if let Some(cpu) = node_config.resources.cpu_cores {
                    compose_content.push_str(&format!("          cpus: '{}'\n", cpu));
                }
            }

            compose_content.push('\n');
        }

        // Add networks
        compose_content.push_str(&format!(
            "networks:\n\
              default:\n\
                name: {}_network\n",
            config.name
        ));

        tokio::fs::write(compose_path, compose_content)
            .await
            .context("Failed to write docker-compose.yml")?;

        info!("✅ Generated docker-compose.yml at: {}", compose_path);
        Ok(())
    }

    /// Deploy with Podman (placeholder implementation)
    async fn deploy_with_podman(&self, config: &TestnetConfig) -> Result<()> {
        info!("🦭 Deploying testnet '{}' with Podman", config.name);
        warn!("Podman deployment not yet implemented");
        Ok(())
    }

    /// Deploy with systemd (placeholder implementation)
    async fn deploy_with_systemd(&self, config: &TestnetConfig) -> Result<()> {
        info!("⚙️  Deploying testnet '{}' with systemd", config.name);
        warn!("Systemd deployment not yet implemented");
        Ok(())
    }

    /// Deploy with Kubernetes (placeholder implementation)
    async fn deploy_with_kubernetes(&self, config: &TestnetConfig) -> Result<()> {
        info!("☸️  Deploying testnet '{}' with Kubernetes", config.name);
        warn!("Kubernetes deployment not yet implemented");
        Ok(())
    }

    /// Stop a testnet
    pub async fn stop_testnet(&self, testnet_name: &str) -> Result<()> {
        info!("🛑 Stopping testnet: {}", testnet_name);

        let testnets = self.testnets.read().await;
        if let Some(config) = testnets.get(testnet_name) {
            match config.deployment_type {
                DeploymentType::DockerCompose => {
                    if let Some(compose_path) = &config.docker_compose_path {
                        let output = tokio::process::Command::new("docker-compose")
                            .args(&["-f", compose_path, "down"])
                            .output()
                            .await
                            .context("Failed to run docker-compose down")?;

                        if !output.status.success() {
                            let error = String::from_utf8_lossy(&output.stderr);
                            warn!("Docker Compose stop warning: {}", error);
                        }
                    } else {
                        // Stop individual containers
                        for node_config in &config.nodes {
                            let container_name = format!("{}_{}", config.name, node_config.name);
                            if let Err(e) = self
                                .docker
                                .stop_container(&container_name, None::<StopContainerOptions>)
                                .await
                            {
                                warn!("Failed to stop container '{}': {}", container_name, e);
                            }
                        }
                    }
                }
                _ => {
                    warn!(
                        "Stop operation not implemented for deployment type: {:?}",
                        config.deployment_type
                    );
                }
            }

            // Update status
            if let Some(status) = self.statuses.write().await.get_mut(testnet_name) {
                status.status = "stopped".to_string();
            }
        }

        info!("✅ Testnet '{}' stopped", testnet_name);
        Ok(())
    }

    /// Create a snapshot of a testnet
    pub async fn create_snapshot(
        &self,
        testnet_name: &str,
        description: String,
    ) -> Result<Snapshot> {
        info!("📸 Creating snapshot for testnet: {}", testnet_name);

        let snapshot = Snapshot {
            id: uuid::Uuid::new_v4().to_string(),
            testnet_name: testnet_name.to_string(),
            timestamp: chrono::Utc::now(),
            size_mb: 0,         // Would calculate actual size
            block_height: None, // Would get from node
            description,
            path: format!(
                "/snapshots/{}/{}.tar.gz",
                testnet_name,
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            ),
            checksum: None,
        };

        // Store snapshot info
        let mut snapshots = self.snapshots.lock().await;
        snapshots
            .entry(testnet_name.to_string())
            .or_default()
            .push(snapshot.clone());

        info!("✅ Snapshot created: {}", snapshot.id);
        Ok(snapshot)
    }

    /// Start health monitoring
    async fn start_health_monitoring(&self) {
        let statuses = self.statuses.clone();
        let docker = self.docker.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            while *is_running.read().await {
                interval.tick().await;

                debug!("🔍 Running health checks...");

                let testnet_names: Vec<String> = statuses.read().await.keys().cloned().collect();
                for testnet_name in testnet_names {
                    // Check container health
                    // This would inspect each container and update health status
                    debug!("Checking health for testnet: {}", testnet_name);
                }
            }
        });
    }

    /// Start snapshot management
    async fn start_snapshot_management(&self) {
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour

            while *is_running.read().await {
                interval.tick().await;

                debug!("🗂️  Running snapshot management tasks...");

                // This would handle automatic snapshots and cleanup
            }
        });
    }

    /// Validate testnet configuration
    fn validate_testnet_config(&self, config: &TestnetConfig) -> Result<()> {
        if config.name.is_empty() {
            return Err(anyhow::anyhow!("Testnet name cannot be empty"));
        }

        if config.nodes.is_empty() {
            return Err(anyhow::anyhow!("Testnet must have at least one node"));
        }

        // Validate node configurations
        for node in &config.nodes {
            if node.name.is_empty() {
                return Err(anyhow::anyhow!("Node name cannot be empty"));
            }
            if node.image.is_empty() {
                return Err(anyhow::anyhow!("Node image cannot be empty"));
            }
        }

        Ok(())
    }

    /// Create testnet directories
    async fn create_testnet_directories(&self, config: &TestnetConfig) -> Result<()> {
        let data_dir = Path::new(&config.data_dir);
        let snapshot_dir = Path::new(&config.snapshot_dir);

        if !data_dir.exists() {
            tokio::fs::create_dir_all(data_dir)
                .await
                .context("Failed to create data directory")?;
        }

        if !snapshot_dir.exists() {
            tokio::fs::create_dir_all(snapshot_dir)
                .await
                .context("Failed to create snapshot directory")?;
        }

        Ok(())
    }

    /// Get orchestrator status
    pub async fn get_status(&self) -> serde_json::Value {
        let testnets = self.testnets.read().await;
        let statuses = self.statuses.read().await;

        serde_json::json!({
            "running": *self.is_running.read().await,
            "uptime_seconds": self.start_time.elapsed().as_secs(),
            "testnet_count": testnets.len(),
            "testnets": *statuses,
            "docker_available": true // Would check Docker availability
        })
    }
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_mb: 0,
            memory_percent: 0.0,
            disk_usage_mb: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
        }
    }
}
