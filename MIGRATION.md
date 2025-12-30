# MCP Service Migration: JavaScript → Rust

## Status: COMPLETED

The ConHub MCP service has been fully migrated from JavaScript (Node.js + Express) to Rust.

## What Changed

### Before (Deprecated - now in `legacy/`)
- **Location:** `mcp/service/` (Node.js/Express)
- **Purpose:** MCP gateway/proxy + Agent manager
- **Stack:** Express, WebSocket, axios, winston
- **Issues:**
  - Mixed responsibilities (MCP proxy + agent orchestration)
  - Connectors duplicated between JS and Rust
  - No standard MCP protocol support (custom `mcp.*` methods)
  - Agent-specific logic embedded in MCP service

### After (Current)
- **Location:** `mcp/` (Rust crate: `mcp-service`)
- **Purpose:** Pure MCP server implementing standard protocol
- **Stack:** Tokio, Actix, sqlx, tracing
- **Benefits:**
  - Standard MCP protocol (`initialize`, `tools/list`, `tools/call`, etc.)
  - All connectors in Rust with shared security/database layer
  - Stdio transport for direct IDE integration
  - Agent-specific logic moved to `client/` service

## Deprecated Code (Archived in `legacy/`)

The following have been moved to `mcp/legacy/` for reference but should **NOT** be used:

### Directories
- `mcp/service/` - Node.js MCP gateway service
  - `src/server.js` - Express server
  - `src/services/McpService.js` - MCP proxy logic
  - `src/services/AgentManager.js` - Agent orchestration
  - `src/connectors/loader.js` - Connector loader
  - `src/routes/` - REST API routes

- `mcp/connectors/` - JavaScript connectors
  - `amazon-q/`
  - `cline/`
  - `dropbox/`
  - `filesystem/`
  - `github-copilot/`
  - `google-drive/`

- `mcp/servers/` - Standalone JS MCP servers
  - `agents/` - Agent-specific servers
  - `sources/` - Data source servers

### Files
- `mcp/package.json` - Node.js dependencies (root level, for test scripts only now)
- `mcp/migrate-servers.js` - Migration utility (no longer needed)
- `mcp/test-architecture.js` - Old architecture tests

## Migration Mapping

| Old JS Component | New Rust Component | Notes |
|------------------|-------------------|-------|
| `McpService.js` | `mcp/src/protocol/server.rs` | Now implements standard MCP |
| `AgentManager.js` | `client/src/agents/*` | Moved to client service |
| `connectors/*.js` | `mcp/src/connectors/*.rs` | All in Rust |
| `ConnectionManager.js` | Removed | Not needed; agents connect directly |
| REST API (`/api/mcp/*`) | Stdio MCP | Standard protocol, no REST |
| WebSocket | Removed | MCP uses stdio |

## For Developers

### If You See References to Old JS Code

1. **Do NOT use** `mcp/service/` - it's archived
2. **Do NOT add** JS connectors - use Rust connectors in `mcp/src/connectors/`
3. **Use** the Rust `mcp-service` binary for all MCP needs

### Running the New Service

```bash
cd mcp
cargo run --bin mcp-service
```

See `mcp/README.md` for full documentation.

### If You Need Agent Integration

Agent-specific logic (GitHub Copilot, Cline, Cursor, AmazonQ, OpenAI) belongs in:
- `client/src/agents/` - Agent connectors
- `client/src/services/` - AI service orchestration

The MCP server is **data-agnostic** - it just exposes ConHub context via MCP.

## Timeline

- **Nov 2025** - Initial JS MCP service created
- **Nov 20, 2025** - Migrated to Rust, JS code archived
- **Future** - JS code may be deleted after 1-2 release cycles

## Questions?

See:
- `mcp/README.md` - Current Rust MCP server docs
- `client/README.md` - AI service docs (where agent logic lives)
- Architecture docs in `docs/architecture/microservices.md`
