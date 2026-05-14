# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-05-14

### Changed
- 重构 main.rs，将 476 行的入口文件按职责拆分为多个聚焦模块（cmd/run, cmd/detached, cmd/stop, cmd/status, cmd/init, cmd/register）

## [0.2.1]- 2026-05-07

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
