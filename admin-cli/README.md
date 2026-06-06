# openaaas-admin

OpenAaaS Server Admin CLI — A pure command-line, table-oriented administration tool for OpenAaaS Server.

## Features

- **Service Management**: list, show, create, update, delete (with force), services
- **User Management**: list users, delete users
- **Permission Management**: list, grant, revoke user service permissions
- **Task Statistics**: overview of task counts by status
- **Health Check**: verify server health and admin API key validity
- **Configuration**: interactive or param-based config initialization

## Requirements

- Rust **1.85+** (required for the 2024 edition)

## Installation

```bash
cd admin-cli
cargo build --release
```

The binary will be at `target/release/openaaas-admin`.

## Security Notice

> ⚠️ **Avoid passing `--api-key` directly on the command line** — the value will be saved in your shell history. Prefer one of the following:
> - Environment variable: `OPENAAAS_API_KEY`
> - Config file: `openaaas-admin config init`

## Configuration

Config file location: `~/.config/openaaas-admin/config.toml`

### Initialize config

Interactive:
```bash
openaaas-admin config init
```

With parameters:
```bash
openaaas-admin config init --server-url http://localhost:8080 --api-key ak_admin_xxx
```

### Show config

```bash
openaaas-admin config show
```

### Priority

Configuration is loaded with the following priority (highest first):

1. CLI flags: `--server-url`, `--api-key`
2. Environment variables: `OPENAAAS_SERVER_URL`, `OPENAAAS_API_KEY`
3. Config file: `~/.config/openaaas-admin/config.toml`

## Commands

### Health Check

```bash
openaaas-admin health
```

Checks both the public `/health` endpoint and the admin `/api/v1/admin/users` endpoint.

### Services

List all services:
```bash
openaaas-admin services list
```

Show service details:
```bash
openaaas-admin services show <id>
```

Create a service:
```bash
openaaas-admin services create --name "code-agent" --description "Code review agent" --usage "Submit PRs for review" --public
```

Update a service:
```bash
openaaas-admin services update <id> --name "new-name" --description "new desc"
openaaas-admin services update <id> --public
openaaas-admin services update <id> --restricted
```

Delete a service:
```bash
openaaas-admin services delete <id>
```

Force delete (cancels active tasks):
```bash
openaaas-admin services delete <id> --force
```

### Users

List all users:
```bash
openaaas-admin users list
```

Delete a user:
```bash
openaaas-admin users delete <id>
```

### Permissions

List user's permissions:
```bash
openaaas-admin permissions list --user <user_id>
```

Grant permission:
```bash
openaaas-admin permissions grant --user <user_id> --service <service_id>
```

Revoke permission:
```bash
openaaas-admin permissions revoke --user <user_id> --service <service_id>
```

### Stats

Show task statistics:
```bash
openaaas-admin stats
```

## Authentication

All admin endpoints require Bearer token authentication. The CLI automatically sends:

```
Authorization: Bearer <admin_api_key>
```

## Error Handling

- HTTP 4xx/5xx errors display the server's `message`/`detail`/`error` field
- Network errors suggest checking `--server-url`
- All subcommands return non-zero exit codes on failure

## Development

Run tests:
```bash
cargo test
```

Run in debug mode:
```bash
cargo run -- --help
```

## Tech Stack

- Rust 2024 edition
- `clap` — command line parsing
- `reqwest` — HTTP client
- `tokio` — async runtime
- `serde`/`serde_json` — serialization
- `toml` — config persistence
- `tabled` — table output
- `colored` — colored terminal output
- `thiserror` — error enums
- `dirs` — config directory resolution
