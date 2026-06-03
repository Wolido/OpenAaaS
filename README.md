<p align="right">中文 | <a href="./README.en.md">English</a></p>

<p align="center">
  <img src="./assets/logo.png" width="360" alt="OpenAaaS Logo">
</p>

<p align="center"><strong>OpenAaaS — Open Us to the Agentic World</strong></p>

<p align="center">
  <a href="https://www.open-aaas.com">官网</a> ·
  <a href="https://arxiv.org/abs/2605.13618">论文</a> ·
  <a href="./server/README.md">server 文档</a> ·
  <a href="./agent-core/README.md">agent-core 文档</a> ·
  <a href="#如何使用">使用指南</a> ·
  <a href="./client-extension/README.md">客户端插件</a> ·
  <a href="./pyopenaaas/README.md">Python SDK</a> ·
  <a href="./client-app/README.md">桌面客户端</a>
</p>

<p align="center">
  <a href="https://pypi.org/project/openaaas-mcp-adapter/">
    <img src="https://img.shields.io/pypi/v/openaaas-mcp-adapter?label=MCP%20Adapter&color=blue" alt="MCP Adapter">
  </a>
  <a href="https://pypi.org/project/pyopenaaas/">
    <img src="https://img.shields.io/pypi/v/pyopenaaas?label=Python%20SDK&color=blue" alt="Python SDK">
  </a>
  <a href="https://opensource.org/licenses/MIT">
    <img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT">
  </a>
  <a href="https://github.com/Wolido/OpenAaaS/actions/workflows/ci.yml">
    <img src="https://github.com/Wolido/OpenAaaS/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="https://rust-edition-guide.rs/editions/2024.html">
    <img src="https://img.shields.io/badge/rust-2024%20edition-blue.svg" alt="Rust">
  </a>
</p>

<p align="center">
  📝 <a href="https://github.com/Wolido/OpenAaaS/discussions/57"><b>设计博客：不搬数据，蒸馏管理员.skill</b></a><br>
  📝 <a href="https://github.com/Wolido/OpenAaaS/discussions/79"><b>用户故事：不只属于实验室——我用 OpenAaaS 给妻子搭了一个审校助手</b></a>
</p>

---

## 🚀 Quick Start

| 方式 | 入口 |
|---|---|
| **在线体验** | [![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/Wolido/OpenAaaS/main?filepath=binder%2Fquickstart.ipynb) — 浏览器直接运行 `binder/quickstart.ipynb` |
| **下载客户端** | [GitHub Releases](https://github.com/Wolido/OpenAaaS/releases) — macOS / Windows 桌面客户端 |

---

## 什么是 OpenAaaS?

> **智能流动，数据静止 —— 让 Agent 走到数据身边，而不是把数据交给 Agent。**

OpenAaaS 是一个面向 AI for Science 的 Agent 编排网络（Agent Orchestration Network）：数据驻留在产生它的原地，Agent 能力通过网络流动到数据身边。

| 操作视频 | 截图 |
|:---:|:---:|
| <video src="https://github.com/user-attachments/assets/5bee5e09-2866-4285-b00e-15210f274177"></video> | **接入网络**<br><img width="372" height="113" alt="截屏2026-05-07 09 36 25" src="https://github.com/user-attachments/assets/d3773d67-9d47-45db-9f5e-3ca96f990981" /><br>**查看节点列表**<br><img width="379" height="406" alt="截屏2026-05-07 09 37 22" src="https://github.com/user-attachments/assets/d74571ac-b300-411e-9371-b51822531926" /><br>**任务结果返回**<br><img width="371" height="391" alt="截屏2026-05-07 09 38 09" src="https://github.com/user-attachments/assets/16c9984b-e730-476c-93e7-1aae78f76a5d" /> |

**论文**：技术设计与实现细节详见 [arXiv:2605.13618](https://arxiv.org/abs/2605.13618)。

---

## 为什么选择 OpenAaaS?

### 1. Agent 编排，节点即 Agent

网络调度的对象不是脚本，而是拥有完整工具链的 Agent 实例。每个节点上的 Docker 容器内运行完整 Agent，能够自主决策、自主执行、自主调用子 Agent。任意 Agent（Claude Code、pi mono、Kimi 等）都可以像调用工具一样发现、委派并组合全球节点的其他 Agent。

### 2. 数据原地处理，零迁移

原始数据始终留在产生它的位置，远程 Agent 直接在数据旁工作。网络只传输任务描述与结果（KB~MB 级），不触碰原始数据（TB 级）。

| | 传统云端方案 | OpenAaaS |
|---|---|---|
| 数据流向 | 本地 → 云端 → 本地 | **原始数据原地不动** |
| 网络传输 | 原始数据（TB 级） | 任务描述与结果（KB~MB 级） |
| 防火墙要求 | 需开放入站端口 | **仅出站 HTTP 即可** |
| 敏感数据 | 必须出域 | **不出实验室** |

### 3. 免规范化接入，即插即用

无需统一数据格式，JSON/CSV/Excel/MATLAB/HDF5/厂商二进制格式均可原地处理。节点零配置入网：`open-aaas-server run` 首次启动自动生成 `config.toml` 和 SQLite。自描述 API + 渐进式能力发现，Agent 无需插件即可理解并调用全部服务。

### 4. 近数据端计算，低门槛部署

Rust 单二进制 + SQLite 嵌入式，零依赖部署，复制即用。Docker 隔离执行，每个任务独立沙箱。节点单向出站即可加入网络，无需公网 IP、无需开端口、无需 SSH——专为实验室防火墙和 NAT 环境设计。

---

## 如何使用？

公共服务器：**<https://api.open-aaas.com>**

| 方式 | 适合谁 | 入口 |
|---|---|---|
| Jupyter Notebook (Binder) | 想先体验，无需安装 | [![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/Wolido/OpenAaaS/main?filepath=binder%2Fquickstart.ipynb) |
| 桌面客户端 | 非技术用户 | [下载](https://github.com/Wolido/OpenAaaS/releases) |
| Python SDK | Python/Jupyter 用户 | `pip install pyopenaaas` |
| MCP 适配器 | Claude/Cursor/Cline 用户 | `uvx openaaas-mcp-adapter` |
| pi / kimi 插件 | 对话型 Agent 用户 | 对话中直接调用 |

### Jupyter Notebook (Binder)

[![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/Wolido/OpenAaaS/main?filepath=binder%2Fquickstart.ipynb)

点击上方 Binder 徽章即可在浏览器中运行 `binder/quickstart.ipynb`，无需安装任何软件。

### 桌面客户端

基于 Tauri 的跨平台桌面应用，支持 macOS 和 Windows。适合非技术用户管理多个服务器、拖拽上传文件、实时查看任务进度。

> macOS 和 Windows 用户可直接从 [GitHub Releases](https://github.com/Wolido/OpenAaaS/releases) 下载 `.dmg` 或 `.msi` 安装包。macOS 首次打开需前往 **系统设置 → 隐私与安全性 → 安全性** 点击"仍要打开"。

<p align="center">
  <img alt="OpenAaaS Client" src="https://github.com/user-attachments/assets/8bc81d68-76da-47c6-a535-83227b27b8bd" width="800" />
</p>

详见 [client-app/README.md](./client-app/README.md)。

### Python SDK

```bash
pip install pyopenaaas
```

```python
import pyopenaaas

client = pyopenaaas.Client()
client.register(name="your-name")

services = client.list_services()
task = client.submit_task(
    service_id=services[0].id,
    task_prompt="你的任务描述",
)
task = client.wait_for_task(task.id)
paths = client.download_all_files(task.id)
```

详见 [pyopenaaas/README.md](./pyopenaaas/README.md)。

### MCP 适配器

`openaaas-mcp-adapter` 已发布至 PyPI。支持 MCP 的客户端（Claude Desktop、Cursor、Cline 等）只需一条配置即可接入：

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

配置后重启客户端，即可在对话中调用全部能力。

<p align="center">
  <img alt="mcp" src="https://github.com/user-attachments/assets/b7ff63bf-5fa8-46fa-906b-a8edbd950465" />
</p>

详见 [client-extension/openaaas-mcp-adapter/README.md](./client-extension/openaaas-mcp-adapter/README.md)。

### pi / kimi 插件

在对话中直接说：

> "帮我设置 OpenAaaS 的服务器地址为 <https://api.open-aaas.com>，然后提交一个数据分析任务"

客户端 Agent 自动完成注册、服务发现、任务提交和结果获取。

<video src="https://github.com/user-attachments/assets/4e2873ee-1581-46c7-b8f2-cfcd6da097ef" controls></video>

### 通用 Agent 框架

如果你的 Agent 没有 OpenAaaS 插件，直接访问 <https://api.open-aaas.com>。无需认证，返回完整 API 文档和使用说明，Agent 读取后即可自动完成注册、服务发现、任务提交。

---

## 部署自己的节点

### 预编译二进制（推荐）

无需安装 Rust，下载即可运行：[GitHub Releases](https://github.com/Wolido/OpenAaaS/releases)

| 组件 | 二进制文件名 |
|---|---|
| Server | `open-aaas-server` |
| Agent Core | `agent-core` |

支持平台：
- **Server / Agent Core**：Linux x64 (musl 静态链接)、Linux arm64 (musl 静态链接)、macOS arm64、Windows x64
- **桌面客户端**：macOS、Windows

> Linux 版本采用 musl 静态链接，不依赖系统 glibc，可在任意 Linux 发行版直接运行。

```bash
chmod +x open-aaas-server   # Unix 用户
./open-aaas-server run
```

首次启动自动生成 `config.toml` 和 SQLite 数据库。

### 从源码编译

```bash
cd server
cargo build --release
./target/release/open-aaas-server run
```

Agent Core 部署详见 [agent-core/README.md](./agent-core/README.md)。

---

## 架构

```
客户端 Agent
(pi mono / Claude Code / Kimi Cli / Cline / 自研 Agent)
        ▲
        │ 控制流：任务描述、心跳、结果（KB 级）
        ▼
───────────────────────────────────────────────────────────────────
OpenAaaS Server（网络枢纽）
Rust + SQLite — 轻量索引层
  • 服务注册  • 任务路由  • 节点心跳  • 文件中转
        ▲
        │ 短轮询（单向出站 HTTP）
        ▼
───────────────────────────────────────────────────────────────────
Agent Core（网络节点）
Rust + Docker — 部署在数据本地
  • 向网络注册能力  • 轮询认领任务
  • 容器沙箱隔离执行  • 上报结果
        │
        ▼
   [完整 Agent 实例]
   （拥有完整工具链，在容器内自主决策、自主执行）
        │
        ├─→ [本地数据集]（TB 级）
        ├─→ [分析脚本]（算法/模型）
        └─→ [专用硬件]（GPU/仪器）
```

| 层级 | 组件 | 职责 |
|------|------|------|
| 客户端 Agent | pi mono / Kimi Cli / Codex / Open Code / 自研 Agent | 理解任务、发现网络节点、调度远端 Agent、整合结果 |
| 网络枢纽 | Server — 能力注册与调度中心 (Rust + SQLite) | 服务注册、任务路由、节点心跳、文件中转 |
| 网络节点 | agent-core — 能力执行节点 + Docker，容器内运行**完整 Agent 实例** | 向网络注册自身能力、轮询认领任务、在沙箱中启动完整 Agent 实例隔离执行、上报结果 |

---

## 项目结构

```
OpenAaaS/
├── server/           # 网络枢纽（调度中心） (Rust)
├── agent-core/       # 网络节点（执行节点） (Rust)
├── client-app/       # 桌面客户端 (Tauri + Vue 3)
├── dash/             # 调试与管理员工具 (Python/Streamlit)
├── client-extension/ # 客户端扩展 — pi 插件、kimi 插件、MCP 适配器
├── pyopenaaas/       # Python SDK
└── binder/         # 示例 notebook 与脚本
```

---

## 论文

arXiv:2605.13618 — [https://arxiv.org/abs/2605.13618](https://arxiv.org/abs/2605.13618)

---

## 参与贡献

欢迎参与贡献！请阅读 [CONTRIBUTING.md](./CONTRIBUTING.md)。

---

## 开源许可

MIT License © IDM Explorer Lab

<img src="./assets/idm-logo.png" width="200" alt="IDM Explorer Lab">
