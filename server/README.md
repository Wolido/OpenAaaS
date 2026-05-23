# OpenAaaS Server

<p align="right">中文 | <a href="./README.en.md">English</a></p>

OpenAaaS 的 HTTP 服务端，负责接收 Client 任务、调度给 Agent 执行，并管理任务生命周期。

## 编译安装

需要安装 Rust 工具链（1.85+）：

```bash
cd server
cargo build --release
```

编译产物为 `target/release/open-aaas-server`。

## 命令用法

```bash
open-aaas-server [OPTIONS] <COMMAND>
```

### 全局选项

| 选项 | 说明 |
|------|------|
| `--config <FILE>` | 指定配置文件路径，默认读取当前目录的 `config.toml` |

### 子命令

| 命令 | 说明 |
|------|------|
| `run` | 前台运行服务器 |
| `run-detached` | 后台运行服务器 |
| `stop` | 停止后台服务器 |
| `status` | 查看服务器状态 |

## 首次启动

首次执行 `run` 时，若当前目录没有 `config.toml`，会进入交互式配置：

```bash
$ ./open-aaas-server run
Data directory [./data]:                # 回车确认或输入新路径
Admin API Key []:                       # 回车随机生成，或输入自定义值
```

服务端会自动完成以下初始化：

1. 创建数据目录（默认 `./data`）
2. 写入 SQLite 数据库路径：`{data_dir}/app.db`
3. 写入文件存储路径：`{data_dir}/files`
4. 生成随机 `secret_key`（若未配置）
5. 创建 `admin_api_key`（你输入的值或随机生成）
6. 将最终配置写入 `config.toml`

随后正常启动服务。

## 前台运行

```bash
./open-aaas-server run
```

启动后监听默认地址 `0.0.0.0:8080`。按 `Ctrl+C` 或发送 `SIGTERM` 可优雅关闭。

如果当前目录已有 `config.toml`，会直接加载配置启动，不再询问。

## 后台运行

```bash
./open-aaas-server run-detached
```

- Linux/macOS：通过 `nohup` 后台运行，日志输出到 `{data_dir}/server.log`
- Windows：通过 `cmd /C start /B` 后台运行

后台启动后会写入 pidfile，用于后续管理和状态查询。

## 查看状态

```bash
./open-aaas-server status
```

输出示例：

```
配置文件:    /path/to/config.toml
数据目录:    /path/to/data
监听地址:    0.0.0.0:8080
运行状态:    运行中 (PID: 12345)
```

## 停止服务

```bash
./open-aaas-server stop
```

向后台进程发送 `SIGTERM`，等待最多 5 秒优雅退出；超时则发送 `SIGKILL` 强制终止，并清理 pidfile。

## 环境变量

服务端启动时会自动加载当前目录的 `.env` 文件（如果存在）。

以下环境变量可覆盖配置文件中的对应项，前缀为 `APP__`，层级用双下划线分隔：

| 环境变量 | 对应配置项 |
|----------|-----------|
| `APP__SECRET_KEY` | `secret_key` |
| `APP__ADMIN_API_KEY` | `admin_api_key` |
| `APP__LOG_LEVEL` | `log_level` |
| `APP__SERVER__ADDR` | `server.addr` |
| `APP__DATABASE__URL` | `database.url` |
| `APP__AGENT__HEARTBEAT_TIMEOUT_SECS` | `agent.heartbeat_timeout_secs` |
| `APP__TASK__RESULT_RETENTION_DAYS` | `task.result_retention_days` |
| `APP__TASK__FILE_STORAGE_PATH` | `task.file_storage_path` |
| `APP__TASK__MAX_FILE_SIZE_MB` | `task.max_file_size_mb` |

示例：

```bash
APP__SERVER__ADDR="0.0.0.0:3000" APP__LOG_LEVEL=debug ./open-aaas-server run
```

`ADMIN_API_KEY` 环境变量（无前缀）也可以直接覆盖管理员 API Key，优先级高于配置文件。

## 配置文件说明

`config.toml` 完整示例：

```toml
secret_key = "xxx"          # HMAC 密钥，首次启动自动生成
admin_api_key = "xxx"       # 管理员 API Key
log_level = "info"          # trace / debug / info / warn / error

[server]
addr = "0.0.0.0:8080"       # 监听地址
timeout_secs = 30           # HTTP 请求超时（秒）
max_body_size = 10485760    # 最大请求体大小（10MB）

[database]
url = "sqlite:./data/app.db" # SQLite 数据库路径

[agent]
heartbeat_timeout_secs = 60  # Agent 心跳超时（秒）

[task]
result_retention_days = 7    # 任务结果保留天数
file_storage_path = "./data/files" # 文件存储路径
max_file_size_mb = 50        # 单文件大小限制（MB）
```

首次启动后无需手动修改。若需调整，直接编辑 `config.toml` 后重启即可。

## 管理员 API Key

Admin API Key 用于调用管理员接口（创建服务、查看全量任务等）。获取方式：

1. **首次启动自动生成**：控制台会打印 `Admin API Key: ak_admin_xxx`，同时写入 `config.toml`
2. **环境变量覆盖**：启动前设置 `ADMIN_API_KEY` 或 `APP__ADMIN_API_KEY`
3. **查看配置文件**：从 `config.toml` 的 `admin_api_key` 字段读取

请务必妥善保存，避免泄露。
