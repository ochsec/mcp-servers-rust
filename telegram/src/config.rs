use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub api_id: i32,
    pub api_hash: String,
}

impl TelegramConfig {
    pub fn from_env() -> Result<Self> {
        let api_id = env::var("API_ID")
            .context("API_ID environment variable is required")?
            .parse::<i32>()
            .context("API_ID must be a valid integer")?;

        let api_hash = env::var("API_HASH")
            .context("API_HASH environment variable is required")?;

        Ok(Self { api_id, api_hash })
    }
}

pub fn get_state_dir() -> PathBuf {
    dirs::state_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("mcp-telegram")
}

pub fn get_session_file() -> PathBuf {
    get_state_dir().join("session")
}

pub fn get_downloads_dir() -> PathBuf {
    get_state_dir().join("downloads")
}