# 贡献指南

*Contributing Guide*

感谢你对 OpenAaaS 的兴趣！本指南将帮助你快速了解如何参与这个项目的开发。

Thank you for your interest in OpenAaaS! This guide will help you quickly understand how to get involved in the development of this project.

OpenAaaS 是一个面向科研领域的 Agent 网络基础设施，核心理念是**"数据原位驻留，能力跨节点流动"**。无论你是修复 Bug、添加新功能、改进文档，还是提出想法，我们都欢迎你的贡献。

OpenAaaS is an Agent network infrastructure for scientific research. Its core philosophy is **"data remains in place, capabilities flow across nodes"**. Whether you are fixing bugs, adding new features, improving documentation, or sharing ideas, your contributions are welcome.

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
      - [openaaas-mcp-adapter（MCP 适配器）](#openaaas-mcp-adaptermcp-适配器)
      - [pi-extension（PI 扩展）](#pi-extensionpi-扩展)
      - [pyopenaaas（Python SDK）](#pyopenaaaspython-sdk)
  - [提交规范](#提交规范)
    - [Commit Message 格式](#commit-message-格式)
    - [常用 type](#常用-type)
    - [组件作用域（scope）](#组件作用域scope)
    - [示例](#示例)
  - [Pull Request 流程](#pull-request-流程)
    - [分支策略](#分支策略)
    - [本地 GitHub CLI 和贡献者归属](#本地-github-cli-和贡献者归属)
    - [PR 前检查清单](#pr-前检查清单)
    - [PR 标题和描述](#pr-标题和描述)
  - [各组件开发指南](#各组件开发指南)
    - [server](#server)
    - [agent-core](#agent-core)
    - [client-app](#client-app)
    - [dash](#dash)
    - [openaaas-mcp-adapter](#openaaas-mcp-adapter)
    - [pyopenaaas](#pyopenaaas)
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

*Before You Begin*

在提交 Issue 或开始编写代码前，请先花几分钟确认：

Before submitting an Issue or writing code, please take a few minutes to confirm the following:

1. **搜索现有 Issue**：查看是否有人已经报告过相同的问题或提出过类似的功能请求。<br>**Search existing Issues**: Check if someone has already reported the same problem or made a similar feature request.
2. **浏览 Discussions**：一般性使用问题、架构讨论和想法交流请优先在 [GitHub Discussions](https://github.com/Wolido/OpenAaaS/discussions) 中进行。<br>**Browse Discussions**: For general usage questions, architecture discussions, and idea exchanges, please use [GitHub Discussions](https://github.com/Wolido/OpenAaaS/discussions) first.
3. **阅读相关文档**：确认你的问题是否已在 [官网文档](https://www.open-aaas.com) 中有解答。<br>**Read relevant documentation**: Confirm whether your question is already answered in the [official documentation](https://www.open-aaas.com).

如果你确定这是一个新的 Bug 或合理的功能请求，欢迎直接提交 Issue。对于小的文档修复或明显的 Bug，可以直接提交 PR，不必先开 Issue。

If you are sure this is a new bug or a reasonable feature request, feel free to submit an Issue directly. For small documentation fixes or obvious bugs, you can submit a PR directly without opening an Issue first.

## 开发环境搭建

*Development Environment Setup*

### 必备工具

*Required Tools*

| 工具<br>Tool | 版本要求<br>Version | 说明<br>Description |
|------|---------|------|
| Rust | stable（server / agent-core 使用 edition 2024）<br>stable (server / agent-core use edition 2024) | server、agent-core、client-app/src-tauri 的构建需要<br>Required for building server, agent-core, and client-app/src-tauri |
| Node.js | 20 | client-app 前端构建需要<br>Required for building the client-app frontend |
| Python | 3.10+ | dash、openaaas-mcp-adapter、pi-extension 需要<br>Required for dash, openaaas-mcp-adapter, pi-extension |
| Docker | 最新稳定版<br>Latest stable | agent-core 使用 Docker 进行沙箱隔离<br>agent-core uses Docker for sandbox isolation |

### 可选但推荐的工具

*Optional but Recommended Tools*

- **uv**：Python 包管理和虚拟环境工具，openaaas-mcp-adapter 已采用 uv<br>**uv**: Python package manager and virtual environment tool; openaaas-mcp-adapter already uses uv
- **cargo-watch**：Rust 开发时的热重载<br>**cargo-watch**: Hot reload for Rust development

## 快速上手

*Quick Start*

### 克隆仓库

*Clone the Repository*

```bash
git clone https://github.com/Wolido/OpenAaaS.git
cd OpenAaaS
```

本项目为多语言多组件架构，各组件可独立开发。你不需要一次性构建全部组件，只需关注你要修改的部分。

This project has a multi-language, multi-component architecture, and each component can be developed independently. You do not need to build all components at once—just focus on the parts you want to modify.

### 各组件构建与运行

*Build and Run Each Component*

#### server（网络枢纽）

*server (Network Hub)*

```bash
cd server
cargo build --release
./target/release/open-aaas-server run
```

首次启动会自动生成 `config.toml` 和 SQLite 数据库。

The first startup will automatically generate `config.toml` and an SQLite database.

#### agent-core（网络节点）

*agent-core (Network Node)*

```bash
cd agent-core
cargo build --release
./target/release/agent-core init
./target/release/agent-core register --token <token> --name my-agent
./target/release/agent-core run
```

#### client-app（桌面客户端）

*client-app (Desktop Client)*

```bash
cd client-app
npm run tauri:dev
```

这将同时启动 Vue 前端和 Tauri Rust 后端。

This will start both the Vue frontend and the Tauri Rust backend simultaneously.

#### dash（调试/管理员工具）

*dash (Debug / Admin Tool)*

```bash
cd dash
pip install -e ".[dev]"
aaas-dashboard
```

`aaas-dashboard` 是 `pyproject.toml` 中 `[project.scripts]` 定义的入口脚本。

`aaas-dashboard` is the entry script defined in `[project.scripts]` in `pyproject.toml`.

#### openaaas-mcp-adapter（MCP 适配器）

*openaaas-mcp-adapter (MCP Adapter)*

**Python 扩展**（openaaas-mcp-adapter）各目录下有独立的 `pyproject.toml`，安装方式：

**Python extensions** (openaaas-mcp-adapter) each have their own `pyproject.toml` in their respective directories. Installation:

```bash
# openaaas-mcp-adapter（已发布至 PyPI）
# openaaas-mcp-adapter (published on PyPI)
cd openaaas-mcp-adapter
pip install -e ".[dev]"
```

#### pi-extension（PI 扩展）

*pi-extension (PI Extension)*

**Node 扩展**（pi-extension）：

**Node extension** (pi-extension):

```bash
cd pi-extension
npm install
```

#### pyopenaaas（Python SDK）

*pyopenaaas (Python SDK)*

Python SDK 用于与 OpenAaaS 网络交互。

Python SDK for interacting with the OpenAaaS network.

```bash
cd pyopenaaas
pip install -e ".[dev]"
pytest tests/ -v
```

## 提交规范

*Commit Guidelines*

本项目采用 [Conventional Commits](https://www.conventionalcommits.org/) 规范。所有 PR 的标题必须符合此格式。

This project follows the [Conventional Commits](https://www.conventionalcommits.org/) specification. All PR titles must conform to this format.

### Commit Message 格式

*Commit Message Format*

```
<type>(<scope>): <简短描述>

<可选的正文，说明变更动机和具体实现>

<可选的脚注，如 BREAKING CHANGE、Fixes #123>
```

```
<type>(<scope>): <short description>

<optional body explaining motivation and implementation>

<optional footer, e.g. BREAKING CHANGE, Fixes #123>
```

### 常用 type

*Common Types*

| type | 含义<br>Meaning |
|------|------|
| `feat` | 新功能<br>New feature |
| `fix` | 修复 Bug<br>Bug fix |
| `docs` | 仅文档变更<br>Documentation changes only |
| `style` | 代码格式调整（不影响逻辑）<br>Code style changes (no logic impact) |
| `refactor` | 重构（既不是新增功能也不是修复 Bug）<br>Refactoring (neither feature nor bug fix) |
| `perf` | 性能优化<br>Performance improvement |
| `test` | 测试相关<br>Tests |
| `chore` | 构建过程、辅助工具、依赖更新等<br>Build process, tooling, dependency updates |

### 组件作用域（scope）

*Component Scope*

scope 用于标识变更所在的组件。常用的 scope 包括：

Scope identifies the component where the change occurs. Commonly used scopes include:

- `server` — 网络枢纽（调度中心）<br>Network hub (scheduling center)
- `agent-core` — 网络节点（执行节点）<br>Network node (execution node)
- `client-app` — 桌面客户端<br>Desktop client
- `dash` — 调试/管理员工具<br>Debug / admin tool

- `mcp-adapter` — MCP 适配器<br>MCP adapter
- `pi-extension` — pi 扩展<br>pi extension
- `pyopenaaas` — Python SDK<br>Python SDK
- `docs` — 项目级文档<br>Project-level documentation
- `ci` — CI/CD 配置<br>CI/CD configuration
- `release` — 发版流程与 CI/CD<br>Release process and CI/CD
- `root` — 顶层配置（README、LICENSE 等）<br>Top-level configuration (README, LICENSE, etc.)

### 示例

*Examples*

**中文示例 / Chinese Example**

```
feat(server): 新增任务调度优先级队列

fix(agent-core): 修复沙箱容器泄漏问题

docs(client-app): 更新 README 中的安装说明

chore(ci): 升级 GitHub Actions 缓存版本
```

**English Example**

```
feat(server): add task scheduling priority queue

fix(agent-core): fix sandbox container leak

docs(client-app): update installation instructions in README

chore(ci): upgrade GitHub Actions cache version
```

## Pull Request 流程

*Pull Request Workflow*

### 分支策略

*Branch Strategy*

1. 从 `main` 分支切出你的功能分支：<br>Create your feature branch from `main`:

   ```bash
   git checkout -b feat/server-add-priority-queue
   ```

2. 在你的分支上进行开发，保持提交历史整洁（必要时可 `git rebase -i` 整理）。<br>Develop on your branch and keep the commit history clean (use `git rebase -i` if necessary).

3. 推送到你的 fork 或远程分支，然后向 `main` 分支提交 PR。<br>Push to your fork or remote branch, then submit a PR to the `main` branch.

### 本地 GitHub CLI 和贡献者归属

*Local GitHub CLI and Contributor Attribution*

推荐使用 GitHub CLI 创建和检查 PR，并使用能关联到 GitHub 账号的提交作者邮箱，避免贡献记录无法归属到个人账号。<br>
We recommend using GitHub CLI to create and inspect PRs, and using a commit author email that GitHub can associate with your account so contribution records are attributed correctly.

1. 登录 GitHub CLI，并让 Git 操作继续使用 SSH：<br>Log in to GitHub CLI and keep Git operations on SSH:

   ```bash
   gh auth login --hostname github.com --git-protocol ssh --web
   gh auth status
   gh config get git_protocol -h github.com
   ```

2. 为当前仓库配置 Git 作者信息。若不希望暴露私人邮箱，推荐使用 GitHub noreply 邮箱。<br>Configure the Git author for the current repository. If you do not want to expose a private email, use your GitHub noreply email.

   ```bash
   git config user.name "<your-github-username>"
   git config user.email "<your-id>+<your-github-username>@users.noreply.github.com"
   git config user.name
   git config user.email
   ```

   不要使用机器本地邮箱，例如 `user@MacBook.local`，这类邮箱无法被 GitHub 关联到贡献者账号。<br>
   Do not use machine-local emails such as `user@MacBook.local`; GitHub cannot associate them with contributor accounts.

3. 提交前确认最近一次提交的作者会被正确归属：<br>Before opening a PR, confirm the latest commit author is attributable:

   ```bash
   git log -1 --format='%an <%ae>'
   ```

4. 推送分支并使用 GitHub CLI 创建 PR：<br>Push the branch and create the PR with GitHub CLI:

   ```bash
   git push -u fork <branch-name>
   gh pr create \
     --repo Wolido/OpenAaaS \
     --base main \
     --head <your-github-username>:<branch-name> \
     --title "<type>(<scope>): <summary>" \
     --body-file <pr-description-file>
   ```

5. 创建后检查 PR 方向、CI、评论和合并状态：<br>After creation, check the PR direction, CI, comments, and merge state:

   ```bash
   gh pr view <pr-number> --repo Wolido/OpenAaaS \
     --json number,title,state,author,headRefName,baseRefName,mergeStateStatus,statusCheckRollup,url
   ```

### PR 前检查清单

*PR Checklist*

提交 PR 前，请确认以下事项：

Before submitting a PR, please confirm the following:

- [ ] 代码改动已测试通过<br>Code changes have been tested
- [ ] 对应组件的 `CHANGELOG.md` 已更新（在 `[Unreleased]` 下添加了条目）<br>The component's `CHANGELOG.md` has been updated (entry added under `[Unreleased]`)
- [ ] 没有引入无关的文件改动<br>No unrelated file changes have been introduced
- [ ] PR 标题符合 Conventional Commits 规范（如 `feat(agent-core): xxx`）<br>PR title conforms to Conventional Commits (e.g. `feat(agent-core): xxx`)

### PR 标题和描述

*PR Title and Description*

- **标题**：使用 Conventional Commits 格式，如 `feat(server): 新增节点健康检查接口`<br>**Title**: Use the Conventional Commits format, e.g. `feat(server): add node health check endpoint`
- **描述**：简要说明变更内容、动机和影响。如果修复了某个 Issue，请在正文中引用，如 `Fixes #123`。<br>**Description**: Briefly explain the changes, motivation, and impact. If an Issue is fixed, reference it in the body, e.g. `Fixes #123`.

## 各组件开发指南

*Component Development Guides*

### server

- **语言 / Language**：Rust（Axum 0.8 + Tokio + SQLx + SQLite）
- **包管理器 / Package Manager**：Cargo
- **构建 / Build**：`cargo build`
- **测试 / Test**：`cargo test`
- **代码风格 / Code Style**：遵循 `cargo fmt` 和 `cargo clippy` 的默认规则<br>Follow the default rules of `cargo fmt` and `cargo clippy`

### agent-core

- **语言 / Language**：Rust（Tokio + Docker 沙箱隔离）<br>Rust (Tokio + Docker sandbox isolation)
- **包管理器 / Package Manager**：Cargo
- **构建 / Build**：`cargo build`
- **测试 / Test**：`cargo test --features test-utils`
- **注意 / Note**：运行完整测试需要本地 Docker 环境可用<br>Full tests require a local Docker environment

### client-app

- **语言 / Language**：TypeScript / Vue 3 + Rust（Tauri）<br>TypeScript / Vue 3 + Rust (Tauri)
- **包管理器 / Package Manager**：npm（前端）+ Cargo（Tauri 后端）<br>npm (frontend) + Cargo (Tauri backend)
- **前端测试 / Frontend Tests**：`npm ci && npm run test`
- **Tauri 后端测试 / Tauri Backend Tests**：`cd src-tauri && cargo test`
- **开发启动 / Dev Startup**：`npm run tauri:dev`

### dash

- **语言 / Language**：Python + Streamlit
- **包管理器 / Package Manager**：pip / hatchling
- **安装 / Install**：`pip install -e ".[dev]"`
- **测试 / Test**：`pytest tests/ -v`
- **启动 / Start**：`aaas-dashboard`
- **注意 / Note**：CI 中 dash 的测试仅在 Linux 上运行<br>dash tests in CI only run on Linux

### openaaas-mcp-adapter

- **语言 / Language**：Python
- **包管理器 / Package Manager**：uv（也兼容 pip）<br>uv (also compatible with pip)
- **已发布至 PyPI / Published on PyPI**：`pip install openaaas-mcp-adapter`
- **本地安装 / Local Install**：`pip install -e ".[dev]"` 或使用 `uv pip install -e ".[dev]"`<br>or use `uv pip install -e ".[dev]"`
- **测试 / Test**：`pytest tests/ -v`

### pyopenaaas

- **语言 / Language**：Python
- **包管理器 / Package Manager**：pip / setuptools
- **已发布至 PyPI / Published on PyPI**：`pip install pyopenaaas`
- **本地安装 / Local Install**：`pip install -e ".[dev]"`
- **测试 / Test**：`pytest tests/ -v`

### pi-extension

- **语言 / Language**：TypeScript (Node.js)
- **包管理器 / Package Manager**：npm
- **Node 版本要求 / Node Version Requirement**：>= 18
- **安装 / Install**：`npm install`
- **文件 / File**：`pi-extension/index.ts`

## 更新 CHANGELOG

*Updating the CHANGELOG*

主要组件（server、agent-core、client-app）目录下有独立的 `CHANGELOG.md`，采用 [Keep a Changelog](https://keepachangelog.com/) 格式。

Major components (server, agent-core, client-app) each have their own `CHANGELOG.md`, following the [Keep a Changelog](https://keepachangelog.com/) format.

当你提交包含用户可见变更的 PR 时，如果你修改的组件包含 `CHANGELOG.md`，请在 `[Unreleased]` 区块下添加条目：

When you submit a PR that includes user-visible changes, if the component you modified contains a `CHANGELOG.md`, please add an entry under the `[Unreleased]` section:

**中文示例 / Chinese Example**

```markdown
## [Unreleased]

### Added
- 新增节点离线重连机制 (agent-core)

### Fixed
- 修复任务状态同步延迟问题 (server)
```

**English Example**

```markdown
## [Unreleased]

### Added
- Add node offline reconnection mechanism (agent-core)

### Fixed
- Fix task status synchronization delay (server)
```

- `Added` — 新功能 / New features
- `Changed` — 现有功能的变更 / Changes to existing functionality
- `Deprecated` — 即将移除的功能 / Soon-to-be removed features
- `Removed` — 已移除的功能 / Removed features
- `Fixed` — Bug 修复 / Bug fixes
- `Security` — 安全相关修复 / Security-related fixes

纯文档更新、内部重构或测试补充通常不需要更新 CHANGELOG。

Pure documentation updates, internal refactoring, or test additions usually do not require updating the CHANGELOG.

## 报告问题

*Reporting Issues*

### Bug 报告

*Bug Reports*

如果你发现了 Bug，请使用 [Bug Report 模板](https://github.com/Wolido/OpenAaaS/issues/new?template=01-bug-report.yml) 提交 Issue。请尽量提供：

If you discover a bug, please submit an Issue using the [Bug Report template](https://github.com/Wolido/OpenAaaS/issues/new?template=01-bug-report.yml). Please try to provide:

- 复现步骤<br>Reproduction steps
- 预期行为与实际行为<br>Expected behavior and actual behavior
- 环境信息（操作系统、Rust/Node/Python 版本等）<br>Environment information (OS, Rust/Node/Python versions, etc.)
- 相关日志或错误信息<br>Relevant logs or error messages

### 功能请求

*Feature Requests*

请使用 [Feature Request 模板](https://github.com/Wolido/OpenAaaS/issues/new?template=02-feature-request.yml) 提交。描述清楚：

Please submit using the [Feature Request template](https://github.com/Wolido/OpenAaaS/issues/new?template=02-feature-request.yml). Clearly describe:

- 你想要解决什么问题<br>The problem you want to solve
- 你期望的解决方案<br>Your expected solution
- 你考虑过的替代方案<br>Alternative solutions you have considered

### 部署问题

*Deployment Issues*

与部署、运维相关的问题请使用 [Deployment Issue 模板](https://github.com/Wolido/OpenAaaS/issues/new?template=03-deployment-issue.yml)。

For issues related to deployment and operations, please use the [Deployment Issue template](https://github.com/Wolido/OpenAaaS/issues/new?template=03-deployment-issue.yml).

### 安全问题

*Security Issues*

如果你发现了安全漏洞，**请不要公开提交 Issue**。请通过 [GitHub Security Advisories](https://github.com/Wolido/OpenAaaS/security/advisories/new) 私下报告。

If you discover a security vulnerability, **please do not file a public issue**. Report it privately via [GitHub Security Advisories](https://github.com/Wolido/OpenAaaS/security/advisories/new).

## 社区与沟通

*Community and Communication*

- **GitHub Discussions**：[https://github.com/Wolido/OpenAaaS/discussions](https://github.com/Wolido/OpenAaaS/discussions) — 提问、讨论架构、分享使用经验<br>Q&A, architecture discussions, sharing usage experiences
- **官网 / Website**：[https://www.open-aaas.com](https://www.open-aaas.com) — 文档和使用指南<br>Documentation and user guides
- **论文 / Paper**：[arXiv:2605.13618](https://arxiv.org/abs/2605.13618) — 项目的技术背景与设计理念<br>Technical background and design philosophy of the project

再次感谢你的贡献！

Thank you again for your contribution!
