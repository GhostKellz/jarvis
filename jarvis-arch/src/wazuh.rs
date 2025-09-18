use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::process::Command;
use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::{debug, error, info, warn};

use crate::config::WazuhConfig;
use crate::package_manager::{PackageInfo, PackageManager};

/// Wazuh integration for security monitoring and AUR package tracking
pub struct WazuhIntegration {
    config: WazuhConfig,
    package_manager: PackageManager,
}

/// Security event types for Wazuh SIEM
#[derive(Debug, Serialize, Deserialize)]
pub enum SecurityEvent {
    /// New package installation from AUR
    AurPackageInstalled {
        package_name: String,
        version: String,
        maintainer: Option<String>,
        install_time: chrono::DateTime<chrono::Utc>,
        source_url: Option<String>,
    },
    /// Package update detected
    PackageUpdated {
        package_name: String,
        old_version: String,
        new_version: String,
        update_time: chrono::DateTime<chrono::Utc>,
        is_aur: bool,
    },
    /// Vulnerable package detected
    VulnerablePackage {
        package_name: String,
        version: String,
        vulnerability_id: String,
        severity: String,
        description: String,
    },
    /// Suspicious package behavior
    SuspiciousActivity {
        package_name: String,
        activity_type: String,
        details: String,
        risk_level: RiskLevel,
    },
    /// System maintenance event
    MaintenanceEvent {
        event_type: String,
        description: String,
        packages_affected: Vec<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Wazuh log entry structure
#[derive(Debug, Serialize)]
struct WazuhLogEntry {
    timestamp: chrono::DateTime<chrono::Utc>,
    level: String,
    source: String,
    event_type: String,
    data: serde_json::Value,
    host: String,
    agent_name: String,
}

impl WazuhIntegration {
    /// Create new Wazuh integration
    pub fn new(config: WazuhConfig, package_manager: PackageManager) -> Self {
        Self {
            config,
            package_manager,
        }
    }

    /// Initialize Wazuh integration and perform initial AUR scan
    pub async fn initialize(&self) -> Result<()> {
        if !self.config.enabled {
            debug!("Wazuh integration disabled in configuration");
            return Ok(());
        }

        info!("Initializing Wazuh integration for AUR package monitoring");

        // Test connection to Wazuh manager
        self.test_connection().await?;

        // Perform initial AUR package scan
        self.scan_aur_packages().await?;

        // Send initialization event
        self.send_event(SecurityEvent::MaintenanceEvent {
            event_type: "wazuh_init".to_string(),
            description: "Jarvis Wazuh integration initialized".to_string(),
            packages_affected: vec![],
        }).await?;

        info!("Wazuh integration initialized successfully");
        Ok(())
    }

    /// Test connection to Wazuh manager
    async fn test_connection(&self) -> Result<()> {
        let address = format!("{}:{}", self.config.server, self.config.port);
        
        match TcpStream::connect(&address).await {
            Ok(_) => {
                info!("Successfully connected to Wazuh manager at {}", address);
                Ok(())
            }
            Err(e) => {
                error!("Failed to connect to Wazuh manager at {}: {}", address, e);
                Err(anyhow::anyhow!("Wazuh connection failed: {}", e))
            }
        }
    }

    /// Scan all installed AUR packages and report to Wazuh
    pub async fn scan_aur_packages(&self) -> Result<()> {
        info!("Scanning AUR packages for security monitoring");

        let aur_packages = self.get_aur_packages().await?;
        info!("Found {} AUR packages installed", aur_packages.len());

        for package in aur_packages {
            // Check if package has known vulnerabilities
            if let Some(vulnerabilities) = self.check_package_vulnerabilities(&package).await? {
                for vuln in vulnerabilities {
                    self.send_event(SecurityEvent::VulnerablePackage {
                        package_name: package.name.clone(),
                        version: package.version.clone(),
                        vulnerability_id: vuln.id,
                        severity: vuln.severity,
                        description: vuln.description,
                    }).await?;
                }
            }

            // Check for suspicious package characteristics
            self.analyze_package_security(&package).await?;

            // Report package installation (for baseline)
            self.send_event(SecurityEvent::AurPackageInstalled {
                package_name: package.name.clone(),
                version: package.version.clone(),
                maintainer: package.maintainer,
                install_time: package.install_date.unwrap_or_else(chrono::Utc::now),
                source_url: package.url,
            }).await?;
        }

        Ok(())
    }

    /// Get list of installed AUR packages
    async fn get_aur_packages(&self) -> Result<Vec<PackageInfo>> {
        // Use pacman to get foreign packages (AUR packages)
        let output = Command::new("pacman")
            .args(&["-Qm", "--color", "never"])
            .output()
            .context("Failed to execute pacman -Qm")?;

        if !output.status.success() {
            return Ok(vec![]); // No AUR packages installed
        }

        let stdout = String::from_utf8(output.stdout)?;
        let mut packages = Vec::new();

        for line in stdout.lines() {
            if let Some((name, version)) = line.split_once(' ') {
                let package_info = self.get_package_details(name).await?;
                packages.push(PackageInfo {
                    name: name.to_string(),
                    version: version.to_string(),
                    ..package_info
                });
            }
        }

        Ok(packages)
    }

    /// Get detailed package information
    async fn get_package_details(&self, package_name: &str) -> Result<PackageInfo> {
        // Use pacman to get package details
        let output = Command::new("pacman")
            .args(&["-Qi", package_name])
            .output()
            .context("Failed to get package details")?;

        let stdout = String::from_utf8(output.stdout)?;
        let mut package_info = PackageInfo {
            name: package_name.to_string(),
            version: String::new(),
            description: None,
            maintainer: None,
            install_date: None,
            url: None,
            dependencies: vec![],
            size: 0,
        };

        // Parse pacman output
        for line in stdout.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "Version" => package_info.version = value.to_string(),
                    "Description" => package_info.description = Some(value.to_string()),
                    "Packager" => package_info.maintainer = Some(value.to_string()),
                    "Install Date" => {
                        // Parse install date if available
                        if let Ok(date) = chrono::DateTime::parse_from_str(value, "%a %d %b %Y %I:%M:%S %p %Z") {
                            package_info.install_date = Some(date.with_timezone(&chrono::Utc));
                        }
                    }
                    "URL" => package_info.url = Some(value.to_string()),
                    "Installed Size" => {
                        // Parse size (format: "123.45 KiB")
                        if let Some(size_str) = value.split_whitespace().next() {
                            if let Ok(size) = size_str.parse::<f64>() {
                                package_info.size = (size * 1024.0) as u64; // Convert to bytes
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(package_info)
    }

    /// Check for known vulnerabilities in a package
    async fn check_package_vulnerabilities(&self, package: &PackageInfo) -> Result<Option<Vec<Vulnerability>>> {
        // This would integrate with vulnerability databases
        // For now, implement basic checks based on package characteristics
        
        let mut vulnerabilities = Vec::new();

        // Check for packages with suspicious names or characteristics
        if self.is_suspicious_package(package) {
            vulnerabilities.push(Vulnerability {
                id: format!("JARVIS-SUSP-{}", package.name.to_uppercase()),
                severity: "medium".to_string(),
                description: "Package exhibits suspicious characteristics".to_string(),
            });
        }

        // Check for outdated packages (placeholder - would need version comparison)
        if package.version.contains("git") || package.version.contains("dev") {
            vulnerabilities.push(Vulnerability {
                id: format!("JARVIS-DEV-{}", package.name.to_uppercase()),
                severity: "low".to_string(),
                description: "Development version package may contain unstable code".to_string(),
            });
        }

        if vulnerabilities.is_empty() {
            Ok(None)
        } else {
            Ok(Some(vulnerabilities))
        }
    }

    /// Analyze package for security concerns
    async fn analyze_package_security(&self, package: &PackageInfo) -> Result<()> {
        // Check for packages that might need elevated privileges
        if self.requires_elevated_privileges(package) {
            self.send_event(SecurityEvent::SuspiciousActivity {
                package_name: package.name.clone(),
                activity_type: "elevated_privileges".to_string(),
                details: "Package may require elevated system privileges".to_string(),
                risk_level: RiskLevel::Medium,
            }).await?;
        }

        // Check for network-facing packages
        if self.is_network_facing(package) {
            self.send_event(SecurityEvent::SuspiciousActivity {
                package_name: package.name.clone(),
                activity_type: "network_facing".to_string(),
                details: "Package provides network-facing services".to_string(),
                risk_level: RiskLevel::Medium,
            }).await?;
        }

        Ok(())
    }

    /// Check if package is suspicious based on name and characteristics
    fn is_suspicious_package(&self, package: &PackageInfo) -> bool {
        let suspicious_keywords = [
            "miner", "crypto", "bitcoin", "monero", "backdoor", 
            "rootkit", "keylog", "stealer", "rat", "trojan"
        ];

        let name_lower = package.name.to_lowercase();
        suspicious_keywords.iter().any(|&keyword| name_lower.contains(keyword))
    }

    /// Check if package requires elevated privileges
    fn requires_elevated_privileges(&self, package: &PackageInfo) -> bool {
        let privilege_keywords = [
            "sudo", "setuid", "admin", "root", "system", 
            "kernel", "driver", "daemon", "service"
        ];

        let name_lower = package.name.to_lowercase();
        let desc_lower = package.description.as_ref()
            .map(|d| d.to_lowercase())
            .unwrap_or_default();

        privilege_keywords.iter().any(|&keyword| {
            name_lower.contains(keyword) || desc_lower.contains(keyword)
        })
    }

    /// Check if package is network-facing
    fn is_network_facing(&self, package: &PackageInfo) -> bool {
        let network_keywords = [
            "server", "daemon", "service", "web", "http", "https",
            "tcp", "udp", "socket", "port", "proxy", "vpn"
        ];

        let name_lower = package.name.to_lowercase();
        let desc_lower = package.description.as_ref()
            .map(|d| d.to_lowercase())
            .unwrap_or_default();

        network_keywords.iter().any(|&keyword| {
            name_lower.contains(keyword) || desc_lower.contains(keyword)
        })
    }

    /// Send security event to Wazuh manager
    async fn send_event(&self, event: SecurityEvent) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let log_entry = WazuhLogEntry {
            timestamp: chrono::Utc::now(),
            level: "INFO".to_string(),
            source: "jarvis-arch".to_string(),
            event_type: match &event {
                SecurityEvent::AurPackageInstalled { .. } => "aur_install",
                SecurityEvent::PackageUpdated { .. } => "package_update",
                SecurityEvent::VulnerablePackage { .. } => "vulnerability",
                SecurityEvent::SuspiciousActivity { .. } => "suspicious_activity",
                SecurityEvent::MaintenanceEvent { .. } => "maintenance",
            }.to_string(),
            data: serde_json::to_value(&event)?,
            host: gethostname::gethostname().to_string_lossy().to_string(),
            agent_name: "jarvis-arch".to_string(),
        };

        // Send to Wazuh manager
        match self.config.protocol.as_str() {
            "tcp" => self.send_tcp_event(&log_entry).await?,
            "udp" => self.send_udp_event(&log_entry).await?,
            _ => return Err(anyhow::anyhow!("Unsupported Wazuh protocol: {}", self.config.protocol)),
        }

        debug!("Sent event to Wazuh: {:?}", event);
        Ok(())
    }

    /// Send event via TCP to Wazuh manager
    async fn send_tcp_event(&self, entry: &WazuhLogEntry) -> Result<()> {
        let address = format!("{}:{}", self.config.server, self.config.port);
        let stream = TcpStream::connect(&address).await
            .context("Failed to connect to Wazuh manager")?;

        let mut writer = BufWriter::new(stream);
        let json_data = serde_json::to_string(entry)?;
        
        writer.write_all(json_data.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        Ok(())
    }

    /// Send event via UDP to Wazuh manager (placeholder)
    async fn send_udp_event(&self, _entry: &WazuhLogEntry) -> Result<()> {
        // UDP implementation would go here
        warn!("UDP protocol not yet implemented for Wazuh integration");
        Ok(())
    }

    /// Monitor for new package installations
    pub async fn monitor_package_changes(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // This would typically use inotify or similar to monitor for package database changes
        // For now, implement a simple polling mechanism
        info!("Starting package change monitoring");
        
        let mut last_packages = self.get_aur_packages().await?;
        
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await; // Check every 5 minutes
            
            let current_packages = self.get_aur_packages().await?;
            let changes = self.detect_package_changes(&last_packages, &current_packages);
            
            for change in changes {
                match change {
                    PackageChange::Installed(package) => {
                        self.send_event(SecurityEvent::AurPackageInstalled {
                            package_name: package.name.clone(),
                            version: package.version.clone(),
                            maintainer: package.maintainer,
                            install_time: chrono::Utc::now(),
                            source_url: package.url,
                        }).await?;
                    }
                    PackageChange::Updated { package, old_version } => {
                        self.send_event(SecurityEvent::PackageUpdated {
                            package_name: package.name.clone(),
                            old_version,
                            new_version: package.version.clone(),
                            update_time: chrono::Utc::now(),
                            is_aur: true,
                        }).await?;
                    }
                }
            }
            
            last_packages = current_packages;
        }
    }

    /// Detect changes between package lists
    fn detect_package_changes(&self, old_packages: &[PackageInfo], new_packages: &[PackageInfo]) -> Vec<PackageChange> {
        let mut changes = Vec::new();
        
        let old_map: HashMap<&str, &PackageInfo> = old_packages.iter()
            .map(|p| (p.name.as_str(), p))
            .collect();
        
        for new_package in new_packages {
            match old_map.get(new_package.name.as_str()) {
                None => {
                    // New package installed
                    changes.push(PackageChange::Installed(new_package.clone()));
                }
                Some(old_package) => {
                    // Check if version changed
                    if old_package.version != new_package.version {
                        changes.push(PackageChange::Updated {
                            package: new_package.clone(),
                            old_version: old_package.version.clone(),
                        });
                    }
                }
            }
        }
        
        changes
    }
}

#[derive(Debug)]
enum PackageChange {
    Installed(PackageInfo),
    Updated { package: PackageInfo, old_version: String },
}

#[derive(Debug)]
struct Vulnerability {
    id: String,
    severity: String,
    description: String,
}