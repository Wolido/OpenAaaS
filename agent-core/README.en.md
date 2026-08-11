# OpenAaaS Agent Core

<p align="right"><a href="./README.md">中文</a> | English</p>

The Agent scheduler for OpenAaaS, responsible for registering with the Server, polling for tasks, and executing tasks in isolated Docker containers.

## Prerequisites

| Platform | Requirements |
|----------|--------------|
| All platforms | Rust toolchain 1.85+, Docker |
| Windows | Docker Desktop (WSL2 backend recommended), Windows 10 19041+ or Windows 11 |
| GPU mounting (optional) | NVIDIA GPU and driver; Linux requires nvidia-container-toolkit. See [GPU Mounting](#gpu-mounting-optional) |

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

> For GPU mounting, the image must ship its own CUDA runtime libraries (e.g. build on `nvidia/cuda`); agent-core only mounts GPU devices and does not provide a CUDA environment. See [GPU Mounting](#gpu-mounting-optional).

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
  Host access: disabled
  GPU: disabled
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
# enable_host_access = false         # Allow container to access host services (requires Docker 20.10+, defaults to false)
# gpu.vendor = "nvidia"              # GPU vendor (v1 supports nvidia only; amd / intel reserved, disabled by default)
# gpu.devices = "all"                # GPUs to mount: "all" or an index list like "0,1"
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
- **executor**: Task executor configuration. `executor_type` supports `standard` (container default ENTRYPOINT), `bash`, `python`, `custom`; `capacity` controls the number of concurrent tasks. Set `enable_host_access` to `true` (default `false`) to inject `host.docker.internal` pointing to the host, allowing containers to reach host services listening on `0.0.0.0`. Requires Docker 20.10+. For security, only administrators should explicitly enable this in the configuration file. Note: Docker Desktop (macOS/Windows) and OrbStack provide built-in `host.docker.internal` DNS resolution, making this option redundant but harmless on those platforms. It is only required for native Linux Docker Engine. `gpu.vendor` / `gpu.devices` configure GPU mounting (disabled by default; v1 supports nvidia only). See [GPU Mounting](#gpu-mounting-optional).
- **paths**: `data_dir` stores logs and runtime data. `[[paths.mounts]]` defines additional directories mounted into the executor container, commonly used for mounting configuration files or shared data.

After the first run, most configurations do not need to be modified manually. If adjustments are needed, simply edit `config.toml` and restart.

## GPU Mounting (Optional)

v1 supports mounting NVIDIA GPUs into task containers (via the `docker run --gpus` flag); disabled by default. Configure it in the `[executor]` section of `config.toml`:

```toml
[executor]
gpu.vendor = "nvidia"   # v1 supports nvidia only; amd / intel are reserved and generate no GPU flags
gpu.devices = "all"     # Mount all GPUs; or specify indices, e.g. "0,1"
```

Once enabled, every task container gets GPU access. The `status` command shows the current GPU configuration. `gpu.devices` accepts only the default, `"all"`, or a comma-separated list of numeric indices (whitespace-only is treated as `"all"`); invalid values containing spaces, semicolons, letters, or empty segments (e.g. `"0;;1"`) fail configuration loading and abort startup.

### Platform Support

| Platform | Support |
|----------|---------|
| Linux | Natively supported; requires nvidia-container-toolkit |
| Windows (WSL2) | NVIDIA GPUs supported; install the NVIDIA driver on the Windows side |
| Windows (native Docker Desktop) | Not supported; startup fails if configured. Use the WSL2 backend instead |
| macOS | Not supported; startup fails if configured |

At startup, agent-core runs a GPU precheck: configuring GPU on macOS or native Windows aborts startup with an error; on Linux / WSL2, a missing nvidia runtime or a failed `docker info` only logs a warning and does not block startup.

### Installing nvidia-container-toolkit (Linux)

Debian / Ubuntu (apt):

```bash
curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | sudo gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg
curl -s -L https://nvidia.github.io/libnvidia-container/stable/deb/nvidia-container-toolkit.list | sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' | sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list
sudo apt-get update
sudo apt-get install -y nvidia-container-toolkit
```

RHEL / Fedora (dnf):

```bash
curl -s -L https://nvidia.github.io/libnvidia-container/stable/rpm/nvidia-container-toolkit.repo | sudo tee /etc/yum.repos.d/nvidia-container-toolkit.repo
sudo dnf install -y nvidia-container-toolkit
```

After installation, configure the Docker runtime and restart Docker:

```bash
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker
```

> WSL2 users running Docker Desktop do not need the toolkit inside WSL; install the NVIDIA driver on Windows and enable the WSL2 backend. If you run a native Docker Engine inside WSL2, follow the Linux steps above.

### Security & Scheduling Notes

- GPU mounting is quasi-privileged hardware access: once enabled, every task container can access the GPU, with no isolation between tasks. Enable it explicitly only for trusted workloads.
- GPUs are not counted in `capacity` scheduling. On GPU nodes, set `capacity` to the concurrency your VRAM can sustain, so tasks do not compete for GPU memory.
- To run CPU and GPU tasks on the same machine, run two agent-core instances: one with GPU enabled, one without, each taking its own kind of tasks.
- The executor image must ship its own CUDA runtime libraries (e.g. build on `nvidia/cuda`); agent-core only mounts GPU devices and does not provide a CUDA environment.
