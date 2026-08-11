# OpenAaaS Agent Core

<p align="right">中文 | <a href="./README.en.md">English</a></p>

OpenAaaS 的 Agent 调度器，负责向 Server 注册、轮询获取任务，并通过 Docker 容器隔离执行任务。

## 前置条件

| 平台 | 要求 |
|------|------|
| 所有平台 | Rust 工具链 1.85+、Docker |
| Windows | Docker Desktop（推荐 WSL2 后端）、Windows 10 19041+ 或 Windows 11 |
| GPU 挂载（可选） | NVIDIA 显卡与驱动；Linux 需安装 nvidia-container-toolkit，详见 [GPU 挂载](#gpu-挂载可选) |

> Windows 用户如不熟悉 Docker Desktop 配置，建议直接参考下方 [Windows 部署](#windows-部署) 小节。

## 编译安装

```bash
cd agent-core
cargo build --release
```

编译产物为 `target/release/agent-core`（Windows 上为 `target/release/agent-core.exe`）。

## 执行器镜像

Agent Core 通过 Docker 容器隔离执行任务，因此需要提前准备一个 Docker 镜像作为执行器。

交互契约很简单：**Agent Core 把 `task.json` 和输入文件挂进容器，容器执行完把结果文件写到 workspace**。Agent Core 不关心容器内部怎么实现，只要满足这个协议即可。

`executor-example/` 目录提供了一个**示例镜像**（基于 node + python3，恰好用 pi-coding-agent 作为执行逻辑），用来演示这个交互过程。你可以直接基于它修改，也可以完全从零构建自己的镜像。详见 `executor-example/README.md`。

构建示例镜像：

```bash
cd executor-example
docker build -t open-aaas-executor:latest .
```

> 镜像名需要与 `config.toml` 中的 `executor.image` 保持一致（默认值为 `open-aaas-executor:latest`）。

> 如需 GPU 挂载，镜像需自带 CUDA 运行库（如基于 `nvidia/cuda` 镜像构建）；agent-core 只负责挂载 GPU 设备，不提供 CUDA 环境。详见 [GPU 挂载](#gpu-挂载可选)。

### 工作原理

1. Agent Core 从 Server 轮询获取任务。
2. 在本地为任务创建 workspace 目录，写入 `task.json`，下载输入文件到 `input/`。
3. 通过 `docker run` 启动容器，挂载 workspace 到容器的 `/workspace`。
4. 容器读取 `/workspace/task.json`，执行任务，将结果文件写入 workspace。
5. 容器退出后，Agent Core 扫描 workspace 下文件（排除 `task.json` 和 `input/`），作为输出上报 Server。

## 命令用法

```bash
agent-core [OPTIONS] <COMMAND>
```

### 全局选项

| 选项 | 说明 |
|------|------|
| `--config <FILE>` | 指定配置文件路径，默认读取当前目录的 `config.toml` |

### 子命令

| 命令 | 说明 |
|------|------|
| `init` | 在当前目录生成默认 `config.toml` |
| `register --token <TOKEN> [--name <NAME>]` | 向 Server 注册，获取 service_id 和 api_key |
| `run [--interactive]` | 前台运行调度器。`--interactive` 表示未注册时进入交互式注册 |
| `run-detached` | 后台运行调度器 |
| `stop` | 停止后台调度器 |
| `status` | 查看调度器状态 |

## Windows 部署

### 推荐方式：WSL2 内运行（最顺畅）

在 WSL2 Ubuntu 中操作可获得与 Linux 完全一致的原生体验：

1. 安装 [WSL2](https://docs.microsoft.com/zh-cn/windows/wsl/install) 并启动 Ubuntu 发行版。
2. 在 Docker Desktop 中启用 **Settings → Resources → WSL Integration → Enable integration with my default WSL distro**。
3. 在 WSL2 终端内克隆代码，按本文后续 Linux 步骤执行即可（`cargo build --release`、构建镜像等）。
>
> **WSL2 路径提示：** 在 WSL2 终端内编辑 `config.toml` 时，`[[paths.mounts]]` 中的 `host` 路径必须使用 Linux 格式（如 `/mnt/c/Users/xxx/share` 或 WSL 内部路径 `/home/xxx/share`），不能使用 Windows 格式 `C:\...`，否则挂载会失败。

### 备选方式：原生 Windows + Docker Desktop

如不方便使用 WSL2，也可直接在 PowerShell / CMD 中运行：

1. 安装 [Docker Desktop](https://www.docker.com/products/docker-desktop/)。
2. 在 Docker Desktop 设置中启用 **Use the WSL 2 based engine**（性能更好）。
3. 构建 executor 镜像（PowerShell 中路径分隔符用 `./` 即可）：
   ```powershell
   cd executor-example
   docker build -t open-aaas-executor:latest .
   ```
4. 后续命令使用 `.\agent-core.exe` 替代 `./agent-core`。

> 原生 Windows 下，`config.toml` 中的 `host` 路径可使用 `C:/path/to/dir`（推荐）或 `C:\\path\\to\\dir` 或 `./relative/path` 格式，Docker Desktop 均可正确解析。

---

## 首次使用

### 1. 初始化配置

```bash
./agent-core init
```

> **Windows（原生）：** 直接运行 `.\agent-core.exe init`（PowerShell 推荐，CMD 可直接用 `agent-core.exe init`）。

在当前目录生成默认 `config.toml`。

### 2. 编辑配置

打开 `config.toml`，修改 Server 地址：

```toml
[server]
base_url = "http://127.0.0.1:8080"  # 改成你的 Server 地址
```

> **Windows 路径提示：** `[[paths.mounts]]` 中的 `host` 路径在 Windows 上可以是 `C:/path/to/dir`（推荐）或 `C:\\path\\to\\dir` 或 `./relative/path` 格式，Docker Desktop 都能正确挂载。

### 3. 注册

从 Server 获取注册 token 后执行：

```bash
./agent-core register --token rt_xxx --name my-agent
```

> **Windows（原生）：** `.\agent-core.exe register --token rt_xxx --name my-agent`

注册成功后，`service_id` 和 `api_key` 会自动写入 `config.toml`。

### 4. 运行

前台运行：

```bash
./agent-core run
```

> **Windows（原生）：** `.\agent-core.exe run`

首次启动会交互式确认 Server URL 和数据目录（默认 `./data`），随后开始轮询任务。

如果当前目录已有完整配置且已注册，会直接启动，不再询问。

## 前台运行

```bash
./agent-core run
```

> **Windows（原生）：** 使用 `.\agent-core.exe run`。

启动后向 Server 轮询获取任务、发送心跳，并通过 Docker 执行器运行任务。按 `Ctrl+C` 或发送 `SIGTERM` 可优雅关闭。

未注册且带有 `--interactive` 时，会交互式询问 token 并完成注册：

```bash
./agent-core run --interactive
```

## 后台运行

```bash
./agent-core run-detached
```

- Linux/macOS：通过 `nohup` 后台运行，日志输出到 `{data_dir}/agent.log`
- Windows：通过 `cmd /C start /B` 后台运行

> **Windows（原生）：** 使用 `.\agent-core.exe run-detached`。

后台启动后会写入 pidfile，用于后续管理和状态查询。

## 查看状态

```bash
./agent-core status
```

> **Windows（原生）：** 使用 `.\agent-core.exe status`。

输出示例：

```
OpenAaaS Agent 状态
====================
配置文件: /path/to/config.toml
数据目录: /path/to/data

Server URL: http://127.0.0.1:8080
轮询间隔: 5 秒

注册状态: 已注册
Service ID: svc_xxx
Agent 名称: my-agent

执行器配置:
  镜像: open-aaas-executor:latest
  容量: 2
  超时: 0 分钟
  宿主机访问: 已关闭
  GPU: 未开启
```

## 停止服务

```bash
./agent-core stop
```

> **Windows（原生）：** 使用 `.\agent-core.exe stop`。

向后台进程发送 `SIGTERM`，等待最多 5 秒优雅退出；超时则发送 `SIGKILL` 强制终止，并清理 pidfile。

## 配置文件说明

`config.toml` 完整示例：

```toml
[server]
base_url = "http://127.0.0.1:8080"  # Server 地址
poll_interval_secs = 5               # 轮询间隔（秒）
use_system_proxy = false             # 是否使用系统代理

[agent]
service_id = "svc-xxx"               # 注册后自动填充
api_key = "ak_xxx"                   # 注册后自动填充
name = "agent-core"                  # Agent 名称

[executor]
executor_type = "standard"           # 执行器类型：standard / bash / python / custom
image = "open-aaas-executor:latest"  # Docker 镜像
capacity = 2                         # 并发任务数
timeout_minutes = 0                  # 任务超时（分钟），0 表示不限制
# memory_limit = "4g"                # 内存限制（可选）
# enable_host_access = false         # 允许容器访问宿主机服务（需 Docker 20.10+，默认 false）
# gpu.vendor = "nvidia"              # GPU 厂商（v1 仅支持 nvidia；amd / intel 预留，默认关闭）
# gpu.devices = "all"                # 挂载的 GPU："all" 或 "0,1" 等索引列表
working_dir = "/workspace"           # 容器内工作目录
# script_path = "/workspace/run.sh"  # 脚本路径（bash/python 类型用）
custom_entrypoint = ["/bin/sh"]      # 自定义 ENTRYPOINT（custom 类型）
custom_args = ["-c", "echo hi"]      # 自定义参数（custom 类型）

[paths]
data_dir = "./data"                  # 数据目录

[[paths.mounts]]
host = "./share/kimi-config"         # 宿主机路径（相对或绝对）
container = "/shared/kimi-config"    # 容器内路径
readonly = true                      # 是否只读
```

### 配置项说明

- **server**: 连接 Server 的相关配置，`base_url` 必填。
- **agent**: `service_id` 和 `api_key` 由 `register` 命令自动填充，无需手动填写。
- **executor**: 任务执行器配置。`executor_type` 支持 `standard`（容器默认 ENTRYPOINT）、`bash`、`python`、`custom`；`capacity` 控制并发任务数；`enable_host_access`（默认 `false`）设为 `true` 后容器可通过 `host.docker.internal` 访问宿主机服务，需 Docker 20.10+。安全提示：开启后容器可访问宿主机上监听 `0.0.0.0` 的所有服务，仅建议管理员在配置文件中显式开启。提示：Docker Desktop（macOS / Windows）与 OrbStack 等桌面运行时已内置 `host.docker.internal` 解析，开启此功能冗余但无害；仅 Linux 原生 Docker Engine 需要开启。`gpu.vendor` / `gpu.devices` 配置 GPU 挂载（默认关闭，v1 仅支持 nvidia），详见 [GPU 挂载](#gpu-挂载可选)。
- **paths**: `data_dir` 存放日志和运行时数据。`[[paths.mounts]]` 定义额外挂载到执行器容器的目录，常用于挂载配置文件或共享数据。

首次运行后无需手动修改大部分配置。若需调整，直接编辑 `config.toml` 后重启即可。

## GPU 挂载（可选）

v1 支持将 NVIDIA GPU 挂载进任务容器（通过 `docker run --gpus` 参数），默认关闭。在 `config.toml` 的 `[executor]` 节配置：

```toml
[executor]
gpu.vendor = "nvidia"   # v1 仅支持 nvidia；amd / intel 为预留值，配置了也不会生成 GPU 参数
gpu.devices = "all"     # 挂载全部 GPU；也可按索引指定，如 "0,1"
```

开启后所有任务容器都会获得 GPU 访问权，`status` 命令会显示当前 GPU 配置。`gpu.devices` 仅支持缺省、`"all"` 或逗号分隔的数字索引（空白按 `"all"` 处理）；含空格、分号、字母或空段（如 `"0;;1"`）的非法值会在配置加载时报错，拒绝启动。

### 平台支持

| 平台 | 支持情况 |
|------|----------|
| Linux | 原生支持，需安装 nvidia-container-toolkit |
| Windows（WSL2） | 支持 NVIDIA GPU，在 Windows 侧安装 NVIDIA 驱动即可 |
| Windows（原生 Docker Desktop） | 不支持，配置了会启动报错，请改用 WSL2 后端 |
| macOS | 不支持，配置了会启动报错 |

启动时 agent-core 会执行 GPU 预检：macOS 与 Windows 原生配置 GPU 会直接报错退出；Linux / WSL2 下未检测到 nvidia runtime 或 `docker info` 执行失败时仅警告，不阻断启动。

### 安装 nvidia-container-toolkit（Linux）

Debian / Ubuntu（apt）：

```bash
curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | sudo gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg
curl -s -L https://nvidia.github.io/libnvidia-container/stable/deb/nvidia-container-toolkit.list | sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' | sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list
sudo apt-get update
sudo apt-get install -y nvidia-container-toolkit
```

RHEL / Fedora（dnf）：

```bash
curl -s -L https://nvidia.github.io/libnvidia-container/stable/rpm/nvidia-container-toolkit.repo | sudo tee /etc/yum.repos.d/nvidia-container-toolkit.repo
sudo dnf install -y nvidia-container-toolkit
```

安装后配置 Docker runtime 并重启 Docker：

```bash
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker
```

> WSL2 用户使用 Docker Desktop 时无需在 WSL 内安装 toolkit，在 Windows 侧安装 NVIDIA 驱动并启用 WSL2 后端即可；在 WSL2 内运行原生 Docker Engine 则按上述 Linux 步骤安装。

### 安全与调度提示

- GPU 挂载属于准特权硬件访问：开启后所有任务容器都能访问 GPU，任务间不做隔离，仅建议在可信任务场景由管理员显式开启。
- GPU 不参与 `capacity` 调度计数。GPU 节点应根据显存容量自行把 `capacity` 设为可承载的并发数，避免多任务争抢显存。
- 同一台机器需要混跑 CPU 与 GPU 任务时，建议运行两个 agent-core 实例：一个开启 GPU，一个关闭 GPU，分开接任务。
- 执行器镜像需自带 CUDA 运行库（如基于 `nvidia/cuda` 镜像构建）；agent-core 只负责挂载 GPU 设备，不提供 CUDA 环境。
