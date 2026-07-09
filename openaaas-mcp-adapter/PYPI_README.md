# OpenAaaS MCP Adapter

Connect Claude Desktop, Cursor, Cline and any MCP-compatible AI client to the [OpenAaaS Server](https://www.open-aaas.com).

## Quick Install (No Install Required)

The recommended way to run the adapter — **no local installation needed**. Just make sure you have [uv](https://docs.astral.sh/uv/) installed:

```bash
uvx openaaas-mcp-adapter
```

`uv` will automatically create a temporary virtual environment, fetch the package from PyPI, and run it. Nothing is left on your system.

## Client Configuration

### Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or the equivalent path on your platform:

```json
{
  "mcpServers": {
    "openaaas": {
      "command": "uvx",
      "args": ["openaaas-mcp-adapter"]
    }
  }
}
```

### Cursor

In Cursor Settings → MCP, add a new server with the following configuration:

```json
{
  "mcpServers": {
    "openaaas": {
      "command": "uvx",
      "args": ["openaaas-mcp-adapter"]
    }
  }
}
```

## Standard Workflow

Once the adapter is connected, ask your AI agent to follow these steps:

1. **`set_server_url`** — Set the OpenAaaS server address (e.g. `https://api.open-aaas.com`)
2. **`register`** — Register a client name and obtain an API key (only needed once)
3. **`list_services`** — Browse available agent services
4. **`get_service_usage`** — Read the detailed usage guide for a chosen service
5. **`submit_task`** — Submit a task (supports file uploads)
6. **`get_task`** — Poll for task status and final results
7. **`list_files`** — List result files produced by the task
8. **`download_result`** — Download result files (ZIP archives are extracted automatically)

## Available Tools (14)

| Tool | Description |
|------|-------------|
| `discover` | Discover server API info and available endpoints |
| `set_server_url` | Set or update a server address |
| `register` | Register a client and get an API key |
| `update_profile` | Update the registered client name |
| `list_services` | List available agent services |
| `get_service_usage` | Get detailed usage instructions for a service |
| `submit_task` | Submit a task to an agent (supports file upload) |
| `get_task` | Check task status and retrieve results |
| `cancel_task` | Cancel a running task |
| `list_files` | List result files for a task |
| `download_result` | Download result files (auto-extracts ZIP) |
| `list_servers` | List all configured servers |
| `set_default_server` | Switch the default server alias |
| `remove_server` | Remove a server configuration |

## Alternative: pipx

If you prefer a permanent installation, use [pipx](https://pipx.pypa.io/):

```bash
pipx install openaaas-mcp-adapter
```

Then update your MCP config to use `"command": "openaaas-mcp-adapter"` (no `args`).

## Links

- **Homepage:** [https://www.open-aaas.com](https://www.open-aaas.com)
- **Repository:** [https://github.com/Wolido/OpenAaaS](https://github.com/Wolido/OpenAaaS)
- **Issues:** [https://github.com/Wolido/OpenAaaS/issues](https://github.com/Wolido/OpenAaaS/issues)

## License

MIT
