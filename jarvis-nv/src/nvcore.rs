/*!
 * NVIDIA Core Module for JARVIS-NV
 * 
 * Core NVIDIA-specific functionality including CUDA operations,
 * container runtime integration, and hardware acceleration.
 */

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, Instant};
use tracing::{info, warn, error, debug};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NvidiaCoreInfo {
    pub driver_version: String,
    pub cuda_version: Option<String>,
    pub devices: Vec<GpuDeviceInfo>,
    pub runtime_info: ContainerRuntimeInfo,
    pub system_info: SystemInfo,
    pub performance_profiles: Vec<PerformanceProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDeviceInfo {
    pub id: u32,
    pub name: String,
    pub uuid: String,
    pub pci_bus_id: String,
    pub compute_capability_major: u32,
    pub compute_capability_minor: u32,
    pub memory_total_mb: u64,
    pub memory_free_mb: u64,
    pub memory_used_mb: u64,
    pub utilization_gpu: u32,
    pub utilization_memory: u32,
    pub temperature_celsius: u32,
    pub power_draw_watts: u32,
    pub power_limit_watts: u32,
    pub clock_graphics_mhz: u32,
    pub clock_memory_mhz: u32,
    pub fan_speed_percent: u32,
    pub performance_state: String,
    pub persistence_mode: bool,
    pub accounting_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerRuntimeInfo {
    pub nvidia_container_runtime_available: bool,
    pub nvidia_container_toolkit_version: Option<String>,
    pub nvidia_docker_available: bool,
    pub container_runtime_version: Option<String>,
    pub supported_container_engines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub memory_total_gb: f64,
    pub memory_available_gb: f64,
    pub nvidia_ml_available: bool,
    pub cuda_toolkit_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceProfile {
    pub name: String,
    pub description: String,
    pub gpu_clock_offset: i32,
    pub memory_clock_offset: i32,
    pub power_limit_percent: u32,
    pub fan_speed_percent: Option<u32>,
    pub compute_mode: String,
    pub auto_boost_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NvidiaCoreMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub total_gpu_memory_gb: f64,
    pub total_gpu_utilization: f64,
    pub total_power_draw_watts: u32,
    pub max_temperature_celsius: u32,
    pub active_processes: u32,
    pub cuda_context_count: u32,
    pub memory_fragmentation_percent: f64,
}

pub struct NvidiaCore {
    // Device information
    nvidia_info: Arc<RwLock<Option<NvidiaCoreInfo>>>,
    device_metrics: Arc<Mutex<Vec<NvidiaCoreMetrics>>>,
    
    // Performance management
    active_profile: Arc<RwLock<Option<PerformanceProfile>>>,
    thermal_limits: Arc<RwLock<HashMap<u32, u32>>>, // device_id -> temp_limit
    
    // Process tracking
    cuda_processes: Arc<RwLock<HashMap<u32, Vec<CudaProcess>>>>, // device_id -> processes
    
    // Configuration
    monitoring_enabled: bool,
    auto_optimization: bool,
    thermal_protection: bool,
    
    // Runtime state
    is_initialized: Arc<RwLock<bool>>,
    last_update: Arc<RwLock<Option<Instant>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CudaProcess {
    pub pid: u32,
    pub process_name: String,
    pub gpu_memory_usage_mb: u64,
    pub gpu_utilization_percent: u32,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub command_line: Option<String>,
}

impl NvidiaCore {
    /// Create new NVIDIA core manager
    pub async fn new() -> Result<Self> {
        info!("🚀 Initializing NVIDIA Core...");

        let core = Self {
            nvidia_info: Arc::new(RwLock::new(None)),
            device_metrics: Arc::new(Mutex::new(Vec::new())),
            active_profile: Arc::new(RwLock::new(None)),
            thermal_limits: Arc::new(RwLock::new(HashMap::new())),
            cuda_processes: Arc::new(RwLock::new(HashMap::new())),
            monitoring_enabled: true,
            auto_optimization: false,
            thermal_protection: true,
            is_initialized: Arc::new(RwLock::new(false)),
            last_update: Arc::new(RwLock::new(None)),
        };

        // Initialize NVIDIA information
        core.initialize_nvidia_info().await?;

        *core.is_initialized.write().await = true;
        info!("✅ NVIDIA Core initialized successfully");

        Ok(core)
    }

    /// Initialize NVIDIA system information
    async fn initialize_nvidia_info(&self) -> Result<()> {
        info!("🔍 Detecting NVIDIA hardware and drivers...");

        // Check for nvidia-smi
        let nvidia_smi_available = Command::new("nvidia-smi")
            .arg("--version")
            .output()
            .is_ok();

        if !nvidia_smi_available {
            warn!("⚠️ nvidia-smi not available, NVIDIA features will be limited");
            return Ok(());
        }

        // Get driver version
        let driver_version = self.get_driver_version().await?;
        
        // Get CUDA version
        let cuda_version = self.get_cuda_version().await;

        // Get device information
        let devices = self.get_device_info().await?;

        // Get container runtime information
        let runtime_info = self.get_container_runtime_info().await?;

        // Get system information
        let system_info = self.get_system_info().await?;

        // Get performance profiles
        let performance_profiles = self.get_performance_profiles().await;

        let nvidia_info = NvidiaCoreInfo {
            driver_version,
            cuda_version,
            devices,
            runtime_info,
            system_info,
            performance_profiles,
        };

        *self.nvidia_info.write().await = Some(nvidia_info);

        info!("✅ NVIDIA system information collected");
        Ok(())
    }

    /// Get NVIDIA driver version
    async fn get_driver_version(&self) -> Result<String> {
        let output = Command::new("nvidia-smi")
            .arg("--query-gpu=driver_version")
            .arg("--format=csv,noheader,nounits")
            .output()
            .context("Failed to execute nvidia-smi")?;

        if !output.status.success() {
            anyhow::bail!("nvidia-smi failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        let version = String::from_utf8(output.stdout)?
            .trim()
            .lines()
            .next()
            .unwrap_or("unknown")
            .to_string();

        debug!("🔧 NVIDIA Driver Version: {}", version);
        Ok(version)
    }

    /// Get CUDA version
    async fn get_cuda_version(&self) -> Option<String> {
        let output = Command::new("nvcc")
            .arg("--version")
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let version_text = String::from_utf8(output.stdout).ok()?;
        
        // Parse CUDA version from nvcc output
        for line in version_text.lines() {
            if line.contains("release") {
                if let Some(version_part) = line.split("release").nth(1) {
                    if let Some(version) = version_part.trim().split(',').next() {
                        debug!("🔧 CUDA Version: {}", version);
                        return Some(version.to_string());
                    }
                }
            }
        }

        None
    }

    /// Get GPU device information
    async fn get_device_info(&self) -> Result<Vec<GpuDeviceInfo>> {
        let output = Command::new("nvidia-smi")
            .arg("--query-gpu=index,name,uuid,pci.bus_id,compute_cap,memory.total,memory.free,memory.used,utilization.gpu,utilization.memory,temperature.gpu,power.draw,power.limit,clocks.gr,clocks.mem,fan.speed,pstate,persistence_mode,accounting.mode")
            .arg("--format=csv,noheader,nounits")
            .output()
            .context("Failed to execute nvidia-smi")?;

        if !output.status.success() {
            anyhow::bail!("nvidia-smi query failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        let output_text = String::from_utf8(output.stdout)?;
        let mut devices = Vec::new();

        for line in output_text.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if fields.len() < 19 {
                continue;
            }

            // Parse compute capability
            let compute_cap = fields[4];
            let (cc_major, cc_minor) = if let Some((major, minor)) = compute_cap.split_once('.') {
                (major.parse().unwrap_or(0), minor.parse().unwrap_or(0))
            } else {
                (0, 0)
            };

            let device = GpuDeviceInfo {
                id: fields[0].parse().unwrap_or(0),
                name: fields[1].to_string(),
                uuid: fields[2].to_string(),
                pci_bus_id: fields[3].to_string(),
                compute_capability_major: cc_major,
                compute_capability_minor: cc_minor,
                memory_total_mb: fields[5].parse().unwrap_or(0),
                memory_free_mb: fields[6].parse().unwrap_or(0),
                memory_used_mb: fields[7].parse().unwrap_or(0),
                utilization_gpu: fields[8].parse().unwrap_or(0),
                utilization_memory: fields[9].parse().unwrap_or(0),
                temperature_celsius: fields[10].parse().unwrap_or(0),
                power_draw_watts: fields[11].parse().unwrap_or(0),
                power_limit_watts: fields[12].parse().unwrap_or(0),
                clock_graphics_mhz: fields[13].parse().unwrap_or(0),
                clock_memory_mhz: fields[14].parse().unwrap_or(0),
                fan_speed_percent: fields[15].parse().unwrap_or(0),
                performance_state: fields[16].to_string(),
                persistence_mode: fields[17] == "Enabled",
                accounting_mode: fields[18] == "Enabled",
            };

            devices.push(device);
        }

        debug!("🖥️ Found {} GPU device(s)", devices.len());
        Ok(devices)
    }

    /// Get container runtime information
    async fn get_container_runtime_info(&self) -> Result<ContainerRuntimeInfo> {
        let nvidia_container_runtime_available = Command::new("which")
            .arg("nvidia-container-runtime")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let nvidia_docker_available = Command::new("which")
            .arg("nvidia-docker")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // Get nvidia-container-toolkit version
        let nvidia_container_toolkit_version = if let Ok(output) = Command::new("nvidia-container-toolkit")
            .arg("--version")
            .output()
        {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
                    .and_then(|s| s.lines().next().map(|l| l.to_string()))
            } else {
                None
            }
        } else {
            None
        };

        // Check supported container engines
        let mut supported_engines = Vec::new();
        if Command::new("which").arg("docker").output().map(|o| o.status.success()).unwrap_or(false) {
            supported_engines.push("docker".to_string());
        }
        if Command::new("which").arg("podman").output().map(|o| o.status.success()).unwrap_or(false) {
            supported_engines.push("podman".to_string());
        }
        if Command::new("which").arg("containerd").output().map(|o| o.status.success()).unwrap_or(false) {
            supported_engines.push("containerd".to_string());
        }

        Ok(ContainerRuntimeInfo {
            nvidia_container_runtime_available,
            nvidia_container_toolkit_version,
            nvidia_docker_available,
            container_runtime_version: None, // Would need specific detection
            supported_container_engines: supported_engines,
        })
    }

    /// Get system information
    async fn get_system_info(&self) -> Result<SystemInfo> {
        // Get OS information
        let os_info = os_info::get();
        let os_name = os_info.os_type().to_string();
        let os_version = os_info.version().to_string();

        // Get kernel version
        let kernel_output = Command::new("uname").arg("-r").output()
            .context("Failed to get kernel version")?;
        let kernel_version = String::from_utf8(kernel_output.stdout)?
            .trim().to_string();

        // Get CPU information
        let cpu_info = self.get_cpu_info().await;
        let cpu_model = cpu_info.0;
        let cpu_cores = cpu_info.1;

        // Get memory information
        let memory_info = self.get_memory_info().await;

        // Check if NVIDIA ML is available
        let nvidia_ml_available = std::path::Path::new("/usr/lib/x86_64-linux-gnu/libnvidia-ml.so").exists() ||
                                  std::path::Path::new("/usr/lib64/libnvidia-ml.so").exists();

        // Find CUDA toolkit path
        let cuda_toolkit_path = std::env::var("CUDA_HOME").ok()
            .or_else(|| std::env::var("CUDA_PATH").ok())
            .or_else(|| {
                if std::path::Path::new("/usr/local/cuda").exists() {
                    Some("/usr/local/cuda".to_string())
                } else {
                    None
                }
            });

        Ok(SystemInfo {
            os_name,
            os_version,
            kernel_version,
            cpu_model,
            cpu_cores,
            memory_total_gb: memory_info.0,
            memory_available_gb: memory_info.1,
            nvidia_ml_available,
            cuda_toolkit_path,
        })
    }

    /// Get CPU information
    async fn get_cpu_info(&self) -> (String, u32) {
        if let Ok(output) = Command::new("lscpu").output() {
            let output_text = String::from_utf8_lossy(&output.stdout);
            let mut model_name = "Unknown".to_string();
            let mut cpu_cores = 1;

            for line in output_text.lines() {
                if line.starts_with("Model name:") {
                    model_name = line.split(':').nth(1)
                        .unwrap_or("Unknown")
                        .trim()
                        .to_string();
                } else if line.starts_with("CPU(s):") {
                    if let Ok(cores) = line.split(':').nth(1)
                        .unwrap_or("1")
                        .trim()
                        .parse::<u32>()
                    {
                        cpu_cores = cores;
                    }
                }
            }

            (model_name, cpu_cores)
        } else {
            ("Unknown".to_string(), 1)
        }
    }

    /// Get memory information
    async fn get_memory_info(&self) -> (f64, f64) {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            let mut total_kb = 0;
            let mut available_kb = 0;

            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        total_kb = value.parse().unwrap_or(0);
                    }
                } else if line.starts_with("MemAvailable:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        available_kb = value.parse().unwrap_or(0);
                    }
                }
            }

            let total_gb = total_kb as f64 / 1024.0 / 1024.0;
            let available_gb = available_kb as f64 / 1024.0 / 1024.0;

            (total_gb, available_gb)
        } else {
            (0.0, 0.0)
        }
    }

    /// Get performance profiles
    async fn get_performance_profiles(&self) -> Vec<PerformanceProfile> {
        vec![
            PerformanceProfile {
                name: "Power Efficient".to_string(),
                description: "Optimized for power efficiency and lower temperatures".to_string(),
                gpu_clock_offset: -100,
                memory_clock_offset: -50,
                power_limit_percent: 75,
                fan_speed_percent: Some(30),
                compute_mode: "default".to_string(),
                auto_boost_enabled: false,
            },
            PerformanceProfile {
                name: "Balanced".to_string(),
                description: "Balanced performance and power consumption".to_string(),
                gpu_clock_offset: 0,
                memory_clock_offset: 0,
                power_limit_percent: 100,
                fan_speed_percent: Some(50),
                compute_mode: "default".to_string(),
                auto_boost_enabled: true,
            },
            PerformanceProfile {
                name: "High Performance".to_string(),
                description: "Maximum performance for compute workloads".to_string(),
                gpu_clock_offset: 150,
                memory_clock_offset: 100,
                power_limit_percent: 120,
                fan_speed_percent: Some(80),
                compute_mode: "exclusive_process".to_string(),
                auto_boost_enabled: true,
            },
        ]
    }

    /// Update device metrics
    pub async fn update_metrics(&self) -> Result<()> {
        if !*self.is_initialized.read().await {
            return Ok(());
        }

        let devices = if let Some(info) = self.nvidia_info.read().await.as_ref() {
            info.devices.clone()
        } else {
            return Ok(());
        };

        let total_gpu_memory_gb: f64 = devices.iter()
            .map(|d| d.memory_total_mb as f64 / 1024.0)
            .sum();

        let total_gpu_utilization: f64 = devices.iter()
            .map(|d| d.utilization_gpu as f64)
            .sum::<f64>() / devices.len() as f64;

        let total_power_draw_watts: u32 = devices.iter()
            .map(|d| d.power_draw_watts)
            .sum();

        let max_temperature_celsius: u32 = devices.iter()
            .map(|d| d.temperature_celsius)
            .max()
            .unwrap_or(0);

        // Get CUDA processes
        let cuda_processes = self.get_cuda_processes().await?;
        let active_processes = cuda_processes.values()
            .map(|procs| procs.len())
            .sum::<usize>() as u32;

        let metrics = NvidiaCoreMetrics {
            timestamp: chrono::Utc::now(),
            total_gpu_memory_gb,
            total_gpu_utilization,
            total_power_draw_watts,
            max_temperature_celsius,
            active_processes,
            cuda_context_count: 0, // Would need NVML integration
            memory_fragmentation_percent: 0.0, // Would need detailed analysis
        };

        let mut metrics_vec = self.device_metrics.lock().await;
        metrics_vec.push(metrics);

        // Keep only last 1000 metrics
        if metrics_vec.len() > 1000 {
            metrics_vec.drain(0..500);
        }

        *self.last_update.write().await = Some(Instant::now());

        Ok(())
    }

    /// Get CUDA processes
    async fn get_cuda_processes(&self) -> Result<HashMap<u32, Vec<CudaProcess>>> {
        let output = Command::new("nvidia-smi")
            .arg("pmon")
            .arg("-c")
            .arg("1")
            .output();

        let mut processes: HashMap<u32, Vec<CudaProcess>> = HashMap::new();

        if let Ok(output) = output {
            if output.status.success() {
                let output_text = String::from_utf8_lossy(&output.stdout);
                
                for line in output_text.lines().skip(2) { // Skip header lines
                    if line.trim().is_empty() || line.starts_with('#') {
                        continue;
                    }

                    let fields: Vec<&str> = line.split_whitespace().collect();
                    if fields.len() >= 4 {
                        let device_id: u32 = fields[0].parse().unwrap_or(0);
                        let pid: u32 = fields[1].parse().unwrap_or(0);
                        let process_name = fields[3].to_string();
                        let gpu_memory_usage = fields.get(4)
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);

                        let process = CudaProcess {
                            pid,
                            process_name,
                            gpu_memory_usage_mb: gpu_memory_usage,
                            gpu_utilization_percent: 0, // Would need additional query
                            started_at: chrono::Utc::now(), // Would need process start time
                            command_line: None, // Could get from /proc/{pid}/cmdline
                        };

                        processes.entry(device_id).or_insert_with(Vec::new).push(process);
                    }
                }
            }
        }

        *self.cuda_processes.write().await = processes.clone();
        Ok(processes)
    }

    /// Apply performance profile
    pub async fn apply_performance_profile(&self, profile_name: &str) -> Result<()> {
        let nvidia_info = self.nvidia_info.read().await;
        let info = nvidia_info.as_ref()
            .context("NVIDIA info not available")?;

        let profile = info.performance_profiles.iter()
            .find(|p| p.name == profile_name)
            .context("Performance profile not found")?;

        info!("⚡ Applying performance profile: {}", profile_name);

        // Apply profile to all devices
        for device in &info.devices {
            self.apply_profile_to_device(device.id, profile).await?;
        }

        *self.active_profile.write().await = Some(profile.clone());

        info!("✅ Performance profile '{}' applied successfully", profile_name);
        Ok(())
    }

    /// Apply profile to specific device
    async fn apply_profile_to_device(&self, device_id: u32, profile: &PerformanceProfile) -> Result<()> {
        // Set power limit
        if profile.power_limit_percent != 100 {
            let _ = Command::new("nvidia-smi")
                .arg("-i")
                .arg(device_id.to_string())
                .arg("-pl")
                .arg(format!("{}%", profile.power_limit_percent))
                .output();
        }

        // Set compute mode
        if profile.compute_mode != "default" {
            let compute_mode_value = match profile.compute_mode.as_str() {
                "exclusive_process" => "1",
                "prohibited" => "2",
                _ => "0",
            };

            let _ = Command::new("nvidia-smi")
                .arg("-i")
                .arg(device_id.to_string())
                .arg("-c")
                .arg(compute_mode_value)
                .output();
        }

        Ok(())
    }

    /// Get system status
    pub async fn get_status(&self) -> Result<serde_json::Value> {
        let nvidia_info = self.nvidia_info.read().await;
        let active_profile = self.active_profile.read().await;
        let last_update = *self.last_update.read().await;
        let is_initialized = *self.is_initialized.read().await;

        Ok(serde_json::json!({
            "initialized": is_initialized,
            "nvidia_available": nvidia_info.is_some(),
            "active_profile": *active_profile,
            "last_update": last_update.map(|t| t.elapsed().as_secs()),
            "monitoring_enabled": self.monitoring_enabled,
            "auto_optimization": self.auto_optimization,
            "thermal_protection": self.thermal_protection,
            "device_count": nvidia_info.as_ref().map(|i| i.devices.len()).unwrap_or(0)
        }))
    }

    /// Get detailed NVIDIA information
    pub async fn get_nvidia_info(&self) -> Option<NvidiaCoreInfo> {
        self.nvidia_info.read().await.clone()
    }

    /// Get current metrics
    pub async fn get_current_metrics(&self) -> Option<NvidiaCoreMetrics> {
        let metrics = self.device_metrics.lock().await;
        metrics.last().cloned()
    }

    /// Get CUDA processes for device
    pub async fn get_device_processes(&self, device_id: u32) -> Vec<CudaProcess> {
        let processes = self.cuda_processes.read().await;
        processes.get(&device_id).cloned().unwrap_or_else(Vec::new)
    }

    /// Set thermal limit for device
    pub async fn set_thermal_limit(&self, device_id: u32, limit_celsius: u32) -> Result<()> {
        info!("🌡️ Setting thermal limit for device {} to {}°C", device_id, limit_celsius);
        
        self.thermal_limits.write().await.insert(device_id, limit_celsius);
        
        // Apply thermal limit via nvidia-smi
        let output = Command::new("nvidia-smi")
            .arg("-i")
            .arg(device_id.to_string())
            .arg("-gtt")
            .arg(limit_celsius.to_string())
            .output();

        match output {
            Ok(output) if output.status.success() => {
                info!("✅ Thermal limit set successfully");
                Ok(())
            }
            Ok(output) => {
                let error = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Failed to set thermal limit: {}", error);
            }
            Err(e) => {
                anyhow::bail!("Failed to execute nvidia-smi: {}", e);
            }
        }
    }

    /// Reset device settings
    pub async fn reset_device(&self, device_id: u32) -> Result<()> {
        info!("🔄 Resetting device {} to default settings", device_id);

        // Reset to default compute mode
        let _ = Command::new("nvidia-smi")
            .arg("-i")
            .arg(device_id.to_string())
            .arg("-c")
            .arg("0")
            .output();

        // Reset power limit to default
        let _ = Command::new("nvidia-smi")
            .arg("-i")
            .arg(device_id.to_string())
            .arg("-pl")
            .arg("default")
            .output();

        // Clear thermal limits
        self.thermal_limits.write().await.remove(&device_id);

        info!("✅ Device {} reset to default settings", device_id);
        Ok(())
    }
}
