# ConFuse MCP Server

MCP (Model Context Protocol) server for the ConFuse Knowledge Intelligence Platform. Enables AI agents to access organizational knowledge through a standardized protocol.

## Overview

This service implements the **MCP protocol** used by AI agents (Cursor, Claude, ChatGPT, etc.) to:
- Search across connected knowledge sources
- Access file contents from GitHub, GitLab, local FS
- Query the knowledge graph
- Get entity relationships

## Architecture

See [docs/README.md](docs/README.md) for complete protocol documentation.

## Quick Start

```bash
# Build
cargo build --release

# Run
./target/release/mcp-service

# Or with cargo
cargo run
```

## Configuration

```env
DATABASE_URL=postgresql://...
ENABLE_FS=true
ENABLE_GITHUB=true
FS_ROOT_PATHS=/path/to/projects
GITHUB_TOKEN=ghp_...
```

## MCP Tools

| Tool | Description |
|------|-------------|
| `confuse.search` | Hybrid search |
| `confuse.get_entity` | Get entity details |
| `github.get_file` | Read GitHub file |
| `fs.read_file` | Read local file |

## IDE Integration

### Cursor

```json
{
  "mcpServers": {
    "confuse": {
      "command": "./mcp-service",
      "env": { "ENABLE_FS": "true" }
    }
  }
}
```

## Documentation

See [docs/](docs/) folder for complete documentation.

## License

MIT - ConFuse Team
