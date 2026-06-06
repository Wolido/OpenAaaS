# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] - 2026-06-06

### Added
- 美化 client-app 视觉基线，新增品牌配色、统一图标、服务卡片、状态徽章和任务列表演示状态。
- 增加客户端美化流程文档与开发环境示例配置，便于后续视觉迭代。

## [0.5.1] - 2026-05-13

### Fixed
- 修复 client-app 错误提示过于技术化的问题（issue #43）
- 在 HTTP 层正确解析服务端 JSON 错误体
- 新增 friendlyErrorMessage 将常见错误映射为用户友好文案
- 覆盖附件过大等场景

## [0.5.0] - 2026-05-12

### Fixed
- 修复服务授权状态不刷新的问题（#48）
- HomeView 添加手动刷新按钮，支持刷新服务列表和授权状态
- ServiceDetailView 进入页面及路由切换时自动刷新服务列表

## [0.4.0] - 2026-05-11

### Fixed
- 修复文件下载后缺少完成反馈的问题（#44）
- 使用 Tauri dialog + fs 插件让用户选择保存路径，保存完成后显示完整路径的 toast 提示

## [0.3.2] - 2026-05-11

### Fixed
- 修复任务返回结果的Markdown渲染问题（表格、标题、换行等格式异常）

## [0.3.1] - 2026-05-11

### Fixed
- 修复长任务轮询因失败阈值过低而永久停止的问题

## [0.3.0] - 2026-05-11

### Fixed
- 修复server的反代301时，强制修改请求method引发的请求错误
- 修复重定向次数的问题

## [0.2.1] - 2026-05-11

### Fixed
- 增加MacOS版自签名，防止安装时直接报损坏

## [0.2.0] - 2026-05-10

### Added
- 更换logo

## [0.1.1] - 2026-05-10

### Fixed
- 修复output_prompt填写逻辑

## [0.1.0] - 2026-05-09

### Added
- Initial release
