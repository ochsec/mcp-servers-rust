use thiserror::Error;

#[derive(Error, Debug)]
pub enum TelegramError {
    #[error("Telegram client error: {0}")]
    Client(#[from] grammers_client::Error),
    
    #[error("Session error: {0}")]
    Session(#[from] grammers_session::SessionError),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Invalid entity: {0}")]
    InvalidEntity(String),
    
    #[error("Invalid message ID: {0}")]
    InvalidMessageId(i32),
    
    #[error("Media download error: {0}")]
    MediaDownload(String),
    
    #[error("URL parsing error: {0}")]
    UrlParsing(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}