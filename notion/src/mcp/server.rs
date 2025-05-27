use anyhow::Result;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use super::transport::Transport;

#[derive(Clone)]
pub struct ServerOptions {
    pub name: String,
    pub version: String,
}

type Handler = Arc<dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct Server {
    options: ServerOptions,
    handlers: Arc<RwLock<HashMap<String, Handler>>>,
}

impl Server {
    pub fn new(options: ServerOptions) -> Self {
        Self {
            options,
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_handler<F, Fut, Req, Res>(&self, method: &str, handler: F)
    where
        F: Fn(Req) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Res>> + Send + 'static,
        Req: serde::de::DeserializeOwned + Send + 'static,
        Res: serde::Serialize + Send + 'static,
    {
        let method_name = method.to_string();
        let handler = Arc::new(handler);
        let wrapped_handler: Handler = Arc::new(move |value: Value| {
            let handler = Arc::clone(&handler);
            Box::pin(async move {
                let request: Req = serde_json::from_value(value)?;
                let response = handler(request).await?;
                let response_value = serde_json::to_value(response)?;
                Ok(response_value)
            })
        });

        let mut handlers = self.handlers.write().await;
        handlers.insert(method_name, wrapped_handler);
    }

    pub async fn connect<T: Transport + Send + 'static>(&self, mut transport: T) -> Result<()> {
        loop {
            match transport.receive().await {
                Ok(message) => {
                    // Process the message
                    if let Some(method) = message.get("method").and_then(|m| m.as_str()) {
                        let handlers = self.handlers.read().await;
                        if let Some(handler) = handlers.get(method) {
                            if let Ok(result) = handler(message).await {
                                transport.send(result).await?;
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }
        Ok(())
    }
}