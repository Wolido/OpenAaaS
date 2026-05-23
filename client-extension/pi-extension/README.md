# OpenAaaS pi Extension

<p align="right">中文 | <a href="./README.en.md">English</a></p>

为 [pi](https://github.com/badlogic/pi-mono) 开发的 OpenAaaS 扩展，提供统一的 `OpenAaaS` 工具，用于服务发现、客户端注册、任务提交及结果下载。

## 安装

将扩展目录复制到 pi 全局扩展目录，目录名即为扩展名：

```bash
mkdir -p ~/.pi/agent/extensions/OpenAaaS
cp -r /path/to/pi-extension/* ~/.pi/agent/extensions/OpenAaaS/
cd ~/.pi/agent/extensions/OpenAaaS
npm install
```

安装后，在 pi 中执行 `/reload` 即可自动加载扩展。

## 配置文件

配置文件保存在扩展目录下，路径为动态路径：

```
~/.pi/agent/extensions/<扩展目录名>/config.json
```

首次加载时若不存在，会自动创建默认配置：

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

## 多服务器配置

支持同时配置多个服务器，通过 `server` 参数指定目标服务器。

### 为多个服务器分别注册

每个服务器别名有独立的注册信息：

```bash
# 在 prod 服务器注册
OpenAaaS(action: "register", server: "prod", name: "my-prod-client")

# 在 local 服务器注册
OpenAaaS(action: "register", server: "local", name: "my-local-client")
```

两个服务器会分别保存各自的 api_key，互不干扰。

### 设置服务器地址

```
OpenAaaS(action: "set_server_url", server: "prod", server_url: "https://api.open-aaas.com")
OpenAaaS(action: "set_server_url", server: "local", server_url: "http://127.0.0.1:8080")
```

切换默认服务器：
```
OpenAaaS(action: "set_default_server", server: "prod")
```

列出所有配置的服务器：
```
OpenAaaS(action: "list_servers")
```

## 可用工具

| 工具名 | 功能 |
|--------|------|
| `OpenAaaS` | 统一入口工具，通过 `action` 参数调用不同功能 |

### 支持的 action

| action | 功能 |
|--------|------|
| `discover` | 发现服务端 API 信息（返回服务端版本、Base URL、认证方式、可用端点列表、已注册服务列表） |
| `set_server_url` | 设置服务器地址并保存到 config.json。已有注册信息（api_key）的服务器不会被覆盖 |
| `register` | 注册客户端，自动保存 api_key（仅需一次） |
| `update_profile` | 修改用户名 |
| `list_services` | 列出可用服务（返回轻量摘要：id/name/description/agent_status/access_type/has_permission/registration_status，不含 usage 长文本） |
| `get_service_usage` | 获取指定服务的详细 usage（能力范围、调用规范、返回格式、限制条件） |
| `list_history` | 列出当前 Session 中所有 OpenAaaS 任务历史 |
| `submit_task` | 提交任务到远程 Agent（支持文件上传，支持 `session_id` 保持对话上下文） |
| `get_task` | 查询任务状态和最终结果（仅在用户要求时调用，不要主动轮询） |
| `cancel_task` | 取消执行中的任务 |
| `list_files` | 列出任务的结果文件列表 |
| `download_result` | 下载任务结果文件（支持 file_id 单选或 download_all 全选），未指定 file_id 且 download_all=false 时默认优先下载 .zip 文件，否则下载第一个文件。自动检测并解压 .zip 文件，单文件下载不超过 100MB，解压后总大小不超过 300MB（Zip 炸弹防护） |
| `list_servers` | 列出所有已配置的服务器 |
| `set_default_server` | 切换默认服务器 |
| `remove_server` | 删除指定服务器的配置（不能删除默认服务器） |

## 渐进式信息获取

本插件遵循"信息渐进式披露"原则：不要一次性获取所有服务的完整信息。

**标准使用流程**：
1. `set_server_url` — 设置服务端地址（如未设置，默认连接 https://api.open-aaas.com）
2. `register` — 注册获取 api_key（仅需一次）
3. `list_services` — 获取轻量服务列表（id/name/description/agent_status/access_type/has_permission/registration_status），浏览并筛选候选服务
4. `get_service_usage` — 对筛选出的候选服务，按需获取详细 usage（能力范围、调用规范、返回格式、限制条件）
5. 根据 usage 内容，构造正确的 task_prompt 和 output_prompt
6. `submit_task` — 提交任务（可附带文件），保存返回的 task_id
7. `list_history` — 查看当前 Session 中所有任务历史（上下文压缩后可用来恢复记忆）
8. `get_task` — 仅在用户明确要求时调用，查询任务状态和最终结果（不要主动轮询）
9. `download_result` — 任务完成后下载结果文件

**为什么这样设计**：
- `list_services` 返回轻量摘要，不占用 LLM 上下文
- `usage` 通常包含大量文本（能力说明、调用规范、示例等），只应在确定使用该服务时获取
- 避免一次性加载所有服务的完整文档导致上下文溢出

## 自动监控

调用 `submit_task` 提交任务后，扩展会自动在后台监控任务状态：

- **Widget 实时显示**：在编辑器下方显示活跃任务（pending / accepted / running / cancelling）的统计信息和任务列表，状态变更时自动刷新
- **UI 通知**：任务完成、失败或取消时自动推送通知
- **无需轮询**：LLM 不需要主动调用 `get_task` 轮询状态，等待用户告知任务完成后再获取结果即可
- **Widget 可见性约束**：widget 实时显示的任务状态仅对用户可见，你无法直接看到。如果你需要回答用户关于某个任务当前状态的任何问题（例如"任务现在是什么状态""完成了吗"），必须先调用 `get_task` 重新查询最新状态，不要引用之前调用返回的旧状态
- **轮询间隔**：首次 10 秒，后续 30 秒
- **Session 持久化**：任务状态会保存到当前会话中，切换会话树后自动重建监控
- **Session 重建自动提醒**：当 session 被压缩重建后，扩展会自动发送消息到对话中，告知当前有哪些任务仍在监控，防止 LLM 失忆

## 命令

### `/OpenAaaS-tasks`

弹出面板显示当前会话中的任务（包括终态任务，最多 20 个），按 `Escape` 或 `Ctrl+C` 关闭，30 秒无操作自动关闭。

## 注册约束

- 每个服务器别名可以独立注册。如果**当前服务器**已有 `api_key`，说明已完成注册，**请勿重复调用 `register`**
- 如需修改用户名，请使用 `update_profile`（示例：`OpenAaaS(action: "update_profile", name: "new-name")`）
- **`name` 参数约束**：长度不超过 64 个字符，不能包含控制字符及以下非法字符：`/ \ < > | & ; $`
- **服务器地址保护**：`set_server_url` 不会自动覆盖已有注册信息的服务器。如果该服务器已有 `api_key`，修改地址会被阻止，防止意外丢失注册信息
- 如需切换到新服务器地址，请使用新的 `server` 别名（多服务器配置），或先用 `remove_server` 删除旧配置
- **删除服务器**：如需删除某个服务器配置，使用 `remove_server`。注意不能删除默认服务器，删除前需先用 `set_default_server` 切换默认服务器

## 使用示例

1. 设置服务器地址（可选，默认 https://api.open-aaas.com）：
   ```
   OpenAaaS(action: "set_server_url", server_url: "https://www.open-aaas.com")
   ```

   可选：发现服务端信息：
   ```
   OpenAaaS(action: "discover")
   ```

1.5. 切换到其他服务器操作：
   ```
   OpenAaaS(action: "list_services", server: "prod")
   OpenAaaS(action: "submit_task", server: "local", service_id: "my-service", task_prompt: "...")
   ```

2. 注册客户端：
   ```
   OpenAaaS(action: "register", name: "my-client")
   ```

3. 列出服务并筛选候选：
   ```
   OpenAaaS(action: "list_services")
   ```

4. 对目标服务获取详细 usage：
   ```
   OpenAaaS(action: "get_service_usage", service_id: "my-service")
   ```

5. 提交任务（带文件上传）：
   ```
   OpenAaaS(
     action: "submit_task",
     service_id: "my-service",
     task_prompt: "分析数据",
     output_prompt: "返回 JSON",
     input_files: ["./data.csv"],
     session_id: "可选，用于保持对话上下文"
   )
   ```
   文件上传限制：最多支持 10 个文件，单文件不超过 100MB；只能上传当前工作目录下的文件，不支持符号链接。

6. 列出当前 Session 任务历史：
   ```
   OpenAaaS(action: "list_history")
   ```

7. 查询任务（仅在用户要求时调用）：
   ```
   OpenAaaS(action: "get_task", task_id: "xxx")
   ```

8. 下载结果：
   ```
   OpenAaaS(action: "download_result", task_id: "xxx")
   ```

9. 删除服务器配置：
   ```
   OpenAaaS(action: "remove_server", server: "old-server")
   ```
