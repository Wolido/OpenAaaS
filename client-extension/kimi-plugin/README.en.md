# OpenAaaS Kimi Plugin

<p align="right"><a href="./README.md">中文</a> | English</p>

A Kimi Code plugin for OpenAaaS, enabling Kimi to connect to the OpenAaaS network, discover remote Agent services, submit tasks, and retrieve results.

## Features

- **Service Discovery** — `discover` to get server API info, available services, and authentication methods
- **Multi-Server Management** — `set_server_url`, `list_servers`, and `remove_server` for managing multiple server configurations
- **Client Registration** — `register` to obtain and persist `api_key` automatically
- **Service Browsing** — `list_services` for lightweight summaries, `get_service_usage` for detailed capabilities
- **Task Lifecycle** — `submit_task`, `get_task`, `cancel_task`, `list_files`, and `download_result` for full task management
- **Progressive Disclosure** — Follows the principle of browsing lightweight summaries first, then retrieving detailed usage on demand

## Tools

| Tool | Description |
|------|-------------|
| `discover` | Discover server API information |
| `set_server_url` | Configure server URL |
| `list_servers` | List all configured servers |
| `remove_server` | Remove a server configuration |
| `register` | Register client and save API key |
| `update_profile` | Update client profile name |
| `list_services` | List available Agent services |
| `get_service_usage` | Get detailed usage for a service |
| `submit_task` | Submit a task to a remote Agent |
| `get_task` | Query task status and results |
| `cancel_task` | Cancel a running task |
| `list_files` | List result files for a task |
| `download_result` | Download task result files |

## Installation

Copy the `kimi-plugin/` directory to the Kimi plugin directory (e.g. `~/.kimi/plugins/kimi-plugin`), create a `config.json` in the root (refer to `config.json.example`), then load the plugin in Kimi.

## Standard Workflow

1. `set_server_url` — Configure the target OpenAaaS server
2. `register` — Register the client to obtain `api_key` (once per server)
3. `list_services` — Browse available services and filter candidates
4. `get_service_usage` — Get detailed usage for selected services
5. `submit_task` — Submit a task with `task_prompt` and `output_prompt`
6. `get_task` / `download_result` — Query status and download results

> **Note**: Do not poll `get_task` actively. Wait for the user to request status updates.
