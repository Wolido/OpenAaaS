//! 错误处理模块

use axum::{
    Json,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

/// 应用结果类型
pub type Result<T> = std::result::Result<T, AppError>;

/// 应用错误类型
#[derive(Debug, Error)]
pub enum AppError {
    /// 资源不存在
    #[error("资源不存在")]
    NotFound,

    /// 未授权
    #[error("未授权: {0}")]
    Unauthorized(String),

    /// 无效请求
    #[error("无效请求: {0}")]
    BadRequest(String),

    /// 内部错误
    #[error("内部错误: {0}")]
    Internal(String),

    /// 数据库错误
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    /// 配置错误
    #[error("配置错误: {0}")]
    Config(String),

    /// 认证错误
    #[error("认证错误: {0}")]
    Auth(String),

    /// 权限不足
    #[error("权限不足")]
    Forbidden,

    /// 资源冲突
    #[error("资源冲突: {0}")]
    Conflict(String),

    /// 请求过于频繁
    #[error("请求过于频繁，请稍后再试")]
    RateLimited,

    /// 其他错误
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// 错误响应结构
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "NotFound", self.to_string()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "Unauthorized", msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BadRequest", msg.clone()),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal",
                    "服务器内部错误".to_string(),
                )
            }
            AppError::Database(e) => {
                tracing::error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database",
                    "数据库操作失败".to_string(),
                )
            }
            AppError::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "Config", msg.clone()),
            AppError::Auth(_) => {
                // 对外统一返回“认证失败”，避免泄露内部细节（如 key 是否存在）
                (StatusCode::UNAUTHORIZED, "Auth", "认证失败".to_string())
            }
            AppError::RateLimited => {
                let body = Json(ErrorResponse {
                    error: "RateLimited".to_string(),
                    message: "请求过于频繁，请稍后再试".to_string(),
                });
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    [(header::RETRY_AFTER, "60")],
                    body,
                )
                    .into_response();
            }
            AppError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden", self.to_string()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "Conflict", msg.clone()),
            AppError::Other(e) => {
                tracing::error!("Unknown error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Unknown",
                    "未知错误".to_string(),
                )
            }
        };

        let body = Json(ErrorResponse {
            error: error_code.to_string(),
            message,
        });

        (status, body).into_response()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::BadRequest(format!("JSON解析错误: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::response::IntoResponse;

    /// 辅助函数：从响应中提取状态码和错误信息
    async fn extract_response_info(response: Response) -> (u16, String, String) {
        let status = response.status().as_u16();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // 解析JSON响应
        let json_value: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        let error = json_value["error"].as_str().unwrap().to_string();
        let message = json_value["message"].as_str().unwrap().to_string();

        (status, error, message)
    }

    #[test]
    fn test_not_found_error_display() {
        let err = AppError::NotFound;
        assert_eq!(err.to_string(), "资源不存在");
    }

    #[test]
    fn test_unauthorized_error_display() {
        let err = AppError::Unauthorized("Token无效".to_string());
        assert_eq!(err.to_string(), "未授权: Token无效");
    }

    #[test]
    fn test_bad_request_error_display() {
        let err = AppError::BadRequest("缺少必要字段".to_string());
        assert_eq!(err.to_string(), "无效请求: 缺少必要字段");
    }

    #[test]
    fn test_internal_error_display() {
        let err = AppError::Internal("数据库连接失败".to_string());
        assert_eq!(err.to_string(), "内部错误: 数据库连接失败");
    }

    #[test]
    fn test_config_error_display() {
        let err = AppError::Config("配置项缺失".to_string());
        assert_eq!(err.to_string(), "配置错误: 配置项缺失");
    }

    #[test]
    fn test_auth_error_display() {
        let err = AppError::Auth("用户名或密码错误".to_string());
        assert_eq!(err.to_string(), "认证错误: 用户名或密码错误");
    }

    #[test]
    fn test_forbidden_error_display() {
        let err = AppError::Forbidden;
        assert_eq!(err.to_string(), "权限不足");
    }

    #[test]
    fn test_conflict_error_display() {
        let err = AppError::Conflict("资源已存在".to_string());
        assert_eq!(err.to_string(), "资源冲突: 资源已存在");
    }

    #[test]
    fn test_other_error_display() {
        let err = AppError::Other(anyhow::anyhow!("自定义错误"));
        assert_eq!(err.to_string(), "自定义错误");
    }

    #[tokio::test]
    async fn test_not_found_into_response() {
        let err = AppError::NotFound;
        let response = err.into_response();
        let (status, error, message) = extract_response_info(response).await;

        assert_eq!(status, 404);
        assert_eq!(error, "NotFound");
        assert_eq!(message, "资源不存在");
    }

    #[tokio::test]
    async fn test_unauthorized_into_response() {
        let err = AppError::Unauthorized("Token已过期".to_string());
        let response = err.into_response();
        let (status, error, message) = extract_response_info(response).await;

        assert_eq!(status, 401);
        assert_eq!(error, "Unauthorized");
        assert_eq!(message, "Token已过期");
    }

    #[tokio::test]
    async fn test_bad_request_into_response() {
        let err = AppError::BadRequest("ID格式不正确".to_string());
        let response = err.into_response();
        let (status, error, message) = extract_response_info(response).await;

        assert_eq!(status, 400);
        assert_eq!(error, "BadRequest");
        assert_eq!(message, "ID格式不正确");
    }

    #[tokio::test]
    async fn test_internal_into_response() {
        let err = AppError::Internal("内部服务错误".to_string());
        let response = err.into_response();
        let (status, error, message) = extract_response_info(response).await;

        assert_eq!(status, 500);
        assert_eq!(error, "Internal");
        assert_eq!(message, "服务器内部错误");
    }

    #[tokio::test]
    async fn test_config_into_response() {
        let err = AppError::Config("配置文件解析失败".to_string());
        let response = err.into_response();
        let (status, error, message) = extract_response_info(response).await;

        assert_eq!(status, 500);
        assert_eq!(error, "Config");
        assert_eq!(message, "配置文件解析失败");
    }

    #[tokio::test]
    async fn test_auth_into_response() {
        let err = AppError::Auth("认证失败".to_string());
        let response = err.into_response();
        let (status, error, message) = extract_response_info(response).await;

        assert_eq!(status, 401);
        assert_eq!(error, "Auth");
        assert_eq!(message, "认证失败");
    }

    #[tokio::test]
    async fn test_forbidden_into_response() {
        let err = AppError::Forbidden;
        let response = err.into_response();
        let (status, error, message) = extract_response_info(response).await;

        assert_eq!(status, 403);
        assert_eq!(error, "Forbidden");
        assert_eq!(message, "权限不足");
    }

    #[tokio::test]
    async fn test_conflict_into_response() {
        let err = AppError::Conflict("用户已存在".to_string());
        let response = err.into_response();
        let (status, error, message) = extract_response_info(response).await;

        assert_eq!(status, 409);
        assert_eq!(error, "Conflict");
        assert_eq!(message, "用户已存在");
    }

    #[tokio::test]
    async fn test_rate_limited_into_response() {
        let err = AppError::RateLimited;
        let response = err.into_response();
        let (status, error, message) = extract_response_info(response).await;

        assert_eq!(status, 429);
        assert_eq!(error, "RateLimited");
        assert_eq!(message, "请求过于频繁，请稍后再试");
    }

    #[test]
    fn test_rate_limited_includes_retry_after_header() {
        let err = AppError::RateLimited;
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            response.headers().get(header::RETRY_AFTER),
            Some(&header::HeaderValue::from_static("60"))
        );
    }

    #[tokio::test]
    async fn test_other_into_response() {
        let err = AppError::Other(anyhow::anyhow!("未知异常"));
        let response = err.into_response();
        let (status, error, message) = extract_response_info(response).await;

        assert_eq!(status, 500);
        assert_eq!(error, "Unknown");
        assert_eq!(message, "未知错误");
    }

    #[tokio::test]
    async fn test_database_error_into_response() {
        // 使用 sqlx::Error 的 PoolTimedOut 变体
        let sqlx_err = sqlx::Error::PoolTimedOut;
        let err = AppError::Database(sqlx_err);
        let response = err.into_response();
        let (status, error, message) = extract_response_info(response).await;

        assert_eq!(status, 500);
        assert_eq!(error, "Database");
        assert_eq!(message, "数据库操作失败");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "文件未找到");
        let app_err: AppError = io_err.into();

        match app_err {
            AppError::Internal(msg) => assert!(msg.contains("文件未找到")),
            _ => panic!("期望是 Internal 错误类型"),
        }
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let app_err: AppError = json_err.into();

        match app_err {
            AppError::BadRequest(msg) => {
                assert!(msg.contains("JSON解析错误"));
            }
            _ => panic!("期望是 BadRequest 错误类型"),
        }
    }

    #[test]
    fn test_all_error_variants_exist() {
        // 确保所有错误变体都可以被创建
        let _ = AppError::NotFound;
        let _ = AppError::Unauthorized("test".to_string());
        let _ = AppError::BadRequest("test".to_string());
        let _ = AppError::Internal("test".to_string());
        let _ = AppError::Database(sqlx::Error::PoolTimedOut);
        let _ = AppError::Config("test".to_string());
        let _ = AppError::Auth("test".to_string());
        let _ = AppError::Forbidden;
        let _ = AppError::Conflict("test".to_string());
        let _ = AppError::RateLimited;
        let _ = AppError::Other(anyhow::anyhow!("test"));
    }

    #[tokio::test]
    async fn test_error_response_json_format() {
        let err = AppError::BadRequest("测试消息".to_string());
        let response = err.into_response();

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // 验证JSON格式
        let json_value: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        assert!(json_value.get("error").is_some());
        assert!(json_value.get("message").is_some());

        // 验证值类型
        assert!(json_value["error"].is_string());
        assert!(json_value["message"].is_string());
    }

    #[test]
    fn test_result_type_alias() {
        // 验证 Result 类型别名工作正常
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }

        fn returns_err() -> Result<i32> {
            Err(AppError::NotFound)
        }

        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }

    #[tokio::test]
    async fn test_status_code_correctness() {
        // 测试所有错误变体返回正确的状态码
        let test_cases = vec![
            (AppError::NotFound, 404u16),
            (AppError::Unauthorized("".to_string()), 401),
            (AppError::BadRequest("".to_string()), 400),
            (AppError::Internal("".to_string()), 500),
            (AppError::Database(sqlx::Error::PoolTimedOut), 500),
            (AppError::Config("".to_string()), 500),
            (AppError::Auth("".to_string()), 401),
            (AppError::Forbidden, 403),
            (AppError::Conflict("".to_string()), 409),
            (AppError::RateLimited, 429),
            (AppError::Other(anyhow::anyhow!("")), 500),
        ];

        for (err, expected_status) in test_cases {
            let response = err.into_response();
            assert_eq!(response.status().as_u16(), expected_status);
        }
    }
}
