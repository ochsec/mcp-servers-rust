use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlassianConfig {
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    pub email: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub atlassian: AtlassianConfig,
    pub server: ServerConfig,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = env::var("ATLASSIAN_CONFIG_PATH")
            .unwrap_or_else(|_| "config/config.json".to_string());

        if Path::new(&config_path).exists() {
            eprintln!("Loading config from {}", config_path);
            let config_content = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config file: {}", config_path))?;
            
            let config: Config = serde_json::from_str(&config_content)
                .with_context(|| "Failed to parse config JSON")?;
            
            config.validate()?;
            Ok(config)
        } else {
            eprintln!("Config file not found at {}, using environment variables (this is normal when using MCP settings)", config_path);
            let config = Config {
                atlassian: AtlassianConfig {
                    base_url: env::var("ATLASSIAN_BASE_URL")
                        .with_context(|| "ATLASSIAN_BASE_URL environment variable is required")?,
                    email: env::var("ATLASSIAN_EMAIL")
                        .with_context(|| "ATLASSIAN_EMAIL environment variable is required")?,
                    token: env::var("ATLASSIAN_TOKEN")
                        .with_context(|| "ATLASSIAN_TOKEN environment variable is required")?,
                },
                server: ServerConfig {
                    name: env::var("SERVER_NAME").unwrap_or_else(|_| "atlassian-server".to_string()),
                    version: env::var("SERVER_VERSION").unwrap_or_else(|_| "0.1.0".to_string()),
                },
            };
            
            config.validate()?;
            Ok(config)
        }
    }

    fn validate(&self) -> Result<()> {
        if self.atlassian.base_url.is_empty() {
            anyhow::bail!("Atlassian base URL is required");
        }
        if self.atlassian.email.is_empty() {
            anyhow::bail!("Atlassian email is required");
        }
        if self.atlassian.token.is_empty() {
            anyhow::bail!("Atlassian token is required");
        }
        Ok(())
    }
}