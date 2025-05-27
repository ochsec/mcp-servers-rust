use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{stdin, stdout, AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::transport::Transport;

pub struct StdioServerTransport {
    // This is a simplified implementation
}

impl StdioServerTransport {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Transport for StdioServerTransport {
    async fn send(&self, message: Value) -> Result<()> {
        let json_str = serde_json::to_string(&message)?;
        let mut stdout = stdout();
        stdout.write_all(json_str.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
        Ok(())
    }

    async fn receive(&self) -> Result<Value> {
        let stdin = stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let value: Value = serde_json::from_str(&line.trim())?;
        Ok(value)
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}