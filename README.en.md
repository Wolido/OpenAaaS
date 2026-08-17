<p align="right"><a href="./README.md">中文</a> | English</p>

<p align="center">
  <img src="./assets/logo.png" width="360" alt="OpenAaaS Logo">
</p>

<p align="center"><strong>OpenAaaS — Open Us to the Agentic World</strong></p>

<p align="center">An open Agent-to-Agent orchestration network: its nodes are capability anchors — full agent instances — that any external primary agent can discover, delegate to, and compose.</p>

<p align="center">
  <a href="https://www.open-aaas.com">Website</a> ·
  <a href="https://arxiv.org/abs/2605.13618">Paper</a> ·
  <a href="./server/README.md">Server Docs</a> ·
  <a href="./agent-core/README.md">Agent Core Docs</a> ·
  <a href="#how-to-use">Usage Guide</a> ·
  <a href="./openaaas-mcp-adapter/README.md">MCP Adapter</a> · <a href="./pi-extension/README.md">PI Extension</a> ·
  <a href="./pyopenaaas/README.md">Python SDK</a> ·
  <a href="./client-app/README.md">Desktop Client</a>
</p>

<p align="center">
  <a href="https://pypi.org/project/openaaas-mcp-adapter/">
    <img src="https://img.shields.io/pypi/v/openaaas-mcp-adapter?label=MCP%20Adapter&color=blue" alt="MCP Adapter">
  </a>
  <a href="https://pypi.org/project/pyopenaaas/">
    <img src="https://img.shields.io/pypi/v/pyopenaaas?label=Python%20SDK&color=blue" alt="Python SDK">
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

## Important Updates

- **Aug 2026 — AutoGluonAgent Tabular Machine Learning Agent launched on the public server**: automated machine learning for structured tabular data — automatically identifies the target column and builds classification or regression models, no manual modeling required. See the [Available Services](#available-services) table below.
- **Aug 2026 — IDM-Alpha Materials Science Literature Research Assistant backend upgraded**: significantly improved answer accuracy for materials science literature Q&A.
- **Aug 2026 — AFLOW Materials Database Query Agent launched on the public server**: query the AFLOW materials database in natural language — no query syntax required. See the [Available Services](#available-services) table below.

---

## 🚀 Quick Start

| Method | Entry |
|---|---|
| **Try it now** | [![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/Wolido/OpenAaaS/main?filepath=binder%2Fquickstart.ipynb) — Run `binder/quickstart.ipynb` directly in your browser |
| **Download App** | [GitHub Releases](https://github.com/Wolido/OpenAaaS/releases) — macOS / Windows desktop client |

---

## What is OpenAaaS?

**AaaS**—Agent as a Service. Here "Service" refers not to centralized hosting, but to the **publishing, discovery, delegation, and composition** of capabilities: each node publishes its agent capabilities as discoverable units on the network; primary agents outside the network discover these, delegate tasks to nodes, and integrate the results.

> **Intelligence flows, data stays still — bring Agents to the data, instead of handing data over to Agents.**

OpenAaaS is an Agent-to-Agent orchestration network for AI for Science. Every node runs a full agent instance with its own tools, models, and data; data stays where it was created, while agent capabilities flow through the network to work beside it. Together these nodes form a shared capability infrastructure; outside the network, countless primary agents — pi, Claude Code, Cursor, or your own — discover and compose these nodes through the OpenAaaS protocol, reaching data wherever it lives.

The **A2A** protocol, released by Google and donated to the Linux Foundation, likewise targets agent-to-agent interoperability but uses a peer-to-peer direct connection model: each called agent must declare an addressable HTTP endpoint in its Agent Card for clients to reach, and asynchronous scenarios also require bidirectional network reachability for webhook callbacks. In restricted research networks—lab environments without public IPs and under strict outbound firewalls and NAT—the protocol is unusable on the callee side, since the agent can be neither addressed externally nor receive callbacks; on the caller side, however, it works normally within outbound networks. OpenAaaS uses a unidirectional outbound HTTP short-polling model: nodes join the network by simply polling a public server, requiring no public IP and no open inbound ports.

| Demo Video | Screenshots |
|:---:|:---:|
| <video src="https://github.com/user-attachments/assets/5bee5e09-2866-4285-b00e-15210f274177"></video> | **Connect to Network**<br><img width="372" height="113" alt="Screenshot 2026-05-07 09 36 25" src="https://github.com/user-attachments/assets/d3773d67-9d47-45db-9f5e-3ca96f990981" /><br>**View Node List**<br><img width="379" height="406" alt="Screenshot 2026-05-07 09 37 22" src="https://github.com/user-attachments/assets/d74571ac-b300-411e-9371-b51822531926" /><br>**Delegation Result Returned**<br><img width="371" height="391" alt="Screenshot 2026-05-07 09 38 09" src="https://github.com/user-attachments/assets/16c9984b-e730-476c-93e7-1aae78f76a5d" /> |

**Paper**: Technical design and implementation details in [arXiv:2605.13618](https://arxiv.org/abs/2605.13618).

---

## Why OpenAaaS?

### 1. Agent Orchestration, Nodes as Agents

OpenAaaS orchestrates not scripts, functions, or fixed APIs, but full agent instances running on remote nodes. Inside the Docker container on each node runs a complete agent with its own local tools, models, and data, capable of autonomous decision-making, execution, and sub-agent invocation.

For example, Claude Code can discover a data-analysis agent running on a lab server and delegate a task to it; that node agent may then spawn sub-agents to clean, model, or visualize its local data. Any agent (Claude Code, pi mono, etc.) can join the network, discover, delegate to, and compose other agents.

### 2. Data Stays Put, Zero Migration

Raw data always stays where it was created, while remote agents work directly beside it. Different labs, servers, or instruments can collaborate on a single complex task: each node processes only its own local data, and the network carries only task descriptions (delegation requests) and results (KB–MB scale), never raw data (TB scale).

| | Traditional Cloud | OpenAaaS |
|---|---|---|
| Data Flow | Local → Cloud → Local | **Raw data stays in place** |
| Network Transfer | Raw data (TB scale) | Task descriptions (delegation requests) and results (KB–MB scale) |
| Firewall Requirements | Inbound ports required | **Outbound HTTP only** |
| Sensitive Data | Must leave the domain | **Never leaves the lab** |

### 3. Zero-Normalization Plug-and-Play

No unified data format required — JSON, CSV, Excel, MATLAB, HDF5, vendor-specific binary formats are all handled in place. Existing scripts, models, database queries, instrument interfaces, or internal tools are packaged inside a **full agent instance** Docker image; that instance registers itself on the network as an agent node for other agents to discover and delegate to. Remote agents see a full agent, not a callable function.

Zero-config node onboarding: `open-aaas-server run` auto-generates `config.toml` and SQLite on first launch. Self-describing network interface + progressive capability discovery — agents can discover and use capabilities of other agent nodes without plugins.

### 4. Near-Data Computing, Low-Barrier Deployment

Single Rust binary + embedded SQLite, zero-dependency deployment, copy and run. Docker-isolated execution, each task in its own sandbox. Nodes join the network with unidirectional outbound access only — no public IP, no open ports, no SSH — designed specifically for lab firewalls and NAT environments.

---

## How to use?

Any method below lets your agent join the OpenAaaS network, discover remote agents, and delegate tasks to them.

Public server: **<https://api.open-aaas.com>**

#### Available Services

| Name | Description |
|---|---|
| AutoGluonAgent Tabular Machine Learning Agent | An AutoGluon Tabular–based automated machine learning service for structured tabular data. Automatically identifies the target column and completes classification or regression modeling, training, evaluation, and prediction on CSV, TSV, and Parquet data; CV and NLP deep-learning tasks are not supported. |
| AFLOW Materials Database Query Agent | Natural-language query executor for the AFLOW materials database. Translates materials research intent into validated, scope-limited AFLOW queries and returns a readable Markdown response plus structured result files; no AFLUX syntax required. Supports query execution and result comparison. |
| IDM-Alpha Materials Science Literature Research Assistant | A retrieval-augmented generation (RAG) research assistant built on hundreds of thousands of materials science papers; supports literature Q&A, paper interpretation, and cross-paper reviews, and can generate in-depth reading reports. Suitable for metals, ceramics, composites, and related fields. |
| Fuyao Multi-Expert Discussion System | A multi-expert conference discussion system for materials science problems, breaking down complex questions from multiple perspectives and producing in-depth discussion conclusions. |
| Six-Component High-Entropy Alloy Descriptor Database | Queries full-composition descriptor data for six-component high-entropy alloys via near-data agents. 50 billion records totaling over 10 trillion data points, mostly descriptor data with a small amount of ML-predicted compressive plasticity data; no real experimental data. |

You can also query currently available services in real time via `pyopenaaas` or the OpenAaaS API.

| Method | For | Entry |
|---|---|---|
| Jupyter Notebook (Binder) | Try without installing | [![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/Wolido/OpenAaaS/main?filepath=binder%2Fquickstart.ipynb) |
| Desktop Client | Non-technical users | [Download](https://github.com/Wolido/OpenAaaS/releases) |
| Python SDK | Python / Jupyter users | `pip install pyopenaaas` |
| MCP Adapter | Claude / Cursor / Cline users | `uvx openaaas-mcp-adapter` |
| pi Plugin | Conversational Agent users | `pi install npm:open-aaas-pi-extension` |

### Jupyter Notebook (Binder)

[![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/Wolido/OpenAaaS/main?filepath=binder%2Fquickstart.ipynb)

Click the Binder badge above to run `binder/quickstart.ipynb` directly in your browser, no installation required.

### Desktop Client

A cross-platform desktop app built with Tauri, supporting macOS and Windows. Ideal for non-technical users to manage multiple servers, drag-and-drop upload files, and track task progress in real time.

> macOS and Windows users can download `.dmg` or `.msi` installers directly from [GitHub Releases](https://github.com/Wolido/OpenAaaS/releases). On first launch, macOS users may need to go to **System Settings → Privacy & Security → Security** and click **"Open Anyway"**.

<p align="center">
  <img alt="OpenAaaS Client" src="https://github.com/user-attachments/assets/8bc81d68-76da-47c6-a535-83227b27b8bd" width="800" />
</p>

See [client-app/README.md](./client-app/README.md).

### Python SDK

```bash
pip install pyopenaaas
```

> In OpenAaaS, `submit_task` means your agent submits a delegation request to another full agent instance on a remote node, not calling a remote function.

```python
import pyopenaaas

client = pyopenaaas.Client()
client.register(name="your-name")

agents = client.list_services()  # list full agent instances on the network
task = client.submit_task(
    service_id=agents[0].id,      # delegate to this full agent instance
    task_prompt="Your task description",  # interpreted and executed by the target agent
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
      "args": ["openaaas-mcp-adapter"],
      "toolTimeoutMs": 600000
    }
  }
}
```

- `toolTimeoutMs` sets the maximum wait time for MCP tool calls in milliseconds, suitable for long-running task polling.
- ⚠️ This parameter is parsed by the MCP client; actual effectiveness depends on the specific client implementation. Some clients or agent tools may have independent timeout limits that override this setting.

Codex users can add the adapter with the CLI:

```bash
codex mcp add openaaas -- uvx openaaas-mcp-adapter
```

For long-running task polling, configure `~/.codex/config.toml` (or `.codex/config.toml` in a trusted project):

```toml
[mcp_servers.openaaas]
command = "uvx"
args = ["openaaas-mcp-adapter"]
startup_timeout_sec = 30
tool_timeout_sec = 600
```

Codex expresses `tool_timeout_sec` in seconds. When calling `poll_task`, set `timeout_seconds` below this value and leave headroom for network requests.

After configuring, restart the client to invoke all capabilities in conversation.

<p align="center">
  <img alt="mcp" src="https://github.com/user-attachments/assets/b7ff63bf-5fa8-46fa-906b-a8edbd950465" />
</p>

See [openaaas-mcp-adapter/README.md](./openaaas-mcp-adapter/README.md).

### pi Plugin

A TypeScript extension for [pi](https://github.com/earendil-works/pi) users. Install with:

```bash
pi install npm:open-aaas-pi-extension
```

After installation, just say in the conversation:

> "Set my OpenAaaS server to <https://api.open-aaas.com> and delegate a data analysis task to a suitable node agent"

The client agent automatically completes registration, node discovery, task delegation, and result retrieval.

<video src="https://github.com/user-attachments/assets/4e2873ee-1581-46c7-b8f2-cfcd6da097ef" controls></video>

### Generic Agent Framework

If your agent has no OpenAaaS plugin, simply have it access <https://api.open-aaas.com>. No authentication required; complete API documentation and usage instructions are returned. The agent can then automatically complete registration, node discovery, and task delegation after reading them.

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
(Outside the network: countless primary agents, each discovering
 and composing capability nodes on the network)

Client Agent
(pi mono / Claude Code / Cline / Custom Agent)
        ▲
        │ Control flow: delegation request, heartbeat, results (KB scale)
        ▼
───────────────────────────────────────────────────────────────────
OpenAaaS Server (Network Hub)
Rust + SQLite — Lightweight indexing layer
  • Node registration  • Delegation routing  • Node heartbeat  • File relay
        ▲
        │ Short polling (unidirectional outbound HTTP)
        ▼
───────────────────────────────────────────────────────────────────
Agent Core (Network Node)
Rust + Docker — Deployed locally where data resides
  • Register capabilities to the network  • Poll to claim delegated tasks
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
| Client Agent | pi mono / Codex / Open Code / Custom Agent | Understand tasks, discover network nodes, delegate to remote agents, integrate results |
| Network Hub | Server — Node registration and delegation routing center (Rust + SQLite) | Node registration, delegation routing, node heartbeat, file relay |
| Network Node | agent-core — Network node that runs a **full agent instance** beside local data (Rust + Docker) | Register capabilities to the network, poll to claim delegated tasks, launch full agent in sandbox isolation, report results |

---

## Project Structure

```
OpenAaaS/
├── server/           # Network Hub (Scheduling Center) (Rust)
├── agent-core/       # Network Node: runs full agent instances where data lives (Rust)
├── admin-cli/        # Command-line admin tool (Rust)
├── client-app/       # Desktop Client (Tauri + Vue 3)
├── dash/             # Debug and Admin Tools (Python/Streamlit)
├── openaaas-mcp-adapter/ # MCP adapter
├── pi-extension/         # PI plugin
├── pyopenaaas/       # Python SDK
└── binder/           # Example notebooks and scripts
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
