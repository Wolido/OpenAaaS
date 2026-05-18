<p align="right">中文 | <a href="./README.md">English</a></p>

<p align="center">
  <img src="./assets/logo.png" width="360" alt="OpenAaaS Logo">
</p>

<p align="center"><strong>OpenAaaS — Open Us to the Agentic World</strong></p>

<p align="center">
  <a href="https://www.open-aaas.com">官网</a> ·
  <a href="https://arxiv.org/abs/2605.13618">论文</a> ·
  <a href="./server/README.md">server 文档</a> ·
  <a href="./agent-core/README.md">agent-core 文档</a> ·
  <a href="#使用">使用指南</a> ·
  <a href="./client-extension/README.md">客户端插件</a> ·
  <a href="./client-app/README.md">桌面客户端</a>
</p>

<p align="center">
  <a href="https://pypi.org/project/openaaas-mcp-adapter/">
    <img src="https://img.shields.io/pypi/v/openaaas-mcp-adapter?label=PyPI&color=blue" alt="PyPI">
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
  📝 <a href="https://github.com/Wolido/OpenAaaS/discussions/57"><b>设计博客：不搬数据，蒸馏管理员.skill</b></a>
</p>

---

**智能流动，数据静止 —— 让 AI 走到数据身边，而不是把数据交给 AI。**

**OpenAaaS 正在构建一种全新的科研基础设施：数据驻留在产生它的原地，分析能力通过网络流动到数据身边。**

AI 的瓶颈已从模型能力转向科研能力的可达性，而"数据被迫迁移"是比模型更硬的约束。每个实验室都沉淀了独特的数据、算法与流程，但它们分散在孤岛中，无法被发现与调用。OpenAaaS 将 Agent 能力分发到数据节点本地，让任意 Agent 都能发现、调用并组合全球科研节点的能力——数据原地处理，代码与指令在网络中流动。

任何 Agent——无论是 Claude Code、pi mono、Kimi Cli 还是自研系统——都可以通过网络发现并组合全球科研节点的能力。

同时，我们致力于让网络的使用门槛降到最低，哪怕是手机上的通用大模型 App。

| 操作视频 | 截图 |
|:---:|:---:|
| <video src="https://github.com/user-attachments/assets/5bee5e09-2866-4285-b00e-15210f274177"></video> | **接入服务**<br><img width="372" height="113" alt="截屏2026-05-07 09 36 25" src="https://github.com/user-attachments/assets/d3773d67-9d47-45db-9f5e-3ca96f990981" /><br>**查看服务列表**<br><img width="379" height="406" alt="截屏2026-05-07 09 37 22" src="https://github.com/user-attachments/assets/d74571ac-b300-411e-9371-b51822531926" /><br>**服务结果返回**<br><img width="371" height="391" alt="截屏2026-05-07 09 38 09" src="https://github.com/user-attachments/assets/16c9984b-e730-476c-93e7-1aae78f76a5d" /> |

---

### 📄 论文

技术设计与实现细节详见：[arXiv:2605.13618](https://arxiv.org/abs/2605.13618)

---

## 四大核心主张

### 数据原位驻留，能力跨节点流动

数据孤岛的真正解决方式，不是把数据搬在一起，而是让分析能力走到数据身边。每个实验室沉淀的数据集、算法流程与领域经验，通过网络化封装成为 Agent 可直接调用的能力单元。Agent 无需预先掌握特定领域的全部知识，只需发现、编排、调用全球节点的服务，即可在材料科学、生物医学、天文观测等垂直领域不断扩展知识边界。

### 原生数据零迁移，规避迁移损耗

传统方案要求数据汇聚到中心化平台，这不可避免地带来格式转换失真、元数据丢失、版本分叉、合规审计链断裂等问题。OpenAaaS 不建立统一的数据仓库，数据始终保留在产生它的位置，维持最初的存储格式、目录结构与访问权限。分析任务以代码和指令的形式远程到达，结果回传，原始数据从未离开本地。

### 免规范化接入，原生格式即服务能力

我们不对数据提出任何前置的格式要求。JSON、CSV、Excel、MATLAB `.mat`、HDF5、仪器厂商的专有二进制格式——节点本地的解析与处理脚本本身就是网络能力的一部分。Agent 调用的是"解析+分析"的组合服务，而非要求数据预先被清洗、标准化、结构化。实验室现有的任意数据，接入即服务。

### 近数据端计算，调用开销趋近于零

计算发生在数据旁边，而非数据被搬运到计算中心。网络仅传输任务描述与执行结果（KB~MB 级），原始数据就地处理。对于 TB 级数据集和受监管敏感样本，这意味着无需等待上传、无需突破带宽瓶颈、无需面临出域合规审查——数据移动的边际成本趋近于零。

## 核心设计理念

传统云端方案要求数据离开本地：TB 级数据集必须迁移上传，敏感样本交给第三方，实验室防火墙被迫开放入站端口。OpenAaaS 反其道而行——将 Agent 执行节点直接部署在数据本地，网络只传输任务描述、任务文件及结果，原始数据原地不动。

| | 传统云端方案 | OpenAaaS 近数据端方案 |
|---|---|---|
| 数据流向 | 本地 → 云端 → 本地 | **原始数据原地不动** |
| 网络传输 | 原始数据（TB 级） | 任务描述、任务文件及结果（KB~MB 级） |
| 防火墙要求 | 需开放入站端口 | **仅出站 HTTP 即可** |
| 敏感数据 | 必须出域 | **不出实验室** |
| 延迟 | 受带宽限制 | 本地计算，极低延迟 |

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
  • 向网络注册能力  • 轮询认领任务  • 容器沙箱隔离执行  • 上报结果
        │              │                   │
        ▼              ▼                   ▼
   [本地数据集]    [分析脚本]         [专用硬件]
    （TB 级）      （算法/模型）        （GPU/仪器）
```

| 层级 | 组件 | 职责 |
|------|------|------|
| 客户端 Agent | pi mono / Kimi Cli / Codex / Open Code / 自研 Agent | 理解任务、发现网络节点、调度远端能力、整合结果 |
| 网络枢纽 | Server — 能力注册与调度中心 (Rust + SQLite) | 服务注册、任务路由、节点心跳、文件中转 |
| 网络节点 | agent-core — 能力执行节点 + Docker | 向网络注册自身能力、轮询认领任务、在沙箱中隔离执行、上报结果 |

## 设计思路

| 原则 | 说明 | 效果 |
|------|------|------|
| Rust + 单二进制 | `cargo build --release` 得到一个可执行文件 | 零依赖部署，复制即用 |
| SQLite 嵌入式 | 数据库随进程启动，无单独服务 | 零运维，单节点足够 |
| Docker 隔离 | 每个任务独立容器，workspace 挂载 | 安全可控，环境可复现 |
| 节点自组网 | 节点主动向网络注册并轮询任务，Server 仅维护索引。原始数据不出域，任务文件经 Server 流转 | 节点无需公网 IP，单向出站即可加入网络；数据原地处理，天然适应实验室防火墙环境 |

## 特性

### 数据原位驻留与能力跨节点流动

- **🔌 Agent 零学习成本接入，自描述 API 自动暴露服务文档** — 无需认证，返回完整 API 文档和使用说明。Agent 无需插件即可理解并调用全部科研服务。
- **🧩 渐进式能力发现，避免上下文溢出** — 初次查询返回轻量摘要，再按需返回详细用法。类似 SKILL.md 的渐进式披露设计，保护 Agent 的上下文窗口。

### 原生数据零迁移

- **🔒 数据不出域** — Agent 执行节点直接部署在实验室服务器或仪器工作站上，原始大数据集通过本地挂载原地处理，敏感数据不离开防火墙。网络只传输任务描述、任务文件及结果，不触碰原始数据。
- **💾 单二进制零运维** — SQLite 数据库 + 本地文件存储，无需 Redis/MySQL。单节点即可部署，适合实验室边缘节点。
- **⚖️ 节点反向入网，不需要公网 IP** — 节点自行控制并发和任务认领，Server 只做轻量队列管理。实验室节点只需要单向出站即可接入，无需开放端口或 SSH。

### 免规范化接入与近数据端计算

- **🐳 每个实验任务独立沙箱，结果可复现** — 每个任务在独立容器中运行，通过 workspace 挂载实现输入输出。环境隔离，结果可追溯、可复现。
- **🔧 节点零配置入网** — `open-aaas-server run` 首次启动自动生成 `config.toml`、SQLite 数据库、密钥。无需手动配置，开箱即用。
- **🤖 MCP 标准协议兼容** — 通过 `openaaas-mcp-adapter`，Claude Desktop、Cursor、Cline 等任意支持 MCP 的客户端均可一键接入，无需编写插件。

## 使用

公共服务器：**<https://api.open-aaas.com>**

我们在公共服务器中提供了三项试用的科研服务：

- 基于数十万真实文献的 IDM-Alpha 金属材料文献研究助手
- 万亿规模六元高熵合金描述符数据库
- 扶摇智能体圆桌会议系统

可以让 Agent 接入公共服务器使用

### 快速开始

**场景一：使用公共服务器**

无需自建基础设施，直接配置你的 Agent 接入公共服务器，即可调用社区共享的科研服务。适合个人研究者快速接入。

### 用 pi / kimi 插件

在对话中直接说：

> "帮我设置 OpenAaaS 的服务器地址为 <https://api.open-aaas.com>，然后提交一个数据分析任务"

客户端 Agent 自动完成注册、服务发现、任务提交和结果获取。

<video src="https://github.com/user-attachments/assets/4e2873ee-1581-46c7-b8f2-cfcd6da097ef" controls></video>

### 用 MCP 客户端

`openaaas-mcp-adapter` 已发布至 PyPI。如果你使用的是 **OpenClaw** 或其他支持 MCP（Model Context Protocol）的 Agent，接入 OpenAaaS 网络几乎是零成本的——无需编写任何插件，只需一条配置即可调用全部能力。

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

配置后重启客户端，即可在对话中调用 OpenAaaS 的 14 个标准 Tool（`set_server_url`、`register`、`list_services`、`submit_task` 等），无需安装任何插件。

甚至，你可以直接让你的Agent帮你配置。

<p align="center">
  <img alt="mcp" src="https://github.com/user-attachments/assets/b7ff63bf-5fa8-46fa-906b-a8edbd950465" />
</p>

详见 [client-extension/openaaas-mcp-adapter/README.md](./client-extension/openaaas-mcp-adapter/README.md)。

### 用桌面客户端

如果你偏好图形界面操作，可以使用 **OpenAaaS 桌面客户端**——一个基于 Tauri 的跨平台桌面应用，支持 macOS、Windows 和 Linux。

桌面客户端适合：
- 不想配置命令行或插件的非技术用户
- 需要管理多个服务器并直观浏览服务的场景
- 希望拖拽上传文件、实时查看任务进度的用户

📖 详见 [client-app/README.md](./client-app/README.md)

<p align="center">
  <img alt="OpenAaaS Client" src="https://github.com/user-attachments/assets/8bc81d68-76da-47c6-a535-83227b27b8bd" width="800" />
</p>

> macOS 用户请注意：应用使用 ad-hoc 自签名，首次打开时前往 **系统设置 → 隐私与安全性 → 安全性** 点击"仍要打开"即可。

### 用通用 Agent 框架

如果你的 Agent 没有 OpenAaaS 插件，让 Agent 直接访问 <https://api.open-aaas.com>

- 无需认证，返回完整 API 文档和使用说明
- Agent 读取后即可自动完成注册、服务发现、任务提交

**场景二：部署在实验室服务器，接入本地能力**

在机房或实验室的本地服务器上启动 OpenAaaS，将本地分析脚本、专用计算流程注册为网络节点。课题组内的任何 Agent——pi、Kimi、Claude 或自研系统——都能通过统一入口查询节点状态、提交分析任务、获取结果数据。

### 本地部署

**部署 Server（调度中心）**：

```bash
cd server
cargo build --release
./target/release/open-aaas-server run
```

首次启动自动生成 `config.toml`和 SQLite 数据库。

**部署 Agent Core（执行节点）**：

```bash
cd agent-core
cargo build --release
./target/release/agent-core init
./target/release/agent-core register --token <registration_token> --name my-agent
./target/release/agent-core run
```

`registration_token` 需要先在 Server 上创建 Service 获取。Admin 可使用 Server 日志中的 API Key 调用 `POST /api/v1/services/` 创建。

Agent 执行器镜像需要提前构建（在 agent-core 目录下）：

```bash
cd executor-example && docker build -t open-aaas-executor:latest .
```

详见 [agent-core/README.md](./agent-core/README.md)

## 项目结构

```
OpenAaaS/
├── server/           # 网络枢纽（调度中心） (Rust) — 任务调度、队列、鉴权、文件中转
├── agent-core/       # 网络节点（执行节点） (Rust) — 注册、轮询、Docker 隔离执行
├── client-app/       # 桌面客户端 (Tauri + Vue 3) — 服务市场、任务提交、结果查看
├── dash/             # 调试与管理员工具 (Python/Streamlit)
└── client-extension/ # 客户端扩展 — pi 插件、kimi 插件、MCP 适配器（Claude Desktop / Cursor / Cline）
```

## 科研愿景

OpenAaaS 的愿景是让每个实验室都成为 Agentic Science 网络中的一个可组合节点。数据不再因迁移而损耗，知识不再因孤岛而停滞。每个课题组沉淀的数据形态、分析流程与领域方法——无论其存储格式多么独特——都可以通过网络被任意 Agent 发现、调用与编排。

当分析能力能够流动到数据身边，Agent 的知识边界将从单个实验室的闭环，扩展到全球协作的开放生态。数据移动的边际成本趋近于零，意味着任意规模的 Dataset 都可以被任意位置的 Agent 即时调用。科研创新的边界，不再受限于单个团队的数据规模或领域深度。

## 开源许可

MIT License © IDM Explorer Lab

<img src="./assets/idm-logo.png" width="200" alt="IDM Explorer Lab">
