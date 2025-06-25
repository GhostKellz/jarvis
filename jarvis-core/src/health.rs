use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::process::Command;
use jarvis_shell::Environment;

/// System health monitoring and auto-diagnostics
pub struct HealthMonitor {
    pub thresholds: HealthThresholds,
    pub history: Vec<HealthSnapshot>,
    max_history: usize,
}

/// Configurable health thresholds
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthThresholds {
    pub cpu_warning: f32,        // CPU usage warning threshold (%)
    pub cpu_critical: f32,       // CPU usage critical threshold (%)
    pub memory_warning: f32,     // Memory usage warning threshold (%)
    pub memory_critical: f32,    // Memory usage critical threshold (%)
    pub disk_warning: f32,       // Disk usage warning threshold (%)
    pub disk_critical: f32,      // Disk usage critical threshold (%)
    pub load_warning: f32,       // Load average warning threshold
    pub load_critical: f32,      // Load average critical threshold
    pub temperature_warning: f32, // Temperature warning threshold (°C)
    pub temperature_critical: f32, // Temperature critical threshold (°C)
}

/// Complete health snapshot at a point in time
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthSnapshot {
    pub timestamp: DateTime<Utc>,
    pub overall_status: HealthStatus,
    pub cpu_status: ComponentHealth,
    pub memory_status: ComponentHealth,
    pub disk_status: ComponentHealth,
    pub network_status: ComponentHealth,
    pub system_status: ComponentHealth,
    pub services_status: Vec<ServiceHealth>,
    pub auto_fixes_applied: Vec<AutoFix>,
    pub recommendations: Vec<Recommendation>,
}

/// Health status levels
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Component-specific health information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub metric_value: f32,
    pub threshold_warning: f32,
    pub threshold_critical: f32,
    pub details: String,
    pub trend: HealthTrend,
}

/// Health trend analysis
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HealthTrend {
    Improving,
    Stable,
    Degrading,
    Unknown,
}

/// Service health status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub name: String,
    pub status: ServiceStatus,
    pub uptime: Option<String>,
    pub cpu_usage: Option<f32>,
    pub memory_usage: Option<f32>,
    pub last_restart: Option<DateTime<Utc>>,
}

/// Service status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ServiceStatus {
    Active,
    Inactive,
    Failed,
    Unknown,
}

/// Auto-fix actions applied
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoFix {
    pub action: String,
    pub component: String,
    pub success: bool,
    pub details: String,
    pub timestamp: DateTime<Utc>,
}

/// Health recommendations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recommendation {
    pub priority: RecommendationPriority,
    pub category: String,
    pub title: String,
    pub description: String,
    pub action: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            cpu_warning: 70.0,
            cpu_critical: 90.0,
            memory_warning: 80.0,
            memory_critical: 95.0,
            disk_warning: 80.0,
            disk_critical: 95.0,
            load_warning: 2.0,
            load_critical: 4.0,
            temperature_warning: 70.0,
            temperature_critical: 85.0,
        }
    }
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            thresholds: HealthThresholds::default(),
            history: Vec::new(),
            max_history: 100,
        }
    }

    pub fn with_thresholds(thresholds: HealthThresholds) -> Self {
        Self {
            thresholds,
            history: Vec::new(),
            max_history: 100,
        }
    }

    /// Perform comprehensive system health check
    pub async fn check_system_health(&mut self, environment: &Environment) -> Result<HealthSnapshot> {
        let timestamp = Utc::now();
        
        // Check individual components
        let cpu_status = self.check_cpu_health(environment).await?;
        let memory_status = self.check_memory_health(environment).await?;
        let disk_status = self.check_disk_health().await?;
        let network_status = self.check_network_health().await?;
        let system_status = self.check_system_health_metrics(environment).await?;
        let services_status = self.check_critical_services().await?;

        // Determine overall status
        let overall_status = self.calculate_overall_status(&[
            &cpu_status.status,
            &memory_status.status,
            &disk_status.status,
            &network_status.status,
            &system_status.status,
        ]);

        // Generate recommendations
        let recommendations = self.generate_recommendations(
            &cpu_status,
            &memory_status,
            &disk_status,
            &services_status,
        ).await?;

        // Apply auto-fixes if needed
        let auto_fixes_applied = self.apply_auto_fixes(
            &cpu_status,
            &memory_status,
            &services_status,
        ).await?;

        let snapshot = HealthSnapshot {
            timestamp,
            overall_status,
            cpu_status,
            memory_status,
            disk_status,
            network_status,
            system_status,
            services_status,
            auto_fixes_applied,
            recommendations,
        };

        // Store in history
        self.history.push(snapshot.clone());
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        Ok(snapshot)
    }

    /// Check CPU health
    async fn check_cpu_health(&self, environment: &Environment) -> Result<ComponentHealth> {
        // Get CPU usage from top processes
        let cpu_usage = environment.system_stats.top_processes
            .iter()
            .map(|p| p.cpu_percent)
            .sum::<f32>();

        let status = if cpu_usage >= self.thresholds.cpu_critical {
            HealthStatus::Critical
        } else if cpu_usage >= self.thresholds.cpu_warning {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        let trend = self.calculate_trend("cpu", cpu_usage);

        Ok(ComponentHealth {
            status,
            metric_value: cpu_usage,
            threshold_warning: self.thresholds.cpu_warning,
            threshold_critical: self.thresholds.cpu_critical,
            details: format!("CPU usage: {:.1}%", cpu_usage),
            trend,
        })
    }

    /// Check memory health
    async fn check_memory_health(&self, environment: &Environment) -> Result<ComponentHealth> {
        // Parse memory info from environment (if available)
        let memory_usage = self.get_memory_usage().await.unwrap_or(0.0);

        let status = if memory_usage >= self.thresholds.memory_critical {
            HealthStatus::Critical
        } else if memory_usage >= self.thresholds.memory_warning {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        let trend = self.calculate_trend("memory", memory_usage);

        Ok(ComponentHealth {
            status,
            metric_value: memory_usage,
            threshold_warning: self.thresholds.memory_warning,
            threshold_critical: self.thresholds.memory_critical,
            details: format!("Memory usage: {:.1}%", memory_usage),
            trend,
        })
    }

    /// Check disk health
    async fn check_disk_health(&self) -> Result<ComponentHealth> {
        let disk_usage = self.get_disk_usage().await.unwrap_or(0.0);

        let status = if disk_usage >= self.thresholds.disk_critical {
            HealthStatus::Critical
        } else if disk_usage >= self.thresholds.disk_warning {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        let trend = self.calculate_trend("disk", disk_usage);

        Ok(ComponentHealth {
            status,
            metric_value: disk_usage,
            threshold_warning: self.thresholds.disk_warning,
            threshold_critical: self.thresholds.disk_critical,
            details: format!("Disk usage: {:.1}%", disk_usage),
            trend,
        })
    }

    /// Check network health
    async fn check_network_health(&self) -> Result<ComponentHealth> {
        let network_ok = self.check_network_connectivity().await.unwrap_or(false);
        
        let status = if network_ok {
            HealthStatus::Healthy
        } else {
            HealthStatus::Critical
        };

        Ok(ComponentHealth {
            status,
            metric_value: if network_ok { 100.0 } else { 0.0 },
            threshold_warning: 50.0,
            threshold_critical: 10.0,
            details: if network_ok { "Network connectivity OK".to_string() } else { "Network connectivity issues".to_string() },
            trend: HealthTrend::Stable,
        })
    }

    /// Check system metrics (load average, temperatures)
    async fn check_system_health_metrics(&self, environment: &Environment) -> Result<ComponentHealth> {
        let load_avg = environment.system_stats.load_avg_1min;
        
        let status = if load_avg >= self.thresholds.load_critical {
            HealthStatus::Critical
        } else if load_avg >= self.thresholds.load_warning {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        let trend = self.calculate_trend("load", load_avg);

        Ok(ComponentHealth {
            status,
            metric_value: load_avg,
            threshold_warning: self.thresholds.load_warning,
            threshold_critical: self.thresholds.load_critical,
            details: format!("Load average: {:.2}", load_avg),
            trend,
        })
    }

    /// Check critical system services
    async fn check_critical_services(&self) -> Result<Vec<ServiceHealth>> {
        let mut services = Vec::new();
        
        // Critical services to monitor
        let critical_services = [
            "systemd", "dbus", "NetworkManager", "sshd", 
            "docker", "ollama", "nginx", "postgresql"
        ];

        for service in &critical_services {
            if let Ok(status) = self.get_service_status(service).await {
                services.push(status);
            }
        }

        Ok(services)
    }

    /// Generate health recommendations
    async fn generate_recommendations(
        &self,
        cpu_status: &ComponentHealth,
        memory_status: &ComponentHealth,
        disk_status: &ComponentHealth,
        services_status: &[ServiceHealth],
    ) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();

        // CPU recommendations
        if cpu_status.status == HealthStatus::Critical {
            recommendations.push(Recommendation {
                priority: RecommendationPriority::High,
                category: "CPU".to_string(),
                title: "High CPU Usage Detected".to_string(),
                description: format!("CPU usage is at {:.1}%, consider investigating high-usage processes", cpu_status.metric_value),
                action: Some("htop".to_string()),
            });
        }

        // Memory recommendations
        if memory_status.status == HealthStatus::Critical {
            recommendations.push(Recommendation {
                priority: RecommendationPriority::High,
                category: "Memory".to_string(),
                title: "High Memory Usage Detected".to_string(),
                description: format!("Memory usage is at {:.1}%, consider freeing memory or adding RAM", memory_status.metric_value),
                action: Some("free -h".to_string()),
            });
        }

        // Disk recommendations
        if disk_status.status == HealthStatus::Critical {
            recommendations.push(Recommendation {
                priority: RecommendationPriority::Critical,
                category: "Storage".to_string(),
                title: "Disk Space Critical".to_string(),
                description: format!("Disk usage is at {:.1}%, immediate action required", disk_status.metric_value),
                action: Some("du -sh /* | sort -rh | head -10".to_string()),
            });
        }

        // Service recommendations
        for service in services_status {
            if service.status == ServiceStatus::Failed {
                recommendations.push(Recommendation {
                    priority: RecommendationPriority::High,
                    category: "Services".to_string(),
                    title: format!("Service {} Failed", service.name),
                    description: format!("Critical service {} is not running", service.name),
                    action: Some(format!("systemctl restart {}", service.name)),
                });
            }
        }

        Ok(recommendations)
    }

    /// Apply automatic fixes where safe
    async fn apply_auto_fixes(
        &self,
        _cpu_status: &ComponentHealth,
        _memory_status: &ComponentHealth,
        services_status: &[ServiceHealth],
    ) -> Result<Vec<AutoFix>> {
        let mut fixes = Vec::new();

        // Auto-restart failed services (be conservative)
        let safe_to_restart = ["ollama", "nginx"];
        
        for service in services_status {
            if service.status == ServiceStatus::Failed && safe_to_restart.contains(&service.name.as_str()) {
                match self.restart_service(&service.name).await {
                    Ok(_) => {
                        fixes.push(AutoFix {
                            action: format!("Restarted service {}", service.name),
                            component: "Services".to_string(),
                            success: true,
                            details: format!("Service {} was automatically restarted", service.name),
                            timestamp: Utc::now(),
                        });
                    }
                    Err(e) => {
                        fixes.push(AutoFix {
                            action: format!("Attempted to restart service {}", service.name),
                            component: "Services".to_string(),
                            success: false,
                            details: format!("Failed to restart {}: {}", service.name, e),
                            timestamp: Utc::now(),
                        });
                    }
                }
            }
        }

        Ok(fixes)
    }

    // Helper methods
    fn calculate_overall_status(&self, statuses: &[&HealthStatus]) -> HealthStatus {
        if statuses.iter().any(|s| **s == HealthStatus::Critical) {
            HealthStatus::Critical
        } else if statuses.iter().any(|s| **s == HealthStatus::Warning) {
            HealthStatus::Warning
        } else if statuses.iter().all(|s| **s == HealthStatus::Healthy) {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unknown
        }
    }

    fn calculate_trend(&self, component: &str, current_value: f32) -> HealthTrend {
        // Look at last 5 readings for trend analysis
        let recent_values: Vec<f32> = self.history
            .iter()
            .rev()
            .take(5)
            .filter_map(|snapshot| {
                match component {
                    "cpu" => Some(snapshot.cpu_status.metric_value),
                    "memory" => Some(snapshot.memory_status.metric_value),
                    "disk" => Some(snapshot.disk_status.metric_value),
                    "load" => Some(snapshot.system_status.metric_value),
                    _ => None,
                }
            })
            .collect();

        if recent_values.len() < 2 {
            return HealthTrend::Unknown;
        }

        let avg_recent = recent_values.iter().sum::<f32>() / recent_values.len() as f32;
        let diff = current_value - avg_recent;

        if diff > 5.0 {
            HealthTrend::Degrading
        } else if diff < -5.0 {
            HealthTrend::Improving
        } else {
            HealthTrend::Stable
        }
    }

    async fn get_memory_usage(&self) -> Result<f32> {
        let output = Command::new("free")
            .arg("-m")
            .output()
            .await?;
        
        let stdout = String::from_utf8(output.stdout)?;
        for line in stdout.lines() {
            if line.starts_with("Mem:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let total: f32 = parts[1].parse().unwrap_or(1.0);
                    let used: f32 = parts[2].parse().unwrap_or(0.0);
                    return Ok((used / total) * 100.0);
                }
            }
        }
        Ok(0.0)
    }

    async fn get_disk_usage(&self) -> Result<f32> {
        let output = Command::new("df")
            .arg("-h")
            .arg("/")
            .output()
            .await?;
        
        let stdout = String::from_utf8(output.stdout)?;
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let usage_str = parts[4].trim_end_matches('%');
                if let Ok(usage) = usage_str.parse::<f32>() {
                    return Ok(usage);
                }
            }
        }
        Ok(0.0)
    }

    async fn check_network_connectivity(&self) -> Result<bool> {
        let output = Command::new("ping")
            .arg("-c")
            .arg("1")
            .arg("-W")
            .arg("2")
            .arg("1.1.1.1")
            .output()
            .await?;
        
        Ok(output.status.success())
    }

    async fn get_service_status(&self, service_name: &str) -> Result<ServiceHealth> {
        let output = Command::new("systemctl")
            .arg("is-active")
            .arg(service_name)
            .output()
            .await?;
        
        let status_str = String::from_utf8(output.stdout)?.trim().to_string();
        let status = match status_str.as_str() {
            "active" => ServiceStatus::Active,
            "inactive" => ServiceStatus::Inactive,
            "failed" => ServiceStatus::Failed,
            _ => ServiceStatus::Unknown,
        };

        Ok(ServiceHealth {
            name: service_name.to_string(),
            status,
            uptime: None,
            cpu_usage: None,
            memory_usage: None,
            last_restart: None,
        })
    }

    async fn restart_service(&self, service_name: &str) -> Result<()> {
        let output = Command::new("systemctl")
            .arg("restart")
            .arg(service_name)
            .output()
            .await?;
        
        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to restart service: {}", String::from_utf8_lossy(&output.stderr)))
        }
    }

    /// Get health summary for quick status check
    pub fn get_health_summary(&self) -> Option<String> {
        if let Some(latest) = self.history.last() {
            let status_emoji = match latest.overall_status {
                HealthStatus::Healthy => "✅",
                HealthStatus::Warning => "⚠️",
                HealthStatus::Critical => "🔴",
                HealthStatus::Unknown => "❓",
            };

            Some(format!(
                "{} System Status: {:?} | CPU: {:.1}% | Memory: {:.1}% | Disk: {:.1}%",
                status_emoji,
                latest.overall_status,
                latest.cpu_status.metric_value,
                latest.memory_status.metric_value,
                latest.disk_status.metric_value
            ))
        } else {
            None
        }
    }
}