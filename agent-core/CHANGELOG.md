# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- 修复 `spawn_task` 中的静默失败：所有 `let _ = ...` 改为显式 `error!` 日志
- `upsert_task` 失败后立即上报 Failed 并终止任务执行，不再无状态记录地继续运行
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

[0.4.0]: https://github.com/Wolido/OpenAaaS/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/Wolido/OpenAaaS/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/Wolido/OpenAaaS/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/Wolido/OpenAaaS/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Wolido/OpenAaaS/compare/v0.1.3...v0.2.0
[0.1.3]: https://github.com/Wolido/OpenAaaS/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/Wolido/OpenAaaS/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Wolido/OpenAaaS/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Wolido/OpenAaaS/releases/tag/v0.1.0
