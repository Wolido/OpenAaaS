# OpenAaaS Desktop Client

<p align="right">中文 | <a href="./README.en.md">English</a></p>

OpenAaaS 的桌面客户端，提供可视化的服务器管理、服务市场浏览和任务提交能力。

## 技术栈

| 技术 | 用途 |
|------|------|
| Tauri 2.0 | 跨平台桌面框架 |
| Vue 3 + TypeScript | 前端界面与类型安全 |
| Tailwind CSS | 样式系统 |
| Pinia | 状态管理 |
| `@tauri-apps/plugin-http` | 绕过 CORS 直连 Server |

## 功能特性

- **多服务器管理** — 添加、删除、注册和切换默认服务器
- **服务市场** — 浏览所有服务器上的可用服务，查看实时负载与状态
- **服务详情** — 查看 Usage 说明和实时负载
- **向导式任务提交** — 填写 task_prompt、output_prompt，支持文件上传（拖拽或点击）
- **任务自动轮询** — 提交后首次 5 秒轮询，之后 30 秒间隔
- **任务详情** — 查看任务状态、结果（Markdown 渲染）和输出文件下载
- **任务列表** — 按状态筛选，显示服务器来源标识
- **数据持久化** — 服务器配置和本地数据存储在 localStorage
- **骨架屏** — 所有异步加载区域均有 loading 占位，避免白屏

## 构建运行

需要安装 Rust 1.85+ 和 Node.js 18+，以及 [Tauri 对应的系统依赖](https://v2.tauri.app/start/prerequisites/)。

```bash
cd client-app
npm install
cargo tauri dev      # 开发模式
cargo tauri build    # 发行构建
```

开发模式下，Tauri 会自动打开桌面窗口；前端代码支持热更新，Rust 代码修改后需重启。

## 构建产物

发行构建产物位于 `src-tauri/target/release/bundle/`：

| 平台 | 产物 |
|------|------|
| macOS | `.dmg` 安装包 + `.app` 应用 |
| Windows | `.msi` 安装包 + `.exe` 可执行文件 |
| Linux | `.deb` 安装包 |

## macOS 安装说明

由于应用使用 ad-hoc 自签名（未购买 Apple Developer ID），macOS 首次打开时可能提示**"无法验证开发者"**。

解决方式：前往 **系统设置 → 隐私与安全性 → 安全性**，点击**"仍要打开"**即可。

> 💡 我们推荐此方法而非不签名（不签名会提示"已损坏"，需要用终端命令解除隔离属性）。
