use anyhow::{anyhow, Result};
use clap::{Arg, ArgAction, Command};
use std::env;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use github_mcp_server::{GitHubMcpServer, GitHubServerConfig};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("github-mcp-server")
        .version(VERSION)
        .about("A GitHub MCP server that handles various tools and resources")
        .subcommand(
            Command::new("stdio")
                .about("Start stdio server")
                .long_about("Start a server that communicates via standard input/output streams using JSON-RPC messages")
                .arg(
                    Arg::new("toolsets")
                        .long("toolsets")
                        .value_name("TOOLSETS")
                        .help("Comma separated list of groups of tools to allow")
                        .default_value("all")
                        .action(ArgAction::Set)
                )
                .arg(
                    Arg::new("dynamic-toolsets")
                        .long("dynamic-toolsets")
                        .help("Enable dynamic toolsets")
                        .action(ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("read-only")
                        .long("read-only")
                        .help("Restrict the server to read-only operations")
                        .action(ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("log-file")
                        .long("log-file")
                        .value_name("FILE")
                        .help("Path to log file")
                        .action(ArgAction::Set)
                )
                .arg(
                    Arg::new("enable-command-logging")
                        .long("enable-command-logging")
                        .help("Enable logging of all command requests and responses")
                        .action(ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("gh-host")
                        .long("gh-host")
                        .value_name("HOST")
                        .help("Specify the GitHub hostname (for GitHub Enterprise etc.)")
                        .action(ArgAction::Set)
                )
        )
        .get_matches();

    // Initialize logging
    init_logging()?;

    match matches.subcommand() {
        Some(("stdio", sub_matches)) => {
            let token = env::var("GITHUB_PERSONAL_ACCESS_TOKEN")
                .map_err(|_| anyhow!("GITHUB_PERSONAL_ACCESS_TOKEN not set"))?;

            let toolsets_str = sub_matches.get_one::<String>("toolsets").unwrap();
            let enabled_toolsets: Vec<String> = if toolsets_str == "all" {
                vec!["all".to_string()]
            } else {
                toolsets_str.split(',').map(|s| s.trim().to_string()).collect()
            };

            let config = GitHubServerConfig {
                version: VERSION.to_string(),
                host: sub_matches.get_one::<String>("gh-host").cloned(),
                token,
                enabled_toolsets,
                dynamic_toolsets: sub_matches.get_flag("dynamic-toolsets"),
                read_only: sub_matches.get_flag("read-only"),
                enable_command_logging: sub_matches.get_flag("enable-command-logging"),
            };

            info!("Starting GitHub MCP Server v{}", VERSION);
            info!("Configuration: {:?}", config);

            let mut server = GitHubMcpServer::new(config).await?;
            server.run_stdio().await?;
        }
        _ => {
            eprintln!("No subcommand specified. Use 'stdio' to start the server.");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn init_logging() -> Result<()> {
    let filter = env::var("RUST_LOG")
        .unwrap_or_else(|_| "github_mcp_server=info".to_string());

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter))
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    Ok(())
}