mod cli;
mod config;
mod error;
mod server;
mod telegram;
mod types;
mod utils;

use anyhow::Result;
use cli::{run_cli, CliCommand};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "mcp_telegram=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting MCP Telegram server");

    match run_cli().await? {
        CliCommand::Start => {
            server::run().await?;
        }
        CliCommand::Login => {
            cli::login().await?;
        }
        CliCommand::Logout => {
            cli::logout().await?;
        }
        CliCommand::ClearSession => {
            cli::clear_session().await?;
        }
        CliCommand::Tools => {
            cli::tools().await?;
        }
        CliCommand::Version => {
            cli::version().await?;
        }
    }

    Ok(())
}