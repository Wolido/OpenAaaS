# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2026-08-11

### Added
- 任务状态轮询按任务运行时长分档：提交后立即轮询一次，0~2 分钟每 10 秒、2~10 分钟每 20 秒、10 分钟后每 30 秒（运行时长以服务端 created_at 计算，会话重建不会误入高频档）
- 限流与错误退避：429 响应遵守服务端 Retry-After（钳制 1 秒至 5 分钟）；网络/服务器错误按连续失败次数退避（60 秒→120 秒→240 秒→300 秒封顶），成功后恢复运行时长分档间隔
- 永久错误（401/404）停止该任务轮询，错误信息展示在 /OpenAaaS-tasks 面板，停止状态持久化，会话重建后不会恢复轮询
- 新增 polling.ts 纯函数模块（轮询间隔/退避/Retry-After 解析），并引入 node:test 测试框架（65 个用例）

### Fixed
- 修复取消任务与在途轮询响应的状态回写竞态（generation 机制，过期响应不再更新状态与持久化）
- 修复 Retry-After: 0 导致 0 毫秒紧循环重试的问题
- 修复永久失败任务在会话重建后复活轮询的问题
- 修复 session 关闭瞬间在途响应可能回写状态的问题
- 修复 npm 包发布缺少 polling.ts 导致扩展加载失败的问题

## [1.0.2] - 2026-08-09

### Fixed
- 移除 tool_result handler 中残留的 debug 日志（该日志每次工具调用都会写入 stdout，被 pi TUI 捕获并显示为红色错误横幅），无逻辑变更

### Changed
- README 安装方式改为 `pi install npm:open-aaas-pi-extension` 命令
- 子项目目录从 `client-extension/pi-extension/` 迁移至仓库根目录 `pi-extension/`（无代码逻辑变更）
- 依赖更新：`adm-zip` 0.5.17→0.6.0、`pi-coding-agent/pi-tui` 0.80.3→0.80.10、`@types/node` 26.0.1→26.1.1、`typebox` 1.3.3→1.3.6、`typescript` 5.9.3→7.0.2（#177）

## [1.0.1] - 2026-07-03

### Added
- 新增 LICENSE 文件
- `package.json` 补充 npm 发布元数据（license、author、homepage、repository、keywords、publishConfig、peerDependencies、pi.extensions）
- 提交 `package-lock.json` 并接入仓库级 CI 质量门禁

### Changed
- 默认服务器地址从 `localhost:8080` 改为 `https://api.open-aaas.com`
- README 翻译为英文，后经调整：默认 README 为中文，英文版为 README.en.md，并修复语言切换链接与文件名
- 简化错误处理逻辑，统一处理 Windows/Unix 换行符
- Node.js 最低版本要求从 >=18.0.0 提升至 >=20.3.0
- 依赖更新：`mime-types` 2.1.35→3.0.2、`@types/mime-types` 2.1.4→3.0.1、`@types/node` 20.19.43→26.0.1
- 依赖更新：`adm-zip` 0.5.17→0.5.18、`@types/node` 26.0.1→26.1.0、`typescript` 5.9.3→6.0.3

### Fixed
- 修复可对同一个 URL 重复注册的问题
- 修复经 server 反向代理 301 重定向时请求 method 可能被修改的问题
- 修复 HTTP/2 gzip 响应乱码：自动解压 gzip/deflate/br 编码的响应，解压失败时回退原始 body
- 修复服务器返回 HTML 错误页（如 nginx 502）时插件原样输出 HTML 源码的问题：读取错误响应时检测并跳过 HTML body，返回干净的 HTTP 状态文本；content-type 检查大小写不敏感，检测前忽略空白与 BOM（#62）
- 修复配置路径问题

## [1.0.0] - 2026-04-29

### Added
- 初始版本发布
- 统一的 `OpenAaaS` 工具，通过 `action` 参数调用各项功能：服务发现（discover）、客户端注册（register）、服务列表与详情查询（list_services、get_service_usage）、任务提交与查询（submit_task、get_task、cancel_task）、结果文件下载（download_result）等
- 支持多服务器配置：分别注册、切换默认服务器、删除服务器配置（set_server_url、list_servers、set_default_server、remove_server）
- 注册信息自动保存到本地配置文件（`~/.pi/agent/openaaas/config.json`）

[1.1.0]: https://github.com/Wolido/OpenAaaS/releases/tag/pi-extension-v1.1.0
[1.0.2]: https://github.com/Wolido/OpenAaaS/releases/tag/pi-extension-v1.0.2
[1.0.1]: https://github.com/Wolido/OpenAaaS/releases/tag/pi-extension-v1.0.1
