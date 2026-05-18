# OpenAaaS Dashboard

<p align="right"><a href="./README.zh.md">中文</a> | English</p>

> **⚠️ Positioning Note**: This is a **debugging and admin control tool** for developers and system administrators, used for monitoring and managing OpenAaaS tasks and system status. **It is NOT the OpenAaaS user interface / main UI (Main UI)**.

A Streamlit-based web UI for debugging, monitoring and administering OpenAaaS tasks.

## Features

- 📊 **Task Overview & Debugging**: View all tasks in a card layout, inspect task details (input/output/files, etc.), and cancel tasks
- 🔧 **Admin View**: View all user tasks, filter tasks by user
- 🔄 **Auto Refresh**: Real-time updates with configurable refresh interval
- 🔍 **Status Filtering**: Filter tasks by status (All/Pending/Running/Completed/Failed/Cancelled/Cancelling)
- ⚙️ **Flexible Configuration**: Supports CLI arguments, environment variables, and configuration files

## Installation

### Using `uv` (recommended)

```bash
uv tool install open-aaas-dashboard
```

### Using `pip`

```bash
pip install open-aaas-dashboard
```

### Development Install

```bash
cd OpenAaaS/dash
pip install -e .
```

## Usage

### Command Line

```bash
# With command line arguments
aaas-dashboard --server-url http://localhost:8080 --api-key ak_xxx

# With environment variables
export OAAS_SERVER_URL=http://localhost:8080
export OAAS_API_KEY=ak_xxx
aaas-dashboard
```

### Configuration Priority

Configuration is loaded in the following priority (highest first):

1. **Command line arguments**: `--server-url`, `--api-key`
2. **Environment variables**: `OAAS_SERVER_URL`, `OAAS_API_KEY`
3. **Config file**: `~/.config/aaas-dashboard/config.toml`

### Config File Format

Create `~/.config/aaas-dashboard/config.toml`:

```toml
server_url = "http://localhost:8080"
api_key = "ak_xxx"
```

## Development

```bash
# Install dependencies
pip install -e ".[dev]"

# Run the dashboard
streamlit run src/aaas_dashboard/app.py
```

## License

MIT License
