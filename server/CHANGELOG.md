# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
