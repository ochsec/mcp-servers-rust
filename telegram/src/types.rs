use chrono::{DateTime, Utc};
use grammers_tl_types::types::{Channel, Chat, User, Message as GrammersMessage, Document, Photo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DialogType {
    User,
    Group,
    Channel,
    Bot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dialog {
    pub id: i64,
    pub title: String,
    pub username: Option<String>,
    pub phone_number: Option<String>,
    #[serde(rename = "type")]
    pub dialog_type: DialogType,
    pub unread_messages_count: i32,
    pub can_send_message: bool,
}

impl Dialog {
    pub fn from_user(user: &User, can_send_message: bool) -> Self {
        let dialog_type = if user.bot.unwrap_or(false) {
            DialogType::Bot
        } else {
            DialogType::User
        };

        Self {
            id: user.id,
            title: format!("{} {}", 
                user.first_name.as_deref().unwrap_or(""),
                user.last_name.as_deref().unwrap_or("")
            ).trim().to_string(),
            username: user.username.clone(),
            phone_number: user.phone.clone(),
            dialog_type,
            unread_messages_count: 0,
            can_send_message,
        }
    }

    pub fn from_chat(chat: &Chat, can_send_message: bool) -> Self {
        Self {
            id: chat.id,
            title: chat.title.clone(),
            username: None,
            phone_number: None,
            dialog_type: DialogType::Group,
            unread_messages_count: 0,
            can_send_message,
        }
    }

    pub fn from_channel(channel: &Channel, can_send_message: bool) -> Self {
        let dialog_type = if channel.megagroup.unwrap_or(false) {
            DialogType::Group
        } else {
            DialogType::Channel
        };

        Self {
            id: channel.id,
            title: channel.title.clone(),
            username: channel.username.clone(),
            phone_number: None,
            dialog_type,
            unread_messages_count: 0,
            can_send_message,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub media_id: i64,
    pub mime_type: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
}

impl Media {
    pub fn from_document(document: &Document) -> Self {
        let file_name = document.attributes
            .iter()
            .filter_map(|attr| {
                match attr {
                    grammers_tl_types::enums::DocumentAttribute::Filename(f) => Some(f.file_name.clone()),
                    _ => None,
                }
            })
            .next();

        Self {
            media_id: document.id,
            mime_type: Some(document.mime_type.clone()),
            file_name,
            file_size: Some(document.size),
        }
    }

    pub fn from_photo(photo: &Photo) -> Self {
        Self {
            media_id: photo.id,
            mime_type: Some("image/jpeg".to_string()),
            file_name: None,
            file_size: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadedMedia {
    pub path: String,
    pub media: Media,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub message_id: i32,
    pub sender_id: Option<i64>,
    pub message: Option<String>,
    pub outgoing: bool,
    pub date: Option<DateTime<Utc>>,
    pub media: Option<Media>,
    pub reply_to: Option<i32>,
}

impl Message {
    pub fn from_grammers_message(msg: &GrammersMessage, outgoing: bool) -> Self {
        let media = msg.media.as_ref().and_then(|m| {
            match m {
                grammers_tl_types::enums::MessageMedia::Document(doc) => {
                    Some(Media::from_document(&doc.document))
                }
                grammers_tl_types::enums::MessageMedia::Photo(photo) => {
                    Some(Media::from_photo(&photo.photo))
                }
                _ => None,
            }
        });

        let reply_to = msg.reply_to.as_ref().and_then(|r| {
            match r {
                grammers_tl_types::enums::MessageReplyHeader::Header(h) => {
                    h.reply_to_msg_id
                }
            }
        });

        Self {
            message_id: msg.id,
            sender_id: msg.from_id.as_ref().map(|peer| {
                match peer {
                    grammers_tl_types::enums::Peer::User(u) => u.user_id,
                    grammers_tl_types::enums::Peer::Chat(c) => c.chat_id,
                    grammers_tl_types::enums::Peer::Channel(ch) => ch.channel_id,
                }
            }),
            message: msg.message.clone(),
            outgoing,
            date: Some(DateTime::from_timestamp(msg.date as i64, 0).unwrap_or_default()),
            media,
            reply_to,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Messages {
    pub messages: Vec<Message>,
    pub dialog: Option<Dialog>,
}