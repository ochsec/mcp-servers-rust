use crate::error::TelegramError;
use regex::Regex;
use std::path::Path;
use uuid::Uuid;

pub fn parse_entity(entity: &str) -> Result<i64, String> {
    if entity.chars().all(|c| c.is_ascii_digit() || c == '-') {
        entity.parse::<i64>().map_err(|_| entity.to_string())
    } else {
        Err(entity.to_string())
    }
}

pub fn get_unique_filename(original_name: Option<&str>, media_id: i64, mime_type: Option<&str>) -> String {
    let unique_id = Uuid::new_v4().to_string();
    
    if let Some(name) = original_name {
        let path = Path::new(name);
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("download");
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        
        if extension.is_empty() {
            format!("{}_{}", stem, unique_id)
        } else {
            format!("{}_{}.{}", stem, unique_id, extension)
        }
    } else {
        let extension = mime_type
            .and_then(|mime| mime.split('/').nth(1))
            .unwrap_or("bin");
        
        format!("download_{}_{}.{}", media_id, unique_id, extension)
    }
}

pub fn parse_telegram_url(url: &str) -> Result<(String, i32), TelegramError> {
    let pattern = r"^(?:https?://)?t(?:elegram)?\.me/(?:(?P<username>[A-Za-z0-9_]+)/(?P<message_id>\d+)|c/(?P<chat_id>\d+)/(?P<chat_message_id>\d+))/?$";
    let re = Regex::new(pattern).map_err(|e| TelegramError::UrlParsing(e.to_string()))?;
    
    if let Some(captures) = re.captures(url) {
        let entity = captures.name("username")
            .or(captures.name("chat_id"))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| TelegramError::UrlParsing("No entity found in URL".to_string()))?;
        
        let message_id = captures.name("message_id")
            .or(captures.name("chat_message_id"))
            .map(|m| m.as_str())
            .ok_or_else(|| TelegramError::UrlParsing("No message ID found in URL".to_string()))?
            .parse::<i32>()
            .map_err(|e| TelegramError::UrlParsing(format!("Invalid message ID: {}", e)))?;
        
        Ok((entity, message_id))
    } else {
        Err(TelegramError::UrlParsing("Invalid Telegram URL format".to_string()))
    }
}