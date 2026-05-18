# OpenAaaS Executor Example

<p align="right"><a href="./README.zh.md">中文</a> | English</p>

This is **a sample Docker executor image for OpenAaaS**.

Agent Core executes tasks in isolated Docker containers. This example demonstrates the **minimal interaction contract**: the container reads `task.json`, executes the task, and writes result files to the workspace. You can directly modify based on it, or write your own image entirely — as long as it satisfies the same input/output protocol.

---

## Interaction Contract

This is the only agreement between Agent Core and the container. Whether or not you modify based on this example, as long as you follow this contract, Agent Core can correctly schedule your image.

### Input

When Agent Core starts the container, it completes the following preparations:

- Places `task.json` at the container's `/workspace/task.json`
- Places input files in `/workspace/input/`
- Passes two environment variables: `TASK_ID` (task ID) and `TIMEOUT` (timeout in seconds)

`task.json` contains the following fields:

| Field | Description |
|-------|-------------|
| `task_id` | Unique task identifier |
| `task_prompt` | User's original task description |
| `prompt` | Same as `task_prompt`, for backward compatibility |
| `output_prompt` | Requirements for output format/content |
| `session_id` | Session identifier |
| `input_files` | List of input file names |

### Output

After execution is complete, simply place result files under the workspace (recommended under `/workspace/output/`). Agent Core will scan all files under the workspace (excluding `task.json` and `input/`), and report them as output files to the Server.

---

## Architecture Overview

```
Agent Core  →  Create workspace + task.json + input/  →  docker run
                                                       │
                                                       ▼
                                                  Container Execution
                                                       │
                                                       ▼
Agent Core  ←  Scan output files to report to Server  ←  Results written to workspace
```

---

## Build Example Image

```bash
cd OpenAaaS/agent-core/executor-example
docker build -t open-aaas-executor:latest .
```

> The image name (e.g. `open-aaas-executor:latest`) must match the `executor.image` configuration in `agent-core`'s `config.toml`; otherwise Agent Core cannot correctly schedule it.

---

## What's Included in This Example

| File | Description |
|------|-------------|
| `Dockerfile` | Example image definition. Based on `node:22-slim`, installs `jq`/`git`/`python3` and other common tools |
| `entrypoint.sh` | Container entry script, checks for `task.json` existence then calls the execution script |
| `run.sh` | **Example execution logic**. In this example, it calls the pi-coding-agent agent framework to process tasks; you can directly replace it with other agent frameworks |
| `main-agent.md` | System prompt appended to pi in `run.sh`. If you don't use pi, this file can be ignored |
| `pi/` | Configuration directory for pi-coding-agent. If you don't use pi, you can delete it |

---

## Customization

### Method 1: Modify Based on This Example

This is the fastest way to get started:

- **Modify `run.sh`**: Replace execution logic, such as using other Agent frameworks (e.g. Kimi Cli, Open Code, Codex, etc.)
- **Modify `Dockerfile`**: Add or remove dependencies, change base image
- **Delete unnecessary files**: If not using pi, delete the `pi/` directory and `main-agent.md`

### Method 2: Build Your Own Image from Scratch

You can also write your own image entirely; just satisfy the interaction contract:

1. Write a Dockerfile, install the runtime environment and Agent framework you need
2. Write an entry script (or directly write ENTRYPOINT), have the agent read `/workspace/task.json` and execute the task
3. Core requirement: after the agent finishes executing, write result files to the workspace

Agent Core doesn't care about the internal implementation of the container; it only cares whether output files appear in the workspace as required.

---

## Security Reminder

`pi/agent/models.json` contains sensitive API keys, **do not commit to Git**.

Recommended to inject at runtime via `agent-core`'s `config.toml`:

```toml
[[paths.mounts]]
host = "~/.pi/agent/models.json"
container = "/home/executor/.pi/agent/models.json"
readonly = true
```

This avoids packaging API keys into the image, ensuring keys are separate from the image.
