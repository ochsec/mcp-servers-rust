# Perplexity Ask MCP Server (Rust)

A Rust implementation of the MCP server that integrates the Sonar API to provide Claude with unparalleled real-time, web-wide research capabilities.

This is a port of the original TypeScript implementation with improved performance and memory efficiency.

## Features

- **perplexity_ask**: Engage in conversations using the Sonar API for live web searches
- **perplexity_research**: Perform deep research using the sonar-deep-research model
- **perplexity_reason**: Execute reasoning tasks using the sonar-reasoning-pro model

## Installation

### Prerequisites

- Rust 1.70 or later
- A Perplexity API key

### Building from Source

1. Clone this repository:
```bash
git clone <repository-url>
cd perplexity
```

2. Build the project:
```bash
cargo build --release
```

3. The binary will be available at `target/release/mcp-perplexity-ask`

## Configuration

### Step 1: Get a Sonar API Key

1. Sign up for a [Sonar API account](https://docs.perplexity.ai/guides/getting-started)
2. Follow the account setup instructions and generate your API key from the developer dashboard
3. Set the API key in your environment as `PERPLEXITY_API_KEY`

### Step 2: Configure Claude Desktop

Add this to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "perplexity-ask": {
      "command": "/path/to/mcp-perplexity-ask",
      "env": {
        "PERPLEXITY_API_KEY": "YOUR_API_KEY_HERE"
      }
    }
  }
}
```

You can access the file using:
```bash
vim ~/Library/Application\ Support/Claude/claude_desktop_config.json
```

## Tools

### perplexity_ask
- **Description**: Engage in a conversation with the Sonar API for live web searches
- **Model**: sonar-pro
- **Input**:
  - `messages` (array): An array of conversation messages
    - Each message must include:
      - `role` (string): The role of the message (e.g., `system`, `user`, `assistant`)
      - `content` (string): The content of the message

### perplexity_research
- **Description**: Performs deep research using the Perplexity API
- **Model**: sonar-deep-research
- **Input**: Same as perplexity_ask

### perplexity_reason
- **Description**: Performs reasoning tasks using the Perplexity API
- **Model**: sonar-reasoning-pro
- **Input**: Same as perplexity_ask

## Usage

Once configured, the server will automatically start when Claude Desktop loads. You can then use the tools by asking Claude to search for information or perform research tasks.

Example:
```
Can you research the latest developments in quantum computing?
```

Claude will automatically use the appropriate Perplexity tool to provide you with up-to-date information and citations.

## Development

### Running in Development Mode

```bash
# Set your API key
export PERPLEXITY_API_KEY="your-api-key-here"

# Run the server
cargo run
```

### Testing

The server communicates via stdin/stdout using the MCP protocol. You can test it by sending JSON-RPC messages:

```bash
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/list"}' | cargo run
```

## Performance

This Rust implementation offers several advantages over the TypeScript version:

- **Lower Memory Usage**: Rust's zero-cost abstractions and lack of garbage collection
- **Better Performance**: Compiled binary with optimized HTTP handling
- **Improved Reliability**: Rust's type system prevents many runtime errors

## Troubleshooting

1. **API Key Issues**: Ensure `PERPLEXITY_API_KEY` is properly set in your environment
2. **Connection Problems**: Check that your API key is valid and you have network access
3. **Claude Integration**: Verify the path to the binary in your claude_desktop_config.json

For additional support, refer to the [MCP troubleshooting guide](https://modelcontextprotocol.io/docs/tools/debugging).

## License

This MCP server is licensed under the MIT License. This means you are free to use, modify, and distribute the software, subject to the terms and conditions of the MIT License.