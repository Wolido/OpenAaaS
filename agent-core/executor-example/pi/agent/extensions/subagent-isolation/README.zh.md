<p align="right"><a href="./README.md">English</a> | 中文</p>

# subagent-isolation

> 一个 `pi` 扩展，将任务委托给专门的 subagent，在隔离的 `pi` 进程中运行，让每个 subagent 拥有独立的干净上下文窗口。

## 功能特性

- **三种调用模式**
  - `single` - 调用一个 agent 执行单个任务
  - `parallel` - 最多并发运行 8 个任务（每次最多 4 个）
  - `chain` - 按顺序运行多个 agent，通过 `{previous}` 注入上一步的输出
- **Agent 发现与作用域**
  - `user` agents 来自 `~/.pi/agent/agents/`
  - `project` agents 来自 `.pi/agents/`（从工作目录向上搜索）
  - `both` — 合并两个作用域（项目名称冲突时 project 覆盖 user）
  - 运行仓库本地 agent 前可选 `confirmProjectAgents` 确认提示
  - 运行时 `agentScope` 默认为 `"both"`
- **进程隔离与深度控制**
  - 每个 subagent 都会启动一个全新的 `pi --mode json` 进程
  - 每个 subagent 的 system prompt 被写入临时文件，并通过 `--append-system-prompt` 传入
  - 最大递归深度：`2`
  - 在 frontmatter 中设置 per-agent `canDelegate: false` 可阻止进一步委托
- **Skill 隔离**
  - 可使用 `--no-skills` 清除全局 skills
  - per-agent `skills` 列表仅注入指定的 skill，通过 `--skill <path>` 传入
- **超时与优雅终止**
  - 静默超时：15 分钟（stdout 空闲时终止）
  - 硬超时：1 小时
  - `AbortSignal` 触发 `SIGTERM` → 5 秒后 `SIGKILL`
- **TUI 渲染**
  - 实时状态，支持可折叠输出（`Ctrl+O`）
  - Token 用量统计：input / output / cacheRead / cacheWrite / cost / model / turns

## 安装

需要已安装 `pi` 并使其在 `$PATH` 中可用。

将扩展克隆（或复制）到 `pi` 的扩展目录：

```bash
git clone https://github.com/your-username/subagent-isolation.git \
  ~/.pi/agent/extensions/subagent-isolation
```

或手动放置文件，确保 `~/.pi/agent/extensions/subagent-isolation/index.ts` 存在。

## 使用示例

通过 `subagent` 工具调用 agent。必须且只能提供 `agent`/`task`、`tasks` 或 `chain` 中的一种。

### Single 模式

```json
{
  "agent": "coder",
  "task": "Refactor the auth middleware to use async/await.",
  "cwd": "/optional/working/dir"
}
```

### Parallel 模式

```json
{
  "tasks": [
    { "agent": "reviewer", "task": "Review src/auth.ts", "cwd": "src" },
    { "agent": "reviewer", "task": "Review src/db.ts" },
    { "agent": "tester",   "task": "Write unit tests for auth.js" }
  ],
  "agentScope": "both"
}
```

### Chain 模式

```json
{
  "chain": [
    { "agent": "planner", "task": "Design a REST API for user profiles.", "cwd": "src" },
    { "agent": "coder",   "task": "Implement the API based on this plan: {previous}" },
    { "agent": "reviewer", "task": "Review the implementation: {previous}" }
  ]
}
```

如果 chain 中的任何一步失败，执行会立即停止，剩余的步骤不会运行。

## Agent 定义格式

Agent 以 Markdown 文件（`.md`）形式定义在 agents 目录中。Frontmatter 描述 agent 属性；正文内容作为 system prompt。

```markdown
---
name: coder
description: Writes clean TypeScript and handles refactors.
tools: read,edit,write,bash
model: claude-3-7-sonnet
skills: /path/to/skill1, /path/to/skill2
---

You are a senior TypeScript engineer. Prefer async/await, avoid callbacks.
```

### Frontmatter 字段

| 字段 | 类型 | 说明 |
|-------|------|-------------|
| `name` | `string` | **必填。** 在工具调用中使用的唯一标识符。 |
| `description` | `string` | **必填。** 在发现 / 错误消息中显示的简短摘要。 |
| `tools` | `string[]`（逗号分隔） | 可选的 subagent 工具白名单。 |
| `model` | `string` | 可选的模型覆盖（例如 `claude-3-7-sonnet`）。 |
| `skills` | `string[]`（逗号分隔） | 可选的 skill 路径列表。如果存在，全局 skills 会被禁用，仅加载指定的 skills。路径可以是绝对路径或相对于工作目录的路径。 |
| `canDelegate` | `boolean` | 默认为 `true`。设置为 `false` 可阻止该 agent 启动进一步的 subagent。 |

## 环境变量

这些变量控制运行时行为。它们会自动传播到每个 subagent 进程中。

| 变量 | 默认值 | 说明 |
|----------|---------|-------------|
| `PI_SUBAGENT_DEPTH` | `0` | 当前递归深度。每次嵌套调用自动递增。硬上限为 `2`。 |
| `PI_CAN_DELEGATE` | `true` | 当前 agent 是否允许委托。派生自 agent 的 `canDelegate` frontmatter。 |
| `PI_CURRENT_AGENT_NAME` | — | 当前 agent 的名称，注入到每个 subagent 进程中。 |
| `PI_SUBAGENT_SILENCE_TIMEOUT_MS` | `900000`（15 分钟） | stdout 空闲时允许的最大时间，超过则终止 subagent。 |
| `PI_SUBAGENT_HARD_TIMEOUT_MS` | `3600000`（1 小时） | 单次 subagent 调用的绝对最大运行时间。 |

## 项目结构

```
~/.pi/agent/extensions/subagent-isolation/
├── index.ts      # 主扩展源码（约 1,280 行）
└── README.md     # 本文件
```

## 许可证

MIT
