# OpenAaaS Agent Core

<p align="right">中文 | <a href="./README.en.md">English</a></p>

OpenAaaS 的 Agent 调度器，负责向 Server 注册、轮询获取任务，并通过 Docker 容器隔离执行任务。

## 编译安装

需要安装 Rust 工具链（1.85+）和 Docker：

```bash
cd agent-core
cargo build --release
```

编译产物为 `target/release/agent-core`。

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

## 首次使用

### 1. 初始化配置

```bash
./agent-core init
```

在当前目录生成默认 `config.toml`。

### 2. 编辑配置

打开 `config.toml`，修改 Server 地址：

```toml
[server]
base_url = "http://127.0.0.1:8080"  # 改成你的 Server 地址
```

### 3. 注册

从 Server 获取注册 token 后执行：

```bash
./agent-core register --token rt_xxx --name my-agent
```

注册成功后，`service_id` 和 `api_key` 会自动写入 `config.toml`。

### 4. 运行

前台运行：

```bash
./agent-core run
```

首次启动会交互式确认 Server URL 和数据目录（默认 `./data`），随后开始轮询任务。

如果当前目录已有完整配置且已注册，会直接启动，不再询问。

## 前台运行

```bash
./agent-core run
```

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

后台启动后会写入 pidfile，用于后续管理和状态查询。

## 查看状态

```bash
./agent-core status
```

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
```

## 停止服务

```bash
./agent-core stop
```

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
- **executor**: 任务执行器配置。`executor_type` 支持 `standard`（容器默认 ENTRYPOINT）、`bash`、`python`、`custom`；`capacity` 控制并发任务数。
- **paths**: `data_dir` 存放日志和运行时数据。`[[paths.mounts]]` 定义额外挂载到执行器容器的目录，常用于挂载配置文件或共享数据。

首次运行后无需手动修改大部分配置。若需调整，直接编辑 `config.toml` 后重启即可。
