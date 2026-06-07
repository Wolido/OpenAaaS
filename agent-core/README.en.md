# OpenAaaS Agent Core

<p align="right"><a href="./README.md">中文</a> | English</p>

The Agent scheduler for OpenAaaS, responsible for registering with the Server, polling for tasks, and executing tasks in isolated Docker containers.

## Prerequisites

| Platform | Requirements |
|----------|--------------|
| All platforms | Rust toolchain 1.85+, Docker |
| Windows | Docker Desktop (WSL2 backend recommended), Windows 10 19041+ or Windows 11 |

> Windows users unfamiliar with Docker Desktop should refer to the [Windows Deployment](#windows-deployment) section below.

## Build & Install

Requires Rust toolchain (1.85+) and Docker:

```bash
cd agent-core
cargo build --release
```

The compiled binary is located at `target/release/agent-core` (Windows: `target/release/agent-core.exe`).

## Executor Image

Agent Core executes tasks in isolated Docker containers, so a Docker image must be prepared in advance as the executor.

The interaction contract is simple: **Agent Core mounts `task.json` and input files into the container, and the container writes result files to the workspace after execution**. Agent Core does not care how the container is implemented internally, as long as this protocol is satisfied.

The `executor-example/` directory provides a **sample image** (based on node + python3, using pi-coding-agent as the execution logic) to demonstrate this interaction process. You can modify it directly or build your own image from scratch. See `executor-example/README.en.md` for details.

Build the sample image:

```bash
cd executor-example
docker build -t open-aaas-executor:latest .
```

> The image name must match `executor.image` in `config.toml` (default value is `open-aaas-executor:latest`).

### How It Works

1. Agent Core polls the Server for tasks.
2. Creates a local workspace directory for the task, writes `task.json`, and downloads input files to `input/`.
3. Starts a container via `docker run`, mounting the workspace to `/workspace` in the container.
4. The container reads `/workspace/task.json`, executes the task, and writes result files to the workspace.
5. After the container exits, Agent Core scans files in the workspace (excluding `task.json` and `input/`) and reports them as outputs to the Server.

## Command Usage

```bash
agent-core [OPTIONS] <COMMAND>
```

### Global Options

| Option | Description |
|--------|-------------|
| `--config <FILE>` | Specify the configuration file path; defaults to `config.toml` in the current directory |

### Subcommands

| Command | Description |
|---------|-------------|
| `init` | Generate a default `config.toml` in the current directory |
| `register --token <TOKEN> [--name <NAME>]` | Register with the Server to obtain `service_id` and `api_key` |
| `run [--interactive]` | Run the scheduler in the foreground. `--interactive` enters interactive registration if not yet registered |
| `run-detached` | Run the scheduler in the background |
| `stop` | Stop the background scheduler |
| `status` | Check the scheduler status |

## Windows Deployment

### Recommended: Run inside WSL2

Running inside WSL2 Ubuntu provides a native Linux experience:

1. Install [WSL2](https://docs.microsoft.com/en-us/windows/wsl/install) and launch an Ubuntu distribution.
2. In Docker Desktop, enable **Settings → Resources → WSL Integration → Enable integration with my default WSL distro**.
3. Inside the WSL2 terminal, clone the code and follow the build and run steps below (`cargo build --release`, build the image, etc.).

> **WSL2 path tip:** When editing `config.toml` inside a WSL2 terminal, the `host` path in `[[paths.mounts]]` must use Linux format (e.g. `/mnt/c/Users/xxx/share` or `/home/xxx/share`), not Windows format `C:\...`, or the mount will fail.

### Alternative: Native Windows + Docker Desktop

If WSL2 is not convenient, you can also run directly in PowerShell / CMD:

1. Install [Docker Desktop](https://www.docker.com/products/docker-desktop/).
2. In Docker Desktop settings, enable **Use the WSL 2 based engine** (better performance).
3. Build the executor image (use `./` as the path separator in PowerShell):
   ```powershell
   cd executor-example
   docker build -t open-aaas-executor:latest .
   ```
4. For subsequent commands, use `.\agent-core.exe` instead of `./agent-core`.

> On native Windows, the `host` path in `config.toml` can use `C:/path/to/dir` (recommended), `C:\\path\\to\\dir`, or `./relative/path`; Docker Desktop can mount all of them correctly.

---

## First-Time Setup

### 1. Initialize Configuration

```bash
./agent-core init
```

> **Windows (native):** `.\agent-core.exe init` (PowerShell recommended; in CMD you can run `agent-core.exe init` directly)

Generates a default `config.toml` in the current directory.

### 2. Edit Configuration

Open `config.toml` and modify the Server address:

```toml
[server]
base_url = "http://127.0.0.1:8080"  # Change to your Server address
```

> **Windows path tip:** On native Windows, the `host` path in `[[paths.mounts]]` can be `C:/path/to/dir` (recommended), `C:\\path\\to\\dir`, or `./relative/path`; Docker Desktop can mount all of them correctly.

### 3. Register

Obtain a registration token from the Server, then run:

```bash
./agent-core register --token rt_xxx --name my-agent
```

> **Windows (native):** `.\agent-core.exe register --token rt_xxx --name my-agent`

After successful registration, `service_id` and `api_key` will be automatically written to `config.toml`.

### 4. Run

Run in the foreground:

```bash
./agent-core run
```

> **Windows (native):** `.\agent-core.exe run`

On the first startup, it will interactively confirm the Server URL and data directory (default `./data`), then start polling for tasks.

If the current directory already has a complete configuration and is registered, it will start directly without prompting.

## Foreground Run

```bash
./agent-core run
```

> **Windows (native):** `.\agent-core.exe run`

After starting, it polls the Server for tasks, sends heartbeats, and runs tasks through the Docker executor. Press `Ctrl+C` or send `SIGTERM` for graceful shutdown.

If not yet registered and with `--interactive`, it will interactively ask for the token and complete registration:

```bash
./agent-core run --interactive
```

## Background Run

```bash
./agent-core run-detached
```

- Linux/macOS: Runs in the background via `nohup`, logs output to `{data_dir}/agent.log`
- Windows: Runs in the background via `cmd /C start /B`

> **Windows (native):** `.\agent-core.exe run-detached`

After background startup, a pidfile is written for subsequent management and status checks.

## Check Status

```bash
./agent-core status
```

> **Windows (native):** `.\agent-core.exe status`

Example output:

```
OpenAaaS Agent Status
====================
Config file: /path/to/config.toml
Data directory: /path/to/data

Server URL: http://127.0.0.1:8080
Poll interval: 5 seconds

Registration status: Registered
Service ID: svc_xxx
Agent name: my-agent

Executor configuration:
  Image: open-aaas-executor:latest
  Capacity: 2
  Timeout: 0 minutes
```

## Stop Service

```bash
./agent-core stop
```

> **Windows (native):** `.\agent-core.exe stop`

Sends `SIGTERM` to the background process, waiting up to 5 seconds for graceful exit; if timed out, sends `SIGKILL` to force termination and cleans up the pidfile.

## Configuration File Reference

Complete `config.toml` example:

```toml
[server]
base_url = "http://127.0.0.1:8080"  # Server address
poll_interval_secs = 5               # Polling interval (seconds)
use_system_proxy = false             # Whether to use system proxy

[agent]
service_id = "svc-xxx"               # Auto-filled after registration
api_key = "ak_xxx"                   # Auto-filled after registration
name = "agent-core"                  # Agent name

[executor]
executor_type = "standard"           # Executor type: standard / bash / python / custom
image = "open-aaas-executor:latest"  # Docker image
capacity = 2                         # Concurrent task count
timeout_minutes = 0                  # Task timeout (minutes), 0 means unlimited
# memory_limit = "4g"                # Memory limit (optional)
working_dir = "/workspace"           # Working directory inside the container
# script_path = "/workspace/run.sh"  # Script path (for bash/python type)
custom_entrypoint = ["/bin/sh"]      # Custom ENTRYPOINT (custom type)
custom_args = ["-c", "echo hi"]      # Custom arguments (custom type)

[paths]
data_dir = "./data"                  # Data directory

[[paths.mounts]]
host = "./share/kimi-config"         # Host path (relative or absolute)
container = "/shared/kimi-config"    # Container path
readonly = true                      # Read-only
```

### Configuration Items

- **server**: Configuration related to connecting to the Server; `base_url` is required.
- **agent**: `service_id` and `api_key` are auto-filled by the `register` command; no need to fill manually.
- **executor**: Task executor configuration. `executor_type` supports `standard` (container default ENTRYPOINT), `bash`, `python`, `custom`; `capacity` controls the number of concurrent tasks.
- **paths**: `data_dir` stores logs and runtime data. `[[paths.mounts]]` defines additional directories mounted into the executor container, commonly used for mounting configuration files or shared data.

After the first run, most configurations do not need to be modified manually. If adjustments are needed, simply edit `config.toml` and restart.
