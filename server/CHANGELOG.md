# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.12.0] - 2026-07-28

### Added
- 接入 CI 质量门禁：`cargo fmt --check` 与 `cargo clippy --all-targets -- -D warnings`（#134）
- 接入依赖安全扫描：`cargo audit --file server/Cargo.lock`（#134）
- 提交 `server/Cargo.lock` 以锁定依赖版本（#134）

### Changed
- 修复约 112 个 clippy warning（#134）
- 升级 `sqlx` 从 `0.8.6` 到 `0.9.0`（#150）
- 将 `server` 中的动态 SQL 重构为 `sqlx::QueryBuilder`，以适配 sqlx 0.9 的 `SqlSafeStr` 安全要求（#150）
- 将 session ID 生成从 UUID v4 切换为 UUID v7，提升可审计性和索引效率（#172）

### Security
- 消除动态 SQL 拼接，改用参数化查询构建（#150）

## [0.11.0] - 2026-06-20

### Added
- 新增 API Key 维度滑动窗口限流：Client API Key 100 次/分钟，Agent API Key 150 次/分钟
- 新增审计日志模块，记录 Client/Agent 注册、认证失败等关键安全事件
- 新增 `trust_x_forwarded_for` 配置，支持从 `X-Forwarded-For` 提取来源 IP
- 新增 `rate_limit` 相关配置项与示例

### Changed
- 统一认证失败错误响应为“认证失败”，避免泄露内部细节（如 key 是否存在）
- 重构 `auth` 模块，提取 API Key 哈希与统一错误处理逻辑

### Security
- 限流键使用 `auth::hash_api_key` 生成的 HMAC 哈希，避免在内存中保存原始 API Key
- 审计日志对 `registration_token` 进行字符级掩码，防止完整 token 泄露

## [0.10.0] - 2026-06-06

### Added
- `POST /api/v1/client/tasks` 新增 `application/json` 请求体支持，不强制依赖 `multipart/form-data`
- 新增 JSON 路径的 `session_id` 校验（非空、不含 `..` 和 `/`、长度 ≤64、仅允许字母数字及 `_-`）
- 新增 7 个 JSON 路径集成测试，覆盖成功创建、缺少字段、非法 session_id、body 超限等场景

### Changed
- `create_task` handler 重构为 Content-Type 分发器，内部提取 `create_task_inner` 公共逻辑和 `parse_multipart_fields` 解析函数
- Discovery API 文档更新：`create_task` 的 `content_type` 和 `files` 字段说明明确标注 JSON 方式不支持附件上传

### Security
- JSON 路径限制请求体大小为 1MB（`axum::body::to_bytes(req.into_body(), 1024 * 1024)`），防止恶意大请求导致内存耗尽

### Fixed
- 修复 `accept_task` / `complete_task` 中的 race condition：使用数据库事务包裹 tasks 状态更新和 services 负载更新
- `agent_current_load` 改为原子 +1 / -1，避免并发时计数不准确
- `complete_task` SQL 的 `agent_current_load` 递减增加 `> 0` 保护，避免异常状态下出现负数负载或误改 agent_status
- 事务回滚失败不再阻断业务语义，返回 `Ok(false)`

## [0.9.0] - 2026-06-06

### Added
- 新增 `GET /api/v1/admin/services/{id}/users` 接口，支持管理员查询某服务的授权用户清单
- 返回 `is_public` 标志和授权用户列表（user_id, user_name, role, granted_at）
- 包含集成测试覆盖：服务不存在（404）、公开服务无显式授权、受限服务有授权用户

### Changed
- 代码格式化（cargo fmt）

### Added
- 后台清理任务测试补充（仅删除文件，保留数据库记录）

## [0.8.0] - 2026-06-01

### Changed
- 后台清理任务仅删除本地磁盘文件，保留 tasks 和 task_files 数据库记录（此前会同时删除数据库记录）
- 更新相关配置注释和文档注释，明确保留策略

## [0.7.1] - 2026-05-29

### Added
- 新增 `PUT /api/v1/services/{id}` 接口，支持管理员更新服务信息（name、description、usage、is_public）
- 支持部分更新，仅传入需要修改的字段
- 空请求体时直接返回当前服务信息

## [0.7.0] - 2026-05-28

### Changed
- `poll_handler` 不再更新心跳时间戳，poll 变为纯读操作，心跳完全由独立 heartbeat 接口负责
- 离线检测后台任务间隔从 30 秒调整为 10 秒

## [0.6.1] - 2026-05-25

### Fixed
- 修复server中测试代码里存在的unused问题
- 修复server删除服务时日志记录缺失

## [0.6.0] - 2026-05-19

### Added
- 启用 SQLite WAL 模式，提升高并发写场景下数据库读写性能

## [0.5.1] - 2026-05-16

### Changed
- 优化 `--help` 输出，补充 Server 职责、常用流程、首次运行默认行为和子命令说明。

## [0.5.0] - 2026-05-14

### Changed
- 重构 main.rs，将 942 行的入口文件按职责拆分为多个聚焦模块（cli, cmd/run, cmd/detached, cmd/stop, cmd/status, bg_tasks）

## [0.4.1] - 2026-05-10

### Fixed
- 修复service busy状态的判定逻辑

## [0.4.0] - 2026-05-10

### Fixed
- 删除服务预计等待时间，此预计时间不准确，容易引起误解

## [0.3.1] - 2026-05-07

### Added
- 二进制添加`--version`参数


## [0.3.0] - 2026-05-06

### Added
- Add interactive listen address prompt on first startup

## [0.2.1] - 2026-05-01

### Changed
- Revise discovery endpoint display content

## [0.2.0] - 2026-05-01

### Added
- Add instructions field to /discovery endpoint to help plugin-less clients try the server

## [0.1.1] - 2026-04-30

### Fixed
- Fix Windows path issue

## [0.1.0] - 2026-04-30

### Added
- Initial release

[0.12.0]: https://github.com/Wolido/OpenAaaS/compare/server-v0.11.0...server-v0.12.0
[0.11.0]: https://github.com/Wolido/OpenAaaS/compare/server-v0.10.0...server-v0.11.0
[0.10.0]: https://github.com/Wolido/OpenAaaS/compare/server-v0.9.0...server-v0.10.0
[0.9.0]: https://github.com/Wolido/OpenAaaS/compare/server-v0.8.0...server-v0.9.0
[0.8.0]: https://github.com/Wolido/OpenAaaS/compare/server-v0.7.1...server-v0.8.0
[0.7.1]: https://github.com/Wolido/OpenAaaS/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/Wolido/OpenAaaS/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/Wolido/OpenAaaS/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/Wolido/OpenAaaS/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/Wolido/OpenAaaS/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/Wolido/OpenAaaS/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/Wolido/OpenAaaS/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/Wolido/OpenAaaS/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/Wolido/OpenAaaS/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/Wolido/OpenAaaS/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/Wolido/OpenAaaS/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Wolido/OpenAaaS/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/Wolido/OpenAaaS/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Wolido/OpenAaaS/releases/tag/v0.1.0
