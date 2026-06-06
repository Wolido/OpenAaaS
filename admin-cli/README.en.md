# OpenAaaS Admin CLI

<p align="right"><a href="./README.md">中文</a> | English</p>

A command-line administration tool for OpenAaaS Server, providing administrators with pure CLI, table-oriented service management, user management, permission management, task statistics, and health checks.

## Build & Install

Requires the Rust toolchain (1.85+):

```bash
cd admin-cli
cargo build --release
```

The compiled binary is located at `target/release/openaaas-admin`.

## Command Overview

```bash
openaaas-admin [OPTIONS] <COMMAND>
```

### Global Options

| Option | Description |
|--------|-------------|
| `--server-url <URL>` | Specify the Server URL, overrides config and environment variables |
| `--api-key <KEY>` | Specify the Admin API Key, overrides config and environment variables |

### Subcommands

| Command | Description |
|---------|-------------|
| `config init` | Initialize configuration interactively or with parameters |
| `config show` | Show current configuration |
| `health` | Check Server health status and Admin Key validity |
| `services list` | List all services |
| `services show <id>` | Show service details |
| `services create` | Create a new service |
| `services update <id>` | Update service information |
| `services delete <id>` | Delete a service (with confirmation prompt) |
| `users list` | List all users |
| `users delete <id>` | Delete a user (with confirmation prompt) |
| `permissions list` | Query permissions by user or service |
| `permissions grant` | Grant a user permission to a service |
| `permissions revoke` | Revoke a user's permission to a service |
| `stats` | Show task statistics overview |

## Configuration

Config file path: `~/.config/openaaas-admin/config.toml`

### Initialize Config

Interactively (recommended, API Key input is hidden):

```bash
openaaas-admin config init
```

With parameters:

```bash
openaaas-admin config init --server-url http://localhost:8080 --api-key ak_admin_xxx
```

Interactive example:

```bash
$ openaaas-admin config init
Enter server URL [http://localhost:8080]:
Enter admin API key:
✓ Configuration saved to /home/user/.config/openaaas-admin/config.toml
```

### Show Config

```bash
openaaas-admin config show
```

Example output:

```
+------------+------------------------+
| Key        | Value                  |
+------------+------------------------+
| server_url | http://localhost:8080  |
| api_key    | ak_admin_12***         |
+------------+------------------------+
```

### Configuration Priority

Config items are loaded with the following priority (highest first):

1. CLI flags: `--server-url`, `--api-key`
2. Environment variables: `OPENAAAS_SERVER_URL`, `OPENAAAS_API_KEY`
3. Config file: `~/.config/openaaas-admin/config.toml`

## Security Notice

> ⚠️ **Avoid passing `--api-key` directly on the command line** — the value will be saved in your shell history. Prefer one of the following:
> - Environment variable: `OPENAAAS_API_KEY=ak_admin_xxx openaaas-admin ...`
> - Config file: `openaaas-admin config init`
>
> The config file is created with permissions `0o600` (owner-only read/write), and the API Key is automatically masked in `config show` output.

## Detailed Command Usage

### Health Check

```bash
openaaas-admin health
```

Checks both the public `/health` endpoint and the protected Admin endpoint to verify Server connectivity and Admin Key validity.

Example output:

```
Checking server health at http://localhost:8080 ...
  ● Health: healthy
  ● Version: 0.9.0
  ● Timestamp: 2026-06-06T12:00:00Z

Checking admin API key ...
  ● Admin API key is valid

✓ All checks passed
```

### Service Management

#### List all services

```bash
openaaas-admin services list
```

#### Show service details

```bash
openaaas-admin services show <id>
```

#### Create a service

```bash
openaaas-admin services create \
  --name "code-agent" \
  --description "Code review agent" \
  --usage "Submit PRs for code review" \
  --public
```

- `--public`: Public service, all users have access by default; omit for restricted services
- Upon successful creation, the registration token (`registration_token`) will be printed. Save it for `agent-core` registration.

#### Update a service

```bash
# Update name and description
openaaas-admin services update <id> --name "new-name" --description "new desc"

# Make public
openaaas-admin services update <id> --public

# Make restricted
openaaas-admin services update <id> --restricted
```

Supports partial updates — only pass the fields you want to change. `--public` and `--restricted` cannot be used together.

#### Delete a service

Normal delete (with `y/N` confirmation):

```bash
openaaas-admin services delete <id>
```

Force delete (cancels active tasks, requires typing the service ID for secondary confirmation):

```bash
openaaas-admin services delete <id> --force
```

On successful force delete, the number of cancelled and retained tasks will be displayed.

### User Management

#### List all users

```bash
openaaas-admin users list
```

The API Key is automatically masked in output (first 8 chars + `***`).

#### Delete a user

```bash
openaaas-admin users delete <id>
```

A confirmation prompt `Are you sure you want to delete user <id>? [y/N]:` is shown. Enter `y` or `yes` to confirm.

### Permission Management

#### List permissions by user

```bash
openaaas-admin permissions list --user <user_id>
```

#### List authorized users by service

```bash
openaaas-admin permissions list --service <service_id>
```

If the target service is public, a warning will indicate that all users have access by default.

#### Grant permission

```bash
openaaas-admin permissions grant --user <user_id> --service <service_id>
```

#### Revoke permission

```bash
openaaas-admin permissions revoke --user <user_id> --service <service_id>
```

### Task Statistics

```bash
openaaas-admin stats
```

Fetches all tasks and counts them by status. Example output:

```
+-----------+-------+
| Metric    | Count |
+-----------+-------+
| Total     |  152  |
| Pending   |   3   |
| Running   |   7   |
| Completed |  128  |
| Failed    |   9   |
| Cancelled |   4   |
| Cancelling|   1   |
+-----------+-------+
```

## Tech Stack

- Rust 2024 edition
- `clap` — command line parsing
- `reqwest` — HTTP client
- `tokio` — async runtime
- `serde` / `serde_json` — serialization
- `toml` — config persistence
- `tabled` — table output
- `colored` — colored terminal output
- `thiserror` — error enums
- `dirs` — config directory resolution
- `rpassword` — hidden password input
- `wiremock` — HTTP test mocking (dev)
