use anyhow::Result;
use clap::{Arg, Command};
use gmail_mcp_server::GmailMcpServer;
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let matches = Command::new("gmail-mcp")
        .version("0.1.0")
        .about("Gmail MCP Server")
        .arg(
            Arg::new("auth")
                .long("auth")
                .action(clap::ArgAction::SetTrue)
                .help("Authenticate with Gmail API"),
        )
        .arg(
            Arg::new("callback")
                .long("callback")
                .value_name("URL")
                .help("OAuth callback URL (default: http://localhost:3000/oauth2callback)"),
        )
        .get_matches();

    if matches.get_flag("auth") {
        info!("Starting authentication process...");
        let callback_url = matches
            .get_one::<String>("callback")
            .map(|s| s.as_str())
            .unwrap_or("http://localhost:3000/oauth2callback");
        
        let mut server = GmailMcpServer::new().await?;
        server.authenticate(callback_url).await?;
        info!("Authentication completed successfully");
        return Ok(());
    }

    // Start the MCP server
    info!("Starting Gmail MCP server...");
    let mut server = GmailMcpServer::new().await?;
    server.run().await?;
    
    Ok(())
}