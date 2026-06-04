# 安全政策

*Security Policy*

OpenAaaS 是一个面向科研领域的 Agent 网络基础设施，核心理念是**"数据原位驻留，能力跨节点流动"**。我们高度重视安全问题，并致力于为用户和贡献者提供一个安全可靠的平台。本文档描述了 OpenAaaS 项目的安全政策、漏洞报告流程以及披露规范。

OpenAaaS is an Agent network infrastructure for scientific research. Its core philosophy is **"data remains in place, capabilities flow across nodes"**. We take security very seriously and are committed to providing a safe and reliable platform for users and contributors. This document describes the security policy, vulnerability reporting process, and disclosure guidelines for the OpenAaaS project.

<!-- TOC -->
- [安全政策](#安全政策)
  - [支持的版本](#支持的版本)
  - [报告漏洞](#报告漏洞)
    - [响应时间线](#响应时间线)
  - [披露政策](#披露政策)
  - [已知安全问题](#已知安全问题)

<!-- /TOC -->

## 支持的版本

*Supported Versions*

目前 OpenAaaS 处于早期开发阶段，仅对以下版本提供安全更新支持：

Currently, OpenAaaS is in an early stage of development. Security updates are only provided for the following versions:

| 版本<br>Version | 支持状态<br>Support Status | 说明<br>Description |
|---------|----------|------|
| `main` 分支最新稳定版本<br>Latest stable release on `main` | ✅ 支持安全更新<br>Security updates supported | 当前唯一接受安全补丁的版本<br>The only version currently receiving security patches |
| 历史版本 / 其他分支<br>Historical releases / Other branches | ❌ 不支持<br>Not supported | 建议升级到 `main` 分支最新版本<br>Please upgrade to the latest version on `main` |

随着项目逐渐成熟，我们将引入版本化发布策略并扩展支持窗口。届时本表格将相应更新。

As the project matures, we will introduce a versioned release strategy and expand the support window. This table will be updated accordingly.

## 报告漏洞

*Reporting a Vulnerability*

如果你发现了安全漏洞，**请不要公开提交 Issue**。公开披露安全漏洞可能会给社区用户带来风险，请通过以下私有渠道报告：

If you discover a security vulnerability, **please do not file a public issue**. Public disclosure of security vulnerabilities may put community users at risk. Please report through the following private channel:

1. 通过 [GitHub Security Advisories](https://github.com/Wolido/OpenAaaS/security/advisories/new) 私下提交漏洞报告。<br>Submit a private vulnerability report via [GitHub Security Advisories](https://github.com/Wolido/OpenAaaS/security/advisories/new).

为了提高漏洞处理效率，请在报告中尽可能提供以下信息：

To improve the efficiency of vulnerability handling, please include as much of the following information as possible in your report:

- **复现步骤** — 清晰、可复现的操作步骤，帮助我们快速定位问题。<br>**Reproduction steps** — Clear, reproducible steps to help us quickly identify the issue.
- **影响范围** — 受影响的组件、版本以及可能的利用场景（如远程代码执行、数据泄露等）。<br>**Impact scope** — Affected components, versions, and potential exploitation scenarios (e.g., remote code execution, data leakage, etc.).
- **建议修复方案**（可选）— 如果你有修复思路或参考链接，欢迎一并提供。<br>**Suggested fix** (optional) — If you have ideas for a fix or reference links, feel free to include them.

### 响应时间线

*Response Timeline*

| 阶段<br>Stage | 时间目标<br>Time Target | 说明<br>Description |
|------|-----------|------|
| 确认收到<br>Acknowledgment | 48 小时内<br>Within 48 hours | 确认已收到漏洞报告，并分配调查负责人。<br>Acknowledge receipt of the report and assign an investigator. |
| 初步评估<br>Initial Assessment | 7 个工作日内<br>Within 7 business days | 确认漏洞有效性和严重程度，必要时请求补充信息。<br>Confirm vulnerability validity and severity; request additional information if needed. |
| 修复与披露<br>Fix and Disclosure | 评估后告知<br>To be communicated after assessment | 根据严重程度给出预计修复时间线，修复后发布补丁并公开披露。<br>Provide an estimated fix timeline based on severity; release a patch and disclose after the fix. |

## 披露政策

*Disclosure Policy*

OpenAaaS 遵循**负责任的披露（Responsible Disclosure）**原则：

OpenAaaS follows the **Responsible Disclosure** principle:

1. 漏洞修复完成后，我们将在发布补丁版本的同时，通过 GitHub Security Advisory 公开披露漏洞详情，包括 CVE 编号（如适用）。<br>After a vulnerability is fixed, we will publicly disclose the details via a GitHub Security Advisory at the same time the patched version is released, including a CVE identifier if applicable.
2. 如果报告者同意公开身份，我们将在披露公告中给予适当的致谢（如 `Reported by @username`）。<br>If the reporter agrees to be publicly identified, we will provide appropriate credit in the disclosure announcement (e.g., `Reported by @username`).
3. 在漏洞修复补丁发布之前，我们不会公开任何可能帮助恶意利用的细节。<br>We will not disclose any details that could facilitate malicious exploitation before the fix is released.

## 已知安全问题

*Known Security Issues*

目前 OpenAaaS **没有已知的未修复安全漏洞**。

Currently, OpenAaaS has **no known unpatched security vulnerabilities**.

如有已确认但未修复的安全问题，我们将在此列出相关的 CVE 编号或 GitHub Security Advisory 链接。

If any confirmed but unpatched security issues arise, we will list the relevant CVE identifiers or GitHub Security Advisory links here.
