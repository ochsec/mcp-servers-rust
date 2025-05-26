use crate::config::{get_downloads_dir, get_session_file, TelegramConfig};
use crate::error::TelegramError;
use crate::types::{Dialog, DownloadedMedia, Media, Message, Messages};
use crate::utils::{get_unique_filename, parse_entity, parse_telegram_url};
use anyhow::Result;
use chrono::{DateTime, Utc};
use grammers_client::{Client, Config, SignInError, Update};
use grammers_session::Session;
use grammers_tl_types::enums::{InputPeer, MessageMedia, Peer};
use grammers_tl_types::types::{
    Channel, Chat, InputPeerChannel, InputPeerChat, InputPeerUser, Message as GrammersMessage,
    User,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub struct TelegramClient {
    client: Option<Client>,
    config: TelegramConfig,
    session_file: std::path::PathBuf,
    downloads_dir: std::path::PathBuf,
    entities_cache: RwLock<HashMap<String, InputPeer>>,
}

impl TelegramClient {
    pub fn new(config: TelegramConfig) -> Result<Self> {
        let session_file = get_session_file();
        let downloads_dir = get_downloads_dir();

        // Ensure directories exist
        if let Some(parent) = session_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(&downloads_dir)?;

        Ok(Self {
            client: None,
            config,
            session_file,
            downloads_dir,
            entities_cache: RwLock::new(HashMap::new()),
        })
    }

    pub async fn connect(&mut self) -> Result<(), TelegramError> {
        if self.client.is_some() {
            return Ok(());
        }

        let session = Session::load_file_or_create(&self.session_file)?;
        let client = Client::connect(Config {
            session,
            api_id: self.config.api_id,
            api_hash: self.config.api_hash.clone(),
            params: Default::default(),
        })
        .await?;

        self.client = Some(client);
        info!("Connected to Telegram");
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), TelegramError> {
        if let Some(client) = self.client.take() {
            client.session().save_to_file(&self.session_file)?;
            info!("Disconnected from Telegram and saved session");
        }
        Ok(())
    }

    pub async fn is_authorized(&self) -> bool {
        self.client
            .as_ref()
            .map(|c| c.is_authorized())
            .unwrap_or(false)
    }

    pub async fn sign_in_with_phone(&mut self, phone: &str) -> Result<(), TelegramError> {
        let client = self.client.as_mut().ok_or_else(|| {
            TelegramError::Config("Client not connected. Call connect() first.".to_string())
        })?;

        client.request_login_code(phone, self.config.api_id, &self.config.api_hash).await?;
        Ok(())
    }

    pub async fn sign_in_with_code(&mut self, code: &str) -> Result<(), TelegramError> {
        let client = self.client.as_mut().ok_or_else(|| {
            TelegramError::Config("Client not connected. Call connect() first.".to_string())
        })?;

        match client.sign_in_code(code).await {
            Err(SignInError::PasswordRequired(_)) => {
                Err(TelegramError::Config("2FA password required".to_string()))
            }
            Err(e) => Err(TelegramError::Client(e.into())),
            Ok(_) => {
                info!("Successfully signed in");
                Ok(())
            }
        }
    }

    pub async fn sign_in_with_password(&mut self, password: &str) -> Result<(), TelegramError> {
        let client = self.client.as_mut().ok_or_else(|| {
            TelegramError::Config("Client not connected. Call connect() first.".to_string())
        })?;

        client.check_password(password).await?;
        info!("Successfully signed in with 2FA password");
        Ok(())
    }

    async fn resolve_entity(&self, entity: &str) -> Result<InputPeer, TelegramError> {
        let client = self.client.as_ref().ok_or_else(|| {
            TelegramError::Config("Client not connected".to_string())
        })?;

        // Check cache first
        {
            let cache = self.entities_cache.read().await;
            if let Some(cached) = cache.get(entity) {
                return Ok(cached.clone());
            }
        }

        // Try to parse as numeric ID first
        let input_peer = if let Ok(id) = parse_entity(entity) {
            // For numeric IDs, we need to determine the peer type
            // This is a limitation - in a real implementation, you'd need to
            // store peer type information or use different methods
            InputPeer::User(InputPeerUser {
                user_id: id,
                access_hash: 0, // This would need to be resolved properly
            })
        } else if entity == "me" {
            // Special case for self
            let me = client.get_me().await?;
            InputPeer::User(InputPeerUser {
                user_id: me.id,
                access_hash: me.access_hash.unwrap_or(0),
            })
        } else {
            // Try to resolve as username
            match client.resolve_username(entity).await {
                Ok(resolved) => match resolved {
                    Peer::User(user) => InputPeer::User(InputPeerUser {
                        user_id: user.user_id,
                        access_hash: 0, // Would need proper resolution
                    }),
                    Peer::Chat(chat) => InputPeer::Chat(InputPeerChat {
                        chat_id: chat.chat_id,
                    }),
                    Peer::Channel(channel) => InputPeer::Channel(InputPeerChannel {
                        channel_id: channel.channel_id,
                        access_hash: 0, // Would need proper resolution
                    }),
                },
                Err(_) => {
                    return Err(TelegramError::InvalidEntity(format!(
                        "Could not resolve entity: {}",
                        entity
                    )));
                }
            }
        };

        // Cache the result
        {
            let mut cache = self.entities_cache.write().await;
            cache.insert(entity.to_string(), input_peer.clone());
        }

        Ok(input_peer)
    }

    pub async fn send_message(
        &self,
        entity: &str,
        message: &str,
        file_paths: Option<&[String]>,
        reply_to: Option<i32>,
    ) -> Result<(), TelegramError> {
        let client = self.client.as_ref().ok_or_else(|| {
            TelegramError::Config("Client not connected".to_string())
        })?;

        // Validate file paths if provided
        if let Some(paths) = file_paths {
            for path in paths {
                let path_obj = Path::new(path);
                if !path_obj.exists() || !path_obj.is_file() {
                    return Err(TelegramError::FileNotFound(path.to_string()));
                }
            }
        }

        let input_peer = self.resolve_entity(entity).await?;

        // For now, we'll implement basic text message sending
        // File sending would require more complex implementation with grammers
        client
            .send_message(&input_peer, message)
            .reply_to(reply_to)
            .await?;

        debug!("Message sent to {}", entity);
        Ok(())
    }

    pub async fn edit_message(
        &self,
        entity: &str,
        message_id: i32,
        new_message: &str,
    ) -> Result<(), TelegramError> {
        let client = self.client.as_ref().ok_or_else(|| {
            TelegramError::Config("Client not connected".to_string())
        })?;

        let input_peer = self.resolve_entity(entity).await?;
        client
            .edit_message(&input_peer, message_id, new_message)
            .await?;

        debug!("Message {} edited in {}", message_id, entity);
        Ok(())
    }

    pub async fn delete_messages(
        &self,
        entity: &str,
        message_ids: &[i32],
    ) -> Result<(), TelegramError> {
        let client = self.client.as_ref().ok_or_else(|| {
            TelegramError::Config("Client not connected".to_string())
        })?;

        let input_peer = self.resolve_entity(entity).await?;
        client.delete_messages(&input_peer, message_ids).await?;

        debug!("Deleted {} messages in {}", message_ids.len(), entity);
        Ok(())
    }

    pub async fn get_messages(
        &self,
        entity: &str,
        limit: usize,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        unread_only: bool,
        mark_as_read: bool,
    ) -> Result<Messages, TelegramError> {
        let client = self.client.as_ref().ok_or_else(|| {
            TelegramError::Config("Client not connected".to_string())
        })?;

        let input_peer = self.resolve_entity(entity).await?;

        // Get messages using grammers client
        let mut messages = Vec::new();
        let mut iter = client.iter_messages(&input_peer);

        while let Some(message) = iter.next().await? {
            if messages.len() >= limit {
                break;
            }

            // Filter by date if specified
            if let Some(start) = start_date {
                if message.date() < start.timestamp() as i32 {
                    continue;
                }
            }

            if let Some(end) = end_date {
                if message.date() > end.timestamp() as i32 {
                    break;
                }
            }

            // Convert grammers message to our Message type
            let msg = Message::from_grammers_message(
                &message.msg,
                message.outgoing(),
            );

            if mark_as_read {
                // Mark as read if requested
                // This would require additional implementation
                warn!("Mark as read not fully implemented");
            }

            messages.push(msg);
        }

        Ok(Messages {
            messages,
            dialog: None, // Would need to fetch dialog info separately
        })
    }

    pub async fn search_dialogs(
        &self,
        query: &str,
        limit: usize,
        global_search: bool,
    ) -> Result<Vec<Dialog>, TelegramError> {
        let client = self.client.as_ref().ok_or_else(|| {
            TelegramError::Config("Client not connected".to_string())
        })?;

        // This is a simplified implementation
        // A full implementation would use the contacts.search API
        let mut dialogs = Vec::new();
        let mut iter = client.iter_dialogs();

        while let Some(dialog) = iter.next().await? {
            if dialogs.len() >= limit {
                break;
            }

            let chat = dialog.chat();
            let title = chat.name();

            // Simple case-insensitive search
            if title.to_lowercase().contains(&query.to_lowercase()) {
                let dialog_obj = match chat.pack() {
                    grammers_tl_types::enums::Chat::User(user) => {
                        Dialog::from_user(&user, true) // Assume can send for now
                    }
                    grammers_tl_types::enums::Chat::Chat(chat) => {
                        Dialog::from_chat(&chat, true)
                    }
                    grammers_tl_types::enums::Chat::Channel(channel) => {
                        Dialog::from_channel(&channel, true)
                    }
                    _ => continue,
                };

                dialogs.push(dialog_obj);
            }
        }

        Ok(dialogs)
    }

    pub async fn get_draft(&self, entity: &str) -> Result<String, TelegramError> {
        // Draft functionality would need to be implemented with grammers
        // For now, return empty string
        warn!("get_draft not fully implemented");
        Ok(String::new())
    }

    pub async fn set_draft(&self, entity: &str, message: &str) -> Result<(), TelegramError> {
        // Draft functionality would need to be implemented with grammers
        warn!("set_draft not fully implemented");
        Ok(())
    }

    pub async fn download_media(
        &self,
        entity: &str,
        message_id: i32,
        path: Option<&str>,
    ) -> Result<DownloadedMedia, TelegramError> {
        let client = self.client.as_ref().ok_or_else(|| {
            TelegramError::Config("Client not connected".to_string())
        })?;

        let input_peer = self.resolve_entity(entity).await?;

        // Get the specific message
        let message = client
            .get_messages_by_id(&input_peer, &[message_id])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| TelegramError::InvalidMessageId(message_id))?;

        // Check if message has media
        let media = message.media().ok_or_else(|| {
            TelegramError::MediaDownload("Message does not contain media".to_string())
        })?;

        // Extract media info and create Media object
        let media_obj = match media {
            MessageMedia::Document(doc) => Media::from_document(&doc.document),
            MessageMedia::Photo(photo) => Media::from_photo(&photo.photo),
            _ => {
                return Err(TelegramError::MediaDownload(
                    "Unsupported media type".to_string(),
                ));
            }
        };

        // Generate unique filename
        let filename = get_unique_filename(
            media_obj.file_name.as_deref(),
            media_obj.media_id,
            media_obj.mime_type.as_deref(),
        );

        // Determine save path
        let save_path = if let Some(custom_path) = path {
            Path::new(custom_path).join(&filename)
        } else {
            self.downloads_dir.join(&filename)
        };

        // Download the media
        let downloaded_path = client.download_media(&message, &save_path).await?;

        Ok(DownloadedMedia {
            path: downloaded_path.to_string_lossy().to_string(),
            media: media_obj,
        })
    }

    pub async fn message_from_link(&self, link: &str) -> Result<Message, TelegramError> {
        let (entity, message_id) = parse_telegram_url(link)?;

        let client = self.client.as_ref().ok_or_else(|| {
            TelegramError::Config("Client not connected".to_string())
        })?;

        let input_peer = self.resolve_entity(&entity).await?;

        let message = client
            .get_messages_by_id(&input_peer, &[message_id])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| TelegramError::InvalidMessageId(message_id))?;

        Ok(Message::from_grammers_message(
            &message.msg,
            message.outgoing(),
        ))
    }
}