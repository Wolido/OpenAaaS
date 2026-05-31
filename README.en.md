<p align="right"><a href="./README.md">中文</a> | English</p>

<p align="center">
  <img src="./assets/logo.png" width="360" alt="OpenAaaS Logo">
</p>

<p align="center"><strong>OpenAaaS — Open Us to the Agentic World</strong></p>

<p align="center">
  <a href="https://www.open-aaas.com">Website</a> ·
  <a href="https://arxiv.org/abs/2605.13618">Paper</a> ·
  <a href="./server/README.en.md">Server Docs</a> ·
  <a href="./agent-core/README.en.md">Agent Core Docs</a> ·
  <a href="#Usage">Usage Guide</a> ·
  <a href="./client-extension/README.en.md">Client Extensions</a> ·
  <a href="./client-app/README.en.md">Desktop Client</a>
</p>

<p align="center">
  <a href="https://pypi.org/project/openaaas-mcp-adapter/">
    <img src="https://img.shields.io/pypi/v/openaaas-mcp-adapter?label=PyPI&color=blue" alt="PyPI">
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
  📝 <a href="https://github.com/Wolido/OpenAaaS/discussions/79"><b>User Story: Beyond the Lab — I Built My Wife a Copy Editor with OpenAaaS</b></a>
</p>

---

**Intelligence flows, data stays still — bring Agents to the data, instead of handing data over to Agents.**

**OpenAaaS is an Agent Orchestration Network: data stays where it was created, and Agent capabilities flow through the network to reach it.**

The bottleneck of AI has shifted from model capability to the accessibility of scientific capabilities, while "data being forced to migrate" is a harder constraint than models. Every lab has accumulated unique data, algorithms, and workflows, but they are scattered in silos and cannot be discovered or invoked. OpenAaaS builds an Agent Orchestration Network, enabling any Agent to discover, delegate to, and compose other Agents on scientific nodes around the world — data is processed in place, while Agent capabilities flow through the network.

Any Agent — whether Claude Code, pi mono, Kimi Cli, or a self-built system — can discover and compose other Agents on scientific nodes across the network.

At the same time, we strive to minimize the barrier to using the network, even for general-purpose LLM apps on mobile phones.

| Demo Video | Screenshots |
|:---:|:---:|
| <video src="https://github.com/user-attachments/assets/5bee5e09-2866-4285-b00e-15210f274177"></video> | **Connect to Network**<br><img width="372" height="113" alt="Screenshot 2026-05-07 09 36 25" src="https://github.com/user-attachments/assets/d3773d67-9d47-45db-9f5e-3ca96f990981" /><br>**View Node List**<br><img width="379" height="406" alt="Screenshot 2026-05-07 09 37 22" src="https://github.com/user-attachments/assets/d74571ac-b300-411e-9371-b51822531926" /><br>**Task Result Returned**<br><img width="371" height="391" alt="Screenshot 2026-05-07 09 38 09" src="https://github.com/user-attachments/assets/16c9984b-e730-476c-93e7-1aae78f76a5d" /> |

---

### 📄 Paper

Technical design and implementation details: [arXiv:2605.13618](https://arxiv.org/abs/2605.13618)

---

## What is an Agent Orchestration Network

The traditional approach: an Agent calls a remote script or API; the remote node passively executes a preset piece of code and returns the result.

OpenAaaS takes the opposite approach: **your Agent does not call a script, but rather another full Agent instance.**

This network has a clear **master–subordinate hierarchy**:

- **Master Agent (client Agent)**: understands your task, discovers suitable remote nodes across the network, **delegates** tasks to subordinate Agents, and integrates the returned results into its own workflow.
- **Subordinate Agent (remote Agent)**: runs inside a Docker container deployed right beside the data. Once delegated a task, it works as an autonomous "digital colleague" — making its own decisions and executing independently — then returns the results to the master Agent.

Each Agent Core node runs a **full Agent instance with a complete toolchain** inside a Docker container — pi mono, Kimi Cli, Codex, Claude Code, Open Code, or a self-built Agent. This subordinate Agent possesses autonomous tools such as read, write, bash, grep, subagent, and todo. The master Agent does not "call" a remote function; it **discovers** a suitable subordinate Agent and **delegates** the task to it. The subordinate Agent works autonomously beside the data, and the master Agent waits for the results and weaves them into its own workflow. This is a **unidirectional delegation**, not a peer-to-peer calling relationship.

This means OpenAaaS is fundamentally an **Agent Orchestration Network** — master Agents discover, delegate to, and compose multiple subordinate Agents through the network, rather than calling scripts or services.

---

## Four Core Propositions

### Agent Orchestration: Nodes are Agents

The network schedules not scripts or services, but full Agent instances with complete toolchains. Inside the Docker container on each node runs a complete Agent capable of autonomous decision-making, execution, and sub-Agent invocation. Any Agent can discover, delegate to, and compose other Agents across global nodes as easily as calling a tool.

### Zero Data Migration

Traditional solutions demand that data be aggregated into a centralized platform — inevitably introducing format conversion distortion, metadata loss, version divergence, and broken compliance audit chains. OpenAaaS builds no unified data warehouse. Data remains at its point of origin, preserved in its original storage format, directory structure, and access permissions. The remote Agent works autonomously next to the data; results are sent back. Raw data never leaves.

### Schema-Free Onboarding, Native Agent Capability as a Service

We impose no upfront format requirements on data. JSON, CSV, Excel, MATLAB `.mat`, HDF5, vendor-specific binary formats from instruments — the local parsing and processing scripts on each node are themselves part of the network's capability. Agents invoke a combined "parse + analyze" service, rather than being required to pre-clean, standardize, or structure the data. Whatever format a lab already has, it is service-ready from day one.

### Near-Data Computing, Agents Compute Autonomously Next to Data

Computation happens next to the data, not the other way around. Remote Agents make autonomous decisions and execute tasks beside the data. The network only transmits task descriptions and execution results (KB–MB scale); raw data is processed on-site. For TB-scale datasets and regulated sensitive samples, this means no upload wait, no bandwidth bottleneck, and no outbound compliance review — the marginal cost of moving data approaches zero.

## Core Design Philosophy

Traditional cloud solutions require data to leave the premises: TB-scale datasets must be migrated and uploaded, sensitive samples are handed to third parties, and lab firewalls are forced to open inbound ports. OpenAaaS takes the opposite approach — deploying Agent execution nodes directly where the data resides. The network only transmits task descriptions, task files, and results; raw data stays in place.

| | Traditional Cloud Solution | OpenAaaS Near-Data Solution |
|---|---|---|
| Data Flow | Local → Cloud → Local | **Raw data stays in place** |
| Network Transfer | Raw data (TB scale) | Task descriptions, task files, and results (KB–MB scale) |
| Firewall Requirements | Inbound ports required | **Outbound HTTP only** |
| Sensitive Data | Must leave the domain | **Never leaves the lab** |
| Latency | Bandwidth-limited | Local compute, extremely low latency |

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

## Design Rationale

| Principle | Description | Effect |
|------|------|------|
| Rust + Single Binary | `cargo build --release` produces one executable | Zero-dependency deployment, copy and run |
| Embedded SQLite | Database starts with the process, no separate service | Zero operations, single node is sufficient |
| Docker Isolation | Each task runs in an independent container with workspace mounted | Secure and controllable, reproducible environment |
| Agent Orchestration | Containers run full Agent instances (not scripts) with autonomous toolchains such as read/write/bash/grep/subagent | Remote Agents make autonomous decisions and execute complex tasks; the client only needs to describe the goal |
| Self-Organizing Nodes | Nodes actively register with the network and poll for tasks; Server only maintains an index. Raw data never leaves the domain; task files flow through the Server | Nodes need no public IP; unidirectional outbound is enough to join the network; data is processed on-site, naturally adapting to lab firewall environments |

## Features

### Data In-Situ Retention & Cross-Node Capability Flow

- **🤖 Agent Calls Agent, Not Script** — Remote nodes run full Agents with complete toolchains, capable of autonomous decision-making and executing complex tasks, rather than passively running preset scripts.
- **🔗 Composable Agent Capabilities** — Any Agent can discover, delegate to, and compose other Agents across global nodes as easily as calling a tool, enabling multi-Agent collaboration.
- **🔌 Zero-Learning-Cost Agent Integration, Self-Describing API Auto-Exposes Service Docs** — No authentication required; returns complete API documentation and usage instructions. Agents can understand and invoke other Agents on scientific nodes without any plugins.
- **🧩 Progressive Capability Discovery, Avoiding Context Overflow** — Initial queries return lightweight summaries; detailed usage is returned on demand. A progressive disclosure design similar to SKILL.md protects the Agent's context window.

### Zero Data Migration

- **🔒 Data Never Leaves the Premises** — Agent execution nodes are deployed directly on lab servers or instrument workstations. Raw large datasets are processed in-place via local mounts; sensitive data never crosses the firewall. The network only transmits task descriptions, task files, and results; it never touches raw data.
- **💾 Single Binary, Zero Operations** — SQLite database + local file storage; no Redis/MySQL required. A single node is enough for deployment, ideal for lab edge nodes.
- **⚖️ Nodes Join via Reverse Connection, No Public IP Needed** — Nodes self-manage concurrency and task claiming; Server only does lightweight queue management. Lab nodes only need unidirectional outbound access to join; no open ports or SSH required.

### Schema-Free Onboarding & Near-Data Computing

- **🐳 Independent Sandbox per Experiment, Reproducible Results** — Each task runs in an isolated container with workspace mounts for input and output. Environment isolation makes results traceable and reproducible.
- **🔧 Zero-Config Node Onboarding** — `open-aaas-server run` auto-generates `config.toml`, SQLite database, and keys on first launch. No manual configuration; ready to use out of the box.
- **🤖 MCP Standard Protocol Compatible** — Through `openaaas-mcp-adapter`, any MCP-compatible client such as Claude Desktop, Cursor, or Cline can connect with one click, without writing any plugins.

## Usage

Public Server: **<https://api.open-aaas.com>**

We provide three trial scientific services on the public server:

- IDM-Alpha Metal Materials Literature Research Assistant Based on Hundreds of Thousands of Real Papers
- Trillion-Scale Hexa-High-Entropy Alloy Descriptor Database
- Fuyao Multi-Agent Roundtable System

You can have your Agent connect to the public server to use them.

### Quick Start

**Scenario 1: Use the Public Server**

No need to build your own infrastructure. Simply configure your Agent to connect to the public server and start invoking community-shared scientific Agents. Ideal for individual researchers to get started quickly.

### Using the pi / Kimi Plugin

Just say in the conversation:

> "Help me set the OpenAaaS server address to <https://api.open-aaas.com>, then submit a data analysis task"

The client Agent will automatically complete registration, service discovery, task submission, and result retrieval.

<video src="https://github.com/user-attachments/assets/4e2873ee-1581-46c7-b8f2-cfcd6da097ef" controls></video>

### Using an MCP Client

`openaaas-mcp-adapter` is available on PyPI. If you are using **OpenClaw** or any other Agent that supports MCP (Model Context Protocol), connecting to the OpenAaaS network is nearly zero-cost — no plugins to write, just one configuration entry to invoke all capabilities.

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

After configuring, restart the client, and you can invoke OpenAaaS's 14 standard Tools (`set_server_url`, `register`, `list_services`, `submit_task`, etc.) directly in conversation without installing any plugins.

Or better yet, you can have your Agent set it up for you directly.

<p align="center">
  <img alt="mcp" src="https://github.com/user-attachments/assets/b7ff63bf-5fa8-46fa-906b-a8edbd950465" />
</p>

See [client-extension/openaaas-mcp-adapter/README.en.md](./client-extension/openaaas-mcp-adapter/README.en.md) for details.

### Using the Desktop Client

If you prefer a graphical interface, use the **OpenAaaS Desktop Client** — a cross-platform desktop application based on Tauri, supporting macOS, Windows, and Linux.

The desktop client is ideal for:
- Non-technical users who don't want to configure command-line tools or plugins
- Managing multiple servers and browsing services visually
- Drag-and-drop file uploads and real-time task progress tracking

📖 See [client-app/README.en.md](./client-app/README.en.md) for details.

<p align="center">
  <img alt="OpenAaaS Client" src="https://github.com/user-attachments/assets/8bc81d68-76da-47c6-a535-83227b27b8bd" width="800" />
</p>

> macOS users: The app is ad-hoc signed. On first launch, go to **System Settings → Privacy & Security → Security** and click **"Open Anyway"**.

### Using a General Agent Framework

If your Agent does not have an OpenAaaS plugin, simply have it access <https://api.open-aaas.com>:

- No authentication required; complete API documentation and usage instructions are returned
- The Agent can then automatically complete registration, service discovery, and task submission after reading them

**Scenario 2: Deploy on a Lab Server and Connect Local Capabilities**

Launch OpenAaaS on a local server in your machine room or lab, and register local analysis scripts and specialized computing workflows as network nodes. Any Agent in the research group — pi, Kimi, Claude, or a self-built system — can query node status, submit analysis tasks, and retrieve result data through a unified entry point.

### Local Deployment

**Deploy Server (Scheduling Center)**:

```bash
cd server
cargo build --release
./target/release/open-aaas-server run
```

On first launch, `config.toml` and the SQLite database are auto-generated.

**Deploy Agent Core (Execution Node)**:

```bash
cd agent-core
cargo build --release
./target/release/agent-core init
./target/release/agent-core register --token <registration_token> --name my-agent
./target/release/agent-core run
```

The `registration_token` must be obtained by creating a Service on the Server first. Admins can use the API Key from the Server logs to call `POST /api/v1/services/` to create one.

The Agent executor image needs to be built in advance (under the agent-core directory):

```bash
cd executor-example && docker build -t open-aaas-executor:latest .
```

See [agent-core/README.en.md](./agent-core/README.en.md) for details.

## Project Structure

```
OpenAaaS/
├── server/           # Network Hub (Scheduling Center) (Rust) — Task scheduling, queuing, auth, file relay
├── agent-core/       # Network Node (Execution Node) (Rust) — Registration, polling, Docker-isolated execution
├── client-app/       # Desktop Client (Tauri + Vue 3) — Service marketplace, task submission, result viewing
├── dash/             # Debug and admin tools (Python/Streamlit)
└── client-extension/ # Client extensions — pi plugin, Kimi plugin, MCP adapter (Claude Desktop / Cursor / Cline)
```

## Research Vision

OpenAaaS's vision is to make every lab a **composable Agent node** in the Agentic Science network. Data is no longer degraded by migration, and knowledge is no longer stalled by silos. Every research group's data morphology, analysis workflows, and domain methods — however unique their storage formats may be — can be discovered, invoked, and orchestrated by any Agent across the network.

When Agent capabilities can flow through the network to where data lives, an Agent's knowledge boundary expands from the closed loop of a single lab to an open ecosystem of global Agent collaboration. The marginal cost of moving data approaches zero, meaning datasets of any scale can be invoked on demand by Agents anywhere. The frontier of scientific innovation is no longer limited by a single team's data volume or domain depth.

## Contributing

We welcome contributions! Please read [CONTRIBUTING.md](./CONTRIBUTING.md) for development setup, coding guidelines, and how to get involved.

## Open Source License

MIT License © IDM Explorer Lab

<img src="./assets/idm-logo.png" width="200" alt="IDM Explorer Lab">
