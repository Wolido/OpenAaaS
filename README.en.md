<p align="right"><a href="./README.md">中文</a> | English</p>

<p align="center">
  <img src="./assets/logo.png" width="360" alt="OpenAaaS Logo">
</p>

<p align="center"><strong>OpenAaaS — Open Us to the Agentic World</strong></p>

<p align="center">
  <a href="https://www.open-aaas.com">Website</a> ·
  <a href="https://arxiv.org/abs/2605.13618">Paper</a> ·
  <a href="./server/README.md">Server Docs</a> ·
  <a href="./agent-core/README.md">Agent Core Docs</a> ·
  <a href="#how-to-use">Usage Guide</a> ·
  <a href="./client-extension/README.md">Client Extensions</a> ·
  <a href="./pyopenaaas/README.md">Python SDK</a> ·
  <a href="./client-app/README.md">Desktop Client</a>
</p>

<p align="center">
  <a href="https://pypi.org/project/openaaas-mcp-adapter/">
    <img src="https://img.shields.io/pypi/v/openaaas-mcp-adapter?label=PyPI&color=blue" alt="PyPI">
  </a>
  <a href="https://pypi.org/project/pyopenaaas/">
    <img src="https://img.shields.io/pypi/v/pyopenaaas?label=PyPI%20SDK&color=blue" alt="PyPI SDK">
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
  📝 <a href="https://github.com/Wolido/OpenAaaS/discussions/57"><b>Design Blog: Don't Move Data, Distill an Administrator.skill</b></a><br>
  📝 <a href="https://github.com/Wolido/OpenAaaS/discussions/79"><b>User Story: Not Just for the Lab — I Built My Wife a Copy Editor with OpenAaaS</b></a>
</p>

---

## 🚀 Quick Start

| Method | Entry |
|---|---|
| **Try it now** | [![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/Wolido/OpenAaaS/main?filepath=examples%2Fquickstart.ipynb) — Run `examples/quickstart.ipynb` directly in your browser |
| **Download App** | [GitHub Releases](https://github.com/Wolido/OpenAaaS/releases) — macOS / Windows / Linux desktop client |

---

## What is OpenAaaS?

> **Intelligence flows, data stays still — bring Agents to the data, instead of handing data over to Agents.**

OpenAaaS is an Agent Orchestration Network for AI for Science: data stays where it was created, and Agent capabilities flow through the network to reach it.

| Demo Video | Screenshots |
|:---:|:---:|
| <video src="https://github.com/user-attachments/assets/5bee5e09-2866-4285-b00e-15210f274177"></video> | **Connect to Network**<br><img width="372" height="113" alt="Screenshot 2026-05-07 09 36 25" src="https://github.com/user-attachments/assets/d3773d67-9d47-45db-9f5e-3ca96f990981" /><br>**View Node List**<br><img width="379" height="406" alt="Screenshot 2026-05-07 09 37 22" src="https://github.com/user-attachments/assets/d74571ac-b300-411e-9371-b51822531926" /><br>**Task Result Returned**<br><img width="371" height="391" alt="Screenshot 2026-05-07 09 38 09" src="https://github.com/user-attachments/assets/16c9984b-e730-476c-93e7-1aae78f76a5d" /> |

**Paper**: Technical design and implementation details in [arXiv:2605.13618](https://arxiv.org/abs/2605.13618).

---

## Why OpenAaaS?

### 1. Agent Orchestration, Nodes as Agents

The network schedules not scripts, but full Agent instances with complete toolchains. Inside the Docker container on each node runs a complete Agent capable of autonomous decision-making, execution, and sub-Agent invocation. Any Agent (Claude Code, pi mono, Kimi, etc.) can discover, delegate to, and compose other Agents across global nodes as easily as calling a tool.

### 2. Data Stays Put, Zero Migration

Raw data always stays where it was created, while remote Agents work directly beside it. The network only transmits task descriptions and results (KB–MB scale), never touching raw data (TB scale).

| | Traditional Cloud | OpenAaaS |
|---|---|---|
| Data Flow | Local → Cloud → Local | **Raw data stays in place** |
| Network Transfer | Raw data (TB scale) | Task descriptions and results (KB–MB scale) |
| Firewall Requirements | Inbound ports required | **Outbound HTTP only** |
| Sensitive Data | Must leave the domain | **Never leaves the lab** |

### 3. Zero-Normalization Plug-and-Play

No unified data format required — JSON, CSV, Excel, MATLAB, HDF5, vendor-specific binary formats are all handled in place. Zero-config node onboarding: `open-aaas-server run` auto-generates `config.toml` and SQLite on first launch. Self-describing API + progressive capability discovery — Agents understand and invoke all services without plugins.

### 4. Near-Data Computing, Low-Barrier Deployment

Single Rust binary + embedded SQLite, zero-dependency deployment, copy and run. Docker-isolated execution, each task in its own sandbox. Nodes join the network with unidirectional outbound access only — no public IP, no open ports, no SSH — designed specifically for lab firewalls and NAT environments.

---

## How to use?

Public server: **<https://api.open-aaas.com>**

| Method | For | Entry |
|---|---|---|
| Jupyter Notebook (Binder) | Try without installing | [![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/Wolido/OpenAaaS/main?filepath=examples%2Fquickstart.ipynb) |
| Desktop Client | Non-technical users | [Download](https://github.com/Wolido/OpenAaaS/releases) |
| Python SDK | Python / Jupyter users | `pip install pyopenaaas` |
| MCP Adapter | Claude / Cursor / Cline users | `uvx openaaas-mcp-adapter` |
| pi / kimi Plugin | Conversational Agent users | Invoke directly in chat |

### Jupyter Notebook (Binder)

[![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/Wolido/OpenAaaS/main?filepath=examples%2Fquickstart.ipynb)

Click the Binder badge above to run `examples/quickstart.ipynb` directly in your browser, no installation required.

### Desktop Client

A cross-platform desktop app built with Tauri, supporting macOS, Windows, and Linux. Ideal for non-technical users to manage multiple servers, drag-and-drop upload files, and track task progress in real time.

> macOS and Windows users can download `.dmg` or `.msi` installers directly from [GitHub Releases](https://github.com/Wolido/OpenAaaS/releases). On first launch, macOS users may need to go to **System Settings → Privacy & Security → Security** and click **"Open Anyway"**.

<p align="center">
  <img alt="OpenAaaS Client" src="https://github.com/user-attachments/assets/8bc81d68-76da-47c6-a535-83227b27b8bd" width="800" />
</p>

See [client-app/README.md](./client-app/README.md).

### Python SDK

```bash
pip install pyopenaaas
```

```python
import pyopenaaas

client = pyopenaaas.Client()
client.register(name="your-name")

services = client.list_services()
task = client.submit_task(
    service_id=services[0].id,
    task_prompt="Your task description",
)
task = client.wait_for_task(task.id)
paths = client.download_all_files(task.id)
```

See [pyopenaaas/README.md](./pyopenaaas/README.md).

### MCP Adapter

`openaaas-mcp-adapter` is published on PyPI. MCP-compatible clients (Claude Desktop, Cursor, Cline, etc.) can connect with a single configuration entry:

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

After configuring, restart the client to invoke all capabilities in conversation.

<p align="center">
  <img alt="mcp" src="https://github.com/user-attachments/assets/b7ff63bf-5fa8-46fa-906b-a8edbd950465" />
</p>

See [client-extension/openaaas-mcp-adapter/README.md](./client-extension/openaaas-mcp-adapter/README.md).

### pi / kimi Plugin

Just say in the conversation:

> "Set my OpenAaaS server to <https://api.open-aaas.com> and submit a data analysis task"

The client Agent automatically completes registration, service discovery, task submission, and result retrieval.

<video src="https://github.com/user-attachments/assets/4e2873ee-1581-46c7-b8f2-cfcd6da097ef" controls></video>

### Generic Agent Framework

If your Agent has no OpenAaaS plugin, simply have it access <https://api.open-aaas.com>. No authentication required; complete API documentation and usage instructions are returned. The Agent can then automatically complete registration, service discovery, and task submission after reading them.

---

## Deploy your own node

### Precompiled Binaries (Recommended)

No Rust installation needed — download and run: [GitHub Releases](https://github.com/Wolido/OpenAaaS/releases)

| Component | Binary Name |
|---|---|
| Server | `open-aaas-server` |
| Agent Core | `agent-core` |

Supported platforms:
- **Server / Agent Core**: Linux x64 (musl static linking), Linux arm64 (musl static linking), macOS arm64, Windows x64
- **Desktop Client**: macOS, Windows

> Linux builds use musl static linking and do not depend on system glibc, so they run on any Linux distribution out of the box.

```bash
chmod +x open-aaas-server   # Unix users
./open-aaas-server run
```

`config.toml` and the SQLite database are auto-generated on first launch.

### Build from Source

```bash
cd server
cargo build --release
./target/release/open-aaas-server run
```

For Agent Core deployment, see [agent-core/README.md](./agent-core/README.md).

---

## Architecture

```
Client Agent
(pi mono / Claude Code / Kimi Cli / Cline / Custom Agent)
        ▲
        │ Control flow: task description, heartbeat, results (KB scale)
        ▼
───────────────────────────────────────────────────────────────────
OpenAaaS Server (Network Hub)
Rust + SQLite — Lightweight indexing layer
  • Service registration  • Task routing  • Node heartbeat  • File relay
        ▲
        │ Short polling (unidirectional outbound HTTP)
        ▼
───────────────────────────────────────────────────────────────────
Agent Core (Network Node)
Rust + Docker — Deployed locally where data resides
  • Register capabilities to the network  • Poll for tasks
  • Container sandbox isolation execution  • Report results
        │
        ▼
   [Full Agent Instance]
   (Complete toolchain, autonomously decides & executes inside container)
        │
        ├─→ [Local Dataset] (TB scale)
        ├─→ [Analysis Scripts] (Algorithms/Models)
        └─→ [Specialized Hardware] (GPU/Instruments)
```

| Layer | Component | Responsibility |
|------|------|------|
| Client Agent | pi mono / Kimi Cli / Codex / Open Code / Custom Agent | Understand tasks, discover network nodes, schedule remote Agents, integrate results |
| Network Hub | Server — Capability registration and scheduling center (Rust + SQLite) | Service registration, task routing, node heartbeat, file relay |
| Network Node | agent-core — Capability execution node + Docker, runs a **full Agent instance** in container | Register capabilities to the network, poll for tasks, launch full Agent in sandbox isolation, report results |

---

## Project Structure

```
OpenAaaS/
├── server/           # Network Hub (Scheduling Center) (Rust)
├── agent-core/       # Network Node (Execution Node) (Rust)
├── client-app/       # Desktop Client (Tauri + Vue 3)
├── dash/             # Debug and Admin Tools (Python/Streamlit)
├── client-extension/ # Client Extensions — pi plugin, Kimi plugin, MCP adapter
├── pyopenaaas/       # Python SDK
└── examples/         # Example notebooks and scripts
```

---

## Paper

arXiv:2605.13618 — [https://arxiv.org/abs/2605.13618](https://arxiv.org/abs/2605.13618)

---

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](./CONTRIBUTING.md).

---

## License

MIT License © IDM Explorer Lab

<img src="./assets/idm-logo.png" width="200" alt="IDM Explorer Lab">
