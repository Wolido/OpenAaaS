# OpenAaaS MCP Adapter

<p align="right">中文 | <a href="./README.en.md">English</a></p>

OpenAaaS 的 MCP（Model Context Protocol）适配器，让 Claude Desktop、Cursor、Cline 等支持 MCP 的 AI 客户端能够连接 OpenAaaS Server，发现远程服务、提交任务并获取结果。

基于 [MCP Python SDK](https://github.com/modelcontextprotocol/python-sdk) 构建，提供 14 个核心 Tool，覆盖完整的客户端能力：服务发现、注册认证、任务提交、结果下载及多服务器管理。

---

## 快速开始

### 推荐：uvx（零安装，无需下载到本地）

安装 [uv](https://docs.astral.sh/uv/) 后，直接运行，无需把包下载到本地：

```bash
uvx openaaas-mcp-adapter
```

`uv` 会自动从 PyPI 拉取并创建临时虚拟环境运行，用完即走，不会在系统中留下任何文件。

这是**最推荐的方式**，适合所有用户。

---

### Claude Desktop、Cursor 或 Cline 配置

配置 Claude Desktop、Cursor 或 Cline：

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

- `toolTimeoutMs` 设置 MCP 工具调用的最大等待时间（毫秒），适合长任务轮询场景。
- ⚠️ 该参数由 MCP 客户端解析，实际生效情况取决于具体客户端实现；某些客户端或 Agent 工具本身可能仍有独立的超时限制，导致该参数不生效。

配置修改后，重启客户端即可生效。

---

### Codex 配置

通过 Codex CLI 添加适配器：

```bash
codex mcp add openaaas -- uvx openaaas-mcp-adapter
```

使用以下命令确认配置：

```bash
codex mcp get openaaas
codex mcp list
```

Codex 将 MCP 配置保存在全局 `~/.codex/config.toml` 中；也可以在受信任项目中使用项目级 `.codex/config.toml`。如需支持长时间运行的 `poll_task`，配置如下：

```toml
[mcp_servers.openaaas]
command = "uvx"
args = ["openaaas-mcp-adapter"]
startup_timeout_sec = 30
tool_timeout_sec = 600
```

- `tool_timeout_sec` 是 Codex 等待单次 MCP Tool 调用的最长时间，单位为秒。
- 调用 `poll_task` 时，建议将 `timeout_seconds` 设置为小于 `tool_timeout_sec`，并为网络请求预留时间。例如：`timeout_seconds=540` 搭配 `tool_timeout_sec=600`。
- 避免在 Codex 中进行不设上限的轮询，否则 MCP 客户端仍可能在自身工具超时后终止调用。

配置后重启 Codex App、CLI 会话或 IDE 扩展。在 Codex TUI 中可使用 `/mcp` 查看连接状态。

---

### 其他安装方式（可选）

<details>
<summary>点击查看其他安装方式</summary>

#### pipx（全局安装）

```bash
pipx install openaaas-mcp-adapter
```

MCP 配置改为：
```json
{
  "mcpServers": {
    "openaaas": {
      "command": "openaaas-mcp-adapter"
    }
  }
}
```

#### pip 安装

```bash
pip install openaaas-mcp-adapter
```

#### 本地源码运行（开发）

```bash
cd openaaas-mcp-adapter
uv sync
uv run openaaas-mcp-adapter
```

</details>

---

## 配置文件

配置文件保存在用户主目录下：

```
~/.openaaas-mcp-adapter/config.json
```

首次运行时若不存在，会自动创建默认配置：

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

注册成功后的配置示例：

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

运行 `register` 成功后会自动写入 `api_key`、`client_id` 和 `name`。

---

## 多服务器配置

支持同时配置多个服务器，通过 `server` 参数指定目标服务器。

### 为多个服务器分别注册

每个服务器别名有独立的注册信息：

```
register(name: "my-prod-client", server: "prod")
register(name: "my-local-client", server: "local")
```

两个服务器会分别保存各自的 api_key，互不干扰。

### 设置服务器地址

```
set_server_url(server_url: "https://api.open-aaas.com", server: "prod")
set_server_url(server_url: "http://127.0.0.1:8080", server: "local")
```

### 切换默认服务器

```
set_default_server(server: "prod")
```

### 列出所有配置的服务器

```
list_servers()
```

---

## 可用工具

| 工具名 | 功能描述 |
|--------|----------|
| `discover` | 发现服务端 API 信息（返回服务端版本、支持的 API 端点、可用服务列表、认证方式等。建议在首次连接新服务器时调用） |
| `set_server_url` | 设置服务器地址并保存到 config.json。已有注册信息（api_key）的服务器不会被覆盖 |
| `register` | 注册客户端，自动保存 api_key（仅需一次） |
| `update_profile` | 修改用户名 |
| `list_services` | 列出可用服务（返回轻量摘要：id/name/description/agent_status/access_type/has_permission/registration_status，不含 usage 长文本） |
| `get_service_usage` | 获取指定服务的详细 usage（能力范围、调用规范、返回格式、限制条件） |
| `submit_task` | 提交任务到远程 Agent（支持文件上传，支持 `session_id` 保持对话上下文） |
| `get_task` | 查询任务状态和最终结果（仅在用户明确要求时调用，不要主动轮询） |
| `poll_task` | 轮询任务直到获得最终结果。每 20 秒查询一次，默认无超时，可传入 `timeout_seconds` 限制最大轮询时长。Agent 禁止主动调用，仅在用户明确说"帮我等结果"/"轮询任务"时才使用 |
| `cancel_task` | 取消执行中的任务 |
| `list_files` | 列出任务的结果文件列表 |
| `download_result` | 下载任务结果文件并返回每个文件的完整路径（支持 file_id 单选或 download_all 全选）。未指定 file_id 且 download_all=false 时默认优先下载 .zip 文件，否则下载第一个文件。自动检测并解压 .zip 文件。读取文件时请使用返回结果中列出的完整路径，不要推测子目录 |
| `list_servers` | 列出所有已配置的服务器 |
| `set_default_server` | 切换默认服务器 |
| `remove_server` | 删除指定服务器的配置（不能删除默认服务器） |

### 参数说明

| 工具名 | 参数 | 必填 | 说明 |
|--------|------|------|------|
| `discover` | `server_url` | ✅ | 目标服务器地址 |
| `set_server_url` | `server_url` | ✅ | 服务器地址（以 http:// 或 https:// 开头） |
| | `server` | ❌ | 服务器别名，默认 `"default"` |
| `register` | `name` | ✅ | 客户端名称（长度 ≤64，不含 ASCII 控制字符、Unicode 控制字符及 `\/<>|&;$`） |
| | `server` | ❌ | 服务器别名，默认 `"default"` |
| `update_profile` | `name` | ✅ | 新用户名（同上约束） |
| | `server` | ❌ | 服务器别名，默认 `"default"` |
| `list_services` | `server` | ❌ | 服务器别名，默认 `"default"` |
| `get_service_usage` | `service_id` | ✅ | 目标服务 ID |
| | `server` | ❌ | 服务器别名，默认 `"default"` |
| `submit_task` | `service_id` | ✅ | 目标服务 ID |
| | `task_prompt` | ✅ | 任务描述 prompt |
| | `output_prompt` | ❌ | 输出格式要求，默认 `""` |
| | `input_files` | ❌ | 本地文件路径列表（最多 10 个，单文件 ≤100MB，仅限当前工作目录） |
| | `session_id` | ❌ | 会话 ID，用于保持对话上下文 |
| | `server` | ❌ | 服务器别名，默认 `"default"` |
| `get_task` | `task_id` | ✅ | 任务 ID |
| | `server` | ❌ | 服务器别名，默认 `"default"` |
| `poll_task` | `task_id` | ✅ | 任务 ID |
| | `timeout_seconds` | ❌ | 最大轮询时长（秒），默认不限制 |
| | `server` | ❌ | 服务器别名，默认 `"default"` |
| `cancel_task` | `task_id` | ✅ | 任务 ID |
| | `server` | ❌ | 服务器别名，默认 `"default"` |
| `list_files` | `task_id` | ✅ | 任务 ID |
| | `server` | ❌ | 服务器别名，默认 `"default"` |
| `download_result` | `task_id` | ✅ | 任务 ID |
| | `file_id` | ❌ | 指定文件 ID，默认 `""` |
| | `download_all` | ❌ | 是否下载所有文件，默认 `false` |
| | `server` | ❌ | 服务器别名，默认 `"default"` |
| `list_servers` | — | — | 无参数 |
| `set_default_server` | `server` | ✅ | 要设为默认的服务器别名 |
| `remove_server` | `server` | ✅ | 要删除的服务器别名 |

---

## 渐进式信息获取

本插件遵循"信息渐进式披露"原则：不要一次性获取所有服务的完整信息。

### 标准使用流程

1. `set_server_url` — 设置服务端地址（如未设置，默认连接 https://api.open-aaas.com）
2. `register` — 注册获取 api_key（仅需一次）
3. `list_services` — 获取轻量服务列表（id/name/description/agent_status/access_type/has_permission/registration_status），浏览并筛选候选服务
4. `get_service_usage` — 对筛选出的候选服务，按需获取详细 usage（能力范围、调用规范、返回格式、限制条件）
5. 根据 usage 内容，构造正确的 `task_prompt` 和 `output_prompt`
6. `submit_task` — 提交任务（可附带文件），保存返回的 `task_id`。提交后应询问用户是否需要等待结果，不要主动轮询
7. `get_task` — 仅在用户明确要求查询任务状态时调用（不要主动轮询）
8. `poll_task` — 仅在用户明确说"帮我等结果"/"轮询任务"时使用，每 20 秒查询一次，默认无超时
9. `download_result` — 任务完成后，用户要求下载结果文件时调用

### 为什么这样设计

- `list_services` 返回轻量摘要，不占用 LLM 上下文
- `usage` 通常包含大量文本（能力说明、调用规范、示例等），只应在确定使用该服务时获取
- 避免一次性加载所有服务的完整文档导致上下文溢出

---

## MCP 传输说明

- **MCP Transport**：`stdio`（标准输入输出），适配器作为子进程由 MCP 客户端启动，所有 tool 调用通过 JSON-RPC 在 stdio 上通信
- **文件传输**：文件上传和下载不走 MCP 协议，而是通过独立的 **HTTP（multipart/form-data）** 直连 OpenAaaS Server：
  - `submit_task` 中的 `input_files` 通过 HTTP multipart 上传
  - `download_result` 通过 HTTP GET 下载文件
- 这样设计避免了通过 MCP stdio 传输大体积二进制数据，保证性能和稳定性

---

## 注册约束

- 每个服务器别名可以独立注册。如果**当前服务器**已有 `api_key`，说明已完成注册，**请勿重复调用 `register`**
- 如需修改用户名，请使用 `update_profile`（示例：`update_profile(name: "new-name")`）
- **`name` 参数约束**：长度不超过 64 个字符，不含 ASCII 控制字符、Unicode 控制字符及 `\ / < > | & ; $`
- **服务器地址保护**：`set_server_url` 不会自动覆盖已有注册信息的服务器。如果该服务器已有 `api_key`，修改地址会被阻止，防止意外丢失注册信息
- 如需切换到新服务器地址，请使用新的 `server` 别名（多服务器配置），或先用 `remove_server` 删除旧配置
- **跨别名重复注册检查**：如果某个 `server_url` 已被其他 `server` 别名注册过（即该 URL 已有 api_key），则新别名无法再次注册或设置该地址，防止同一服务器被多个别名重复记录
- **删除服务器**：如需删除某个服务器配置，使用 `remove_server`。注意不能删除默认服务器，删除前需先用 `set_default_server` 切换默认服务器

---

## 使用示例

### 基础流程

1. **设置服务器地址**（可选，默认 https://api.open-aaas.com）：
   ```
   set_server_url(server_url: "https://api.open-aaas.com")
   ```

   可选：发现服务端信息：
   ```
   discover(server_url: "https://api.open-aaas.com")
   ```

2. **注册客户端**：
   ```
   register(name: "my-client")
   ```

3. **列出服务并筛选候选**：
   ```
   list_services()
   ```

4. **对目标服务获取详细 usage**：
   ```
   get_service_usage(service_id: "my-service")
   ```

5. **提交任务**：
   ```
   submit_task(
     service_id: "my-service",
     task_prompt: "分析数据并生成报告",
     output_prompt: "返回 Markdown 格式的分析报告"
   )
   ```

6. **查询任务**（仅在用户要求时调用）：
   ```
   get_task(task_id: "xxx")
   ```

7. **下载结果**：
   ```
   download_result(task_id: "xxx")
   ```

### 带文件上传的任务

```
submit_task(
  service_id: "data-analysis-agent",
  task_prompt: "分析附件中的销售数据，找出增长趋势",
  output_prompt: "返回 JSON 格式的分析结果，包含 trend 和 insights 字段",
  input_files: ["./sales_data.csv", "./notes.txt"],
  session_id: "可选，用于保持对话上下文"
)
```

文件上传限制：最多支持 10 个文件，单文件不超过 100MB；只能上传当前工作目录下的文件，不支持符号链接。

### 多服务器切换

```
list_services(server: "prod")
submit_task(server: "local", service_id: "my-service", task_prompt: "...")
```

### 删除服务器配置

```
remove_server(server: "old-server")
```

---

## 安全特性

- **路径遍历防护**：文件上传仅限当前工作目录下；zip 解压逐文件验证路径，防止 `../` 路径穿越
- **zip 炸弹防护**：最大压缩比 500、解压后总大小 100MB、最大文件数 1000、单文件最大 50MB
- **符号链接拒绝**：上传文件和 zip 中的符号链接均被直接拒绝
- **原子写入**：配置文件使用临时文件 + `replace` 保证写入过程不损坏
- **下载限制**：单文件下载不超过 100MB

---

## 开发

本地开发运行方式：

```bash
cd openaaas-mcp-adapter
uv sync
uv run openaaas-mcp-adapter
```

或使用 Python 模块方式：

```bash
uv run python -m openaaas_mcp_adapter
```

---

## 链接

- GitHub: [https://github.com/Wolido/OpenAaaS](https://github.com/Wolido/OpenAaaS)
- 官网: [https://www.open-aaas.com](https://www.open-aaas.com)

---

## 许可证

MIT License
