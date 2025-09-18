use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use chrono::{DateTime, Utc};
use regex::Regex;
use crate::arch_config::PacmanConfig;

/// Package manager for Arch Linux operations
#[derive(Debug, Clone)]
pub struct PackageManager {
    config: Option<PacmanConfig>,
    pacman_path: String,
    yay_path: Option<String>,
    cache: PackageCache,
}

#[derive(Debug, Clone)]
struct PackageCache {
    packages: HashMap<String, PackageInfo>,
    last_update: DateTime<Utc>,
    cache_duration_hours: u64,
}

/// Information about a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub repository: String,
    pub installed_size: u64,
    pub install_date: Option<DateTime<Utc>>,
    pub dependencies: Vec<String>,
    pub required_by: Vec<String>,
    pub groups: Vec<String>,
    pub url: Option<String>,
    pub license: Vec<String>,
    pub maintainer: Option<String>,
    pub build_date: Option<DateTime<Utc>>,
    pub checksum: Option<String>,
    pub signature: Option<String>,
}

/// Package operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackageOperation {
    Install,
    Update,
    Remove,
    Search,
    Info,
    ListInstalled,
    ListUpdates,
    Clean,
    Verify,
}

/// Package status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackageStatus {
    Installed,
    Available,
    Outdated,
    Broken,
    Missing,
    Unknown,
}

/// Package operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageOperationResult {
    pub operation: PackageOperation,
    pub packages: Vec<String>,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub changes: Vec<PackageChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageChange {
    pub package: String,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    pub operation: String,
}

impl PackageManager {
    pub fn new() -> Self {
        Self {
            config: None,
            pacman_path: "/usr/bin/pacman".to_string(),
            yay_path: None,
            cache: PackageCache {
                packages: HashMap::new(),
                last_update: DateTime::UNIX_EPOCH,
                cache_duration_hours: 1,
            },
        }
    }

    pub async fn initialize(&mut self, config: &PacmanConfig) -> Result<()> {
        self.config = Some(config.clone());
        
        // Verify pacman is available
        if !std::path::Path::new(&self.pacman_path).exists() {
            return Err(anyhow::anyhow!("pacman not found at {}", self.pacman_path));
        }

        // Check for AUR helper (yay)
        if let Ok(output) = Command::new("which").arg("yay").output().await {
            if output.status.success() {
                let yay_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                self.yay_path = Some(yay_path);
                tracing::info!("Found yay AUR helper");
            }
        }

        // Initialize package database
        self.update_package_cache().await?;
        
        tracing::info!("Package manager initialized successfully");
        Ok(())
    }

    /// Update system packages
    pub async fn update_packages(&self, packages: Option<Vec<String>>) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();
        
        let mut cmd = Command::new(&self.pacman_path);
        cmd.arg("-S");
        
        if let Some(config) = &self.config {
            if config.no_confirm {
                cmd.arg("--noconfirm");
            }
            if config.verbose {
                cmd.arg("-v");
            }
        }

        match packages {
            Some(pkg_list) => {
                cmd.args(&pkg_list);
                tracing::info!("Updating specific packages: {:?}", pkg_list);
            }
            None => {
                cmd.arg("-u");
                tracing::info!("Updating all packages");
            }
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute pacman update")?;

        let duration = start_time.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let changes = self.parse_pacman_output(&stdout).await?;

        Ok(serde_json::json!({
            "operation": "update_packages",
            "success": output.status.success(),
            "packages_updated": changes.len(),
            "duration_ms": duration,
            "output": stdout.to_string(),
            "error": if stderr.is_empty() { None } else { Some(stderr.to_string()) },
            "changes": changes
        }))
    }

    /// Install a package
    pub async fn install_package(&self, package: &str, from_aur: bool) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();
        
        let mut cmd = if from_aur && self.yay_path.is_some() {
            Command::new(self.yay_path.as_ref().unwrap())
        } else {
            Command::new(&self.pacman_path)
        };

        cmd.arg("-S").arg(package);

        if let Some(config) = &self.config {
            if config.no_confirm {
                cmd.arg("--noconfirm");
            }
        }

        tracing::info!("Installing package: {} (AUR: {})", package, from_aur);

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute package install")?;

        let duration = start_time.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(serde_json::json!({
            "operation": "install_package",
            "package": package,
            "from_aur": from_aur,
            "success": output.status.success(),
            "duration_ms": duration,
            "output": stdout.to_string(),
            "error": if stderr.is_empty() { None } else { Some(stderr.to_string()) }
        }))
    }

    /// Remove a package
    pub async fn remove_package(&self, package: &str, remove_deps: bool) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();
        
        let mut cmd = Command::new(&self.pacman_path);
        cmd.arg("-R").arg(package);

        if remove_deps {
            cmd.arg("-s"); // Remove dependencies
        }

        if let Some(config) = &self.config {
            if config.no_confirm {
                cmd.arg("--noconfirm");
            }
        }

        tracing::info!("Removing package: {} (remove deps: {})", package, remove_deps);

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute package removal")?;

        let duration = start_time.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(serde_json::json!({
            "operation": "remove_package",
            "package": package,
            "remove_deps": remove_deps,
            "success": output.status.success(),
            "duration_ms": duration,
            "output": stdout.to_string(),
            "error": if stderr.is_empty() { None } else { Some(stderr.to_string()) }
        }))
    }

    /// Search for packages
    pub async fn search_packages(&self, query: &str, include_aur: bool) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();
        
        // Search official repositories
        let mut cmd = Command::new(&self.pacman_path);
        cmd.arg("-Ss").arg(query);

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search packages")?;

        let mut results = self.parse_search_output(&String::from_utf8_lossy(&output.stdout)).await?;

        // Search AUR if requested and available
        if include_aur && self.yay_path.is_some() {
            let mut aur_cmd = Command::new(self.yay_path.as_ref().unwrap());
            aur_cmd.arg("-Ss").arg(query);

            if let Ok(aur_output) = aur_cmd.output().await {
                let aur_results = self.parse_search_output(&String::from_utf8_lossy(&aur_output.stdout)).await?;
                results.extend(aur_results);
            }
        }

        let duration = start_time.elapsed().as_millis() as u64;

        Ok(serde_json::json!({
            "operation": "search_packages",
            "query": query,
            "include_aur": include_aur,
            "results": results,
            "count": results.len(),
            "duration_ms": duration
        }))
    }

    /// Get information about a package
    pub async fn get_package_info(&self, package: &str) -> Result<Option<PackageInfo>> {
        // Check cache first
        if let Some(cached) = self.cache.packages.get(package) {
            if self.is_cache_valid() {
                return Ok(Some(cached.clone()));
            }
        }

        let mut cmd = Command::new(&self.pacman_path);
        cmd.arg("-Qi").arg(package);

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            return Ok(None);
        }

        let info_text = String::from_utf8_lossy(&output.stdout);
        let package_info = self.parse_package_info(&info_text).await?;

        Ok(package_info)
    }

    /// List installed packages
    pub async fn list_installed_packages(&self) -> Result<Vec<PackageInfo>> {
        if self.is_cache_valid() {
            return Ok(self.cache.packages.values().cloned().collect());
        }

        let mut cmd = Command::new(&self.pacman_path);
        cmd.arg("-Q");

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list installed packages")?;

        let package_list = String::from_utf8_lossy(&output.stdout);
        let packages = self.parse_package_list(&package_list).await?;

        Ok(packages)
    }

    /// Check for available updates
    pub async fn check_updates(&self) -> Result<Vec<PackageInfo>> {
        let mut cmd = Command::new(&self.pacman_path);
        cmd.arg("-Qu");

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            return Ok(vec![]); // No updates available
        }

        let update_list = String::from_utf8_lossy(&output.stdout);
        let updates = self.parse_update_list(&update_list).await?;

        Ok(updates)
    }

    /// Clean package cache
    pub async fn clean_cache(&self, aggressive: bool) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();
        
        let mut cmd = Command::new(&self.pacman_path);
        cmd.arg("-Sc");

        if aggressive {
            cmd.arg("-c"); // Clean all cached packages
        }

        if let Some(config) = &self.config {
            if config.no_confirm {
                cmd.arg("--noconfirm");
            }
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to clean package cache")?;

        let duration = start_time.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(serde_json::json!({
            "operation": "clean_cache",
            "aggressive": aggressive,
            "success": output.status.success(),
            "duration_ms": duration,
            "output": stdout.to_string(),
            "error": if stderr.is_empty() { None } else { Some(stderr.to_string()) }
        }))
    }

    /// Verify package integrity
    pub async fn verify_packages(&self, packages: Option<Vec<String>>) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();
        
        let mut cmd = Command::new(&self.pacman_path);
        cmd.arg("-Qk");

        if let Some(pkg_list) = packages {
            cmd.args(&pkg_list);
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to verify packages")?;

        let duration = start_time.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let verification_results = self.parse_verification_output(&stdout).await?;

        Ok(serde_json::json!({
            "operation": "verify_packages",
            "success": output.status.success(),
            "duration_ms": duration,
            "results": verification_results,
            "error": if stderr.is_empty() { None } else { Some(stderr.to_string()) }
        }))
    }

    // Private helper methods

    async fn update_package_cache(&mut self) -> Result<()> {
        tracing::info!("Updating package cache...");
        
        let packages = self.list_installed_packages().await?;
        
        self.cache.packages.clear();
        for package in packages {
            self.cache.packages.insert(package.name.clone(), package);
        }
        
        self.cache.last_update = Utc::now();
        tracing::info!("Package cache updated with {} packages", self.cache.packages.len());
        
        Ok(())
    }

    fn is_cache_valid(&self) -> bool {
        let age = Utc::now()
            .signed_duration_since(self.cache.last_update)
            .num_hours();
        age < self.cache.cache_duration_hours as i64
    }

    async fn parse_pacman_output(&self, output: &str) -> Result<Vec<PackageChange>> {
        let mut changes = Vec::new();
        
        // Parse pacman output for package changes
        let lines: Vec<&str> = output.lines().collect();
        
        for line in lines {
            if line.contains("installing") || line.contains("upgrading") || line.contains("removing") {
                // Extract package information from pacman output
                // This is a simplified parser - real implementation would be more robust
                if let Some(change) = self.extract_package_change(line).await {
                    changes.push(change);
                }
            }
        }
        
        Ok(changes)
    }

    async fn extract_package_change(&self, line: &str) -> Option<PackageChange> {
        // Parse lines like "installing foo-1.2.3-1..." or "upgrading foo (1.2.2-1 -> 1.2.3-1)..."
        let re = Regex::new(r"(installing|upgrading|removing)\s+(\S+)").ok()?;
        
        if let Some(captures) = re.captures(line) {
            let operation = captures.get(1)?.as_str();
            let package_info = captures.get(2)?.as_str();
            
            // Extract package name and version
            let parts: Vec<&str> = package_info.split('-').collect();
            if let Some(name) = parts.first() {
                return Some(PackageChange {
                    package: name.to_string(),
                    old_version: None, // Would parse from upgrade info
                    new_version: None, // Would parse from package info
                    operation: operation.to_string(),
                });
            }
        }
        
        None
    }

    async fn parse_search_output(&self, output: &str) -> Result<Vec<serde_json::Value>> {
        let mut results = Vec::new();
        let lines: Vec<&str> = output.lines().collect();
        
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            if line.starts_with(' ') {
                i += 1;
                continue;
            }
            
            // Parse package line: "repository/package version [group]"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let repo_package: Vec<&str> = parts[0].split('/').collect();
                if repo_package.len() == 2 {
                    let repository = repo_package[0];
                    let package = repo_package[1];
                    let version = parts[1];
                    
                    // Get description from next line if available
                    let description = if i + 1 < lines.len() && lines[i + 1].starts_with("    ") {
                        lines[i + 1].trim().to_string()
                    } else {
                        String::new()
                    };
                    
                    results.push(serde_json::json!({
                        "repository": repository,
                        "package": package,
                        "version": version,
                        "description": description
                    }));
                }
            }
            i += 1;
        }
        
        Ok(results)
    }

    async fn parse_package_info(&self, info_text: &str) -> Result<Option<PackageInfo>> {
        let lines: Vec<&str> = info_text.lines().collect();
        let mut package_info = HashMap::new();
        
        for line in lines {
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim();
                let value = line[colon_pos + 1..].trim();
                package_info.insert(key.to_lowercase().replace(' ', "_"), value.to_string());
            }
        }
        
        if let Some(name) = package_info.get("name") {
            Ok(Some(PackageInfo {
                name: name.clone(),
                version: package_info.get("version").cloned().unwrap_or_default(),
                description: package_info.get("description").cloned().unwrap_or_default(),
                repository: package_info.get("repository").cloned().unwrap_or_default(),
                installed_size: package_info
                    .get("installed_size")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                install_date: package_info
                    .get("install_date")
                    .and_then(|s| chrono::DateTime::parse_from_str(s, "%a %d %b %Y %I:%M:%S %p %Z").ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                dependencies: package_info
                    .get("depends_on")
                    .map(|s| s.split_whitespace().map(String::from).collect())
                    .unwrap_or_default(),
                required_by: package_info
                    .get("required_by")
                    .map(|s| s.split_whitespace().map(String::from).collect())
                    .unwrap_or_default(),
                groups: package_info
                    .get("groups")
                    .map(|s| s.split_whitespace().map(String::from).collect())
                    .unwrap_or_default(),
                url: package_info.get("url").cloned(),
                license: package_info
                    .get("licenses")
                    .map(|s| s.split_whitespace().map(String::from).collect())
                    .unwrap_or_default(),
                maintainer: package_info.get("packager").cloned(),
                build_date: package_info
                    .get("build_date")
                    .and_then(|s| chrono::DateTime::parse_from_str(s, "%a %d %b %Y %I:%M:%S %p %Z").ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                checksum: package_info.get("md5_sum").cloned(),
                signature: package_info.get("validated_by").cloned(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn parse_package_list(&self, package_list: &str) -> Result<Vec<PackageInfo>> {
        let mut packages = Vec::new();
        
        for line in package_list.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                packages.push(PackageInfo {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    description: String::new(),
                    repository: "local".to_string(),
                    installed_size: 0,
                    install_date: None,
                    dependencies: Vec::new(),
                    required_by: Vec::new(),
                    groups: Vec::new(),
                    url: None,
                    license: Vec::new(),
                    maintainer: None,
                    build_date: None,
                    checksum: None,
                    signature: None,
                });
            }
        }
        
        Ok(packages)
    }

    async fn parse_update_list(&self, update_list: &str) -> Result<Vec<PackageInfo>> {
        let mut updates = Vec::new();
        
        for line in update_list.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                // Format: "package current_version -> new_version"
                updates.push(PackageInfo {
                    name: parts[0].to_string(),
                    version: parts[2].to_string(), // new version
                    description: String::new(),
                    repository: "update".to_string(),
                    installed_size: 0,
                    install_date: None,
                    dependencies: Vec::new(),
                    required_by: Vec::new(),
                    groups: Vec::new(),
                    url: None,
                    license: Vec::new(),
                    maintainer: None,
                    build_date: None,
                    checksum: None,
                    signature: None,
                });
            }
        }
        
        Ok(updates)
    }

    async fn parse_verification_output(&self, output: &str) -> Result<Vec<serde_json::Value>> {
        let mut results = Vec::new();
        
        for line in output.lines() {
            if line.contains("warning:") || line.contains("error:") {
                // Parse verification warnings/errors
                results.push(serde_json::json!({
                    "type": if line.contains("warning:") { "warning" } else { "error" },
                    "message": line.trim()
                }));
            }
        }
        
        Ok(results)
    }
}