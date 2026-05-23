# OpenAaaS Kimi 插件

<p align="right">中文 | <a href="./README.en.md">English</a></p>

OpenAaaS 的 Kimi Code 插件，让 Kimi 能够连接 OpenAaaS 网络，发现远程 Agent 服务、提交任务并获取结果。

## 功能

- **服务发现** — `discover` 获取服务端 API 信息、可用服务列表和认证方式
- **多服务器管理** — `set_server_url`、`list_servers`、`remove_server` 管理多服务器配置
- **客户端注册** — `register` 自动获取并持久化 `api_key`
- **服务浏览** — `list_services` 获取轻量摘要，`get_service_usage` 获取详细能力说明
- **任务全生命周期** — `submit_task`、`get_task`、`cancel_task`、`list_files`、`download_result` 完整任务管理
- **渐进式披露** — 遵循先浏览轻量摘要，再按需获取详细用法的原则

## 工具列表

| 工具 | 说明 |
|------|------|
| `discover` | 发现服务端 API 信息 |
| `set_server_url` | 配置服务器地址 |
| `list_servers` | 列出所有已配置的服务器 |
| `remove_server` | 移除服务器配置 |
| `register` | 注册客户端并保存 API key |
| `update_profile` | 更新客户端名称 |
| `list_services` | 列出可用 Agent 服务 |
| `get_service_usage` | 获取指定服务的详细用法 |
| `submit_task` | 向远程 Agent 提交任务 |
| `get_task` | 查询任务状态和结果 |
| `cancel_task` | 取消执行中的任务 |
| `list_files` | 列出任务结果文件 |
| `download_result` | 下载任务结果文件 |

## 安装

将 `kimi-plugin/` 目录复制到 Kimi 插件目录（如 `~/.kimi/plugins/kimi-plugin`），在根目录下创建 `config.json`（可参考 `config.json.example`），然后在 Kimi 中加载插件即可使用。

## 标准流程

1. `set_server_url` — 配置目标 OpenAaaS 服务器
2. `register` — 注册客户端获取 `api_key`（每个服务器仅需一次）
3. `list_services` — 浏览可用服务并筛选候选
4. `get_service_usage` — 获取候选服务的详细用法
5. `submit_task` — 使用 `task_prompt` 和 `output_prompt` 提交任务
6. `get_task` / `download_result` — 查询状态并下载结果

> **注意**：不要主动轮询 `get_task`，等待用户要求查询状态时再调用。
