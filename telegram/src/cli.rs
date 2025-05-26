use crate::config::TelegramConfig;
use crate::telegram::TelegramClient;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::io::{self, Write};
use tracing::info;

#[derive(Parser)]
#[command(name = "mcp-telegram")]
#[command(about = "MCP Server for Telegram")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the MCP Telegram server
    Start,
    /// Login to Telegram
    Login,
    /// Show instructions on how to logout from Telegram
    Logout,
    /// Delete the local Telegram session file
    ClearSession,
    /// List all available tools
    Tools,
    /// Show version information
    Version,
}

pub enum CliCommand {
    Start,
    Login,
    Logout,
    ClearSession,
    Tools,
    Version,
}

pub async fn run_cli() -> Result<CliCommand> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start => Ok(CliCommand::Start),
        Commands::Login => Ok(CliCommand::Login),
        Commands::Logout => Ok(CliCommand::Logout),
        Commands::ClearSession => Ok(CliCommand::ClearSession),
        Commands::Tools => Ok(CliCommand::Tools),
        Commands::Version => Ok(CliCommand::Version),
    }
}

pub async fn login() -> Result<()> {
    println!("🚀 Welcome to MCP Telegram!");
    println!();
    println!("To proceed with login, you'll need your Telegram API credentials:");
    println!("1. Visit https://my.telegram.org/apps");
    println!("2. Create a new application if you haven't already");
    println!("3. Copy your API ID and API Hash");
    println!();

    // Get API credentials
    print!("🔑 API ID: ");
    io::stdout().flush()?;
    let mut api_id_input = String::new();
    io::stdin().read_line(&mut api_id_input)?;
    let api_id = api_id_input.trim().to_string();

    print!("🔒 API Hash: ");
    io::stdout().flush()?;
    let mut api_hash = String::new();
    io::stdin().read_line(&mut api_hash)?;
    let api_hash = api_hash.trim().to_string();

    print!("📱 Phone Number (e.g., +1234567890): ");
    io::stdout().flush()?;
    let mut phone = String::new();
    io::stdin().read_line(&mut phone)?;
    let phone = phone.trim();

    // Create config and client
    let config = TelegramConfig {
        api_id: api_id.parse().context("API ID must be a valid number")?,
        api_hash,
    };

    let mut client = TelegramClient::new(config)?;
    client.connect().await?;

    println!("📞 Requesting login code...");
    client.sign_in_with_phone(phone).await?;

    // Get verification code
    print!("🔢 Enter the verification code sent to your Telegram: ");
    io::stdout().flush()?;
    let mut code = String::new();
    io::stdin().read_line(&mut code)?;
    let code = code.trim();

    match client.sign_in_with_code(code).await {
        Ok(_) => {
            println!("✅ Successfully logged in!");
            client.disconnect().await?;
            Ok(())
        }
        Err(crate::error::TelegramError::Config(msg)) if msg.contains("2FA") => {
            // 2FA required
            print!("🔐 Enter your 2FA password: ");
            io::stdout().flush()?;
            let mut password = String::new();
            io::stdin().read_line(&mut password)?;
            let password = password.trim();

            client.sign_in_with_password(password).await?;
            println!("✅ Successfully logged in with 2FA!");
            client.disconnect().await?;
            Ok(())
        }
        Err(e) => {
            client.disconnect().await?;
            Err(e.into())
        }
    }
}

pub async fn logout() -> Result<()> {
    println!("🚪 How to Logout from Telegram");
    println!();
    println!("To logout from your Telegram account, please follow these steps:");
    println!();
    println!("1. Open your Telegram app");
    println!("2. Go to Settings");
    println!("3. Select Privacy and Security");
    println!("4. Scroll down to find 'Active Sessions'");
    println!("5. Find and terminate the session with the name of your app");
    println!("   (This is the app name you created on my.telegram.org/apps)");
    println!();
    println!("Note: After logging out, you can use the 'clear-session' command to remove local session data.");

    Ok(())
}

pub async fn clear_session() -> Result<()> {
    let session_file = crate::config::get_session_file();
    let session_file_with_ext = session_file.with_extension("session");

    if session_file_with_ext.exists() {
        std::fs::remove_file(&session_file_with_ext)?;
        println!("🗑️ Session file successfully deleted!");
        println!("You can now safely create a new session by logging in again.");
    } else if session_file.exists() {
        std::fs::remove_file(&session_file)?;
        println!("🗑️ Session file successfully deleted!");
        println!("You can now safely create a new session by logging in again.");
    } else {
        println!("ℹ️ No session file found!");
        println!("The session file may have already been deleted or never existed.");
    }

    Ok(())
}

pub async fn tools() -> Result<()> {
    println!("🔧 Available Tools");
    println!();
    
    let tools = vec![
        ("send_message", "Send text messages or files to any user, group, or channel"),
        ("edit_message", "Modify content of previously sent messages"),
        ("delete_message", "Remove one or multiple messages"),
        ("get_messages", "Retrieve message history with advanced filtering options"),
        ("search_dialogs", "Find users, groups, and channels by name or username"),
        ("message_from_link", "Access specific messages using Telegram links"),
        ("get_draft", "View current message draft for any chat"),
        ("set_draft", "Create or clear message drafts"),
        ("media_download", "Download photos, videos, and documents from messages"),
    ];

    for (name, description) in tools {
        println!("• {}: {}", name, description);
    }

    println!();
    println!("For detailed parameter information, refer to the MCP tool schemas when connected to a client.");

    Ok(())
}

pub async fn version() -> Result<()> {
    println!("📦 MCP Telegram Rust Server");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Description: {}", env!("CARGO_PKG_DESCRIPTION"));
    
    Ok(())
}