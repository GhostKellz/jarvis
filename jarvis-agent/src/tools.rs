use anyhow::Result;
use std::process::Command;

pub struct SystemTools;

impl SystemTools {
    pub async fn new() -> Result<Self> {
        Ok(Self)
    }

    pub async fn diagnose(&self, target: &str) -> Result<String> {
        let mut output = String::new();

        // Check if it's a systemd service
        if target.contains("service") || target.contains(".service") {
            let service_name = target.replace(" service", "").replace(".service", "");
            output.push_str(&self.check_systemd_service(&service_name).await?);
        }

        // Check if it's a network interface
        if target.contains("network") || target.contains("interface") {
            output.push_str(&self.check_network().await?);
        }

        // Check if it's about disk/storage
        if target.contains("disk") || target.contains("btrfs") || target.contains("mount") {
            output.push_str(&self.check_storage().await?);
        }

        Ok(output)
    }

    pub async fn check_status(&self, target: &str) -> Result<String> {
        let mut output = String::new();

        if target.contains("btrfs") {
            output.push_str(&self.check_btrfs_status().await?);
        }

        if target.contains("mount") {
            output.push_str(&self.check_mounts().await?);
        }

        Ok(output)
    }

    async fn check_systemd_service(&self, service: &str) -> Result<String> {
        let output = Command::new("systemctl")
            .args(&["status", service])
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn check_network(&self) -> Result<String> {
        let output = Command::new("ip").args(&["addr", "show"]).output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn check_storage(&self) -> Result<String> {
        let mut result = String::new();

        // Check disk usage
        let df_output = Command::new("df").args(&["-h"]).output()?;
        result.push_str("Disk Usage:\n");
        result.push_str(&String::from_utf8_lossy(&df_output.stdout));
        result.push_str("\n");

        // Check if btrfs is available
        if Command::new("which")
            .arg("btrfs")
            .output()?
            .status
            .success()
        {
            let btrfs_output = Command::new("btrfs")
                .args(&["filesystem", "show"])
                .output()?;
            result.push_str("Btrfs Filesystems:\n");
            result.push_str(&String::from_utf8_lossy(&btrfs_output.stdout));
        }

        Ok(result)
    }

    async fn check_btrfs_status(&self) -> Result<String> {
        let output = Command::new("btrfs")
            .args(&["filesystem", "usage", "/"])
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn check_mounts(&self) -> Result<String> {
        let output = Command::new("mount").output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
