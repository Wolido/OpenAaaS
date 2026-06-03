# 贡献指南

感谢你对 OpenAaaS 的兴趣！本指南将帮助你快速了解如何参与这个项目的开发。

OpenAaaS 是一个面向科研领域的 Agent 网络基础设施，核心理念是**"数据原位驻留，能力跨节点流动"**。无论你是修复 Bug、添加新功能、改进文档，还是提出想法，我们都欢迎你的贡献。

<!-- TOC -->
- [贡献指南](#贡献指南)
  - [在开始之前](#在开始之前)
  - [开发环境搭建](#开发环境搭建)
    - [必备工具](#必备工具)
    - [可选但推荐的工具](#可选但推荐的工具)
  - [快速上手](#快速上手)
    - [克隆仓库](#克隆仓库)
    - [各组件构建与运行](#各组件构建与运行)
      - [server（网络枢纽）](#server网络枢纽)
      - [agent-core（网络节点）](#agent-core网络节点)
      - [client-app（桌面客户端）](#client-app桌面客户端)
      - [dash（调试/管理员工具）](#dash调试管理员工具)
      - [client-extension（客户端扩展）](#client-extension客户端扩展)
  - [提交规范](#提交规范)
    - [Commit Message 格式](#commit-message-格式)
    - [常用 type](#常用-type)
    - [组件作用域（scope）](#组件作用域scope)
    - [示例](#示例)
  - [Pull Request 流程](#pull-request-流程)
    - [分支策略](#分支策略)
    - [PR 前检查清单](#pr-前检查清单)
    - [PR 标题和描述](#pr-标题和描述)
  - [各组件开发指南](#各组件开发指南)
    - [server](#server)
    - [agent-core](#agent-core)
    - [client-app](#client-app)
    - [dash](#dash)
    - [kimi-plugin](#kimi-plugin)
    - [openaaas-mcp-adapter](#openaaas-mcp-adapter)
    - [pi-extension](#pi-extension)
  - [更新 CHANGELOG](#更新-changelog)
  - [报告问题](#报告问题)
    - [Bug 报告](#bug-报告)
    - [功能请求](#功能请求)
    - [部署问题](#部署问题)
    - [安全问题](#安全问题)
  - [社区与沟通](#社区与沟通)

<!-- /TOC -->

## 在开始之前

在提交 Issue 或开始编写代码前，请先花几分钟确认：

1. **搜索现有 Issue**：查看是否有人已经报告过相同的问题或提出过类似的功能请求。
2. **浏览 Discussions**：一般性使用问题、架构讨论和想法交流请优先在 [GitHub Discussions](https://github.com/Wolido/OpenAaaS/discussions) 中进行。
3. **阅读相关文档**：确认你的问题是否已在 [官网文档](https://www.open-aaas.com) 中有解答。

如果你确定这是一个新的 Bug 或合理的功能请求，欢迎直接提交 Issue。对于小的文档修复或明显的 Bug，可以直接提交 PR，不必先开 Issue。

## 开发环境搭建

### 必备工具

| 工具 | 版本要求 | 说明 |
|------|---------|------|
| Rust | stable（server / agent-core 使用 edition 2024） | server、agent-core、client-app/src-tauri 的构建需要 |
| Node.js | 20 | client-app 前端构建需要 |
| Python | 3.10+ | dash、client-extension 各插件需要 |
| Docker | 最新稳定版 | agent-core 使用 Docker 进行沙箱隔离 |

### 可选但推荐的工具

- **uv**：Python 包管理和虚拟环境工具，openaaas-mcp-adapter 已采用 uv
- **cargo-watch**：Rust 开发时的热重载

## 快速上手

### 克隆仓库

```bash
git clone https://github.com/Wolido/OpenAaaS.git
cd OpenAaaS
```

本项目为多语言多组件架构，各组件可独立开发。你不需要一次性构建全部组件，只需关注你要修改的部分。

### 各组件构建与运行

#### server（网络枢纽）

```bash
cd server
cargo build --release
./target/release/open-aaas-server run
```

首次启动会自动生成 `config.toml` 和 SQLite 数据库。

#### agent-core（网络节点）

```bash
cd agent-core
cargo build --release
./target/release/agent-core init
./target/release/agent-core register --token <token> --name my-agent
./target/release/agent-core run
```

#### client-app（桌面客户端）

```bash
cd client-app
npm run tauri:dev
```

这将同时启动 Vue 前端和 Tauri Rust 后端。

#### dash（调试/管理员工具）

```bash
cd dash
pip install -e ".[dev]"
aaas-dashboard
```

`aaas-dashboard` 是 `pyproject.toml` 中 `[project.scripts]` 定义的入口脚本。

#### client-extension（客户端扩展）

**Python 扩展**（kimi-plugin、openaaas-mcp-adapter）各目录下有独立的 `pyproject.toml`，安装方式：

```bash
# kimi-plugin
cd client-extension/kimi-plugin
pip install -e ".[dev]"

# openaaas-mcp-adapter（已发布至 PyPI）
cd client-extension/openaaas-mcp-adapter
pip install -e ".[dev]"
```

#### pyopenaaas（Python SDK）

```bash
cd pyopenaaas
pip install -e ".[dev]"
pytest tests/ -v
```

**Node 扩展**（pi-extension）：

```bash
cd client-extension/pi-extension
npm install
```

## 提交规范

本项目采用 [Conventional Commits](https://www.conventionalcommits.org/) 规范。所有 PR 的标题必须符合此格式。

### Commit Message 格式

```
<type>(<scope>): <简短描述>

<可选的正文，说明变更动机和具体实现>

<可选的脚注，如 BREAKING CHANGE、Fixes #123>
```

### 常用 type

| type | 含义 |
|------|------|
| `feat` | 新功能 |
| `fix` | 修复 Bug |
| `docs` | 仅文档变更 |
| `style` | 代码格式调整（不影响逻辑） |
| `refactor` | 重构（既不是新增功能也不是修复 Bug） |
| `perf` | 性能优化 |
| `test` | 测试相关 |
| `chore` | 构建过程、辅助工具、依赖更新等 |

### 组件作用域（scope）

scope 用于标识变更所在的组件。常用的 scope 包括：

- `server` — 网络枢纽（调度中心）
- `agent-core` — 网络节点（执行节点）
- `client-app` — 桌面客户端
- `dash` — 调试/管理员工具
- `client-extension` — 客户端扩展整体
- `kimi-plugin` — Kimi 插件
- `mcp-adapter` — MCP 适配器
- `pi-extension` — pi 扩展
- `pyopenaaas` — Python SDK
- `docs` — 项目级文档
- `ci` — CI/CD 配置
- `release` — 发版流程与 CI/CD
- `root` — 顶层配置（README、LICENSE 等）

### 示例

```
feat(server): 新增任务调度优先级队列

fix(agent-core): 修复沙箱容器泄漏问题

docs(client-app): 更新 README 中的安装说明

chore(ci): 升级 GitHub Actions 缓存版本
```

## Pull Request 流程

### 分支策略

1. 从 `main` 分支切出你的功能分支：
   ```bash
   git checkout -b feat/server-add-priority-queue
   ```
2. 在你的分支上进行开发，保持提交历史整洁（必要时可 `git rebase -i` 整理）。
3. 推送到你的 fork 或远程分支，然后向 `main` 分支提交 PR。

### PR 前检查清单

提交 PR 前，请确认以下事项：

- [ ] 代码改动已测试通过
- [ ] 对应组件的 `CHANGELOG.md` 已更新（在 `[Unreleased]` 下添加了条目）
- [ ] 没有引入无关的文件改动
- [ ] PR 标题符合 Conventional Commits 规范（如 `feat(agent-core): xxx`）

### PR 标题和描述

- **标题**：使用 Conventional Commits 格式，如 `feat(server): 新增节点健康检查接口`
- **描述**：简要说明变更内容、动机和影响。如果修复了某个 Issue，请在正文中引用，如 `Fixes #123`。

## 各组件开发指南

### server

- **语言**：Rust（Axum 0.8 + Tokio + SQLx + SQLite）
- **包管理器**：Cargo
- **构建**：`cargo build`
- **测试**：`cargo test`
- **代码风格**：遵循 `cargo fmt` 和 `cargo clippy` 的默认规则

### agent-core

- **语言**：Rust（Tokio + Docker 沙箱隔离）
- **包管理器**：Cargo
- **构建**：`cargo build`
- **测试**：`cargo test --features test-utils`
- **注意**：运行完整测试需要本地 Docker 环境可用

### client-app

- **语言**：TypeScript / Vue 3 + Rust（Tauri）
- **包管理器**：npm（前端）+ Cargo（Tauri 后端）
- **前端测试**：`npm ci && npm run test`
- **Tauri 后端测试**：`cd src-tauri && cargo test`
- **开发启动**：`npm run tauri:dev`

### dash

- **语言**：Python + Streamlit
- **包管理器**：pip / hatchling
- **安装**：`pip install -e ".[dev]"`
- **测试**：`pytest tests/ -v`
- **启动**：`aaas-dashboard`
- **注意**：CI 中 dash 的测试仅在 Linux 上运行

### kimi-plugin

- **语言**：Python
- **包管理器**：pip / setuptools
- **安装**：`pip install -e ".[dev]"`
- **测试**：`pytest tests/ -v`

### openaaas-mcp-adapter

- **语言**：Python
- **包管理器**：uv（也兼容 pip）
- **已发布至 PyPI**：`pip install openaaas-mcp-adapter`
- **本地安装**：`pip install -e ".[dev]"` 或使用 `uv pip install -e ".[dev]"`
- **测试**：`pytest tests/ -v`

### pyopenaaas

- **语言**：Python
- **包管理器**：pip / setuptools
- **已发布至 PyPI**：`pip install pyopenaaas`
- **本地安装**：`pip install -e ".[dev]"`
- **测试**：`pytest tests/ -v`

### pi-extension

- **语言**：TypeScript (Node.js)
- **包管理器**：npm
- **Node 版本要求**：>= 18
- **安装**：`npm install`
- **文件**：`client-extension/pi-extension/index.ts`

## 更新 CHANGELOG

主要组件（server、agent-core、client-app）目录下有独立的 `CHANGELOG.md`，采用 [Keep a Changelog](https://keepachangelog.com/) 格式。

当你提交包含用户可见变更的 PR 时，如果你修改的组件包含 `CHANGELOG.md`，请在 `[Unreleased]` 区块下添加条目：

```markdown
## [Unreleased]

### Added
- 新增节点离线重连机制 (agent-core)

### Fixed
- 修复任务状态同步延迟问题 (server)
```

- `Added` — 新功能
- `Changed` — 现有功能的变更
- `Deprecated` — 即将移除的功能
- `Removed` — 已移除的功能
- `Fixed` — Bug 修复
- `Security` — 安全相关修复

纯文档更新、内部重构或测试补充通常不需要更新 CHANGELOG。

## 报告问题

### Bug 报告

如果你发现了 Bug，请使用 [Bug Report 模板](https://github.com/Wolido/OpenAaaS/issues/new?template=01-bug-report.yml) 提交 Issue。请尽量提供：

- 复现步骤
- 预期行为与实际行为
- 环境信息（操作系统、Rust/Node/Python 版本等）
- 相关日志或错误信息

### 功能请求

请使用 [Feature Request 模板](https://github.com/Wolido/OpenAaaS/issues/new?template=02-feature-request.yml) 提交。描述清楚：

- 你想要解决什么问题
- 你期望的解决方案
- 你考虑过的替代方案

### 部署问题

与部署、运维相关的问题请使用 [Deployment Issue 模板](https://github.com/Wolido/OpenAaaS/issues/new?template=03-deployment-issue.yml)。

### 安全问题

如果你发现了安全漏洞，**请不要公开提交 Issue**。请通过邮件私下报告：

**security@open-aaas.com**

## 社区与沟通

- **GitHub Discussions**：[https://github.com/Wolido/OpenAaaS/discussions](https://github.com/Wolido/OpenAaaS/discussions) — 提问、讨论架构、分享使用经验
- **官网**：[https://www.open-aaas.com](https://www.open-aaas.com) — 文档和使用指南
- **论文**：[arXiv:2605.13618](https://arxiv.org/abs/2605.13618) — 项目的技术背景与设计理念

再次感谢你的贡献！
