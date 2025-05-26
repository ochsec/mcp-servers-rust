# MCP Telegram Server (Rust)

A Rust implementation of the MCP (Model Context Protocol) server for Telegram, porting the functionality from the original Python version.

## Features

This MCP server enables AI agents to interact with Telegram through the following tools:

### 📨 Messaging Tools
- `send_message` - Send text messages or files to any user, group, or channel
- `edit_message` - Modify content of previously sent messages  
- `delete_message` - Remove one or multiple messages
- `get_messages` - Retrieve message history with advanced filtering options

### 🔍 Search & Navigation
- `search_dialogs` - Find users, groups, and channels by name or username
- `message_from_link` - Access specific messages using Telegram links

### 📝 Draft Management
- `get_draft` - View current message draft for any chat
- `set_draft` - Create or clear message drafts

### 📂 Media Handling
- `media_download` - Download photos, videos, and documents from messages

## Installation

### Prerequisites
- Rust 1.70 or higher
- Telegram API credentials (API ID and API Hash from [my.telegram.org/apps](https://my.telegram.org/apps))

### Build from Source

```bash
git clone <repository-url>
cd mcp-telegram-rust
cargo build --release
```

## Usage

### Authentication

First, authenticate with your Telegram account:

```bash
./target/release/mcp-telegram login
```

This will prompt you for:
- **API ID & API Hash** from [my.telegram.org/apps](https://my.telegram.org/apps)
- **Phone Number** in international format (e.g., `+1234567890`)
- **Verification Code** sent to your Telegram account
- **2FA Password** if you have Two-Factor Authentication enabled

### Running the MCP Server

Start the server:

```bash
./target/release/mcp-telegram start
```

### MCP Client Configuration

To use with MCP clients like Claude Desktop, add the following to your MCP configuration:

```json
{
  "mcpServers": {
    "mcp-telegram": {
      "command": "/path/to/mcp-telegram",
      "args": ["start"],
      "env": {
        "API_ID": "<your_api_id>",
        "API_HASH": "<your_api_hash>"
      }
    }
  }
}
```

### Other Commands

```bash
# List all available tools
./target/release/mcp-telegram tools

# Show logout instructions
./target/release/mcp-telegram logout

# Clear local session data
./target/release/mcp-telegram clear-session

# Show version
./target/release/mcp-telegram version
```

## Configuration

The server requires the following environment variables:

- `API_ID` - Your Telegram API ID
- `API_HASH` - Your Telegram API Hash

These can be set in your shell or provided through the MCP client configuration.

## Session Management

Session data is stored in:
- **Linux/macOS**: `~/.local/state/mcp-telegram/session`
- **Windows**: `%LOCALAPPDATA%/mcp-telegram/session`

Downloads are saved to:
- **Linux/macOS**: `~/.local/state/mcp-telegram/downloads/`
- **Windows**: `%LOCALAPPDATA%/mcp-telegram/downloads/`

## Important Notes

> **⚠️ Warning:** Please ensure you have read and understood Telegram's [Terms of Service](https://telegram.org/tos) before using this tool. Misuse may result in account restrictions.

> **🔒 Security:** Keep your API credentials private and never share them publicly. The session file contains sensitive authentication data.

## Architecture

This Rust implementation uses:
- **grammers-client** - Telegram client library for Rust
- **mcp-server** - MCP server framework for Rust
- **tokio** - Async runtime
- **clap** - Command-line interface
- **serde** - Serialization/deserialization

## Differences from Python Version

While this Rust port aims to maintain feature parity with the Python version, there are some implementation differences:

1. **Telegram Client Library**: Uses `grammers-client` instead of `telethon`
2. **Session Storage**: Compatible session format but different internal structure
3. **Performance**: Generally faster due to Rust's performance characteristics
4. **Memory Usage**: Lower memory footprint
5. **Dependencies**: Fewer runtime dependencies

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Logging

Set the `RUST_LOG` environment variable to control logging levels:

```bash
RUST_LOG=debug ./target/release/mcp-telegram start
```

## Troubleshooting

### Database Locked Errors
Running multiple instances with the same session file can cause database lock errors. Ensure only one instance uses a session file at a time.

### Connection Issues
If you experience connection problems:
1. Check your internet connection
2. Verify your API credentials are correct
3. Ensure you're not being rate-limited by Telegram

### Build Issues
If you encounter build issues:
1. Ensure you have Rust 1.70 or higher
2. Update your dependencies: `cargo update`
3. Clean and rebuild: `cargo clean && cargo build`

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit issues, feature requests, or pull requests.

## Acknowledgments

- Original Python implementation by [Yeabsira Driba](https://github.com/dryeab/mcp-telegram)
- [grammers](https://github.com/Lonami/grammers) Telegram client library
- [Model Context Protocol](https://modelcontextprotocol.io/) specification