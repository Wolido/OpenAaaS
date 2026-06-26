//! 认证和授权模块

use axum::{
    extract::{Extension, Request, State},
    http::{HeaderMap, header},
    middleware::Next,
    response::Response,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::SqlitePool;

use crate::audit::{extract_client_ip, log_auth_failure};
use crate::error::{AppError, Result};
use crate::state::AppState;

/// HMAC-SHA256 哈希 API Key
pub fn hash_api_key(secret_key: &str, api_key: &str) -> String {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(secret_key.as_bytes()).expect("secret_key must not be empty");
    mac.update(api_key.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

// ============================================================================
// 认证结构体
// ============================================================================

/// 认证用户信息（附加到请求扩展）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    /// 用户ID
    pub user_id: String,
    /// API Key
    pub api_key: String,
    /// 用户角色
    pub role: String,
}

/// 认证Agent信息（附加到请求扩展）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthAgent {
    /// Service ID（一对一模型下，service_id就是agent_id）
    pub agent_id: String,
    /// API Key
    pub api_key: String,
}

// ============================================================================
// Client 鉴权 - Bearer Token 方式
// ============================================================================

/// Client 鉴权中间件
/// 从 Authorization: Bearer <api_key> 提取并验证
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    // 从 Authorization header 提取 api_key
    let api_key = extract_bearer_token(&request)?;

    // 从配置中获取 secret_key
    let secret_key = state
        .config
        .secret_key
        .as_ref()
        .ok_or_else(|| AppError::Internal("Secret key not configured".to_string()))?;

    let client_ip = extract_client_ip(&request, state.config.server.trust_x_forwarded_for);

    // 验证 api_key 并查询用户信息
    let auth_user =
        verify_client_api_key(state.db.pool(), secret_key, &api_key, client_ip.as_deref()).await?;

    // 将用户信息附加到请求扩展
    request.extensions_mut().insert(auth_user);

    let response = next.run(request).await;
    Ok(response)
}

// ============================================================================
// Admin 鉴权
// ============================================================================

/// 检查管理员权限的中间件
pub async fn require_admin(
    Extension(auth_user): Extension<AuthUser>,
    request: Request,
    next: Next,
) -> Result<Response> {
    if auth_user.role != "admin" {
        return Err(AppError::Forbidden);
    }
    Ok(next.run(request).await)
}

/// 从 Authorization header 提取 Bearer token
pub(crate) fn extract_bearer_token(request: &Request) -> Result<String> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .ok_or_else(|| AppError::Auth("缺少 Authorization header".to_string()))?
        .to_str()
        .map_err(|_| AppError::Auth("无效的 Authorization header".to_string()))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Auth(
            "Authorization header 格式错误，应为: Bearer <api_key>".to_string(),
        ));
    }

    let token = auth_header[7..].trim();
    if token.is_empty() {
        return Err(AppError::Auth("API Key 不能为空".to_string()));
    }

    Ok(token.to_string())
}

/// 验证 Client API Key，查询 users 表
pub async fn verify_client_api_key(
    pool: &SqlitePool,
    secret_key: &str,
    api_key: &str,
    client_ip: Option<&str>,
) -> Result<AuthUser> {
    let hashed_api_key = hash_api_key(secret_key, api_key);
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, api_key, role FROM users WHERE api_key = ?",
    )
    .bind(&hashed_api_key)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    match row {
        Some((user_id, api_key, role)) => Ok(AuthUser {
            user_id,
            api_key,
            role,
        }),
        None => {
            // 使用 Key 哈希前 16 位作为脱敏标识
            let identifier = &hashed_api_key[..hashed_api_key.len().min(16)];
            log_auth_failure("client", identifier, client_ip, "无效的 API Key");
            Err(AppError::Auth("无效的 API Key".to_string()))
        }
    }
}

// ============================================================================
// Agent 鉴权 - Header 方式（基于 Service）
// ============================================================================

/// 验证 Agent 凭证（基于 Service）
/// Agent 通过 X-Service-ID 和 X-API-Key headers 认证
pub async fn verify_agent_credentials(
    pool: &SqlitePool,
    secret_key: &str,
    service_id: &str,
    api_key: &str,
    client_ip: Option<&str>,
) -> Result<()> {
    let hashed_api_key = hash_api_key(secret_key, api_key);
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT id, agent_api_key FROM services WHERE id = ?",
    )
    .bind(service_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    match row {
        Some((_id, stored_api_key)) => {
            if stored_api_key == hashed_api_key {
                Ok(())
            } else {
                log_auth_failure("agent", service_id, client_ip, "API Key 无效");
                Err(AppError::Auth("API Key 无效".to_string()))
            }
        }
        None => {
            log_auth_failure("agent", service_id, client_ip, "服务不存在");
            Err(AppError::Auth("服务不存在".to_string()))
        }
    }
}

/// 从 headers 提取 Agent 认证信息（基于 Service）
pub fn extract_service_headers(request: &Request) -> Result<(String, String)> {
    // 提取 X-Service-ID
    let service_id = request
        .headers()
        .get("X-Service-ID")
        .ok_or_else(|| AppError::Auth("缺少 X-Service-ID header".to_string()))?
        .to_str()
        .map_err(|_| AppError::Auth("无效的 X-Service-ID header".to_string()))?
        .to_string();

    if service_id.is_empty() {
        return Err(AppError::Auth("X-Service-ID 不能为空".to_string()));
    }

    // 提取 API Key（支持 X-API-Key 和 Authorization: Bearer 两种方式）
    let api_key = extract_api_key(request.headers())
        .ok_or_else(|| AppError::Auth("缺少 X-API-Key 或 Authorization header".to_string()))?
        .to_string();

    if api_key.is_empty() {
        return Err(AppError::Auth("API Key 不能为空".to_string()));
    }

    Ok((service_id, api_key))
}

/// 从请求头中提取 API Key（支持两种方式）
///
/// 方式1: X-API-Key 头
/// 方式2: Authorization: Bearer <token>
pub fn extract_api_key(headers: &HeaderMap) -> Option<&str> {
    // 方式1: X-API-Key 头（优先级更高）
    if let Some(value) = headers.get("X-API-Key")
        && let Ok(api_key) = value.to_str()
    {
        // 空值也返回，让调用者决定如何处理
        return Some(api_key);
    }

    // 方式2: Authorization: Bearer <token>
    if let Some(value) = headers.get(header::AUTHORIZATION)
        && let Ok(auth_str) = value.to_str()
        && let Some(token) = auth_str.strip_prefix("Bearer ")
    {
        let token = token.trim();
        if !token.is_empty() {
            return Some(token);
        }
    }

    None
}

// ============================================================================
// Agent 鉴权中间件
// ============================================================================

/// Agent 认证中间件
pub async fn agent_auth_middleware(
    State(state): State<AppState>,
    mut req: axum::extract::Request,
    next: Next,
) -> Result<Response> {
    // 从 headers 提取 X-Service-ID
    let service_id = req
        .headers()
        .get("X-Service-ID")
        .ok_or_else(|| AppError::Auth("缺少 X-Service-ID header".to_string()))?
        .to_str()
        .map_err(|_| AppError::Auth("无效的 X-Service-ID header".to_string()))?
        .to_string();

    if service_id.is_empty() {
        return Err(AppError::Auth("X-Service-ID 不能为空".to_string()));
    }

    // 从 headers 提取 API Key（支持 X-API-Key 和 Authorization: Bearer 两种方式）
    let api_key = extract_api_key(req.headers())
        .ok_or_else(|| {
            AppError::Auth("缺少 X-API-Key 或 Authorization: Bearer header".to_string())
        })?
        .to_string();

    if api_key.is_empty() {
        return Err(AppError::Auth("API Key 不能为空".to_string()));
    }

    let client_ip = extract_client_ip(&req, state.config.server.trust_x_forwarded_for);

    // 验证 Agent 凭证
    let secret_key = state
        .config
        .secret_key
        .as_ref()
        .ok_or_else(|| AppError::Internal("Secret key not configured".to_string()))?;
    verify_agent_credentials(
        state.db.pool(),
        secret_key,
        &service_id,
        &api_key,
        client_ip.as_deref(),
    )
    .await?;

    // 将 agent 信息附加到请求扩展
    req.extensions_mut().insert(AuthAgent {
        agent_id: service_id,
        api_key,
    });

    Ok(next.run(req).await)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    // ==================== AuthUser 结构体测试 ====================

    #[test]
    fn test_auth_user_creation() {
        let auth_user = AuthUser {
            user_id: "user_123".to_string(),
            api_key: "ak_test_key".to_string(),
            role: "client".to_string(),
        };

        assert_eq!(auth_user.user_id, "user_123");
        assert_eq!(auth_user.api_key, "ak_test_key");
        assert_eq!(auth_user.role, "client");
    }

    #[test]
    fn test_auth_user_clone() {
        let auth_user = AuthUser {
            user_id: "user_123".to_string(),
            api_key: "ak_test_key".to_string(),
            role: "client".to_string(),
        };

        let cloned = auth_user.clone();
        assert_eq!(auth_user.user_id, cloned.user_id);
        assert_eq!(auth_user.api_key, cloned.api_key);
        assert_eq!(auth_user.role, cloned.role);
    }

    #[test]
    fn test_auth_user_serialization() {
        let auth_user = AuthUser {
            user_id: "user_123".to_string(),
            api_key: "ak_test_key".to_string(),
            role: "admin".to_string(),
        };

        let json = serde_json::to_string(&auth_user).unwrap();
        assert!(json.contains("user_123"));
        assert!(json.contains("ak_test_key"));
        assert!(json.contains("admin"));

        let deserialized: AuthUser = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_id, auth_user.user_id);
    }

    // ==================== AuthAgent 结构体测试 ====================

    #[test]
    fn test_auth_agent_creation() {
        let auth_agent = AuthAgent {
            agent_id: "agent_123".to_string(),
            api_key: "ak_agent_key".to_string(),
        };

        assert_eq!(auth_agent.agent_id, "agent_123");
        assert_eq!(auth_agent.api_key, "ak_agent_key");
    }

    #[test]
    fn test_auth_agent_serialization() {
        let auth_agent = AuthAgent {
            agent_id: "agent_123".to_string(),
            api_key: "ak_agent_key".to_string(),
        };

        let json = serde_json::to_string(&auth_agent).unwrap();
        assert!(json.contains("agent_123"));
        assert!(json.contains("ak_agent_key"));

        let deserialized: AuthAgent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.agent_id, auth_agent.agent_id);
    }

    // ==================== extract_bearer_token 测试 ====================

    #[test]
    fn test_extract_bearer_token_success() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer test_api_key".parse().unwrap(),
        );

        // 创建一个简单的请求
        let request = Request::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, "Bearer test_api_key")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = extract_bearer_token(&request);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_api_key");
    }

    #[test]
    fn test_extract_bearer_token_missing_header() {
        let request = Request::builder()
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = extract_bearer_token(&request);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Auth(msg) => assert!(msg.contains("缺少"), "错误消息应包含'缺少': {}", msg),
            _ => panic!("应该是 Auth 错误"),
        }
    }

    #[test]
    fn test_extract_bearer_token_invalid_format() {
        let request = Request::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, "Basic dGVzdDpwYXNz")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = extract_bearer_token(&request);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Auth(msg) => assert!(
                msg.contains("格式错误"),
                "错误消息应包含'格式错误': {}",
                msg
            ),
            _ => panic!("应该是 Auth 错误"),
        }
    }

    #[test]
    fn test_extract_bearer_token_empty_token() {
        let request = Request::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, "Bearer ")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = extract_bearer_token(&request);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Auth(msg) => assert!(
                msg.contains("不能为空"),
                "错误消息应包含'不能为空': {}",
                msg
            ),
            _ => panic!("应该是 Auth 错误"),
        }
    }

    #[test]
    fn test_extract_bearer_token_with_whitespace() {
        let request = Request::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, "Bearer   ")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = extract_bearer_token(&request);
        assert!(result.is_err());
    }

    // ==================== extract_service_headers 测试 ====================

    #[test]
    fn test_extract_service_headers_success() {
        let request = Request::builder()
            .uri("/test")
            .header("X-Service-ID", "service_123")
            .header("X-API-Key", "api_key_456")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = extract_service_headers(&request);
        assert!(result.is_ok());
        let (service_id, api_key) = result.unwrap();
        assert_eq!(service_id, "service_123");
        assert_eq!(api_key, "api_key_456");
    }

    #[test]
    fn test_extract_service_headers_missing_service_id() {
        let request = Request::builder()
            .uri("/test")
            .header("X-API-Key", "api_key_456")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = extract_service_headers(&request);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Auth(msg) => assert!(msg.contains("X-Service-ID")),
            _ => panic!("应该是 Auth 错误"),
        }
    }

    #[test]
    fn test_extract_service_headers_missing_api_key() {
        let request = Request::builder()
            .uri("/test")
            .header("X-Service-ID", "service_123")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = extract_service_headers(&request);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Auth(msg) => assert!(msg.contains("X-API-Key")),
            _ => panic!("应该是 Auth 错误"),
        }
    }

    #[test]
    fn test_extract_service_headers_empty_service_id() {
        let request = Request::builder()
            .uri("/test")
            .header("X-Service-ID", "")
            .header("X-API-Key", "api_key_456")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = extract_service_headers(&request);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Auth(msg) => assert!(msg.contains("不能为空")),
            _ => panic!("应该是 Auth 错误"),
        }
    }

    #[test]
    fn test_extract_service_headers_empty_api_key() {
        let request = Request::builder()
            .uri("/test")
            .header("X-Service-ID", "service_123")
            .header("X-API-Key", "")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = extract_service_headers(&request);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Auth(msg) => assert!(msg.contains("不能为空")),
            _ => panic!("应该是 Auth 错误"),
        }
    }

    // ==================== 数据库相关测试 ====================

    use crate::test_utils::setup_test_db;

    #[sqlx::test]
    async fn test_verify_client_api_key_success() {
        let pool = setup_test_db().await;
        let secret_key = "test-secret-key-for-unit-tests-only";

        // 插入测试用户（存储 hash）
        sqlx::query("INSERT INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)")
            .bind("user_123")
            .bind(hash_api_key(secret_key, "ak_test_valid_key"))
            .bind("Test User")
            .bind("client")
            .execute(&pool)
            .await
            .unwrap();

        let result = verify_client_api_key(&pool, secret_key, "ak_test_valid_key", None).await;
        assert!(result.is_ok());

        let auth_user = result.unwrap();
        assert_eq!(auth_user.user_id, "user_123");
        assert_eq!(auth_user.role, "client");
    }

    #[sqlx::test]
    async fn test_verify_client_api_key_not_found() {
        let pool = setup_test_db().await;
        let secret_key = "test-secret-key-for-unit-tests-only";

        let result = verify_client_api_key(&pool, secret_key, "ak_nonexistent", None).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Auth(msg) => assert!(msg.contains("无效")),
            _ => panic!("应该是 Auth 错误"),
        }
    }

    #[sqlx::test]
    async fn test_verify_client_api_key_with_admin_role() {
        let pool = setup_test_db().await;
        let secret_key = "test-secret-key-for-unit-tests-only";

        sqlx::query("INSERT INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)")
            .bind("admin_123")
            .bind(hash_api_key(secret_key, "ak_admin_key"))
            .bind("Admin User")
            .bind("admin")
            .execute(&pool)
            .await
            .unwrap();

        let result = verify_client_api_key(&pool, secret_key, "ak_admin_key", None).await;
        assert!(result.is_ok());

        let auth_user = result.unwrap();
        assert_eq!(auth_user.user_id, "admin_123");
        assert_eq!(auth_user.role, "admin");
    }

    #[sqlx::test]
    async fn test_verify_agent_credentials_success() {
        let pool = setup_test_db().await;
        let secret_key = "test-secret-key-for-unit-tests-only";

        // 插入测试服务（存储 hash）
        sqlx::query("INSERT INTO services (id, name, description, usage, agent_api_key) VALUES (?, ?, ?, ?, ?)")
            .bind("service_123")
            .bind("Test Service")
            .bind("")
            .bind("")
            .bind(hash_api_key(secret_key, "ak_agent_valid"))
            .execute(&pool)
            .await
            .unwrap();

        let result =
            verify_agent_credentials(&pool, secret_key, "service_123", "ak_agent_valid", None)
                .await;
        assert!(result.is_ok());
    }

    #[sqlx::test]
    async fn test_verify_agent_credentials_invalid_key() {
        let pool = setup_test_db().await;
        let secret_key = "test-secret-key-for-unit-tests-only";

        sqlx::query("INSERT INTO services (id, name, description, usage, agent_api_key) VALUES (?, ?, ?, ?, ?)")
            .bind("service_123")
            .bind("Test Service")
            .bind("")
            .bind("")
            .bind(hash_api_key(secret_key, "ak_agent_valid"))
            .execute(&pool)
            .await
            .unwrap();

        let result =
            verify_agent_credentials(&pool, secret_key, "service_123", "wrong_key", None).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Auth(msg) => assert!(msg.contains("无效")),
            _ => panic!("应该是 Auth 错误"),
        }
    }

    #[sqlx::test]
    async fn test_verify_agent_credentials_service_not_found() {
        let pool = setup_test_db().await;
        let secret_key = "test-secret-key-for-unit-tests-only";

        let result =
            verify_agent_credentials(&pool, secret_key, "nonexistent_service", "some_key", None)
                .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Auth(msg) => assert!(msg.contains("不存在")),
            _ => panic!("应该是 Auth 错误"),
        }
    }

    // ==================== extract_api_key 测试 ====================

    #[test]
    fn test_extract_api_key_from_x_api_key() {
        let mut headers = HeaderMap::new();
        headers.insert("X-API-Key", "test_api_key".parse().unwrap());

        let result = extract_api_key(&headers);
        assert_eq!(result, Some("test_api_key"));
    }

    #[test]
    fn test_extract_api_key_from_authorization_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer test_bearer_token".parse().unwrap(),
        );

        let result = extract_api_key(&headers);
        assert_eq!(result, Some("test_bearer_token"));
    }

    #[test]
    fn test_extract_api_key_x_api_key_priority() {
        // X-API-Key 优先级更高，应优先返回
        let mut headers = HeaderMap::new();
        headers.insert("X-API-Key", "x_api_key_value".parse().unwrap());
        headers.insert(
            header::AUTHORIZATION,
            "Bearer bearer_token_value".parse().unwrap(),
        );

        let result = extract_api_key(&headers);
        assert_eq!(result, Some("x_api_key_value"));
    }

    #[test]
    fn test_extract_api_key_from_authorization_with_whitespace() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer   token_with_whitespace   ".parse().unwrap(),
        );

        let result = extract_api_key(&headers);
        assert_eq!(result, Some("token_with_whitespace"));
    }

    #[test]
    fn test_extract_api_key_missing_both() {
        let headers = HeaderMap::new();

        let result = extract_api_key(&headers);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_api_key_empty_x_api_key() {
        let mut headers = HeaderMap::new();
        headers.insert("X-API-Key", "".parse().unwrap());

        let result = extract_api_key(&headers);
        // 空的 X-API-Key 也返回 Some(""), 让调用者处理空值
        assert_eq!(result, Some(""));
    }

    #[test]
    fn test_extract_api_key_invalid_authorization_format() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, "Basic dGVzdDpwYXNz".parse().unwrap());

        let result = extract_api_key(&headers);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_api_key_empty_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, "Bearer ".parse().unwrap());

        let result = extract_api_key(&headers);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_service_headers_with_authorization_bearer() {
        let request = Request::builder()
            .uri("/test")
            .header("X-Service-ID", "service_123")
            .header(header::AUTHORIZATION, "Bearer api_key_456")
            .body(axum::body::Body::empty())
            .unwrap();

        let result = extract_service_headers(&request);
        assert!(result.is_ok());
        let (service_id, api_key) = result.unwrap();
        assert_eq!(service_id, "service_123");
        assert_eq!(api_key, "api_key_456");
    }
}
