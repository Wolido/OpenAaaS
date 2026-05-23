# OpenAaaS Desktop Client

<p align="right"><a href="./README.md">中文</a> | English</p>

The desktop client for OpenAaaS, providing visual server management, service marketplace browsing, and task submission.

## Tech Stack

| Technology | Purpose |
|------------|---------|
| Tauri 2.0 | Cross-platform desktop framework |
| Vue 3 + TypeScript | Frontend UI and type safety |
| Tailwind CSS | Styling system |
| Pinia | State management |
| `@tauri-apps/plugin-http` | Direct server connection bypassing CORS |

## Features

- **Multi-server management** — Add, remove, register, and switch default servers
- **Service marketplace** — Browse available services across all servers, view real-time load and status
- **Service details** — View usage instructions and real-time load
- **Guided task submission** — Fill in task_prompt and output_prompt, with file upload support (drag-and-drop or click)
- **Auto task polling** — 5-second polling after submission, then 30-second intervals
- **Task details** — View task status, results (Markdown rendering), and download output files
- **Task list** — Filter by status, with server origin indicators
- **Data persistence** — Server configuration and local data stored in localStorage
- **Skeleton screens** — Loading placeholders on all async-loaded areas to prevent blank screens

## Build & Run

Requires Rust 1.85+, Node.js 18+, and [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/).

```bash
cd client-app
npm install
cargo tauri dev      # Development mode
cargo tauri build    # Release build
```

In development mode, Tauri automatically opens a desktop window; frontend code supports hot reload, Rust code changes require a restart.

## Build Artifacts

Release build artifacts are located at `src-tauri/target/release/bundle/`:

| Platform | Artifacts |
|----------|-----------|
| macOS | `.dmg` installer + `.app` bundle |
| Windows | `.msi` installer + `.exe` executable |
| Linux | `.deb` package |

## macOS Installation Note

The app is ad-hoc signed (without an Apple Developer ID). On first launch, macOS may warn **"cannot be opened because the developer cannot be verified."**

To open: Go to **System Settings → Privacy & Security → Security**, then click **"Open Anyway"**.

> 💡 This is preferable to no signing at all, which would show "damaged" and require a terminal command to remove quarantine attributes.
