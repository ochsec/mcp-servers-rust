use thiserror::Error;

#[derive(Error, Debug)]
pub enum TelegramError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}