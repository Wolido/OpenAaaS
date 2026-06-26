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

/// 对敏感 token 进行字符级掩码，避免泄露完整 token。
///
/// 掩码策略按字符长度分级：
/// - 长度 ≤ 2：统一返回 `...`，避免泄露 50%~100% 原始字符。
/// - 长度 ≤ 8：保留首尾各 1 个字符，中间用 `...` 代替。
/// - 长度 ≤ 12：保留首尾各 2 个字符，中间用 `...` 代替。
/// - 长度 > 12：保留前 4 个字符和后 4 个字符，中间用 `...` 代替。
///
/// 使用字符边界切片，对多字节 UTF-8 字符安全，不会 panic。
///
/// 例如 `rt_abc123def456` -> `rt_a...f456`。
pub fn mask_token(token: &str) -> String {
    let chars: Vec<char> = token.chars().collect();
    let len = chars.len();

    if len <= 2 {
        "...".to_string()
    } else if len <= 8 {
        format!("{}...{}", chars[0], chars[len - 1])
    } else if len <= 12 {
        let head: String = chars.iter().take(2).collect();
        let tail: String = chars.iter().rev().take(2).rev().collect();
        format!("{}...{}", head, tail)
    } else {
        let head: String = chars.iter().take(4).collect();
        let tail: String = chars.iter().rev().take(4).rev().collect();
        format!("{}...{}", head, tail)
    }
}

/// 记录 Agent 注册审计日志
///
/// `registration_token` 在记录前会被掩码，避免完整 token 泄露。
pub fn log_agent_register(registration_token: &str, ip: Option<&str>) {
    tracing::info!(
        audit_event = "agent_register",
        registration_token = mask_token(registration_token),
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::HeaderValue;
    use std::net::{Ipv4Addr, SocketAddr};

    fn request_with_xff(xff: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().uri("/test");
        if let Some(value) = xff {
            builder = builder.header("X-Forwarded-For", value);
        }
        builder.body(Body::empty()).unwrap()
    }

    fn request_with_connect_info(ip: &str) -> Request<Body> {
        let mut req = Request::builder().uri("/test").body(Body::empty()).unwrap();
        let addr = SocketAddr::new(ip.parse::<Ipv4Addr>().unwrap().into(), 8080);
        req.extensions_mut().insert(ConnectInfo(addr));
        req
    }

    fn request_with_both(xff: &str, ip: &str) -> Request<Body> {
        let mut req = request_with_connect_info(ip);
        req.headers_mut()
            .insert("X-Forwarded-For", HeaderValue::from_str(xff).unwrap());
        req
    }

    #[test]
    fn test_extract_client_ip_no_xff_uses_connect_info() {
        let req = request_with_connect_info("192.168.1.1");
        assert_eq!(
            extract_client_ip(&req, false),
            Some("192.168.1.1".to_string())
        );
    }

    #[test]
    fn test_extract_client_ip_distrust_xff_uses_connect_info() {
        let req = request_with_both("10.0.0.1", "192.168.1.1");
        assert_eq!(
            extract_client_ip(&req, false),
            Some("192.168.1.1".to_string())
        );
    }

    #[test]
    fn test_extract_client_ip_trust_xff_single_ip() {
        let req = request_with_both("10.0.0.1", "192.168.1.1");
        assert_eq!(extract_client_ip(&req, true), Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_trust_xff_multiple_ips_takes_leftmost() {
        let req = request_with_both("203.0.113.1, 10.0.0.2, 10.0.0.3", "192.168.1.1");
        assert_eq!(
            extract_client_ip(&req, true),
            Some("203.0.113.1".to_string())
        );
    }

    #[test]
    fn test_extract_client_ip_trust_xff_trims_whitespace() {
        let req = request_with_both("  203.0.113.1  ", "192.168.1.1");
        assert_eq!(
            extract_client_ip(&req, true),
            Some("203.0.113.1".to_string())
        );
    }

    #[test]
    fn test_extract_client_ip_xff_empty_falls_back_to_connect_info() {
        let req = request_with_both("", "192.168.1.1");
        assert_eq!(
            extract_client_ip(&req, true),
            Some("192.168.1.1".to_string())
        );
    }

    #[test]
    fn test_extract_client_ip_xff_only_commas_falls_back_to_connect_info() {
        let req = request_with_both(", ,", "192.168.1.1");
        assert_eq!(
            extract_client_ip(&req, true),
            Some("192.168.1.1".to_string())
        );
    }

    #[test]
    fn test_extract_client_ip_no_xff_no_connect_info() {
        let req = request_with_xff(None);
        assert_eq!(extract_client_ip(&req, false), None);
    }

    #[test]
    fn test_mask_token_ascii() {
        assert_eq!(mask_token("rt_abc123def456"), "rt_a...f456");
    }

    #[test]
    fn test_mask_token_multibyte_utf8() {
        // 前缀与后缀都是多字节 UTF-8 字符，长度 > 12，保留前 4 / 后 4 字符
        let token = "测试令牌abc123xyz";
        assert_eq!(mask_token(token), "测试令牌...3xyz");
    }

    #[test]
    fn test_mask_token_empty() {
        assert_eq!(mask_token(""), "...");
    }

    #[test]
    fn test_mask_token_short_very_short() {
        assert_eq!(mask_token("a"), "...");
        assert_eq!(mask_token("ab"), "...");
    }

    #[test]
    fn test_mask_token_short_up_to_8() {
        assert_eq!(mask_token("abc"), "a...c");
        assert_eq!(mask_token("abcdefgh"), "a...h");
    }

    #[test]
    fn test_mask_token_short_up_to_12() {
        assert_eq!(mask_token("abcdefghi"), "ab...hi");
        assert_eq!(mask_token("abcdefghijkl"), "ab...kl");
    }

    #[test]
    fn test_mask_token_multibyte_does_not_panic() {
        // 验证字符级切片对多字节 UTF-8 安全，不会 panic。
        let token = "中文字符测试令牌";
        let _ = mask_token(token);
    }
}
