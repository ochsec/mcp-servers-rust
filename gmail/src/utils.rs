use crate::client::{MessageHeader, MessagePayload};
use crate::error::{GmailError, Result};
use base64::{engine::general_purpose, Engine as _};
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailContent {
    pub text: String,
    pub html: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailAttachment {
    pub id: String,
    pub filename: String,
    pub mime_type: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendEmailArgs {
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
    pub html_body: Option<String>,
    pub mime_type: Option<String>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub thread_id: Option<String>,
    pub in_reply_to: Option<String>,
}

pub fn validate_email(email: &str) -> bool {
    let email_regex = Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
    email_regex.is_match(email)
}

pub fn encode_email_header(text: &str) -> String {
    // Only encode if the text contains non-ASCII characters
    if text.chars().any(|c| !c.is_ascii()) {
        // Use MIME Words encoding (RFC 2047)
        format!("=?UTF-8?B?{}?=", general_purpose::STANDARD.encode(text.as_bytes()))
    } else {
        text.to_string()
    }
}

pub fn create_email_message(args: &SendEmailArgs) -> Result<String> {
    let encoded_subject = encode_email_header(&args.subject);
    
    // Determine content type based on available content and explicit mime_type
    let mut mime_type = args.mime_type.as_deref().unwrap_or("text/plain");
    
    // If html_body is provided and mime_type isn't explicitly set to text/plain,
    // use multipart/alternative to include both versions
    if args.html_body.is_some() && mime_type != "text/plain" {
        mime_type = "multipart/alternative";
    }

    // Generate a random boundary string for multipart messages
    let boundary = format!("----=_NextPart_{}", Uuid::new_v4().simple());

    // Validate email addresses
    for email in &args.to {
        if !validate_email(email) {
            return Err(GmailError::InvalidEmail(email.clone()));
        }
    }

    if let Some(cc_emails) = &args.cc {
        for email in cc_emails {
            if !validate_email(email) {
                return Err(GmailError::InvalidEmail(email.clone()));
            }
        }
    }

    if let Some(bcc_emails) = &args.bcc {
        for email in bcc_emails {
            if !validate_email(email) {
                return Err(GmailError::InvalidEmail(email.clone()));
            }
        }
    }

    // Common email headers
    let mut email_parts = vec![
        "From: me".to_string(),
        format!("To: {}", args.to.join(", ")),
    ];

    if let Some(cc) = &args.cc {
        if !cc.is_empty() {
            email_parts.push(format!("Cc: {}", cc.join(", ")));
        }
    }

    if let Some(bcc) = &args.bcc {
        if !bcc.is_empty() {
            email_parts.push(format!("Bcc: {}", bcc.join(", ")));
        }
    }

    email_parts.push(format!("Subject: {}", encoded_subject));

    // Add thread-related headers if specified
    if let Some(in_reply_to) = &args.in_reply_to {
        email_parts.push(format!("In-Reply-To: {}", in_reply_to));
        email_parts.push(format!("References: {}", in_reply_to));
    }

    email_parts.push("MIME-Version: 1.0".to_string());

    // Construct the email based on the content type
    match mime_type {
        "multipart/alternative" => {
            // Multipart email with both plain text and HTML
            email_parts.push(format!("Content-Type: multipart/alternative; boundary=\"{}\"", boundary));
            email_parts.push("".to_string());
            
            // Plain text part
            email_parts.push(format!("--{}", boundary));
            email_parts.push("Content-Type: text/plain; charset=UTF-8".to_string());
            email_parts.push("Content-Transfer-Encoding: 7bit".to_string());
            email_parts.push("".to_string());
            email_parts.push(args.body.clone());
            email_parts.push("".to_string());
            
            // HTML part
            email_parts.push(format!("--{}", boundary));
            email_parts.push("Content-Type: text/html; charset=UTF-8".to_string());
            email_parts.push("Content-Transfer-Encoding: 7bit".to_string());
            email_parts.push("".to_string());
            email_parts.push(args.html_body.as_ref().unwrap_or(&args.body).clone());
            email_parts.push("".to_string());
            
            // Close the boundary
            email_parts.push(format!("--{}--", boundary));
        }
        "text/html" => {
            // HTML-only email
            email_parts.push("Content-Type: text/html; charset=UTF-8".to_string());
            email_parts.push("Content-Transfer-Encoding: 7bit".to_string());
            email_parts.push("".to_string());
            email_parts.push(args.html_body.as_ref().unwrap_or(&args.body).clone());
        }
        _ => {
            // Plain text email (default)
            email_parts.push("Content-Type: text/plain; charset=UTF-8".to_string());
            email_parts.push("Content-Transfer-Encoding: 7bit".to_string());
            email_parts.push("".to_string());
            email_parts.push(args.body.clone());
        }
    }

    Ok(email_parts.join("\r\n"))
}

pub fn encode_message_for_gmail(message: &str) -> String {
    general_purpose::URL_SAFE_NO_PAD.encode(message.as_bytes())
}

pub fn extract_email_content(message_part: &MessagePayload) -> EmailContent {
    let mut text_content = String::new();
    let mut html_content = String::new();

    // If the part has a body with data, process it based on MIME type
    if let Some(body) = &message_part.body {
        if let Some(data) = &body.data {
            if let Ok(decoded) = general_purpose::URL_SAFE_NO_PAD.decode(data) {
                if let Ok(content) = String::from_utf8(decoded) {
                    // Store content based on its MIME type
                    if let Some(mime_type) = &message_part.mime_type {
                        match mime_type.as_str() {
                            "text/plain" => text_content = content,
                            "text/html" => html_content = content,
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // If the part has nested parts, recursively process them
    if let Some(parts) = &message_part.parts {
        for part in parts {
            let extracted = extract_email_content(part);
            if !extracted.text.is_empty() {
                text_content.push_str(&extracted.text);
            }
            if !extracted.html.is_empty() {
                html_content.push_str(&extracted.html);
            }
        }
    }

    EmailContent {
        text: text_content,
        html: html_content,
    }
}

pub fn extract_attachments(message_part: &MessagePayload) -> Vec<EmailAttachment> {
    let mut attachments = Vec::new();

    fn process_attachment_parts(part: &MessagePayload, attachments: &mut Vec<EmailAttachment>) {
        if let Some(body) = &part.body {
            if let Some(attachment_id) = &body.attachment_id {
                let filename = part.filename.as_deref().unwrap_or(&format!("attachment-{}", attachment_id)).to_string();
                attachments.push(EmailAttachment {
                    id: attachment_id.clone(),
                    filename,
                    mime_type: part.mime_type.as_deref().unwrap_or("application/octet-stream").to_string(),
                    size: body.size.unwrap_or(0),
                });
            }
        }

        if let Some(parts) = &part.parts {
            for subpart in parts {
                process_attachment_parts(subpart, attachments);
            }
        }
    }

    process_attachment_parts(message_part, &mut attachments);
    attachments
}

pub fn get_header_value(headers: &[MessageHeader], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|h| h.name.to_lowercase() == name.to_lowercase())
        .map(|h| h.value.clone())
}

pub fn format_email_for_display(
    message: &crate::client::GmailMessage,
    content: &EmailContent,
    attachments: &[EmailAttachment],
) -> String {
    let empty_headers = vec![];
    let headers = message.payload.as_ref()
        .and_then(|p| p.headers.as_ref())
        .unwrap_or(&empty_headers);

    let thread_id = message.thread_id.as_deref().unwrap_or("");
    let subject = get_header_value(headers, "subject").unwrap_or_default();
    let from = get_header_value(headers, "from").unwrap_or_default();
    let to = get_header_value(headers, "to").unwrap_or_default();
    let date = get_header_value(headers, "date").unwrap_or_default();

    // Use plain text content if available, otherwise use HTML content
    let body = if !content.text.is_empty() {
        content.text.clone()
    } else {
        content.html.clone()
    };

    // If we only have HTML content, add a note for the user
    let content_type_note = if content.text.is_empty() && !content.html.is_empty() {
        "[Note: This email is HTML-formatted. Plain text version not available.]\n\n"
    } else {
        ""
    };

    // Add attachment info to output if any are present
    let attachment_info = if !attachments.is_empty() {
        format!(
            "\n\nAttachments ({}):\n{}",
            attachments.len(),
            attachments
                .iter()
                .map(|a| format!("- {} ({}, {} KB)", a.filename, a.mime_type, a.size / 1024))
                .collect::<Vec<_>>()
                .join("\n")
        )
    } else {
        String::new()
    };

    format!(
        "Thread ID: {}\nSubject: {}\nFrom: {}\nTo: {}\nDate: {}\n\n{}{}{}",
        thread_id, subject, from, to, date, content_type_note, body, attachment_info
    )
}