# Atlassian MCP Server (Rust)

A Rust implementation of the Model Context Protocol (MCP) server for Atlassian JIRA and Confluence integration. This is a one-to-one port of the original TypeScript implementation from [ATLASSIAN-MCP](ATLASSIAN-MCP/).

## Features

This MCP server provides tools to interact with Atlassian JIRA and Confluence, enabling:

### JIRA Integration
- **Get JIRA ticket details** by ticket key
- **Search JIRA tickets** using JQL (JIRA Query Language)
- **Create new JIRA tickets** with project key, summary, description, and issue type
- **Add comments** to existing JIRA tickets

### Confluence Integration  
- **Get Confluence pages** by page ID
- **Search Confluence content** using text queries

## Installation

### Prerequisites
- Rust 1.70 or later
- An Atlassian account with API token access

### Building from Source

```bash
cd atlassian
cargo build --release
```

The binary will be available at `target/release/atlassian-mcp-server`.

## Configuration

### Option 1: Configuration File

Create a `config/config.json` file (you can copy from `config/config.sample.json`):

```json
{
  "atlassian": {
    "baseUrl": "https://your-instance.atlassian.net",
    "email": "your-email@example.com", 
    "token": "your-api-token-here"
  },
  "server": {
    "name": "atlassian-server",
    "version": "0.1.0"
  }
}
```

### Option 2: Environment Variables

Set the following environment variables:

```bash
export ATLASSIAN_BASE_URL="https://your-instance.atlassian.net"
export ATLASSIAN_EMAIL="your-email@example.com"
export ATLASSIAN_TOKEN="your-api-token-here"
export SERVER_NAME="atlassian-server"        # Optional
export SERVER_VERSION="0.1.0"                # Optional
```

### Getting Your Atlassian API Token

1. Go to [https://id.atlassian.com/manage-profile/security/api-tokens](https://id.atlassian.com/manage-profile/security/api-tokens)
2. Click "Create API token"
3. Give it a label (e.g., "MCP Server")
4. Copy the generated token

## Usage

### Running the Server

```bash
# Using config file
./target/release/atlassian-mcp-server

# Using environment variables  
ATLASSIAN_BASE_URL="https://your-instance.atlassian.net" \
ATLASSIAN_EMAIL="your-email@example.com" \
ATLASSIAN_TOKEN="your-token" \
./target/release/atlassian-mcp-server
```

### Available Tools

#### JIRA Tools

1. **get_jira_ticket**
   - Get details of a JIRA ticket by key
   - Parameters: `ticket_key` (string, required)
   - Example: `{"ticket_key": "PROJ-123"}`

2. **search_jira_tickets**
   - Search for JIRA tickets using JQL
   - Parameters: 
     - `jql` (string, required) - JQL query
     - `max_results` (number, optional, default: 10)
   - Example: `{"jql": "project = PROJ AND status = Open", "max_results": 20}`

3. **create_jira_ticket**
   - Create a new JIRA ticket
   - Parameters:
     - `project_key` (string, required)
     - `summary` (string, required)
     - `description` (string, required)
     - `issue_type` (string, optional, default: "Task")
   - Example: `{"project_key": "PROJ", "summary": "New bug", "description": "Bug description", "issue_type": "Bug"}`

4. **add_comment_to_jira_ticket**
   - Add a comment to a JIRA ticket
   - Parameters:
     - `ticket_key` (string, required)
     - `comment` (string, required)
   - Example: `{"ticket_key": "PROJ-123", "comment": "This is a comment"}`

#### Confluence Tools

5. **get_confluence_page**
   - Get a Confluence page by ID
   - Parameters: `page_id` (string, required)
   - Example: `{"page_id": "123456"}`

6. **search_confluence**
   - Search for content in Confluence
   - Parameters:
     - `query` (string, required)
     - `limit` (number, optional, default: 10)
   - Example: `{"query": "documentation", "limit": 5}`

## Architecture

The Rust implementation maintains the same structure as the original TypeScript version:

- **`config.rs`**: Configuration management with support for both file and environment variable configuration
- **`atlassian.rs`**: Atlassian API client with methods for JIRA and Confluence operations
- **`main.rs`**: MCP server implementation using the `rmcp` crate with tool definitions

## Dependencies

- [`rmcp`](https://crates.io/crates/rmcp): Official Rust MCP SDK
- [`reqwest`](https://crates.io/crates/reqwest): HTTP client for Atlassian API calls
- [`serde`](https://crates.io/crates/serde): Serialization/deserialization
- [`tokio`](https://crates.io/crates/tokio): Async runtime
- [`tracing`](https://crates.io/crates/tracing): Logging
- [`anyhow`](https://crates.io/crates/anyhow): Error handling
- [`clap`](https://crates.io/crates/clap): Command-line argument parsing

## Differences from TypeScript Version

This Rust implementation provides the same functionality as the original TypeScript version but with:

- **Performance**: Compiled Rust binary for better performance
- **Memory Safety**: Rust's ownership system prevents common runtime errors
- **Type Safety**: Strong compile-time type checking
- **Modern Dependencies**: Uses the latest official `rmcp` crate

The API and tool interfaces remain identical to ensure compatibility.

## License

MIT

## Contributing

This is a port of the original [ATLASSIAN-MCP](https://github.com/kompallik/ATLASSIAN-MCP) project by Koundinya Kompalli. For feature requests or bug reports, please refer to the original project or create issues in the appropriate repository.