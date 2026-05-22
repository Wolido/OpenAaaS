# OpenAaaS Dashboard

<p align="right"><a href="./README.zh.md">中文</a> | English</p>

> **⚠️ 定位说明**：这是一个面向开发者和系统管理员的**调试与管理员控制工具**，用于监控和管理 OpenAaaS 的任务与系统状态。**它不是 OpenAaaS 的用户界面 / 主界面（Main UI）**。

基于 Streamlit 的 Web UI，用于调试、监控和管理 OpenAaaS 任务。

## 功能特性

- 📊 **任务概览与调试**：以卡片布局查看所有任务，查看任务详情（输入/输出/文件等），支持取消任务
- 🔧 **管理员视图**：查看所有用户任务，按用户筛选任务
- 🔄 **自动刷新**：实时更新，支持可配置刷新间隔
- 🔍 **状态过滤**：按状态筛选任务（All/Pending/Running/Completed/Failed/Cancelled/Cancelling）
- ⚙️ **灵活配置**：支持 CLI 参数、环境变量和配置文件

## 安装

### 使用 uv（推荐）

```bash
uv tool install open-aaas-dashboard
```

### 使用 pip

```bash
pip install open-aaas-dashboard
```

### 开发安装

```bash
cd OpenAaaS/dash
pip install -e .
```

## 用法

### 命令行

```bash
# With command line arguments
aaas-dashboard --server-url http://localhost:8080 --api-key ak_xxx

# With environment variables
export OAAS_SERVER_URL=http://localhost:8080
export OAAS_API_KEY=ak_xxx
aaas-dashboard
```

### 配置优先级

配置按以下优先级加载（从高到低）：

1. **命令行参数**：`--server-url`, `--api-key`
2. **环境变量**：`OAAS_SERVER_URL`, `OAAS_API_KEY`
3. **配置文件**：`~/.config/aaas-dashboard/config.toml`

### 配置文件格式

创建 `~/.config/aaas-dashboard/config.toml`：

```toml
server_url = "http://localhost:8080"
api_key = "ak_xxx"
```

## 开发

```bash
# Install dependencies
pip install -e ".[dev]"

# Run the dashboard
streamlit run src/aaas_dashboard/app.py
```

## 许可证

MIT 许可证
