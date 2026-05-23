# OpenAaaS Server

<p align="right"><a href="./README.md">中文</a> | English</p>

The HTTP server for OpenAaaS, responsible for receiving client tasks, dispatching them to agents for execution, and managing the task lifecycle.

## Build & Install

Requires the Rust toolchain (1.85+):

```bash
cd server
cargo build --release
```

The compiled binary is located at `target/release/open-aaas-server`.

## Command Usage

```bash
open-aaas-server [OPTIONS] <COMMAND>
```

### Global Options

| Option | Description |
|--------|-------------|
| `--config <FILE>` | Specify the configuration file path; defaults to `config.toml` in the current directory |

### Subcommands

| Command | Description |
|---------|-------------|
| `run` | Run the server in the foreground |
| `run-detached` | Run the server in the background |
| `stop` | Stop the background server |
| `status` | Check the server status |

## First Launch

When running `run` for the first time, if no `config.toml` exists in the current directory, an interactive configuration will start:

```bash
$ ./open-aaas-server run
Data directory [./data]:                # Press Enter to confirm or enter a new path
Admin API Key []:                       # Press Enter to generate randomly, or enter a custom value
```

The server will automatically perform the following initialization steps:

1. Create the data directory (default: `./data`)
2. Set the SQLite database path: `{data_dir}/app.db`
3. Set the file storage path: `{data_dir}/files`
4. Generate a random `secret_key` (if not configured)
5. Create the `admin_api_key` (either the value you entered or a randomly generated one)
6. Write the final configuration to `config.toml`

The service will then start normally.

## Foreground Run

```bash
./open-aaas-server run
```

After starting, it listens on the default address `0.0.0.0:8080`. Press `Ctrl+C` or send `SIGTERM` for a graceful shutdown.

If `config.toml` already exists in the current directory, it will load the configuration and start directly without prompting.

## Background Run

```bash
./open-aaas-server run-detached
```

- Linux/macOS: Runs in the background via `nohup`, with logs output to `{data_dir}/server.log`
- Windows: Runs in the background via `cmd /C start /B`

After starting in the background, a pidfile will be written for subsequent management and status queries.

## Check Status

```bash
./open-aaas-server status
```

Example output:

```
Config file:    /path/to/config.toml
Data directory: /path/to/data
Listen address: 0.0.0.0:8080
Status:         Running (PID: 12345)
```

## Stop Service

```bash
./open-aaas-server stop
```

Sends `SIGTERM` to the background process, waiting up to 5 seconds for a graceful exit; if timed out, sends `SIGKILL` to force termination and cleans up the pidfile.

## Environment Variables

The server automatically loads the `.env` file in the current directory (if it exists) on startup.

The following environment variables can override the corresponding items in the configuration file, with the prefix `APP__` and levels separated by double underscores:

| Environment Variable | Corresponding Config Item |
|----------------------|---------------------------|
| `APP__SECRET_KEY` | `secret_key` |
| `APP__ADMIN_API_KEY` | `admin_api_key` |
| `APP__LOG_LEVEL` | `log_level` |
| `APP__SERVER__ADDR` | `server.addr` |
| `APP__DATABASE__URL` | `database.url` |
| `APP__AGENT__HEARTBEAT_TIMEOUT_SECS` | `agent.heartbeat_timeout_secs` |
| `APP__TASK__RESULT_RETENTION_DAYS` | `task.result_retention_days` |
| `APP__TASK__FILE_STORAGE_PATH` | `task.file_storage_path` |
| `APP__TASK__MAX_FILE_SIZE_MB` | `task.max_file_size_mb` |

Example:

```bash
APP__SERVER__ADDR="0.0.0.0:3000" APP__LOG_LEVEL=debug ./open-aaas-server run
```

The `ADMIN_API_KEY` environment variable (without prefix) can also directly override the admin API key, taking precedence over the configuration file.

## Configuration File

Complete `config.toml` example:

```toml
secret_key = "xxx"          # HMAC key, auto-generated on first launch
admin_api_key = "xxx"       # Admin API key
log_level = "info"          # trace / debug / info / warn / error

[server]
addr = "0.0.0.0:8080"       # Listen address
timeout_secs = 30           # HTTP request timeout (seconds)
max_body_size = 10485760    # Maximum request body size (10MB)

[database]
url = "sqlite:./data/app.db" # SQLite database path

[agent]
heartbeat_timeout_secs = 60  # Agent heartbeat timeout (seconds)

[task]
result_retention_days = 7    # Task result retention days
file_storage_path = "./data/files" # File storage path
max_file_size_mb = 50        # Single file size limit (MB)
```

No manual modification is needed after the first launch. If adjustments are required, simply edit `config.toml` and restart.

## Admin API Key

The Admin API Key is used to call admin interfaces (creating services, viewing all tasks, etc.). Ways to obtain it:

1. **Auto-generated on first launch**: The console will print `Admin API Key: ak_admin_xxx`, and it will also be written to `config.toml`
2. **Environment variable override**: Set `ADMIN_API_KEY` or `APP__ADMIN_API_KEY` before starting
3. **Check config file**: Read the `admin_api_key` field from `config.toml`

Please keep it safe and avoid leakage.
