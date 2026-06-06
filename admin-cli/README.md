# OpenAaaS Admin CLI

<p align="right">中文 | <a href="./README.en.md">English</a></p>

OpenAaaS Server 的命令行管理工具，面向管理员提供纯命令行、表格化的服务管理、用户管理、权限管理、任务统计和健康检查能力。

## 编译安装

需要安装 Rust 工具链（1.85+）：

```bash
cd admin-cli
cargo build --release
```

编译产物为 `target/release/openaaas-admin`。

## 命令总览

```bash
openaaas-admin [OPTIONS] <COMMAND>
```

### 全局选项

| 选项 | 说明 |
|------|------|
| `--server-url <URL>` | 指定 Server 地址，覆盖配置和环境变量 |
| `--api-key <KEY>` | 指定 Admin API Key，覆盖配置和环境变量 |

### 子命令

| 命令 | 说明 |
|------|------|
| `config init` | 交互式或参数式初始化配置 |
| `config show` | 查看当前配置 |
| `health` | 检查 Server 健康状态和 Admin Key 有效性 |
| `services list` | 列出所有服务 |
| `services show <id>` | 查看服务详情 |
| `services create` | 创建服务 |
| `services update <id>` | 更新服务信息 |
| `services delete <id>` | 删除服务（含确认保护） |
| `users list` | 列出所有用户 |
| `users delete <id>` | 删除用户（含确认保护） |
| `permissions list` | 按用户或服务查询权限 |
| `permissions grant` | 为用户授予服务权限 |
| `permissions revoke` | 撤销用户的服务权限 |
| `stats` | 查看任务统计概览 |

## 配置说明

配置文件路径：`~/.config/openaaas-admin/config.toml`

### 初始化配置

交互式（推荐，API Key 输入隐藏）：

```bash
openaaas-admin config init
```

带参数初始化：

```bash
openaaas-admin config init --server-url http://localhost:8080 --api-key ak_admin_xxx
```

交互式流程示例：

```bash
$ openaaas-admin config init
Enter server URL [http://localhost:8080]:
Enter admin API key:
✓ Configuration saved to /home/user/.config/openaaas-admin/config.toml
```

### 查看配置

```bash
openaaas-admin config show
```

输出示例：

```
+------------+------------------------+
| Key        | Value                  |
+------------+------------------------+
| server_url | http://localhost:8080  |
| api_key    | ak_admin_12***         |
+------------+------------------------+
```

### 配置优先级

配置项加载优先级（从高到低）：

1. CLI 参数：`--server-url`、`--api-key`
2. 环境变量：`OPENAAAS_SERVER_URL`、`OPENAAAS_API_KEY`
3. 配置文件：`~/.config/openaaas-admin/config.toml`

## 安全提醒

> ⚠️ **避免直接在命令行中传递 `--api-key`** —— 该值会被记录到 Shell 历史中。建议使用以下方式之一：
> - 环境变量：`OPENAAAS_API_KEY=ak_admin_xxx openaaas-admin ...`
> - 配置文件：`openaaas-admin config init`
>
> 配置文件默认权限为 `0o600`（仅所有者可读写），且 `config show` 输出中的 API Key 会自动脱敏显示。

## 各命令详细用法

### 健康检查

```bash
openaaas-admin health
```

同时检查公共 `/health` 接口和受保护的 Admin 接口，验证 Server 连通性与 Admin Key 有效性。

输出示例：

```
Checking server health at http://localhost:8080 ...
  ● Health: healthy
  ● Version: 0.9.0
  ● Timestamp: 2026-06-06T12:00:00Z

Checking admin API key ...
  ● Admin API key is valid

✓ All checks passed
```

### 服务管理

#### 列出所有服务

```bash
openaaas-admin services list
```

#### 查看服务详情

```bash
openaaas-admin services show <id>
```

#### 创建服务

```bash
openaaas-admin services create \
  --name "code-agent" \
  --description "代码审查 Agent" \
  --usage "提交 PR 进行代码审查" \
  --public
```

- `--public`：公开服务，所有用户默认可访问；不加则为受限服务
- 创建成功后会输出注册 Token（`registration_token`），供 `agent-core` 注册时使用，请妥善保存

#### 更新服务

```bash
# 更新名称和描述
openaaas-admin services update <id> --name "new-name" --description "new desc"

# 设为公开
openaaas-admin services update <id> --public

# 设为受限
openaaas-admin services update <id> --restricted
```

支持部分更新，仅传入需要修改的字段。`--public` 和 `--restricted` 不能同时使用。

#### 删除服务

普通删除（含 `y/N` 确认）：

```bash
openaaas-admin services delete <id>
```

强制删除（取消活跃任务，需输入服务 ID 二次确认）：

```bash
openaaas-admin services delete <id> --force
```

强制删除成功后会显示被取消和保留的任务数量。

### 用户管理

#### 列出所有用户

```bash
openaaas-admin users list
```

输出中 API Key 会自动脱敏显示（前 8 位 + `***`）。

#### 删除用户

```bash
openaaas-admin users delete <id>
```

删除前会提示 `Are you sure you want to delete user <id>? [y/N]:`，输入 `y` 或 `yes` 确认。

### 权限管理

#### 按用户查询权限

```bash
openaaas-admin permissions list --user <user_id>
```

#### 按服务查询授权用户

```bash
openaaas-admin permissions list --service <service_id>
```

如果目标服务为公开服务，会提示所有用户默认拥有访问权限。

#### 授予权限

```bash
openaaas-admin permissions grant --user <user_id> --service <service_id>
```

#### 撤销权限

```bash
openaaas-admin permissions revoke --user <user_id> --service <service_id>
```

### 任务统计

```bash
openaaas-admin stats
```

拉取全部任务并按状态统计，输出示例：

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

## 技术栈

- Rust 2024 edition
- `clap` — 命令行解析
- `reqwest` — HTTP 客户端
- `tokio` — 异步运行时
- `serde` / `serde_json` — 序列化
- `toml` — 配置持久化
- `tabled` — 表格输出
- `colored` — 终端彩色输出
- `thiserror` — 错误枚举
- `dirs` — 配置目录解析
- `rpassword` — 隐藏密码输入
- `wiremock` — HTTP 测试模拟（dev）
