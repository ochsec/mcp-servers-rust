use thiserror::Error;

#[derive(Error, Debug)]
pub enum GmailError {
    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Invalid email address: {0}")]
    InvalidEmail(String),

    #[error("Label not found: {0}")]
    LabelNotFound(String),

    #[error("Message not found: {0}")]
    MessageNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("OAuth error: {0}")]
    OAuthError(String),

    #[error("MCP error: {0}")]
    McpError(String),
}

pub type Result<T> = std::result::Result<T, GmailError>;