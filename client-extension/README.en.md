## Client Extensions

<p align="right"><a href="./README.md">中文</a> | English</p>

`client-extension/` is the collection of client extensions for OpenAaaS, enabling different Agent clients (pi, etc.) to connect to the OpenAaaS network, discover remote services, submit tasks, and retrieve results.

Currently includes two extensions:

### pi-extension

A TypeScript extension for [pi](https://github.com/badlogic/pi-mono). Provides a unified `OpenAaaS` entry tool with different functionalities invoked via the `action` parameter. Supports multi-server configuration, automatic task monitoring (widget + toast notifications), Session persistence, and reconstruction reminders.

### openaaas-mcp-adapter

A Python adapter for MCP clients such as Claude Desktop, Cursor, and Cline. Built on the MCP SDK, with `stdio` Transport, providing 14 core Tools, supporting file upload/download, multi-server configuration, path traversal, and zip bomb protection.

---

## Quick Start

### pi-extension

```bash
mkdir -p ~/.pi/agent/extensions/OpenAaaS
cp -r pi-extension/* ~/.pi/agent/extensions/OpenAaaS/
cd ~/.pi/agent/extensions/OpenAaaS
npm install
```

Execute `/reload` in pi to load the extension. A default configuration file will be created automatically on first use. Then you can invoke via conversation:

```
OpenAaaS(action: "set_server_url", server_url: "https://api.open-aaas.com")
OpenAaaS(action: "register", name: "my-client")
OpenAaaS(action: "list_services")
```

### openaaas-mcp-adapter

Run with zero installation using uvx (requires [uv](https://docs.astral.sh/uv/)):

```bash
uvx openaaas-mcp-adapter
```

Or add to Claude Desktop's `claude_desktop_config.json`:

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

After configuration, restart Claude Desktop, then invoke tools in conversation:

```
set_server_url(server_url: "https://api.open-aaas.com")
register(name: "my-client")
list_services()
```

---

## Standard Workflow

Regardless of which client extension you use, the standard interaction flow with OpenAaaS is consistent:

1. **Set up server** — Configure the target OpenAaaS server address
2. **Register** — Register the client with the server, obtain and save the `api_key` (only once per server)
3. **Browse services** — Use `list_services` to get lightweight summaries of available services and filter candidates
4. **Get usage** — Use `get_service_usage` to view the detailed capabilities, calling conventions, and return format of the target service
5. **Submit task** — Use `submit_task` to construct `task_prompt` and `output_prompt`, save the returned `task_id`
6. **Query result** — Only call `get_task` to query task status and results when the user explicitly requests it (do not poll actively)
7. **Download result** — Use `download_result` to retrieve task output files

> **Note**: The pi-extension additionally supports `list_history` for querying the current Session's task history, enabling context reconstruction after session interruption.

Follow the principle of progressive disclosure: first browse the lightweight service list to filter candidates, then get detailed usage on demand, to avoid loading complete documentation for all services at once, which could cause context overflow.
