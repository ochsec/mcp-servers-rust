# Gmail MCP Server (Rust)

A Model Context Protocol (MCP) server implementation for Gmail in Rust, providing comprehensive email management capabilities.

## Features

- **Email Operations**
  - Send emails with support for plain text, HTML, and multipart content
  - Create email drafts
  - Read email content with attachment information
  - Search emails using Gmail's search syntax
  - Delete emails

- **Label Management**
  - List all Gmail labels (system and user)
  - Create custom labels
  - Update existing labels
  - Delete labels
  - Get or create labels (idempotent operation)

- **Batch Operations**
  - Batch modify labels for multiple emails
  - Batch delete multiple emails
  - Configurable batch sizes for performance optimization

- **Security & Authentication**
  - OAuth 2.0 authentication with PKCE
  - Secure credential storage
  - Automatic token refresh
  - Configurable OAuth callback URLs

## Installation

### Prerequisites

- Rust 1.70 or later
- Gmail API credentials (OAuth 2.0)

### Build from Source

```bash
git clone <repository-url>
cd gmail-mcp-server
cargo build --release
```

## Setup

### 1. Get Gmail API Credentials

1. Go to the [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select an existing one
3. Enable the Gmail API
4. Create OAuth 2.0 credentials (Desktop application type)
5. Download the credentials JSON file

### 2. Configure Credentials

Place your OAuth credentials file in one of these locations:
- Current directory: `./gcp-oauth.keys.json`
- Global config: `~/.gmail-mcp/gcp-oauth.keys.json`
- Or set the `GMAIL_OAUTH_PATH` environment variable

### 3. Authenticate

Run the authentication command:

```bash
./target/release/gmail-mcp --auth
```

This will:
- Open your browser for Gmail authentication
- Start a local server to receive the OAuth callback
- Save your credentials securely

### 4. Run the MCP Server

```bash
./target/release/gmail-mcp
```

## Usage

### As an MCP Server

The server implements the Model Context Protocol and can be used with MCP-compatible clients like Claude Desktop.

#### Configure with Claude Desktop

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "gmail": {
      "command": "/path/to/gmail-mcp",
      "args": []
    }
  }
}
```

### Available Tools

#### Email Operations

- **send_email**: Send a new email
- **draft_email**: Create an email draft
- **read_email**: Read email content by message ID
- **search_emails**: Search emails using Gmail syntax
- **modify_email**: Add/remove labels from emails
- **delete_email**: Permanently delete an email

#### Label Management

- **list_email_labels**: List all available labels
- **create_label**: Create a new label
- **update_label**: Update an existing label
- **delete_label**: Delete a label
- **get_or_create_label**: Get existing or create new label

#### Batch Operations

- **batch_modify_emails**: Modify labels for multiple emails
- **batch_delete_emails**: Delete multiple emails

### Example Tool Calls

#### Send an Email

```json
{
  "name": "send_email",
  "arguments": {
    "to": ["recipient@example.com"],
    "subject": "Hello from MCP",
    "body": "This is a test email sent via the Gmail MCP server.",
    "mimeType": "text/plain"
  }
}
```

#### Search Emails

```json
{
  "name": "search_emails",
  "arguments": {
    "query": "from:sender@example.com is:unread",
    "maxResults": 10
  }
}
```

#### Create a Label

```json
{
  "name": "create_label",
  "arguments": {
    "name": "My Custom Label",
    "messageListVisibility": "show",
    "labelListVisibility": "labelShow"
  }
}
```

## Configuration

### Environment Variables

- `GMAIL_OAUTH_PATH`: Path to OAuth credentials file
- `GMAIL_CREDENTIALS_PATH`: Path to stored user credentials

### File Locations

- OAuth keys: `~/.gmail-mcp/gcp-oauth.keys.json`
- User credentials: `~/.gmail-mcp/credentials.json`

## Development

### Project Structure

```
src/
├── main.rs           # Application entry point
├── lib.rs            # Library root
├── auth.rs           # OAuth 2.0 authentication
├── client.rs         # Gmail API client
├── error.rs          # Error types
├── label_manager.rs  # Label management operations
├── server.rs         # MCP server implementation
├── tools.rs          # Tool implementations
└── utils.rs          # Utility functions
```

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

## Security

- All credentials are stored securely in the user's home directory
- OAuth 2.0 with PKCE for secure authentication
- No credentials are logged or exposed
- Local callback server runs only during authentication

## Error Handling

The server provides detailed error messages for:
- Authentication failures
- API rate limiting
- Invalid email addresses
- Missing permissions
- Network connectivity issues

## Performance

- Configurable batch sizes for bulk operations
- Efficient JSON parsing and serialization
- Async/await for non-blocking I/O
- Connection reuse for API calls

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Troubleshooting

### Common Issues

**Authentication fails**
- Ensure OAuth credentials are valid and properly formatted
- Check that the Gmail API is enabled in Google Cloud Console
- Verify redirect URI matches the callback URL

**API errors**
- Check internet connectivity
- Verify Gmail API quotas and limits
- Ensure proper OAuth scopes are granted

**Permission errors**
- Make sure the credentials file is readable
- Check file system permissions for config directory

### Getting Help

For issues and questions:
1. Check the troubleshooting section above
2. Review error messages and logs
3. Open an issue on the project repository