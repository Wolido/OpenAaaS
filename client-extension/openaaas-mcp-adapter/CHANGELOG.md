# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-06-16

### Added

- 新增 `poll_task` 工具：轮询任务直到获得最终结果，每 20 秒查询一次，默认无超时，支持 `timeout_seconds` 参数限制最大轮询时长。
- 新增 `CHANGELOG.md`，用于记录版本变更。

### Changed

- `poll_task` 工具描述中明确说明：Agent 禁止主动调用，仅在用户明确说"帮我等结果"/"轮询任务"时才使用。
- `get_task` 工具描述中补充：仅在用户明确要求查询任务状态时调用，不要主动轮询。
- `submit_task` 返回信息中补充：提交后应询问用户是否需要等待结果，不要主动调用 `poll_task` 或 `get_task`。
- 更新 MCP 客户端配置示例，新增 `toolTimeoutMs` 字段说明，用于长任务轮询场景。
- 在主项目 `README.md`、MCP 适配器中文和英文 `README.md` 中补充 `toolTimeoutMs` 配置示例及生效提示。
- 在 MCP 适配器中文和英文 `README.md` 的工具列表、参数表和标准使用流程中加入 `poll_task` 说明。

### Notes

- `toolTimeoutMs` 由 MCP 客户端解析，实际生效情况取决于具体客户端实现；某些客户端或 Agent 工具本身可能仍有独立的超时限制。
