# OpenAaaS MCP Adapter

<p align="right"><a href="./README.md">中文</a> | English</p>

The MCP (Model Context Protocol) adapter for OpenAaaS, enabling MCP-enabled AI clients such as Claude Desktop, Cursor, and Cline to connect to the OpenAaaS Server, discover remote services, submit tasks, and retrieve results.

Built on the [MCP Python SDK](https://github.com/modelcontextprotocol/python-sdk), providing 14 core Tools covering the complete client capabilities: service discovery, registration and authentication, task submission, result download, and multi-server management.

---

## Quick Start

### Recommended: uvx (Zero Install, No Local Download)

After installing [uv](https://docs.astral.sh/uv/), run directly without downloading the package locally:

```bash
uvx openaaas-mcp-adapter
```

`uv` will automatically pull from PyPI and create a temporary virtual environment to run, cleaning up afterward without leaving any files on the system.

This is the **most recommended method**, suitable for all users.

---

### Client Configuration

Configure Claude Desktop, Cursor, or Cline:

```json
{
  "mcpServers": {
    "openaaas": {
      "command": "uvx",
      "args": ["openaaas-mcp-adapter"],
      "toolTimeoutMs": 600000
    }
  }
}
```

- `toolTimeoutMs` sets the maximum wait time for an MCP tool call (milliseconds), suitable for long-running task polling.
- ⚠️ This parameter is parsed by the MCP client; actual behavior depends on the specific client implementation. Some clients or the Agent tool itself may have independent timeout limits, causing this parameter to not take effect.

After modifying the configuration, restart the client for it to take effect.

---

### Other Installation Methods (Optional)

<details>
<summary>Click to view other installation methods</summary>

#### pipx (Global Install)

```bash
pipx install openaaas-mcp-adapter
```

MCP configuration changes to:
```json
{
  "mcpServers": {
    "openaaas": {
      "command": "openaaas-mcp-adapter"
    }
  }
}
```

#### pip Install

```bash
pip install openaaas-mcp-adapter
```

#### Local Source Run (Development)

```bash
cd openaaas-mcp-adapter
uv sync
uv run openaaas-mcp-adapter
```

</details>

---

## Configuration File

The configuration file is saved in the user's home directory:

```
~/.openaaas-mcp-adapter/config.json
```

If it does not exist on first run, a default configuration will be created automatically:

```json
{
  "servers": {
    "default": {
      "server_url": "https://api.open-aaas.com",
      "api_key": "",
      "client_id": "",
      "name": ""
    }
  },
  "default_server": "default"
}
```

Example configuration after successful registration:

```json
{
  "servers": {
    "default": {
      "server_url": "https://api.open-aaas.com",
      "api_key": "ak_client_xxx",
      "client_id": "xxx",
      "name": "my-client"
    }
  },
  "default_server": "default"
}
```

After running `register`, `api_key`, `client_id`, and `name` will be written automatically.

---

## Multi-Server Configuration

Supports configuring multiple servers simultaneously. Use the `server` parameter to specify the target server.

### Register for Multiple Servers Separately

Each server alias has independent registration information:

```
register(name: "my-prod-client", server: "prod")
register(name: "my-local-client", server: "local")
```

The two servers will save their respective api_keys independently without interference.

### Set Server URL

```
set_server_url(server_url: "https://api.open-aaas.com", server: "prod")
set_server_url(server_url: "http://127.0.0.1:8080", server: "local")
```

### Switch Default Server

```
set_default_server(server: "prod")
```

### List All Configured Servers

```
list_servers()
```

---

## Available Tools

| Tool Name | Function Description |
|-----------|----------------------|
| `discover` | Discover server API information (returns server version, supported API endpoints, available service list, authentication method, etc. Recommended to call when connecting to a new server for the first time) |
| `set_server_url` | Set server URL and save to config.json. Servers with existing registration information (api_key) will not be overwritten |
| `register` | Register client, automatically save api_key (only once) |
| `update_profile` | Modify user name |
| `list_services` | List available services (returns lightweight summary: id/name/description/agent_status/access_type/has_permission/registration_status, without usage long text) |
| `get_service_usage` | Get detailed usage for the specified service (capabilities, calling conventions, return format, constraints) |
| `submit_task` | Submit task to remote Agent (supports file upload, supports `session_id` for conversation context) |
| `get_task` | Query task status and final result (only call when the user requests it, do not poll actively) |
| `poll_task` | Poll a task until a final result is obtained. Queries every 20 seconds, no timeout by default; pass `timeout_seconds` to limit maximum polling duration. Not recommended for Agent-initiated use; only use when the user explicitly asks to wait for or poll a task result |
| `cancel_task` | Cancel a running task |
| `list_files` | List result files for the task |
| `download_result` | Download task result files (supports single file via file_id or all files via download_all). When neither file_id is specified nor download_all is true, defaults to preferring .zip files, otherwise downloads the first file. Automatically detects and extracts .zip files |
| `list_servers` | List all configured servers |
| `set_default_server` | Switch default server |
| `remove_server` | Delete configuration for the specified server (cannot delete the default server) |

### Parameter Description

| Tool Name | Parameter | Required | Description |
|-----------|-----------|----------|-------------|
| `discover` | `server_url` | ✅ | Target server URL |
| `set_server_url` | `server_url` | ✅ | Server URL (must start with http:// or https://) |
| | `server` | ❌ | Server alias, default `"default"` |
| `register` | `name` | ✅ | Client name (length ≤64, no ASCII control characters, Unicode control characters, or `\/<>|&;$`) |
| | `server` | ❌ | Server alias, default `"default"` |
| `update_profile` | `name` | ✅ | New user name (same constraints) |
| | `server` | ❌ | Server alias, default `"default"` |
| `list_services` | `server` | ❌ | Server alias, default `"default"` |
| `get_service_usage` | `service_id` | ✅ | Target service ID |
| | `server` | ❌ | Server alias, default `"default"` |
| `submit_task` | `service_id` | ✅ | Target service ID |
| | `task_prompt` | ✅ | Task description prompt |
| | `output_prompt` | ❌ | Output format requirement, default `""` |
| | `input_files` | ❌ | Local file path list (up to 10 files, single file ≤100MB, current working directory only) |
| | `session_id` | ❌ | Session ID, for maintaining conversation context |
| | `server` | ❌ | Server alias, default `"default"` |
| `get_task` | `task_id` | ✅ | Task ID |
| | `server` | ❌ | Server alias, default `"default"` |
| `poll_task` | `task_id` | ✅ | Task ID |
| | `timeout_seconds` | ❌ | Maximum polling duration (seconds), no limit by default |
| | `server` | ❌ | Server alias, default `"default"` |
| `cancel_task` | `task_id` | ✅ | Task ID |
| | `server` | ❌ | Server alias, default `"default"` |
| `list_files` | `task_id` | ✅ | Task ID |
| | `server` | ❌ | Server alias, default `"default"` |
| `download_result` | `task_id` | ✅ | Task ID |
| | `file_id` | ❌ | Specify file ID, default `""` |
| | `download_all` | ❌ | Whether to download all files, default `false` |
| | `server` | ❌ | Server alias, default `"default"` |
| `list_servers` | — | — | No parameters |
| `set_default_server` | `server` | ✅ | Server alias to set as default |
| `remove_server` | `server` | ✅ | Server alias to delete |

---

## Progressive Information Retrieval

This adapter follows the principle of "progressive information disclosure": do not fetch complete information for all services at once.

### Standard Usage Flow

1. `set_server_url` — Set server address (if not set, defaults to https://api.open-aaas.com)
2. `register` — Register to get api_key (only once)
3. `list_services` — Get lightweight service list (id/name/description/agent_status/access_type/has_permission/registration_status), browse and filter candidate services
4. `get_service_usage` — For filtered candidate services, get detailed usage on demand (capabilities, calling conventions, return format, constraints)
5. Based on usage content, construct correct `task_prompt` and `output_prompt`
6. `submit_task` — Submit task (with optional files), save returned `task_id`
7. `get_task` — Only call when the user explicitly requests it, query task status and final result (do not poll actively)
8. `poll_task` — If you need to wait for task completion, poll until the task finishes (every 20 seconds, no timeout by default)
9. `download_result` — Download result files after task completion

### Why This Design

- `list_services` returns a lightweight summary, not occupying LLM context
- `usage` usually contains large amounts of text (capability descriptions, calling conventions, examples, etc.), and should only be retrieved when the service is determined to be used
- Avoid loading complete documentation for all services at once, which could cause context overflow

---

## MCP Transport Description

- **MCP Transport**: `stdio` (standard input/output). The adapter is launched as a child process by the MCP client; all tool calls communicate via JSON-RPC over stdio
- **File Transfer**: File upload and download do not go through the MCP protocol, but instead use independent **HTTP (multipart/form-data)** direct connection to the OpenAaaS Server:
  - `input_files` in `submit_task` are uploaded via HTTP multipart
  - `download_result` downloads files via HTTP GET
- This design avoids transferring large binary data through MCP stdio, ensuring performance and stability

---

## Registration Constraints

- Each server alias can be registered independently. If the **current server** already has an `api_key`, registration is complete, **do not call `register` repeatedly**
- To modify the user name, use `update_profile` (example: `update_profile(name: "new-name")`)
- **`name` parameter constraint**: Length does not exceed 64 characters, no ASCII control characters, Unicode control characters, or `\ / < > | & ; $`
- **Server URL protection**: `set_server_url` will not automatically overwrite servers with existing registration information. If the server already has an `api_key`, address modification will be blocked to prevent accidental loss of registration information
- To switch to a new server address, use a new `server` alias (multi-server configuration), or first delete the old configuration with `remove_server`
- **Cross-alias duplicate registration check**: If a `server_url` has already been registered under another `server` alias (i.e. the URL already has an api_key), the new alias cannot register or set that address again, preventing the same server from being recorded multiple times under different aliases
- **Delete server**: To delete a server configuration, use `remove_server`. Note that the default server cannot be deleted; switch the default server with `set_default_server` first

---

## Usage Examples

### Basic Flow

1. **Set server URL** (optional, default https://api.open-aaas.com):
   ```
   set_server_url(server_url: "https://api.open-aaas.com")
   ```

   Optional: Discover server information:
   ```
   discover(server_url: "https://api.open-aaas.com")
   ```

2. **Register client**:
   ```
   register(name: "my-client")
   ```

3. **List services and filter candidates**:
   ```
   list_services()
   ```

4. **Get detailed usage for the target service**:
   ```
   get_service_usage(service_id: "my-service")
   ```

5. **Submit task**:
   ```
   submit_task(
     service_id: "my-service",
     task_prompt: "Analyze data and generate report",
     output_prompt: "Return analysis report in Markdown format"
   )
   ```

6. **Query task** (only call when the user requests it):
   ```
   get_task(task_id: "xxx")
   ```

7. **Download result**:
   ```
   download_result(task_id: "xxx")
   ```

### Task with File Upload

```
submit_task(
  service_id: "data-analysis-agent",
  task_prompt: "Analyze sales data in the attachment and identify growth trends",
  output_prompt: "Return analysis results in JSON format, containing trend and insights fields",
  input_files: ["./sales_data.csv", "./notes.txt"],
  session_id: "Optional, for maintaining conversation context"
)
```

File upload limits: Up to 10 files supported, single file not exceeding 100MB; only files in the current working directory can be uploaded, symbolic links are not supported.

### Multi-Server Switching

```
list_services(server: "prod")
submit_task(server: "local", service_id: "my-service", task_prompt: "...")
```

### Delete Server Configuration

```
remove_server(server: "old-server")
```

---

## Security Features

- **Path traversal protection**: File uploads are limited to the current working directory; zip extraction validates paths file by file, preventing `../` path traversal
- **Zip bomb protection**: Maximum compression ratio 500, total size after extraction 100MB, maximum file count 1000, single file maximum 50MB
- **Symbolic link rejection**: Symbolic links in uploaded files and zip archives are directly rejected
- **Atomic write**: Configuration files use temporary file + `replace` to ensure the write process does not corrupt data
- **Download limit**: Single file download does not exceed 100MB

---

## Development

Local development run method:

```bash
cd openaaas-mcp-adapter
uv sync
uv run openaaas-mcp-adapter
```

Or using Python module mode:

```bash
uv run python -m openaaas_mcp_adapter
```

---

## Links

- GitHub: [https://github.com/Wolido/OpenAaaS](https://github.com/Wolido/OpenAaaS)
- Website: [https://www.open-aaas.com](https://www.open-aaas.com)

---

## License

MIT License
