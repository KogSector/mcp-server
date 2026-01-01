# MCP Server Documentation

## Overview

The MCP Server implements the **Model Context Protocol (MCP)** — the standard interface for AI agents to access ConFuse's knowledge infrastructure. It provides tools and resources that agents (Cursor, Claude, ChatGPT, etc.) can use to query organizational knowledge.

## Role in ConFuse

```
┌─────────────────────────────────────────────────────────────────────┐
│                          AI AGENTS                                   │
│       Cursor  │  Windsurf  │  Claude  │  ChatGPT  │  Custom         │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ MCP Protocol (JSON-RPC 2.0)
                                │ via client-connector or direct
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     MCP-SERVER (This Service)                        │
│                            Port: 3004                                │
│                                                                      │
│   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐              │
│   │  Protocol   │   │  Connector  │   │   Tools     │              │
│   │  Handler    │   │   Manager   │   │  & Resources│              │
│   │             │   │             │   │             │              │
│   │ • JSON-RPC  │   │ • GitHub    │   │ • Search    │              │
│   │ • Routing   │   │ • GitLab    │   │ • Get file  │              │
│   │ • Errors    │   │ • Local FS  │   │ • List repos│              │
│   └─────────────┘   │ • G Drive   │   │ • Graph     │              │
│                     │ • Notion    │   │   query     │              │
│                     └─────────────┘   └─────────────┘              │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ HTTP calls
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      KNOWLEDGE LAYER                                 │
│   relation-graph (Neo4j + Zilliz)  │  embeddings  │  chunker        │
└─────────────────────────────────────────────────────────────────────┘
```

## MCP Protocol

The Model Context Protocol uses **JSON-RPC 2.0** over various transports:
- **stdio** (default): Process spawned by agent, communication via stdin/stdout
- **HTTP+SSE**: For remote connections (via client-connector)
- **WebSocket**: For persistent connections (via client-connector)

### Protocol Messages

#### Initialize

```json
// Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "clientInfo": {
      "name": "Cursor",
      "version": "0.40.0"
    }
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "resources": { "subscribe": false, "listChanged": false },
      "tools": { "listChanged": false },
      "prompts": { "listChanged": false }
    },
    "serverInfo": {
      "name": "ConFuse MCP Server",
      "version": "1.0.0"
    }
  }
}
```

#### List Tools

```json
// Request
{ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }

// Response
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "tools": [
      {
        "name": "confuse.search",
        "description": "Hybrid search across all connected knowledge sources",
        "inputSchema": {
          "type": "object",
          "properties": {
            "query": { "type": "string", "description": "Search query" },
            "limit": { "type": "integer", "default": 10 }
          },
          "required": ["query"]
        }
      }
    ]
  }
}
```

#### Call Tool

```json
// Request
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "confuse.search",
    "arguments": {
      "query": "how does authentication work",
      "limit": 5
    }
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Found 5 results:\n\n1. src/auth/handler.py:45\ndef authenticate(user, password):\n    ..."
      }
    ],
    "isError": false
  }
}
```

## Available Tools

### Knowledge Tools

| Tool | Description |
|------|-------------|
| `confuse.search` | Hybrid search (vector + graph) |
| `confuse.search_code` | Code-specific search |
| `confuse.search_docs` | Documentation search |
| `confuse.get_entity` | Get entity with relationships |
| `confuse.graph_query` | Direct graph traversal |

### Source Tools

| Tool | Description |
|------|-------------|
| `github.list_repos` | List connected GitHub repos |
| `github.get_file` | Read file from GitHub |
| `github.search_code` | Search code in GitHub |
| `gitlab.list_projects` | List GitLab projects |
| `fs.list_files` | List local files |
| `fs.read_file` | Read local file |
| `gdrive.list_files` | List Google Drive files |
| `gdrive.read_file` | Read Google Drive document |

## Connector Architecture

Each connector implements a common trait:

```rust
#[async_trait]
pub trait Connector: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    
    fn list_tools(&self) -> Vec<Tool>;
    fn list_resources(&self) -> Vec<Resource>;
    
    async fn call_tool(&self, name: &str, args: Value) -> Result<Value>;
    async fn read_resource(&self, uri: &str) -> Result<ResourceContent>;
}
```

### Enabled Connectors

Controlled via environment variables:

```env
ENABLE_FS=true
ENABLE_GITHUB=true
ENABLE_GITLAB=false
ENABLE_BITBUCKET=false
ENABLE_GDRIVE=false
ENABLE_DROPBOX=false
ENABLE_NOTION=false
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MCP_PORT` | HTTP health check port | `3004` |
| `DATABASE_URL` | PostgreSQL connection | Required |
| `FS_ROOT_PATHS` | Allowed local paths (comma-sep) | - |
| `GITHUB_TOKEN` | GitHub API token | - |
| `GITLAB_TOKEN` | GitLab API token | - |
| `RELATION_GRAPH_URL` | Knowledge graph service | `http://localhost:3018` |
| `EMBEDDING_SERVICE_URL` | Embedding service | `http://localhost:3005` |

## Usage with AI Agents

### Cursor IDE

Add to `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "confuse": {
      "command": "path/to/mcp-service",
      "env": {
        "DATABASE_URL": "postgresql://...",
        "ENABLE_FS": "true",
        "ENABLE_GITHUB": "true",
        "FS_ROOT_PATHS": "/path/to/projects"
      }
    }
  }
}
```

### Via client-connector (Remote)

Connect to WebSocket endpoint:
```
wss://api.confuse.io/mcp/ws?key=YOUR_API_KEY
```

## Security

1. **Row-level security**: Users only access their connected sources
2. **Tool authorization**: Permissions checked per-tool
3. **Path restrictions**: Local FS limited to configured paths
4. **Audit logging**: All tool calls logged
5. **Token scoping**: API keys have limited scopes

## Related Services

| Service | Relationship |
|---------|--------------|
| client-connector | Proxies MCP requests for remote agents |
| relation-graph | Provides hybrid search capabilities |
| embeddings | Generates vectors for similarity search |
| data-connector | Sources connected via data-connector |
