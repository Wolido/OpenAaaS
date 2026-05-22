# OpenAaaS Executor Example

<p align="right"><a href="./README.zh.md">中文</a> | English</p>

这是 **OpenAaaS 的一个 Docker 执行器镜像示例**。

Agent Core 通过 Docker 容器隔离执行任务。这个示例展示了**最小的交互契约**：容器读取 `task.json`，执行任务，将结果文件写入 workspace。你可以直接基于它修改，也可以完全自己写一个镜像——只要满足同样的输入输出协议即可。

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
| `prompt` | 同 `task_prompt`，向后兼容 |
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
                                                  容器执行
                                                       │
                                                       ▼
Agent Core  ←  扫描输出文件上报 Server  ←  结果写入 workspace
```

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
| `Dockerfile` | 示例镜像定义。基于 `node:22-slim`，安装了 `jq`/`git`/`python3` 等常用工具 |
| `entrypoint.sh` | 容器入口脚本，检查 `task.json` 存在后调用执行脚本 |
| `run.sh` | **示例的执行逻辑**。本示例中用它调用 pi-coding-agent 这个 agent 框架处理任务，你可以直接替换为其他 agent 框架 |
| `main-agent.md` | `run.sh` 中给 pi 追加的系统提示词。如果你不用 pi，这个文件可以忽略 |
| `pi/` | pi-coding-agent 的配置目录。如果你不用 pi，可以删除 |

---

## 自定义

### 方式一：基于本示例修改

这是最快的上手方式：

- **修改 `run.sh`**：替换执行逻辑，比如使用其他 Agent 框架（如 Kimi Cli、Open Code、Codex 等）
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
