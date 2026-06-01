# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[0.8.0]: https://github.com/Wolido/OpenAaaS/compare/v0.7.1...v0.8.0
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
