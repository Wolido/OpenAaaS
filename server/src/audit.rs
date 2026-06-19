//! 审计日志模块
//!
//! 记录关键安全事件：注册请求、认证失败等。
//! 来源 IP 默认使用连接 IP；当配置 `trust_x_forwarded_for = true` 时，
//! 从 `X-Forwarded-For` 最左侧取值。

use axum::extract::Request;
use axum::extract::connect_info::ConnectInfo;
use std::net::SocketAddr;

/// 从请求中提取来源 IP
pub fn extract_client_ip(req: &Request, trust_x_forwarded_for: bool) -> Option<String> {
    if trust_x_forwarded_for
        && let Some(value) = req.headers().get("X-Forwarded-For")
        && let Ok(s) = value.to_str()
        && let Some(first) = s.split(',').next()
    {
        let ip = first.trim();
        if !ip.is_empty() {
            return Some(ip.to_string());
        }
    }

    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
}

/// 记录 Client 注册审计日志
pub fn log_client_register(name: &str, ip: Option<&str>) {
    tracing::info!(
        audit_event = "client_register",
        name = name,
        client_ip = ip.unwrap_or("unknown"),
        "Client 注册请求"
    );
}

/// 记录 Agent 注册审计日志
pub fn log_agent_register(service_name: &str, ip: Option<&str>) {
    tracing::info!(
        audit_event = "agent_register",
        service_name = service_name,
        client_ip = ip.unwrap_or("unknown"),
        "Agent 注册请求"
    );
}

/// 记录认证失败审计日志
///
/// `identifier` 应当是已脱敏的标识，例如 API Key 的 HMAC 前缀或服务 ID。
pub fn log_auth_failure(kind: &str, identifier: &str, ip: Option<&str>, reason: &str) {
    tracing::warn!(
        audit_event = "auth_failure",
        kind = kind,
        identifier = identifier,
        client_ip = ip.unwrap_or("unknown"),
        reason = reason,
        "认证失败"
    );
}
