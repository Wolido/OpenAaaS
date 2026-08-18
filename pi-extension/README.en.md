# OpenAaaS pi Extension

<p align="right"><a href="./README.md">中文</a> | English</p>

An OpenAaaS extension for [pi](https://github.com/earendil-works/pi), providing a unified `OpenAaaS` tool for service discovery, client registration, task submission, and result download.

## Installation

```bash
pi install npm:open-aaas-pi-extension
```

After installation, execute `/reload` in pi to load the extension automatically.

## Configuration

The configuration file is saved at a fixed path:

```
~/.pi/agent/openaaas/config.json
```

If it does not exist on first load, a default configuration will be created automatically:

```json
{
  "servers": {
    "default": {
      "server_url": "https://api.open-aaas.com"
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

## Multi-Server Configuration

Supports configuring multiple servers simultaneously. Use the `server` parameter to specify the target server.

### Register for Multiple Servers Separately

Each server alias has independent registration information:

```bash
# Register on the prod server
OpenAaaS(action: "register", server: "prod", name: "my-prod-client")

# Register on the local server
OpenAaaS(action: "register", server: "local", name: "my-local-client")
```

The two servers will save their respective api_keys independently without interference.

### Set Server URL

```
OpenAaaS(action: "set_server_url", server: "prod", server_url: "https://api.open-aaas.com")
OpenAaaS(action: "set_server_url", server: "local", server_url: "http://127.0.0.1:8080")
```

Switch default server:
```
OpenAaaS(action: "set_default_server", server: "prod")
```

List all configured servers:
```
OpenAaaS(action: "list_servers")
```

## Available Tools

| Tool Name | Function |
|-----------|----------|
| `OpenAaaS` | Unified entry tool, invoking different functionalities via the `action` parameter |

### Supported Actions

| Action | Function |
|--------|----------|
| `discover` | Discover server API information (returns server version, Base URL, authentication method, available endpoint list, registered service list) |
| `set_server_url` | Set server URL and save to config.json. Servers with existing registration information (api_key) will not be overwritten |
| `register` | Register client, automatically save api_key (only once) |
| `update_profile` | Modify user name |
| `list_services` | List available services (returns lightweight summary: id/name/description/agent_status/access_type/has_permission/registration_status, without usage long text) |
| `get_service_usage` | Get detailed usage for the specified service (capabilities, calling conventions, return format, constraints) |
| `list_history` | List all OpenAaaS task history in the current Session |
| `submit_task` | Submit task to remote Agent (supports file upload, supports `session_id` for conversation context) |
| `get_task` | Query task status and final result (only call when the user requests it, do not poll actively) |
| `cancel_task` | Cancel a running task |
| `list_files` | List result files for the task |
| `download_result` | Download task result files (supports single file via file_id or all files via download_all). When neither file_id is specified nor download_all is true, defaults to preferring .zip files, otherwise downloads the first file. Automatically detects and extracts .zip files. Single file download does not exceed 100MB, total size after extraction does not exceed 300MB (zip bomb protection) |
| `list_servers` | List all configured servers |
| `set_default_server` | Switch default server |
| `remove_server` | Delete configuration for the specified server (cannot delete the default server) |

## Progressive Information Retrieval

This extension follows the principle of "progressive information disclosure": do not fetch complete information for all services at once.

**Standard Usage Flow**:
1. `set_server_url` — Set server address (if not set, defaults to https://api.open-aaas.com)
2. `register` — Register to get api_key (only once)
3. `list_services` — Get lightweight service list (id/name/description/agent_status/access_type/has_permission/registration_status), browse and filter candidate services
4. `get_service_usage` — For filtered candidate services, get detailed usage on demand (capabilities, calling conventions, return format, constraints)
5. Based on usage content, construct correct task_prompt and output_prompt
6. `submit_task` — Submit task (with optional files), save returned task_id
7. `list_history` — View all task history in the current Session (can be used to restore memory after context compression)
8. `get_task` — Only call when the user explicitly requests it, query task status and final result (do not poll actively)
9. `download_result` — Download result files after task completion

**Why this design**:
- `list_services` returns a lightweight summary, not occupying LLM context
- `usage` usually contains large amounts of text (capability descriptions, calling conventions, examples, etc.), and should only be retrieved when the service is determined to be used
- Avoid loading complete documentation for all services at once, which could cause context overflow

## Automatic Monitoring

After calling `submit_task` to submit a task, the extension will automatically monitor task status in the background:

- **Widget real-time display**: Shows active task (pending / accepted / running / cancelling) statistics and task list below the editor, automatically refreshing on status changes
- **UI notifications**: Automatically push notifications when tasks complete, fail, or are cancelled
- **Auto-inject notification on task completion**: When a task reaches a terminal state (completed / failed / cancelled), the extension automatically injects an `[OpenAaaS-task-result]` notification into the conversation context, including status, service name, task summary (truncated), elapsed time, and an in-flight task snapshot. This notification is a system notification, not a user instruction; result retrieval stays the same: use `download_result` for result files and `get_task` for the task summary
- **No polling needed**: The LLM does not need to actively call `get_task` to poll status; after the task completes, the extension automatically injects an `[OpenAaaS-task-result]` notification, and results can be retrieved on demand upon receipt
- **Widget visibility constraint**: The widget's real-time task status is only visible to the user; you cannot see it directly. If you need to answer the user about any task's current status (e.g. "What is the task status now?" "Is it complete?"), you must call `get_task` to re-query the latest status, do not reference the old status returned by previous calls
- **Polling interval**: Tiered by task runtime (calculated from the server-side `created_at`) — polls once immediately after submission, then every 10 seconds for 0–2 minutes, every 20 seconds for 2–10 minutes, and every 30 seconds after 10 minutes
- **Rate limiting and backoff**: On 429 rate-limit responses, honors the server's `Retry-After` suggestion (clamped between 1 second and 5 minutes); network or server errors back off by consecutive failure count (60s → 120s → 240s → 300s cap), restoring the runtime-tiered interval after a success
- **Permanent errors stop polling**: Permanent errors such as authentication failure (401) or task not found (404) stop polling for that task and display the error in the `/OpenAaaS-tasks` panel; the stopped state is persisted and will not resume after session reconstruction
- **Session persistence**: Task status is saved to the current session, and monitoring is automatically reconstructed after switching the session tree
- **Session reconstruction reminder**: After the session is compressed and reconstructed, the extension will automatically send a message to the conversation, informing which tasks are still being monitored, to prevent LLM amnesia

## Commands

### `/OpenAaaS-tasks`

Pops up a panel showing tasks in the current session (including terminal tasks, up to 20), press `Escape` or `Ctrl+C` to close, automatically closes after 30 seconds of inactivity.

## Registration Constraints

- Each server alias can be registered independently. If the **current server** already has an `api_key`, registration is complete, **do not call `register` repeatedly**
- To modify the user name, use `update_profile` (example: `OpenAaaS(action: "update_profile", name: "new-name")`)
- **`name` parameter constraint**: Length does not exceed 64 characters, cannot contain control characters and the following illegal characters: `/ \ < > | & ; $`
- **Server URL protection**: `set_server_url` will not automatically overwrite servers with existing registration information. If the server already has an `api_key`, address modification will be blocked to prevent accidental loss of registration information
- To switch to a new server address, use a new `server` alias (multi-server configuration), or first delete the old configuration with `remove_server`
- **Delete server**: To delete a server configuration, use `remove_server`. Note that the default server cannot be deleted; switch the default server with `set_default_server` first

## Usage Examples

1. Set server URL (optional, default https://api.open-aaas.com):
   ```
   OpenAaaS(action: "set_server_url", server_url: "https://www.open-aaas.com")
   ```

   Optional: Discover server information:
   ```
   OpenAaaS(action: "discover")
   ```

1.5. Switch to another server for operations:
   ```
   OpenAaaS(action: "list_services", server: "prod")
   OpenAaaS(action: "submit_task", server: "local", service_id: "my-service", task_prompt: "...")
   ```

2. Register client:
   ```
   OpenAaaS(action: "register", name: "my-client")
   ```

3. List services and filter candidates:
   ```
   OpenAaaS(action: "list_services")
   ```

4. Get detailed usage for the target service:
   ```
   OpenAaaS(action: "get_service_usage", service_id: "my-service")
   ```

5. Submit task (with file upload):
   ```
   OpenAaaS(
     action: "submit_task",
     service_id: "my-service",
     task_prompt: "Analyze data",
     output_prompt: "Return JSON",
     input_files: ["./data.csv"],
     session_id: "Optional, for maintaining conversation context"
   )
   ```
   File upload limits: Up to 10 files supported, single file not exceeding 100MB; only files in the current working directory can be uploaded, symbolic links are not supported.

6. List current Session task history:
   ```
   OpenAaaS(action: "list_history")
   ```

7. Query task (only call when the user requests it):
   ```
   OpenAaaS(action: "get_task", task_id: "xxx")
   ```

8. Download result:
   ```
   OpenAaaS(action: "download_result", task_id: "xxx")
   ```

9. Delete server configuration:
   ```
   OpenAaaS(action: "remove_server", server: "old-server")
   ```
