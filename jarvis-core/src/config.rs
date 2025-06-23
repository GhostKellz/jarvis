use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use dirs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub llm: LLMConfig,
    pub system: SystemConfig,
    pub database_path: String,
    pub plugin_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub primary_provider: String,
    pub ollama_url: String,
    pub openai_api_key: Option<String>,
    pub claude_api_key: Option<String>,
    pub default_model: String,
    pub context_window: usize,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub arch_package_manager: String, // "pacman", "yay", "paru"
    pub dotfiles_path: Option<String>,
    pub homelab_config: Option<String>,
    pub gpu_enabled: bool,
    pub gpu_devices: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LLMConfig {
                primary_provider: "ollama".to_string(),
                ollama_url: "http://localhost:11434".to_string(),
                openai_api_key: None,
                claude_api_key: None,
                default_model: "llama3.1:8b".to_string(),
                context_window: 8192,
                temperature: 0.7,
            },
            system: SystemConfig {
                arch_package_manager: "pacman".to_string(),
                dotfiles_path: None,
                homelab_config: None,
                gpu_enabled: false,
                gpu_devices: vec![],
            },
            database_path: "~/.local/share/jarvis/memory.db".to_string(),
            plugin_paths: vec![
                "~/.config/jarvis/plugins".to_string(),
                "/usr/local/share/jarvis/plugins".to_string(),
            ],
        }
    }
}

impl Config {
    pub async fn load(config_path: Option<&str>) -> Result<Self> {
        let path = match config_path {
            Some(p) => PathBuf::from(p),
            None => {
                let config_dir = dirs::config_dir()
                    .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
                config_dir.join("jarvis").join("jarvis.toml")
            }
        };

        if path.exists() {
            let content = tokio::fs::read_to_string(&path).await?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            // Create default config
            let config = Config::default();
            config.save(&path).await?;
            Ok(config)
        }
    }

    pub async fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let content = toml::to_string_pretty(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    pub async fn init() -> Result<()> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        let config_path = config_dir.join("jarvis").join("jarvis.toml");
        
        let config = Config::default();
        config.save(&config_path).await?;
        
        // Also create other directories
        let data_dir = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find data directory"))?;
        tokio::fs::create_dir_all(data_dir.join("jarvis")).await?;
        
        Ok(())
    }

    pub async fn set(key: &str, value: &str) -> Result<()> {
        // TODO: Implement dynamic config setting
        println!("Setting {} = {} (not implemented yet)", key, value);
        Ok(())
    }
}
