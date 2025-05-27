use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait Transport {
    async fn send(&self, message: Value) -> Result<()>;
    async fn receive(&self) -> Result<Value>;
    async fn close(&self) -> Result<()>;
}