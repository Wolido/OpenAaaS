## Client Extensions

<p align="right"><a href="./README.md">English</a> | 中文</p>

`client-extension/` 是 OpenAaaS 的客户端扩展集合，让不同的 Agent 客户端（pi、Kimi 等）能够连接到 OpenAaaS 网络，发现远程服务、提交任务并获取结果。

目前包含三个扩展：

### pi-extension

面向 [pi](https://github.com/badlogic/pi-mono) 的 TypeScript 扩展。提供统一的 `OpenAaaS` 入口工具，通过 `action` 参数调用不同功能。支持多服务器配置、自动任务监控（widget + toast 通知）、Session 持久化与重建提醒。

### kimi-plugin

面向 Kimi 的 Python 插件。通过 `plugin.json` 定义多个独立工具，支持多服务器管理、渐进式信息获取。包含完整的测试套件。

### openaaas-mcp-adapter

面向 Claude Desktop、Cursor、Cline 等 MCP 客户端的 Python 适配器。基于 MCP SDK 构建，Transport 为 `stdio`，提供 14 个核心 Tool，支持文件上传下载、多服务器配置、路径遍历与 zip 炸弹防护。

---

## Quick Start

### pi-extension

```bash
mkdir -p ~/.pi/agent/extensions/OpenAaaS
cp -r pi-extension/* ~/.pi/agent/extensions/OpenAaaS/
cd ~/.pi/agent/extensions/OpenAaaS
npm install
```

在 pi 中执行 `/reload` 加载扩展。首次使用时会自动创建默认配置文件，然后即可通过对话调用：

```
OpenAaaS(action: "set_server_url", server_url: "https://api.open-aaas.com")
OpenAaaS(action: "register", name: "my-client")
OpenAaaS(action: "list_services")
```

### openaaas-mcp-adapter

使用 uvx 零安装运行（需已安装 [uv](https://docs.astral.sh/uv/)）：

```bash
uvx openaaas-mcp-adapter
```

或在 Claude Desktop 的 `claude_desktop_config.json` 中添加：

```json
{
  "mcpServers": {
    "openaaas": {
      "command": "uvx",
      "args": ["openaaas-mcp-adapter"]
    }
  }
}
```

配置完成后重启 Claude Desktop，即可在对话中调用工具：

```
set_server_url(server_url: "https://api.open-aaas.com")
register(name: "my-client")
list_services()
```

### kimi-plugin

将 `kimi-plugin/` 目录复制到 Kimi 插件目录（如 `~/.kimi/plugins/kimi-plugin`），在其根目录下创建 `config.json`（可参考 `config.json.example`），然后在 Kimi 中加载插件即可使用。

---

## Standard Workflow

无论你使用哪个客户端扩展，与 OpenAaaS 交互的标准流程一致：

1. **设置服务器** — 配置目标 OpenAaaS 服务器地址
2. **注册** — 向服务器注册客户端，获取并保存 `api_key`（每个服务器仅需一次）
3. **浏览服务** — `list_services` 获取可用服务的轻量摘要，筛选候选服务
4. **获取用法** — `get_service_usage` 查看目标服务的详细能力范围、调用规范和返回格式
5. **提交任务** — `submit_task` 构造 `task_prompt` 和 `output_prompt`，保存返回的 `task_id`
6. **查询结果** — 仅在用户明确要求时调用 `get_task` 查询任务状态和结果（不要主动轮询）
7. **下载结果** — `download_result` 获取任务输出的文件

> **注意**：pi-extension 额外支持 `list_history` 用于查询当前 Session 的任务历史，实现会话中断后的上下文重建。

遵循渐进式披露原则：先浏览轻量服务列表筛选候选，再按需获取详细用法，避免一次性加载所有服务的完整文档导致上下文溢出。
