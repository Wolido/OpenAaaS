# OpenAaaS Executor Example

<p align="right">中文 | <a href="./README.en.md">English</a></p>

这是 **OpenAaaS 的一个基于智能体的 Docker 执行器镜像示例**。

> 这个容器内部运行的是 **pi-coding-agent（一个 LLM 智能体）**。它不是简单的确定性脚本流水线，而是由 `run.sh` 从 `/workspace/task.json` 中提取 `task_prompt` 与 `output_prompt`，交由智能体理解任务意图、自主选择工具并完成任务的执行环境。

Agent Core 通过 Docker 容器隔离执行任务。本示例展示了**最小的交互契约**：`run.sh` 读取 `task.json` 并调用智能体执行任务，智能体将结果文件写入 workspace。你可以直接基于它修改，也可以完全自己写一个镜像——只要满足同样的输入输出协议即可。

---

## 交互契约

这是 Agent Core 与容器之间的唯一约定。无论你是否基于本示例修改，只要遵守这个契约，Agent Core 就能正确调度你的镜像。

### 输入

Agent Core 启动容器时，会完成以下准备：

- 将 `task.json` 放在容器的 `/workspace/task.json`
- 将输入文件放在 `/workspace/input/`
- 传入两个环境变量：`TASK_ID`（任务 ID）和 `TIMEOUT`（超时秒数）

`task.json` 包含以下字段：

| 字段 | 说明 |
|------|------|
| `task_id` | 任务唯一标识 |
| `task_prompt` | 用户原始任务描述 |
| `prompt` | 本示例未读取；Agent Core 可能同时传入该字段，但 `run.sh` 仅使用 `task_prompt` |
| `output_prompt` | 对输出格式/内容的要求 |
| `session_id` | 会话标识 |
| `input_files` | 输入文件名列表 |

### 输出

执行完成后，把结果文件放在 workspace 下即可（推荐放在 `/workspace/output/` 下）。Agent Core 会扫描 workspace 下所有文件（排除 `task.json` 和 `input/`），作为输出文件上报 Server。

---

## 架构概述

```
Agent Core  →  创建 workspace + task.json + input/  →  docker run
                                                       │
                                                       ▼
                                                  ┌─────────────┐
                                                  │   容器内部   │
                                                  │             │
                                                  │ entrypoint  │
                                                  │    .sh      │
                                                  │     │       │
                                                  │     ▼       │
                                                  │   run.sh    │
                                                  │     │       │
                                                  │     ▼       │
                                                  │ pi-coding   │
                                                  │   agent     │
                                                  │  (LLM 智能体)│
                                                  │  /  │  \    │
                                                  │ read write  │
                                                  │ bash ls ... │
                                                  │     │       │
                                                  └─────┼───────┘
                                                       │
                                                       ▼
Agent Core  ←  扫描输出文件上报 Server  ←  结果写入 workspace
```

---

## 智能体如何执行任务

容器启动后，内部执行流程如下：

1. **`entrypoint.sh` 启动容器**
   - 检查 `/workspace/task.json` 是否存在
   - 输出任务 ID 与超时时间
   - 调用 `/opt/run.sh`

2. **`run.sh` 解析 `task.json` 并准备两阶段调用**
   - 从 `task.json` 中提取 `task_prompt` 与 `output_prompt`
   - 将 `task_prompt` 注入第一阶段提示词模板
   - 准备第二阶段格式整理提示词（第二阶段始终执行，不受 `output_prompt` 是否为空影响）

3. **第一阶段：pi-coding-agent 执行任务**
   - `run.sh` 以 `/opt/main-agent.md` 作为追加系统提示词启动智能体
   - 智能体根据 `task_prompt` 自主检查 `/workspace/input/` 中的输入文件并选择工具
   - 任务执行过程的输出会同时 `tee` 到 `/workspace/step1.log`
   - 执行结果写入 `/workspace/output/`（推荐保存为 `response.md`）

4. **第二阶段：按 `output_prompt` 整理输出**
   - 第一阶段执行完成后总是触发
   - `run.sh` 再次调用 pi-coding-agent，要求它读取 `/workspace/output/` 并根据 `output_prompt` 重新整理 `/workspace/output/response.md`；若 `output_prompt` 为空，智能体可能仅做检查或保持文件不变
   - 此阶段允许失败；失败后仍会继续执行后续兜底逻辑

5. **`run.sh` 确保最终输出**
   - 如果 `/workspace/output/response.md` 不存在，则将 `/workspace/step1.log` 复制为兜底文件
   - 将最终响应复制到 `/workspace/response.md`

---

## 构建示例镜像

```bash
cd OpenAaaS/agent-core/executor-example
docker build -t open-aaas-executor:latest .
```

> 镜像名（如 `open-aaas-executor:latest`）需要与 `agent-core` 的 `config.toml` 中 `executor.image` 配置保持一致，否则 Agent Core 无法正确调度。

---

## 本示例包含什么

| 文件 | 说明 |
|------|------|
| `Dockerfile` | 示例镜像定义。基于 `node:22-slim`，安装 `jq`/`git`/`python3` 等常用工具，并全局安装 `pi-coding-agent` |
| `entrypoint.sh` | 容器入口脚本，检查 `task.json` 存在后调用 `run.sh` |
| `run.sh` | **智能体调用脚本**。它负责解析 `task.json`、构造提示词并启动 pi-coding-agent |
| `main-agent.md` | `run.sh` 中通过 `--append-system-prompt` 追加给 pi 的系统提示词，用于约束智能体行为 |
| `pi/` | pi-coding-agent 的配置目录，会被复制到容器内 `/home/executor/.pi/` |

> 核心关系：`Dockerfile` 安装 pi 运行时 → `entrypoint.sh` 启动 → `run.sh` 调用 pi → `main-agent.md` 与 `pi/` 共同配置智能体行为。

---

## 自定义

### 方式一：基于本示例修改（推荐）

这是最快的上手方式。由于本示例的核心是**智能体执行**，建议优先从智能体层面调整：

- **修改 `main-agent.md`**：调整系统提示词，改变智能体在这个执行环境中的行为、输出格式、工具使用偏好等
- **修改 `run.sh`**：调整任务提示词模板、调用 pi 的参数，或者替换为其他 Agent 框架（如 Kimi Cli、Open Code、Codex 等）
- **修改 `pi/`**：调整 pi-coding-agent 的配置，例如可用模型、工具白名单等
- **修改 `Dockerfile`**：增减依赖、换基础镜像
- **删除不需要的文件**：如果不用 pi，删掉 `pi/` 目录和 `main-agent.md`

### 方式二：从零构建自己的镜像

你也可以完全自己写一个镜像，只需要满足交互契约即可：

1. 写一个 Dockerfile，安装你需要的运行环境和 Agent 框架
2. 写一个入口脚本（或直接写 ENTRYPOINT），让 agent 读取 `/workspace/task.json` 并执行任务
3. 核心要求：agent 执行完成后，将结果文件写入 workspace

Agent Core 不关心容器内部怎么实现，只关心输出文件是否按要求出现在 workspace 中。

---

## 安全提醒

`pi/agent/models.json` 包含敏感 API key，**不要提交到 Git**。

推荐通过 `agent-core` 的 `config.toml` 在运行时注入：

```toml
[[paths.mounts]]
host = "~/.pi/agent/models.json"
container = "/home/executor/.pi/agent/models.json"
readonly = true
```

这样可以避免将 API key 打包进镜像，确保密钥与镜像分离。
