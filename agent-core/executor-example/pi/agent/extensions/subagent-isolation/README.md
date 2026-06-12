# subagent-isolation

> A `pi` extension that delegates tasks to specialized subagents in isolated `pi` processes, giving each subagent its own clean context window.

## Features

- **Three invocation modes**
  - `single` - call one agent with a task
  - `parallel` - run up to 8 tasks concurrently (max 4 at a time)
  - `chain` - run agents sequentially, injecting the previous step's output via `{previous}`
- **Agent discovery & scoping**
  - `user` agents from `~/.pi/agent/agents/`
  - `project` agents from `.pi/agents/` (searched upward from the working directory)
  - `both` — merges both scopes (project overrides user on name collisions)
  - Optional `confirmProjectAgents` prompt before running repo-local agents
  - `agentScope` defaults to `"both"` at runtime
- **Process isolation & depth control**
  - Every subagent spawns a fresh `pi --mode json` process
  - Each subagent’s system prompt is written to a temporary file and passed via `--append-system-prompt`
  - Max recursion depth: `2`
  - Per-agent `canDelegate: false` in frontmatter blocks further delegation
- **Skill isolation**
  - Global skills can be cleared with `--no-skills`
  - Per-agent `skills` list injects only the specified skills via `--skill <path>`
- **Timeout & graceful termination**
  - Activity timeout: 10 minutes (kills if stdout is idle)
  - Hard timeout: 30 minutes
  - `AbortSignal` triggers `SIGTERM` → `SIGKILL` after 5 seconds
- **TUI rendering**
  - Real-time status with collapsible output (`Ctrl+O`)
  - Token usage stats: input / output / cacheRead / cacheWrite / cost / model / turns

## Installation

Requires `pi` to be installed and available on your `$PATH`.

Clone (or copy) the extension into your `pi` extensions directory:

```bash
git clone https://github.com/your-username/subagent-isolation.git \
  ~/.pi/agent/extensions/subagent-isolation
```

Or manually place the files so that `~/.pi/agent/extensions/subagent-isolation/index.ts` exists.

## Usage Examples

Agents are invoked via the `subagent` tool. Provide exactly one of `agent`/`task`, `tasks`, or `chain`.

### Single mode

```json
{
  "agent": "coder",
  "task": "Refactor the auth middleware to use async/await.",
  "cwd": "/optional/working/dir"
}
```

### Parallel mode

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

### Chain mode

```json
{
  "chain": [
    { "agent": "planner", "task": "Design a REST API for user profiles.", "cwd": "src" },
    { "agent": "coder",   "task": "Implement the API based on this plan: {previous}" },
    { "agent": "reviewer", "task": "Review the implementation: {previous}" }
  ]
}
```

If any step in the chain fails, execution stops immediately and the remaining steps are not run.

## Agent Definition Format

Agents are defined as Markdown files (`.md`) in the agents directory. Frontmatter describes the agent; the body becomes the system prompt.

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

### Frontmatter fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | **Required.** Unique identifier used in tool calls. |
| `description` | `string` | **Required.** Short summary shown in discovery / error messages. |
| `tools` | `string[]` (comma-separated) | Optional tool whitelist for the subagent. |
| `model` | `string` | Optional model override (e.g. `claude-3-7-sonnet`). |
| `skills` | `string[]` (comma-separated) | Optional list of skill paths. If present, global skills are disabled and only these are loaded. Paths can be absolute or relative to the working directory. |
| `canDelegate` | `boolean` | Defaults to `true`. Set to `false` to prevent this agent from spawning further subagents. |

## Environment Variables

These variables control runtime behavior. They are propagated into every subagent process automatically.

| Variable | Default | Description |
|----------|---------|-------------|
| `PI_SUBAGENT_DEPTH` | `0` | Current recursion depth. Automatically incremented per nested invocation. Hard cap is `2`. |
| `PI_CAN_DELEGATE` | `true` | Whether the current agent is allowed to delegate. Derived from the agent's `canDelegate` frontmatter. |
| `PI_CURRENT_AGENT_NAME` | — | Name of the current agent, injected into every subagent process. |
| `PI_SUBAGENT_ACTIVITY_TIMEOUT_MS` | `600000` (10 min) | Max allowed idle time on stdout before the subagent is killed. |
| `PI_SUBAGENT_HARD_TIMEOUT_MS` | `1800000` (30 minutes) | Absolute maximum runtime for a single subagent call. |

## Project Structure

```
~/.pi/agent/extensions/subagent-isolation/
├── index.ts      # Main extension source (~1,280 lines)
└── README.md     # This file
```

## License

MIT
