<p align="right">中文 | <a href="./README.en.md">English</a></p>

<p align="center">
  <img src="./assets/logo.png" width="360" alt="OpenAaaS Logo">
</p>

<p align="center"><strong>OpenAaaS — Open Us to the Agentic World</strong></p>

<p align="center">一个开放的 Agent-to-Agent 编排网络：网络中的节点是完整 Agent 实例构成的能力锚点，任何外部主智能体都可以发现、委派并组合它们。</p>

<p align="center">
  <a href="https://www.open-aaas.com">官网</a> ·
  <a href="https://arxiv.org/abs/2605.13618">论文</a> ·
  <a href="./server/README.md">server 文档</a> ·
  <a href="./agent-core/README.md">agent-core 文档</a> ·
  <a href="#如何使用">使用指南</a> ·
  <a href="./openaaas-mcp-adapter/README.md">MCP 适配器</a> · <a href="./pi-extension/README.md">PI 扩展</a> ·
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

## 重要更新

- **2026 年 8 月 — IDM-Alpha材料科学文献研究助手后端升级**：材料科学文献问答的回答准确度大幅提升。
- **2026 年 8 月 — AFLOW材料数据库查询智能体上线公共服务器**：用自然语言即可查询 AFLOW 材料数据库，无需了解查询语法，详见下方[当前可用服务](#当前可用服务)表格。

---

## 🚀 Quick Start

| 方式 | 入口 |
|---|---|
| **在线体验** | [![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/Wolido/OpenAaaS/main?filepath=binder%2Fquickstart.ipynb) — 浏览器直接运行 `binder/quickstart.ipynb` |
| **下载客户端** | [GitHub Releases](https://github.com/Wolido/OpenAaaS/releases) — macOS / Windows 桌面客户端 |

---

## 什么是 OpenAaaS?

> **智能流动，数据静止 —— 让 Agent 走到数据身边，而不是把数据交给 Agent。**

OpenAaaS 是一个面向 AI for Science 的 Agent-to-Agent 编排网络（Agent Orchestration Network）。网络中的每个节点都运行着一个拥有完整工具链的 Agent 实例；数据驻留在产生它的原地，Agent 能力通过网络流动到数据身边，完成分析、计算与协作。这些节点构成共享的能力基础设施；网络之外，无数主智能体——pi、Claude Code、Cursor 或自研 Agent——各自通过 OpenAaaS 协议发现和组合这些能力节点，把手伸向数据所在的任意角落。

| 操作视频 | 截图 |
|:---:|:---:|
| <video src="https://github.com/user-attachments/assets/5bee5e09-2866-4285-b00e-15210f274177"></video> | **接入网络**<br><img width="372" height="113" alt="截屏2026-05-07 09 36 25" src="https://github.com/user-attachments/assets/d3773d67-9d47-45db-9f5e-3ca96f990981" /><br>**查看节点列表**<br><img width="379" height="406" alt="截屏2026-05-07 09 37 22" src="https://github.com/user-attachments/assets/d74571ac-b300-411e-9371-b51822531926" /><br>**委派结果返回**<br><img width="371" height="391" alt="截屏2026-05-07 09 38 09" src="https://github.com/user-attachments/assets/16c9984b-e730-476c-93e7-1aae78f76a5d" /> |

**论文**：技术设计与实现细节详见 [arXiv:2605.13618](https://arxiv.org/abs/2605.13618)。

---

## 为什么选择 OpenAaaS?

### 1. Agent 编排，节点即 Agent

OpenAaaS 编排的不是脚本、函数或固定 API，而是运行在远程节点上的完整 Agent 实例。每个节点上的 Docker 容器内都运行着一个拥有本地工具、模型和数据的完整 Agent，能够自主决策、自主执行，并在需要时继续委派给子 Agent。

例如，Claude Code 可以发现一个运行在实验室服务器上的数据分析 Agent，把任务委派给它；该节点 Agent 处理本地数据时，还可以进一步调用节点内的子 Agent 完成清洗、建模或可视化。任意 Agent（Claude Code、pi mono 等）都可以加入网络，发现、委派并组合其他节点的 Agent。

### 2. 数据原地处理，零迁移

原始数据始终留在产生它的位置，远程 Agent 直接在数据旁工作。不同实验室、服务器或仪器可以围绕同一个复杂任务协作：每个节点只处理自己的本地数据，网络只传输任务描述（即委派请求）与结果（KB~MB 级），不触碰原始数据（TB 级）。

| | 传统云端方案 | OpenAaaS |
|---|---|---|
| 数据流向 | 本地 → 云端 → 本地 | **原始数据原地不动** |
| 网络传输 | 原始数据（TB 级） | 委派请求与结果（KB~MB 级） |
| 防火墙要求 | 需开放入站端口 | **仅出站 HTTP 即可** |
| 敏感数据 | 必须出域 | **不出实验室** |

### 3. 免规范化接入，即插即用

无需统一数据格式，JSON/CSV/Excel/MATLAB/HDF5/厂商二进制格式均可原地处理。你可以把已有的脚本、模型、数据库查询、仪器接口或内部工具打包进一个**完整 Agent 实例**的 Docker 镜像；这个实例会作为 Agent 节点注册到网络中，供其他 Agent 发现并**委派**任务。远程 Agent 看到的不是一个可被调用的函数，而是一个能自主决策、使用本地工具链完成任务的完整 Agent。节点零配置入网：`open-aaas-server run` 首次启动自动生成 `config.toml` 和 SQLite。自描述网络接口 + 渐进式能力发现，Agent 无需插件即可发现并使用其他 Agent 节点的能力。

### 4. 近数据端计算，低门槛部署

Rust 单二进制 + SQLite 嵌入式，零依赖部署，复制即用。Docker 隔离执行，每个任务独立沙箱。节点单向出站即可加入网络，无需公网 IP、无需开端口、无需 SSH——专为实验室防火墙和 NAT 环境设计。

---

## 如何使用？

以下任一方式都可以让你的 Agent 加入 OpenAaaS 网络，发现远程 Agent 并向它们委派任务。

公共服务器：**<https://api.open-aaas.com>**

#### 当前可用服务

| 名称 | 描述 |
|---|---|
| AFLOW材料数据库查询智能体 | 面向 AFLOW 材料数据库的自然语言查询执行器。将材料研究意图转换为经过校验、范围受限的 AFLOW 查询，返回便于阅读的 Markdown 响应与结构化结果文件；无需了解 AFLUX 查询语法，支持查询执行与结果比较。 |
| IDM-Alpha材料科学文献研究助手 | 基于数十万篇材料科学论文语料构建的检索增强生成（RAG）文献研究助手，支持文献问答、论文解读与跨论文综述，可生成深度阅读报告。适用于金属材料、陶瓷材料、复合材料等方向。 |
| 扶摇多专家研讨系统 | 面向材料科学问题的多专家会议研讨系统，从多视角拆分复杂问题并输出深度研讨结论。 |
| 六元高熵合金描述符数据库 | 利用近数据端 Agent 查询六元高熵合金全成分描述符数据。总数据量 500 亿条，超 10 万亿数据点，主要为描述符数据，含少量机器学习预测的压缩塑性数据，不含真实实验数据。 |

亦可通过 `pyopenaaas` 或 OpenAaaS API 实时查询当前可用服务。

| 方式 | 适合谁 | 入口 |
|---|---|---|
| Jupyter Notebook (Binder) | 想先体验，无需安装 | [![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/Wolido/OpenAaaS/main?filepath=binder%2Fquickstart.ipynb) |
| 桌面客户端 | 非技术用户 | [下载](https://github.com/Wolido/OpenAaaS/releases) |
| Python SDK | Python/Jupyter 用户 | `pip install pyopenaaas` |
| MCP 适配器 | Claude/Cursor/Cline 用户 | `uvx openaaas-mcp-adapter` |
| pi 插件 | 对话型 Agent 用户 | `pi install npm:open-aaas-pi-extension` |

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

> 在 OpenAaaS 中，`submit_task` 是指让你的 Agent 向远程节点上的另一个完整 Agent 实例提交委派请求，而不是调用一个远程函数。

```python
import pyopenaaas

client = pyopenaaas.Client()
client.register(name="your-name")

agents = client.list_services()  # 获取网络中的 Agent 节点列表
task = client.submit_task(
    service_id=agents[0].id,      # 向该完整 Agent 实例委派任务
    task_prompt="你的任务描述",    # 由目标 Agent 实例自主理解并执行
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
      "args": ["openaaas-mcp-adapter"],
      "toolTimeoutMs": 600000
    }
  }
}
```

- `toolTimeoutMs` 设置 MCP 工具调用的最大等待时间（毫秒），适合长任务轮询场景。
- ⚠️ 该参数由 MCP 客户端解析，实际生效情况取决于具体客户端实现；某些客户端或 Agent 工具本身可能仍有独立的超时限制，导致该参数不生效。

Codex 用户可以通过 CLI 添加适配器：

```bash
codex mcp add openaaas -- uvx openaaas-mcp-adapter
```

如需支持长任务轮询，在 `~/.codex/config.toml`（或受信任项目的 `.codex/config.toml`）中配置：

```toml
[mcp_servers.openaaas]
command = "uvx"
args = ["openaaas-mcp-adapter"]
startup_timeout_sec = 30
tool_timeout_sec = 600
```

Codex 的 `tool_timeout_sec` 单位为秒；调用 `poll_task` 时，建议将 `timeout_seconds` 设置为小于该值，并预留网络请求时间。

配置后重启客户端，即可在对话中调用全部能力。

<p align="center">
  <img alt="mcp" src="https://github.com/user-attachments/assets/b7ff63bf-5fa8-46fa-906b-a8edbd950465" />
</p>

详见 [openaaas-mcp-adapter/README.md](./openaaas-mcp-adapter/README.md)。

### pi 插件

面向 [pi](https://github.com/earendil-works/pi) 用户的 TypeScript 扩展，安装方式：

```bash
pi install npm:open-aaas-pi-extension
```

安装后在对话中直接说：

> "帮我设置 OpenAaaS 的服务器地址为 <https://api.open-aaas.com>，然后向合适的远程 Agent 节点委派一个数据分析任务"

客户端 Agent 自动完成注册、节点发现、任务委派和结果获取。

<video src="https://github.com/user-attachments/assets/4e2873ee-1581-46c7-b8f2-cfcd6da097ef" controls></video>

### 通用 Agent 框架

如果你的 Agent 没有 OpenAaaS 插件，直接访问 <https://api.open-aaas.com>。无需认证，返回完整 API 文档和使用说明，Agent 读取后即可自动完成注册、节点发现、任务委派。

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
（网络外部：无数主智能体，各自发现并组合网络中的能力节点）

客户端 Agent
(pi mono / Claude Code / Cline / 自研 Agent)
        ▲
        │ 控制流：委派请求、心跳、结果（KB 级）
        ▼
───────────────────────────────────────────────────────────────────
OpenAaaS Server（网络枢纽）
Rust + SQLite — 轻量索引层
  • 节点注册  • 委派路由  • 节点心跳  • 文件中转
        ▲
        │ 短轮询（单向出站 HTTP）
        ▼
───────────────────────────────────────────────────────────────────
Agent Core（网络节点）
Rust + Docker — 部署在数据本地
  • 向网络注册能力  • 轮询认领委派任务
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
| 客户端 Agent | pi mono / Codex / Open Code / 自研 Agent | 理解任务、发现网络中的其他 Agent、委派任务并整合结果 |
| 网络枢纽 | Server — 节点注册与委派路由中心 (Rust + SQLite) | 节点注册、委派路由、节点心跳、文件中转 |
| 网络节点 | agent-core — 在数据本地运行完整 Agent 实例的网络节点 (Rust + Docker) | 向网络注册自身能力、轮询认领任务、在沙箱中启动完整 Agent 实例隔离执行、上报结果 |

---

## 项目结构

```
OpenAaaS/
├── server/           # 网络枢纽（调度中心） (Rust)
├── agent-core/       # 网络节点：在数据本地运行完整 Agent 实例 (Rust)
├── admin-cli/        # 命令行管理员工具 (Rust)
├── client-app/       # 桌面客户端 (Tauri + Vue 3)
├── dash/             # 调试与管理员工具 (Python/Streamlit)
├── openaaas-mcp-adapter/ # MCP 适配器
├── pi-extension/         # PI 插件
├── pyopenaaas/       # Python SDK
└── binder/           # 示例 notebook 与脚本
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
