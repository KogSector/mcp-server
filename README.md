# ConHub MCP Server

The unified Model Context Protocol server for ConHub. Exposes all ConHub data sources (repositories, documents, etc.) via the standard MCP protocol to AI agents and IDEs.

## What is This?

This is ConHub's **canonical MCP server** - a Rust-based service that:

- Implements the Model Context Protocol (MCP) over stdio
- Provides unified access to all ConHub connectors (GitHub, GitLab, Bitbucket, Google Drive, Dropbox, Local FS, Notion)
- Can be used by any MCP-compatible AI agent (Cursor, Windsurf, Cline, etc.)
- Enforces ConHub's security policies and rules

## Architecture

```
AI Agent (Cursor/Cline/Windsurf)
    ↓ (MCP over stdio)
ConHub MCP Server (this crate)
    ↓
Connector Manager
    ├─ GitHub Connector
    ├─ GitLab Connector
    ├─ Bitbucket Connector
    ├─ Google Drive Connector
    ├─ Dropbox Connector
    ├─ Local FS Connector
    └─ Notion Connector
```

## Quick Start - Local Development

### 1. Prerequisites

- Rust 1.70+
- PostgreSQL (or Neon DB) running
- `.env` file configured (see below)

### 2. Configure Environment

Create `mcp/.env`:

```env
# Database (required for full connector features)
DATABASE_URL=postgresql://conhub:conhub_password@localhost:5432/conhub
# OR use Neon:
# DATABASE_URL_NEON=postgresql://neondb_owner:npg_xxx@ep-xxx.neon.tech/neondb?sslmode=require

# MCP Service Config
MCP_SERVICE_PORT=3016
HOST=0.0.0.0
RUST_LOG=info

# Connector Enable/Disable
ENABLE_FS=true
ENABLE_GITHUB=true
ENABLE_GITLAB=false
ENABLE_BITBUCKET=false
ENABLE_GDRIVE=false
ENABLE_DROPBOX=false
ENABLE_NOTION=false

# Local Filesystem Connector
FS_ROOT_PATHS=c:\Users\risha\Desktop\Work\ConHub
# Multiple paths: FS_ROOT_PATHS=/path1,/path2,/path3

# GitHub API (if GitHub connector enabled)
GITHUB_API_BASE=https://api.github.com
GITHUB_TOKEN=ghp_your_token_here

# GitLab API (if GitLab connector enabled)
GITLAB_BASE_URL=https://gitlab.com
GITLAB_TOKEN=your_token

# Bitbucket API (if Bitbucket connector enabled)
BITBUCKET_BASE_URL=https://api.bitbucket.org/2.0
BITBUCKET_TOKEN=your_token

# Notion API (if Notion connector enabled)
NOTION_TOKEN=secret_your_token

# Timeouts and Rate Limiting
REQUEST_TIMEOUT_SECS=30
CACHE_TTL_SECS=300
RATE_LIMIT_PER_MINUTE=60
```

### 3. Run Locally

```bash
cd mcp
cargo run --bin mcp-service
```

You should see:
```
🔗 ConHub MCP Server starting on stdio
📡 Model Context Protocol ready
🔌 2 connectors enabled
```

### 4. Test with an MCP Client

The server communicates over stdio using JSON-RPC. You can test it manually:

```bash
# In one terminal, run the server
cargo run --bin mcp-service

# In the same terminal, type (or pipe):
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"test","version":"1.0"}}}

# Expected response:
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{...},"serverInfo":{"name":"ConHub MCP Server","version":"1.0.0"}}}
```

## Connecting AI Agents

### Cursor / Windsurf / Cline

Add to your MCP client configuration (varies by tool):

```json
{
  "mcpServers": {
    "conhub": {
      "command": "c:/Users/risha/.cargo/bin/mcp-service",
      "args": [],
      "cwd": "c:/Users/risha/Desktop/Work/ConHub/mcp",
      "env": {
        "DATABASE_URL": "postgresql://conhub:conhub_password@localhost:5432/conhub",
        "FS_ROOT_PATHS": "c:/Users/risha/Desktop/Work/ConHub",
        "ENABLE_FS": "true",
        "RUST_LOG": "info"
      }
    }
  }
}
```

The agent will:
1. Launch `mcp-service` as a subprocess
2. Send `initialize` request
3. Discover available tools via `tools/list`
4. Call tools like `fs.read_file`, `github.list_repositories`, etc.

## Available Tools

Tools are namespaced by connector:

- **fs.*** - Local filesystem operations
  - `fs.list_files` - List files in a directory
  - `fs.read_file` - Read file contents
  - `fs.search_files` - Search files by pattern

- **github.*** - GitHub operations
  - `github.list_repositories` - List user/org repositories
  - `github.get_file` - Get file from repository
  - `github.search_code` - Search code across repos

- **gitlab.*** - GitLab operations
- **bitbucket.*** - Bitbucket operations
- **gdrive.*** - Google Drive operations
- **dropbox.*** - Dropbox operations
- **notion.*** - Notion operations

Use `tools/list` to discover all available tools and their schemas.

## Resources

Resources use URI schemes:

- `fs://path/to/file` - Local files
- `github://owner/repo/path` - GitHub files
- `gitlab://...` - GitLab resources
- `dropbox://...` - Dropbox files
- `gdrive://...` - Google Drive files
- `notion://...` - Notion pages

## MCP Protocol Methods

This server implements standard MCP methods:

| Method | Description |
|--------|-------------|
| `initialize` | Initialize MCP connection, return capabilities |
| `tools/list` | List all available tools |
| `tools/call` | Execute a tool |
| `resources/list` | List available resources |
| `resources/read` | Read resource content |

## Security & Rules

The MCP server integrates with ConHub's security layer:

- All tool calls go through `SecurityClient` for authorization
- User permissions from ConHub DB are enforced
- Rate limiting per connector
- Audit logging for all operations

## Building for Production

```bash
cargo build --release
```

Binary will be at: `target/release/mcp-service`

## Troubleshooting

**"Database connection failed"**
- Check `DATABASE_URL` or `DATABASE_URL_NEON` is set correctly
- Ensure PostgreSQL is running
- Run migrations: `cd database && sqlx migrate run`

**"No tools available"**
- Check at least one connector is enabled via `ENABLE_*` env vars
- Check connector-specific tokens are set (GITHUB_TOKEN, etc.)

**"Agent can't connect"**
- Ensure the MCP client config points to the correct binary path
- Check `cwd` is set to the `mcp/` directory so `.env` is found
- Enable debug logging: `RUST_LOG=debug`

## Development

Run tests:
```bash
cargo test
```

Run with debug logging:
```bash
RUST_LOG=debug cargo run
```

Format code:
```bash
cargo fmt
```

## Migration from Legacy JS Service

The old `mcp/service/` (Node.js) is now **deprecated**. All functionality has been moved to this Rust implementation. The JS code will be archived.

## Related Services

- `client/` - AI Service (manages external AI agents like Copilot, uses this MCP server for context)
- `data/` - Data ingestion service (uses connectors for document processing)
- `backend/` - Main API gateway

---

For more information, see the [ConHub Architecture Documentation](../docs/architecture/).
