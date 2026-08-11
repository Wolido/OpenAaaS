# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `ExecutorConfig` 新增 `gpu: Option<GpuConfig>` 配置项（`gpu.vendor` / `gpu.devices`，默认关闭）。开启后任务容器通过 `docker run --gpus` 参数挂载 GPU：v1 仅支持 `nvidia`（`amd` / `intel` 为预留值，不生成 GPU 参数），`devices` 缺省为 `"all"`，也可按索引指定如 `"0,1"`。新增启动 GPU 预检：macOS 与 Windows 原生配置 GPU 会阻断启动，Linux / WSL2 缺 nvidia runtime 或 `docker info` 失败仅警告；崩溃恢复（recover_tasks）时会取消残留容器，防止 GPU 显存泄漏。仅建议管理员在配置文件中显式开启。`config.toml.example` 与 README 已同步更新。
- `ExecutorConfig` 新增 `enable_host_access: bool` 配置项（默认 `false`）。开启后任务容器启动时注入 `--add-host host.docker.internal:host-gateway`，容器内可通过 `http://host.docker.internal:<port>` 访问宿主机服务。需 Docker 20.10+，仅建议管理员在配置文件中显式开启。`config.toml.example` 与 README 已同步更新。

### Fixed
- 修复 init/生成的配置文件中 `memory_limit` 未设置时不输出的问题，现在会以注释示例形式（`# memory_limit = "4g"`）展示

## [0.4.3] - 2026-07-28

### Added
- 接入 CI 质量门禁：`cargo fmt --check` 与 `cargo clippy --all-targets --features test-utils -- -D warnings`（#134）
- 接入依赖安全扫描：`cargo audit --file agent-core/Cargo.lock`（#134）
- 提交 `agent-core/Cargo.lock` 以锁定依赖版本（#134）

### Changed
- 修复 3 个 clippy warning（#134）
- 升级 `reqwest` 从 `0.11` 到 `0.12`（#134）

## [0.4.2] - 2026-06-19

### Fixed
- 修复 `spawn_task` 中的静默失败：所有 `let _ = ...` 改为显式 `error!` 日志
- `upsert_task` 失败后立即上报 Failed 并终止任务执行，不再无状态记录地继续运行
- 移除 `register.rs` 中 API Key 的 stdout 输出及不安全切片，并增加 Service ID 为空的回退提示 (#137)
- 在 info 日志中对 `registration_token` 脱敏，避免非调试日志泄露敏感信息 (#135)

## [0.4.1] - 2026-06-04

### Changed
- 升级 sqlx 依赖从 0.7 到 0.8，与 server 保持一致 (#81)

## [0.4.0] - 2026-05-28

### Changed
- 心跳间隔从 30 秒调整为 20 秒，与 server 固定超时 60 秒形成 T/3 比例，容错 3 次连续丢包

## [0.3.1] - 2026-05-16

### Changed
- 优化 `--help` 输出，补充 Agent Core 职责、首次使用流程、默认值和子命令说明。

## [0.3.0] - 2026-05-14

### Changed
- 重构 main.rs，将 476 行的入口文件按职责拆分为多个聚焦模块（cmd/run, cmd/detached, cmd/stop, cmd/status, cmd/init, cmd/register）

## [0.2.1] - 2026-05-07

### Added
- 二进制添加`--version`参数

## [0.2.0] - 2026-05-06

### Added
- Add interactive executor image and capacity prompts on first startup

## [0.1.3] - 2026-05-01

### Fixed
- Fix Windows path issue

## [0.1.2] - 2026-04-30

### Added
- Add release workflow

### Fixed
- Fix Windows compatibility

## [0.1.1] - 2026-04-30

### Fixed
- Fix cargo test errors

## [0.1.0] - 2026-04-30

### Added
- Initial release

[0.4.3]: https://github.com/Wolido/OpenAaaS/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/Wolido/OpenAaaS/compare/v0.4.1...v0.4.2
[0.4.0]: https://github.com/Wolido/OpenAaaS/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/Wolido/OpenAaaS/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/Wolido/OpenAaaS/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/Wolido/OpenAaaS/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Wolido/OpenAaaS/compare/v0.1.3...v0.2.0
[0.1.3]: https://github.com/Wolido/OpenAaaS/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/Wolido/OpenAaaS/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Wolido/OpenAaaS/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Wolido/OpenAaaS/releases/tag/v0.1.0
