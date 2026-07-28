# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.2] - 2026-07-28

### Added
- 接入 CI 依赖安全扫描：`pip-audit`（#134）
- 提交 `uv.lock` 以锁定依赖版本（#134）

### Security
- 通过 `uv lock --upgrade` 修复 20 个已知 CVE（涉及 cryptography、idna、pydantic-settings、pyjwt、python-multipart、starlette 等）（#134）

## [0.3.1] - 2026-06-17

### Fixed

- 防止 Agent 在 `submit_task` 后自动轮询：
  - `poll_task` 工具描述明确：Agent 禁止主动调用，仅在用户明确说"帮我等结果"/"轮询任务"时才使用。
  - `get_task` 工具描述明确：仅在用户明确要求查询任务状态时调用，不要主动轮询。
  - `submit_task` 返回信息提示：提交后应询问用户是否需要等待结果，未明确授权不要调用 `poll_task` 或 `get_task`。
- 修复 `download_result` 返回路径不清晰问题：返回结果中明确列出每个文件的完整路径，并提示读取文件时不要推测子目录。

### Changed

- 同步更新中英文 README 中 `poll_task`、`get_task`、`submit_task` 和 `download_result` 的工具描述及使用流程。

## [0.3.0] - 2026-06-16

### Added

- 新增 `poll_task` 工具：轮询任务直到获得最终结果，每 20 秒查询一次，默认无超时，支持 `timeout_seconds` 参数限制最大轮询时长。
- 新增 `CHANGELOG.md`，用于记录版本变更。

### Changed

- 更新 MCP 客户端配置示例，新增 `toolTimeoutMs` 字段说明，用于长任务轮询场景。
- 在主项目 `README.md`、MCP 适配器中文和英文 `README.md` 中补充 `toolTimeoutMs` 配置示例及生效提示。
- 在 MCP 适配器中文和英文 `README.md` 的工具列表、参数表和标准使用流程中加入 `poll_task` 说明。

### Notes

- `toolTimeoutMs` 由 MCP 客户端解析，实际生效情况取决于具体客户端实现；某些客户端或 Agent 工具本身可能仍有独立的超时限制。

[0.3.2]: https://github.com/Wolido/OpenAaaS/releases/tag/v0.3.2
