//! API Key 限流模块
//!
//! 按 API Key 维度进行滑动窗口限流：
//! - Client API Key：100 次/分钟
//! - Agent API Key：150 次/分钟
//!
//! 限流键为 `{kind}:{HMAC(key)}`，使用 `auth::hash_api_key` 生成，
//! 避免在内存中保存原始 API Key。

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::time::{Duration, Instant};

use crate::audit::extract_client_ip;
use crate::error::{AppError, Result};
use crate::state::AppState;

/// 限流窗口大小
const WINDOW_SIZE: Duration = Duration::from_secs(60);
/// Client API Key 每分钟最大请求数
const CLIENT_LIMIT_PER_MIN: usize = 100;
/// Agent API Key 每分钟最大请求数
const AGENT_LIMIT_PER_MIN: usize = 150;

/// 限流维度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitKind {
    /// Client / Admin 等使用 Bearer Token 的接口
    Client,
    /// Agent 使用 X-API-Key / Bearer Token 的接口
    Agent,
}

impl RateLimitKind {
    fn as_str(&self) -> &'static str {
        match self {
            RateLimitKind::Client => "client",
            RateLimitKind::Agent => "agent",
        }
    }

    fn limit(&self) -> usize {
        match self {
            RateLimitKind::Client => CLIENT_LIMIT_PER_MIN,
            RateLimitKind::Agent => AGENT_LIMIT_PER_MIN,
        }
    }
}

/// 基于 API Key 的滑动窗口限流器
#[derive(Debug)]
pub struct RateLimiter {
    buckets: dashmap::DashMap<String, Vec<Instant>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    /// 创建新的限流器
    pub fn new() -> Self {
        Self {
            buckets: dashmap::DashMap::new(),
        }
    }

    /// 检查并记录一次请求
    ///
    /// 超过阈值时返回 `AppError::RateLimited`。
    pub fn check(&self, kind: RateLimitKind, key_hash: &str) -> Result<()> {
        let bucket_key = format!("{}:{}", kind.as_str(), key_hash);
        let limit = kind.limit();

        let mut entry = self.buckets.entry(bucket_key).or_default();
        entry.retain(|t| t.elapsed() < WINDOW_SIZE);

        if entry.len() >= limit {
            return Err(AppError::RateLimited);
        }

        entry.push(Instant::now());
        Ok(())
    }
}

/// Client 接口限流中间件
///
/// 从 `Authorization: Bearer <token>` 中提取 Client API Key 并限流。
/// 未提供 Key 时不限流，由后续认证中间件处理。
pub async fn client_rate_limit_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response> {
    let _ip = extract_client_ip(&req, state.config.server.trust_x_forwarded_for);

    if let Ok(api_key) = crate::auth::extract_bearer_token(&req)
        && let Some(secret_key) = state.config.secret_key.as_deref()
    {
        let key_hash = crate::auth::hash_api_key(secret_key, &api_key);
        state.rate_limiter.check(RateLimitKind::Client, &key_hash)?;
    }

    Ok(next.run(req).await)
}

/// Agent 接口限流中间件
///
/// 从 `X-API-Key` 或 `Authorization: Bearer <token>` 中提取 Agent API Key 并限流。
pub async fn agent_rate_limit_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response> {
    let _ip = extract_client_ip(&req, state.config.server.trust_x_forwarded_for);

    if let Some(api_key) = crate::auth::extract_api_key(req.headers())
        && let Some(secret_key) = state.config.secret_key.as_deref()
    {
        let key_hash = crate::auth::hash_api_key(secret_key, api_key);
        state.rate_limiter.check(RateLimitKind::Agent, &key_hash)?;
    }

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new();
        let hash = "test_hash";

        for _ in 0..CLIENT_LIMIT_PER_MIN {
            assert!(limiter.check(RateLimitKind::Client, hash).is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_rejects_over_limit() {
        let limiter = RateLimiter::new();
        let hash = "test_hash";

        for _ in 0..CLIENT_LIMIT_PER_MIN {
            limiter.check(RateLimitKind::Client, hash).unwrap();
        }

        assert!(matches!(
            limiter.check(RateLimitKind::Client, hash).unwrap_err(),
            AppError::RateLimited
        ));
    }

    #[test]
    fn test_rate_limiter_separate_keys() {
        let limiter = RateLimiter::new();

        for _ in 0..CLIENT_LIMIT_PER_MIN {
            limiter.check(RateLimitKind::Client, "key_a").unwrap();
        }

        // 不同 Key 不受影响
        assert!(limiter.check(RateLimitKind::Client, "key_b").is_ok());
    }

    #[test]
    fn test_rate_limiter_separate_kinds() {
        let limiter = RateLimiter::new();

        for _ in 0..CLIENT_LIMIT_PER_MIN {
            limiter.check(RateLimitKind::Client, "shared").unwrap();
        }

        // Client 维度已超限，但 Agent 维度独立
        assert!(limiter.check(RateLimitKind::Agent, "shared").is_ok());
    }

    #[test]
    fn test_agent_limit_is_150() {
        let limiter = RateLimiter::new();
        let hash = "agent_hash";

        for _ in 0..AGENT_LIMIT_PER_MIN {
            assert!(limiter.check(RateLimitKind::Agent, hash).is_ok());
        }

        assert!(matches!(
            limiter.check(RateLimitKind::Agent, hash).unwrap_err(),
            AppError::RateLimited
        ));
    }
}
