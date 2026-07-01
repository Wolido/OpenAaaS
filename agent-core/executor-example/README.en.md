# OpenAaaS Executor Example

<p align="right"><a href="./README.md">中文</a> | English</p>

This is **an agent-based Docker executor image example for OpenAaaS**.

> The container runs **pi-coding-agent (an LLM agent)**. It is not a simple deterministic script pipeline; instead, `run.sh` extracts `task_prompt` and `output_prompt` from `/workspace/task.json` and passes them to the agent, which then understands the task intent, chooses tools autonomously, and completes the task.

Agent Core executes tasks in isolated Docker containers. This example demonstrates the **minimal interaction contract**: `run.sh` reads `task.json` and invokes the agent to execute the task, and the agent writes result files to the workspace. You can directly modify based on it, or write your own image entirely — as long as it satisfies the same input/output protocol.

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
| `prompt` | Not read by this example; Agent Core may pass it alongside `task_prompt`, but `run.sh` only uses `task_prompt` |
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
                                                  ┌─────────────┐
                                                  │ In Container│
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
                                                  │ (LLM agent) │
                                                  │  /  │  \    │
                                                  │ read write  │
                                                  │ bash ls ... │
                                                  │     │       │
                                                  └─────┼───────┘
                                                       │
                                                       ▼
Agent Core  ←  Scan output files to report to Server  ←  Results written to workspace
```

---

## How the Agent Executes the Task

After the container starts, the internal execution flow is as follows:

1. **`entrypoint.sh` starts the container**
   - Checks whether `/workspace/task.json` exists
   - Prints the task ID and timeout
   - Invokes `/opt/run.sh`

2. **`run.sh` parses `task.json` and prepares the two-stage invocation**
   - Extracts `task_prompt` and `output_prompt` from `task.json`
   - Injects `task_prompt` into the Stage 1 prompt template
   - Prepares the Stage 2 formatting prompt (Stage 2 always runs regardless of whether `output_prompt` is empty)

3. **Stage 1: pi-coding-agent executes the task**
   - `run.sh` starts the agent with `/opt/main-agent.md` appended as the system prompt
   - Based on `task_prompt`, the agent inspects input files under `/workspace/input/` and chooses tools autonomously
   - The task execution output is also `tee`'d to `/workspace/step1.log`
   - Results are written to `/workspace/output/` (recommended as `response.md`)

4. **Stage 2: format output according to `output_prompt`**
   - Always executed after Stage 1
   - `run.sh` invokes pi-coding-agent again, asking it to read `/workspace/output/` and reformat `/workspace/output/response.md` according to `output_prompt`; if `output_prompt` is empty, the agent may simply review or leave it unchanged
   - This stage is allowed to fail; execution continues to the fallback step on failure

5. **`run.sh` ensures the final output**
   - If `/workspace/output/response.md` does not exist, `/workspace/step1.log` is copied as the fallback file
   - Copies the final response to `/workspace/response.md`

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
| `Dockerfile` | Example image definition. Based on `node:22-slim`, installs `jq`/`git`/`python3` and other common tools, and installs `pi-coding-agent` globally |
| `entrypoint.sh` | Container entry script, checks for `task.json` existence then calls `run.sh` |
| `run.sh` | **Agent invocation script**. It parses `task.json`, constructs the prompt, and starts pi-coding-agent |
| `main-agent.md` | System prompt appended to pi via `--append-system-prompt` in `run.sh`, used to constrain agent behavior |
| `pi/` | Configuration directory for pi-coding-agent, copied into `/home/executor/.pi/` in the container |

> Core relationship: `Dockerfile` installs the pi runtime → `entrypoint.sh` starts → `run.sh` invokes pi → `main-agent.md` and `pi/` together configure the agent's behavior.

---

## Customization

### Method 1: Modify Based on This Example (Recommended)

This is the fastest way to get started. Since the core of this example is **agent execution**, it is recommended to adjust from the agent level first:

- **Modify `main-agent.md`**: Adjust the system prompt to change the agent's behavior, output format, and tool usage preferences in this execution environment
- **Modify `run.sh`**: Adjust the task prompt template, pi invocation parameters, or replace it with another agent framework (e.g. Kimi Cli, Open Code, Codex, etc.)
- **Modify `pi/`**: Adjust pi-coding-agent configuration, such as available models and tool allowlists
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
