use anyhow::Result;
use git2::Repository;
use jarvis_core::types::{GitContext, SystemInfo};
use std::env;
use std::path::PathBuf;

pub struct Environment {
    pub working_directory: PathBuf,
    pub git_context: Option<GitContext>,
    pub system_info: SystemInfo,
    pub dotfiles_path: Option<PathBuf>,
    pub arch_info: ArchInfo,
}

pub struct ArchInfo {
    pub package_manager: String,
    pub aur_helper: Option<String>,
    pub kernel_version: String,
    pub desktop_environment: Option<String>,
}

impl Environment {
    pub async fn detect() -> Result<Self> {
        let working_directory = env::current_dir()?;
        let git_context = detect_git_context(&working_directory).await?;
        let system_info = detect_system_info().await?;
        let dotfiles_path = detect_dotfiles_path().await?;
        let arch_info = detect_arch_info().await?;

        Ok(Self {
            working_directory,
            git_context,
            system_info,
            dotfiles_path,
            arch_info,
        })
    }

    pub fn system_info(&self) -> String {
        format!(
            "OS: {} | Kernel: {} | Host: {} | Arch: {} | Uptime: {}s",
            self.system_info.os,
            self.system_info.kernel,
            self.system_info.hostname,
            self.system_info.arch,
            self.system_info.uptime
        )
    }

    pub fn is_arch_linux(&self) -> bool {
        self.system_info.os.to_lowercase().contains("arch")
    }

    pub fn has_aur_helper(&self) -> bool {
        self.arch_info.aur_helper.is_some()
    }

    /// Get comprehensive environment summary for LLM context
    pub fn get_context_summary(&self) -> String {
        let mut summary = vec![];
        
        summary.push(format!("System: {}", self.system_info()));
        
        if let Some(git) = &self.git_context {
            summary.push(format!(
                "Git: {} on {} ({}{})",
                git.repo_path,
                git.current_branch,
                git.last_commit,
                if git.dirty { " - dirty" } else { "" }
            ));
        }
        
        if let Some(dotfiles) = &self.dotfiles_path {
            summary.push(format!("Dotfiles: {}", dotfiles.display()));
        }
        
        if self.is_arch_linux() {
            summary.push(format!(
                "Arch: {} {}",
                self.arch_info.package_manager,
                self.arch_info.aur_helper.as_deref().unwrap_or("no AUR helper")
            ));
        }
        
        if let Some(de) = &self.arch_info.desktop_environment {
            summary.push(format!("DE: {}", de));
        }
        
        summary.join(" | ")
    }

    /// Detect available development tools
    pub async fn detect_dev_tools(&self) -> Vec<String> {
        let tools = ["git", "cargo", "rustc", "docker", "kubectl", "npm", "node", "python", "go"];
        let mut available = Vec::new();
        
        for tool in tools {
            if which::which(tool).is_ok() {
                available.push(tool.to_string());
            }
        }
        
        available
    }

    /// Get memory and disk usage
    pub async fn get_resource_usage(&self) -> Result<ResourceUsage> {
        let memory_info = std::fs::read_to_string("/proc/meminfo")
            .map(|content| {
                let mut total = 0u64;
                let mut available = 0u64;
                
                for line in content.lines() {
                    if line.starts_with("MemTotal:") {
                        total = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                    } else if line.starts_with("MemAvailable:") {
                        available = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                    }
                }
                
                (total * 1024, (total - available) * 1024) // Convert KB to bytes
            })
            .unwrap_or((0, 0));

        Ok(ResourceUsage {
            memory_total: memory_info.0,
            memory_used: memory_info.1,
            load_avg: (
                self.system_info.load_avg.0 as f32,
                self.system_info.load_avg.1 as f32,
                self.system_info.load_avg.2 as f32,
            ),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub memory_total: u64,
    pub memory_used: u64,
    pub load_avg: (f32, f32, f32),
}

async fn detect_git_context(working_dir: &PathBuf) -> Result<Option<GitContext>> {
    match Repository::discover(working_dir) {
        Ok(repo) => {
            let head = repo.head()?;
            let branch_name = head.shorthand().unwrap_or("unknown").to_string();

            let repo_path = repo
                .workdir()
                .unwrap_or_else(|| repo.path())
                .to_string_lossy()
                .to_string();

            let status = repo.statuses(None)?;
            let dirty = !status.is_empty();

            let last_commit = if let Ok(commit) = head.peel_to_commit() {
                commit.id().to_string()[..8].to_string()
            } else {
                "unknown".to_string()
            };

            Ok(Some(GitContext {
                repo_path,
                current_branch: branch_name,
                dirty,
                last_commit,
            }))
        }
        Err(_) => Ok(None),
    }
}

async fn detect_system_info() -> Result<SystemInfo> {
    use std::process::Command;

    let hostname = hostname::get()?.to_string_lossy().to_string();

    let kernel = Command::new("uname")
        .arg("-r")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let arch = Command::new("uname")
        .arg("-m")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let os = if std::path::Path::new("/etc/arch-release").exists() {
        "Arch Linux".to_string()
    } else if std::path::Path::new("/etc/os-release").exists() {
        // Parse /etc/os-release for better detection
        std::fs::read_to_string("/etc/os-release")
            .unwrap_or_default()
            .lines()
            .find(|line| line.starts_with("PRETTY_NAME="))
            .and_then(|line| line.split('=').nth(1))
            .map(|s| s.trim_matches('"').to_string())
            .unwrap_or_else(|| "Linux".to_string())
    } else {
        "Linux".to_string()
    };

    // Get uptime
    let uptime = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|content| content.split_whitespace().next().map(|s| s.to_string()))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0) as u64;

    // Get load average
    let load_avg = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|content| {
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() >= 3 {
                let load1 = parts[0].parse().ok()?;
                let load5 = parts[1].parse().ok()?;
                let load15 = parts[2].parse().ok()?;
                Some((load1, load5, load15))
            } else {
                None
            }
        })
        .unwrap_or((0.0, 0.0, 0.0));

    Ok(SystemInfo {
        os,
        kernel,
        hostname,
        arch,
        uptime,
        load_avg,
    })
}

async fn detect_dotfiles_path() -> Result<Option<PathBuf>> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    // Common dotfiles locations
    let candidates = vec![
        home.join(".dotfiles"),
        home.join("dotfiles"),
        home.join(".config"),
        home.join("dev").join("dotfiles"),
    ];

    for path in candidates {
        if path.exists() && path.is_dir() {
            // Check if it looks like a dotfiles repo
            if path.join(".git").exists()
                || path.join("README.md").exists()
                || path.join("install.sh").exists()
            {
                return Ok(Some(path));
            }
        }
    }

    Ok(None)
}

async fn detect_arch_info() -> Result<ArchInfo> {
    use std::process::Command;

    let package_manager = if which::which("pacman").is_ok() {
        "pacman".to_string()
    } else {
        "unknown".to_string()
    };

    let aur_helper = ["yay", "paru", "trizen", "aurman"]
        .iter()
        .find(|&&helper| which::which(helper).is_ok())
        .map(|s| s.to_string());

    let kernel_version = Command::new("uname")
        .arg("-r")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let desktop_environment = env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| env::var("DESKTOP_SESSION"))
        .ok();

    Ok(ArchInfo {
        package_manager,
        aur_helper,
        kernel_version,
        desktop_environment,
    })
}
