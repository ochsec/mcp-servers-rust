# GitHub MCP Server (Rust)

A Rust implementation of the GitHub Model Context Protocol (MCP) server that provides one-to-one functionality with the Go implementation.

## Overview

This is a complete port of the [GitHub MCP Server](../github-mcp-server/) from Go to Rust, maintaining full compatibility and feature parity. The server enables LLMs and other tools to interact with GitHub's API through the Model Context Protocol.

## Features

### Tool Categories

#### **Repository Tools** (`repos`)
- `search_repositories` - Search for GitHub repositories with filtering and sorting
- `get_file_contents` - Get contents of a file or directory from a repository
- `get_repository` - Get detailed information about a repository
- `create_or_update_file` - Create or update a single file in a repository (write mode only)

#### **Issue Tools** (`issues`)
- `get_issue` - Get details of a specific issue by number
- `list_issues` - List and filter repository issues with pagination
- `create_issue` - Create a new issue with title, body, assignees, and labels (write mode only)

#### **Pull Request Tools** (`pull_requests`)
- `get_pull_request` - Get details of a specific pull request by number
- `list_pull_requests` - List and filter repository pull requests with pagination
- `create_pull_request` - Create a new pull request between branches (write mode only)

#### **User Tools** (`users`)
- `search_users` - Search for GitHub users with filtering and sorting

#### **Context Tools** (`context`) - Always Enabled
- `get_me` - Get details of the authenticated user

#### **Dynamic Tools** (`dynamic`) - Always Enabled when `--dynamic-toolsets` is used
- `list_available_toolsets` - List all available toolsets and their descriptions
- `get_toolset_tools` - List all tools available in a specific toolset
- `enable_toolset` - Enable additional toolsets at runtime

### Resources

Repository content accessible via URI templates:
- `repo://{owner}/{repo}/contents{/path*}` - Repository content
- `repo://{owner}/{repo}/refs/heads/{branch}/contents{/path*}` - Branch-specific content
- `repo://{owner}/{repo}/sha/{sha}/contents{/path*}` - Commit-specific content
- `repo://{owner}/{repo}/refs/tags/{tag}/contents{/path*}` - Tag-specific content
- `repo://{owner}/{repo}/refs/pull/{prNumber}/head/contents{/path*}` - Pull request content

## Installation

### Prerequisites

- Rust 1.70+ with Cargo
- GitHub Personal Access Token

### Building

```bash
cargo build --release
```

### Configuration

Set the required environment variable:

```bash
export GITHUB_PERSONAL_ACCESS_TOKEN="your_token_here"
```

## Usage

### Start the MCP Server

```bash
# Start with all toolsets enabled
./target/release/github-mcp-server stdio

# Start with specific toolsets
./target/release/github-mcp-server stdio --toolsets "repos,issues,pull_requests"

# Start in read-only mode
./target/release/github-mcp-server stdio --read-only

# Enable dynamic toolset management
./target/release/github-mcp-server stdio --dynamic-toolsets

# Use with GitHub Enterprise
./target/release/github-mcp-server stdio --gh-host "https://github.enterprise.com"
```

### Command Line Options

- `--toolsets <TOOLSETS>`: Comma-separated list of toolsets to enable (default: "all")
- `--dynamic-toolsets`: Enable runtime toolset management
- `--read-only`: Restrict to read-only operations
- `--log-file <FILE>`: Path to log file
- `--enable-command-logging`: Log all commands and responses
- `--gh-host <HOST>`: GitHub hostname for Enterprise installations

### Available Toolsets

- `repos`: Repository management (search, file operations, branch management)
- `issues`: Issue management (create, list, update, comment)
- `pull_requests`: Pull request management (create, list, review, merge)
- `users`: User search and information
- `context`: Current user context (always enabled)
- `dynamic`: Runtime toolset management (always enabled when `--dynamic-toolsets` is used)

## Architecture

### Core Components

- **GitHub Client** (`src/github/`): Abstraction over GitHub REST and GraphQL APIs
- **MCP Server** (`src/server/`): Protocol implementation and request handling
- **Tool Registry** (`src/tools/`): Dynamic tool management and execution
- **Resource Registry** (`src/resources/`): Repository content access
- **Toolsets** (`src/tools/toolsets.rs`): Logical grouping of related tools

### Key Design Principles

- **Async/Await**: Full async implementation using Tokio
- **Type Safety**: Strongly typed API interactions and parameter validation
- **Error Handling**: Comprehensive error handling with anyhow
- **Modularity**: Clean separation between toolsets and functionality
- **Observability**: Structured logging with tracing

## Development

### Project Structure

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library exports
├── mcp_core.rs          # MCP protocol implementation
├── github/              # GitHub API client
│   ├── mod.rs
│   ├── client.rs        # REST/GraphQL client
│   └── types.rs         # GitHub API types
├── server/              # MCP server implementation
│   └── mod.rs           # Server logic and protocol handling
├── tools/               # Tool implementations
│   ├── mod.rs
│   ├── registry.rs      # Tool registration and execution
│   ├── toolsets.rs      # Toolset organization
│   ├── context.rs       # Context tools
│   ├── repos.rs         # Repository tools
│   ├── issues.rs        # Issue tools
│   ├── pull_requests.rs # Pull request tools
│   ├── users.rs         # User tools
│   └── dynamic.rs       # Dynamic toolset management
└── resources/           # Resource implementations
    └── mod.rs           # Repository content resources
```

### Dependencies

- **tokio**: Async runtime and I/O
- **serde**: JSON serialization/deserialization
- **reqwest**: HTTP client for GitHub API calls
- **clap**: Command-line interface and argument parsing
- **tracing**: Structured logging and diagnostics
- **anyhow**: Error handling and context
- **chrono**: Date and time handling
- **base64**: Base64 encoding/decoding for file content
- **url**: URL parsing and manipulation
- **urlencoding**: URL encoding for API parameters

### Testing

```bash
# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo test
```

## Differences from Go Implementation

### Language-Specific Adaptations

1. **Async/Await**: Full async implementation vs Go's goroutines
2. **Type System**: Rust's strict type system provides additional safety
3. **Error Handling**: `Result<T, E>` pattern vs Go's `error` interface
4. **Memory Management**: Rust's ownership system vs Go's garbage collection

### Implementation Choices

1. **JSON-RPC**: Direct implementation using serde_json vs using external library
2. **GitHub API**: Custom reqwest-based client vs `go-github` library
3. **Configuration**: `clap` for CLI vs `cobra`/`viper` in Go
4. **Logging**: `tracing` ecosystem vs `logrus` in Go
5. **Async**: Tokio-based async/await vs Go's goroutines and channels

## License

MIT License - see LICENSE file for details.

## Contributing

This project maintains feature parity with the Go implementation. When contributing:

1. Ensure all Go functionality is preserved
2. Follow Rust best practices and idioms
3. Add tests for new functionality
4. Update documentation

## Related Projects

- [Original Go Implementation](../github-mcp-server/)
- [MCP Specification](https://github.com/anthropics/mcp)