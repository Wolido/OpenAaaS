# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2] - 2026-07-28

### Added
- 接入 CI 质量门禁：`cargo fmt --check` 与 `cargo clippy --all-targets -- -D warnings`（#134）
- 接入依赖安全扫描：`cargo audit --file admin-cli/Cargo.lock`（#134）
- 提交 `admin-cli/Cargo.lock` 以锁定依赖版本（#134）

### Changed
- 应用 `cargo fmt` 代码格式化（#134）

## [0.1.1] - 2026-06-12

### Fixed
- admin-cli `--help` 输出增加初始化配置引导说明，提示首次使用执行 `config init`（Issue #123）

## [0.1.0] - 2026-06-06

### Added
- 初始版本发布
- 服务管理：list, show, create, update, delete（含 force delete）
- 用户管理：list, delete（含确认保护）
- 权限管理：按用户/服务查询、grant、revoke
- 任务统计概览（stats）
- 配置管理：init（交互式，API Key 输入隐藏）、show
- 健康检查：验证 Server 连通性和 Admin Key 有效性
- 39 个单元/集成测试（含 wiremock HTTP 测试）
- 安全配置：文件权限 0o600、API Key 脱敏、删除确认

[0.1.2]: https://github.com/Wolido/OpenAaaS/releases/tag/admin-cli-v0.1.2
[0.1.1]: https://github.com/Wolido/OpenAaaS/releases/tag/admin-cli-v0.1.1
[0.1.0]: https://github.com/Wolido/OpenAaaS/releases/tag/admin-cli-v0.1.0
