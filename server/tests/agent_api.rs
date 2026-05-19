//! Agent API 集成测试

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{create_registered_service, create_test_app, create_test_service, create_test_task};
use serde_json::json;
use tower::ServiceExt;

// ============================================================================
// Agent 注册测试
// ============================================================================

#[tokio::test]
async fn test_register_agent_success() {
    let (app, _state, pool) = create_test_app().await;
    let (_service_id, registration_token) = create_test_service(&pool).await;

    let request_body = json!({
        "registration_token": registration_token,
        "capacity": 2
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/agent/register")
        .header("Content-Type", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["service_id"].as_str().is_some());
    assert_eq!(json["name"].as_str(), Some("Test Service"));
    assert!(json["api_key"].as_str().is_some());
    assert!(json["api_key"].as_str().unwrap().starts_with("ak_agent_"));
    assert_eq!(json["status"].as_str(), Some("online"));
}

#[tokio::test]
async fn test_register_agent_invalid_token() {
    let (app, _state, _pool) = create_test_app().await;

    let request_body = json!({
        "registration_token": "invalid_token_12345",
        "capacity": 1
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/agent/register")
        .header("Content-Type", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_register_agent_duplicate_registration() {
    let (app, _state, pool) = create_test_app().await;
    let (_service_id, registration_token) = create_test_service(&pool).await;

    // 第一次注册
    let request_body = json!({
        "registration_token": &registration_token,
        "capacity": 1
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/agent/register")
        .header("Content-Type", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 第二次注册（应该失败）
    let request_body = json!({
        "registration_token": &registration_token,
        "capacity": 1
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/agent/register")
        .header("Content-Type", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_register_agent_revoked_service() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, registration_token) = create_test_service(&pool).await;

    // 将服务状态设置为 revoked
    sqlx::query("UPDATE services SET registration_status = 'revoked' WHERE id = ?")
        .bind(&service_id)
        .execute(&pool)
        .await
        .unwrap();

    let request_body = json!({
        "registration_token": registration_token,
        "capacity": 1
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/agent/register")
        .header("Content-Type", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// ============================================================================
// 任务轮询测试
// ============================================================================

#[tokio::test]
async fn test_poll_returns_task_when_pending() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;
    // 创建一个待处理任务
    let _task_id = create_test_task(&pool, &service_id, "pending").await;

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/poll", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["has_task"].as_bool(), Some(true));
    assert_eq!(json["should_cancel"].as_bool(), Some(false));
    assert!(json["task"].is_object());
}

#[tokio::test]
async fn test_poll_returns_empty_when_no_task() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/poll", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["has_task"].as_bool(), Some(false));
    assert_eq!(json["should_cancel"].as_bool(), Some(false));
    assert!(json["task"].is_null());
}

#[tokio::test]
async fn test_poll_returns_cancel_when_cancelling_task() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;
    // 创建一个 cancelling 状态的任务
    let task_id = create_test_task(&pool, &service_id, "cancelling").await;

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/poll", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["has_task"].as_bool(), Some(false));
    assert_eq!(json["should_cancel"].as_bool(), Some(true));
    assert_eq!(json["cancel_task_id"].as_str(), Some(task_id.as_str()));
}

#[tokio::test]
async fn test_poll_with_invalid_credentials() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, _api_key, _) = create_registered_service(&pool).await;

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/poll", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", "invalid_api_key")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_poll_missing_headers() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, _api_key, _) = create_registered_service(&pool).await;

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/poll", service_id))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_poll_service_id_mismatch() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/agent/other-service-id/poll")
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// ============================================================================
// 接受任务测试
// ============================================================================

#[tokio::test]
async fn test_accept_task_success() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;
    // 创建待处理任务
    let task_id = create_test_task(&pool, &service_id, "pending").await;

    let request_body = json!({
        "task_id": task_id
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/accept", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"].as_bool(), Some(true));
    assert_eq!(json["task_id"].as_str(), Some(task_id.as_str()));
    assert_eq!(json["message"].as_str(), Some("Task accepted"));

    // 验证任务状态已更新为 running
    let status: (String,) = sqlx::query_as("SELECT status FROM tasks WHERE id = ?")
        .bind(&task_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status.0, "running");
}

#[tokio::test]
async fn test_accept_task_already_accepted() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;
    let _user_id = "test-user-1";

    // 创建已经是 running 状态的任务
    let task_id = create_test_task(&pool, &service_id, "running").await;

    let request_body = json!({
        "task_id": task_id
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/accept", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_accept_task_not_found() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    let request_body = json!({
        "task_id": "non-existent-task-id"
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/accept", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_accept_task_wrong_service() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;
    let (other_service_id, _other_api_key, _) = create_registered_service(&pool).await;

    // 为其他服务创建任务
    let task_id = create_test_task(&pool, &other_service_id, "pending").await;

    let request_body = json!({
        "task_id": task_id
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/accept", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// 完成任务测试
// ============================================================================

#[tokio::test]
async fn test_complete_task_success() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;
    let _user_id = "test-user-1";

    // 创建 running 状态的任务
    let task_id = create_test_task(&pool, &service_id, "running").await;

    let request_body = json!({
        "task_id": task_id,
        "status": "completed",
        "output": {
            "result": "Task completed successfully"
        }
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/complete", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"].as_bool(), Some(true));
    assert_eq!(json["task_id"].as_str(), Some(task_id.as_str()));
    assert_eq!(json["status"].as_str(), Some("completed"));

    // 验证任务状态
    let row: (String, Option<String>) =
        sqlx::query_as("SELECT status, output FROM tasks WHERE id = ?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row.0, "completed");
    assert!(row.1.is_some());
}

#[tokio::test]
async fn test_complete_task_with_failure() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;
    let _user_id = "test-user-1";

    // 创建 running 状态的任务
    let task_id = create_test_task(&pool, &service_id, "running").await;

    let request_body = json!({
        "task_id": task_id,
        "status": "failed",
        "error_message": "Task execution failed"
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/complete", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 验证任务状态
    let row: (String, Option<String>) =
        sqlx::query_as("SELECT status, error_message FROM tasks WHERE id = ?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row.0, "failed");
    assert_eq!(row.1, Some("Task execution failed".to_string()));
}

#[tokio::test]
async fn test_complete_task_not_found() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    let request_body = json!({
        "task_id": "non-existent-task-id",
        "status": "completed"
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/complete", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_complete_task_wrong_agent() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;
    let (other_service_id, _other_api_key, _) = create_registered_service(&pool).await;

    // 为其他服务创建 running 状态的任务
    let task_id = create_test_task(&pool, &other_service_id, "running").await;

    let request_body = json!({
        "task_id": task_id,
        "status": "completed"
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/complete", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_complete_task_with_file_ids() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;
    let _user_id = "test-user-1";

    // 创建 running 状态的任务
    let task_id = create_test_task(&pool, &service_id, "running").await;

    let request_body = json!({
        "task_id": task_id,
        "status": "completed",
        "output": {
            "result": "Task completed with files"
        },
        "file_ids": ["file-1", "file-2"]
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/complete", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 验证输出中包含 file_ids
    let row: (Option<String>,) = sqlx::query_as("SELECT output FROM tasks WHERE id = ?")
        .bind(&task_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let output: serde_json::Value = serde_json::from_str(&row.0.unwrap()).unwrap();
    assert!(output["file_ids"].is_array());
    assert_eq!(output["file_ids"].as_array().unwrap().len(), 2);
}

// ============================================================================
// 心跳测试
// ============================================================================

#[tokio::test]
async fn test_heartbeat_success() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    // 先将状态设置为 offline
    sqlx::query("UPDATE services SET agent_status = 'offline' WHERE id = ?")
        .bind(&service_id)
        .execute(&pool)
        .await
        .unwrap();

    let request_body = json!({
        "status": "online",
        "current_load": 0
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/heartbeat", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["acknowledged"].as_bool(), Some(true));
    assert_eq!(json["service_id"].as_str(), Some(service_id.as_str()));
    assert!(json["timestamp"].as_str().is_some());

    // 验证状态已更新为 online
    let status: (String,) = sqlx::query_as("SELECT agent_status FROM services WHERE id = ?")
        .bind(&service_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status.0, "online");
}

#[tokio::test]
async fn test_heartbeat_status_busy() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    let request_body = json!({
        "status": "busy",
        "current_load": 1
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/heartbeat", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 验证状态已更新为 busy
    let status: (String,) = sqlx::query_as("SELECT agent_status FROM services WHERE id = ?")
        .bind(&service_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status.0, "busy");
}

#[tokio::test]
async fn test_heartbeat_invalid_credentials() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, _api_key, _) = create_registered_service(&pool).await;

    let request_body = json!({
        "status": "online"
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/heartbeat", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", "invalid-api-key")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_heartbeat_missing_service() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    let request_body = json!({
        "status": "online"
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/agent/non-existent-service/heartbeat")
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_heartbeat_without_status_update() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    // 只发送心跳，不更新状态
    let request_body = json!({
        "current_load": 0
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/heartbeat", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================================
// 完整工作流测试
// ============================================================================

#[tokio::test]
async fn test_complete_agent_workflow() {
    let (app, _state, pool) = create_test_app().await;

    // 1. 创建待注册服务
    let (service_id, registration_token) = create_test_service(&pool).await;

    // 2. Agent 注册
    let register_body = json!({
        "registration_token": registration_token,
        "capacity": 2
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/agent/register")
        .header("Content-Type", "application/json")
        .body(Body::from(register_body.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let register_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let api_key = register_response["api_key"].as_str().unwrap().to_string();

    // 3. 创建任务
    let task_id = create_test_task(&pool, &service_id, "pending").await;

    // 4. Agent 轮询任务
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/poll", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 5. Agent 接受任务
    let accept_body = json!({
        "task_id": task_id
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/accept", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(accept_body.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 6. Agent 发送心跳
    let heartbeat_body = json!({
        "status": "busy",
        "current_load": 1
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/heartbeat", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(heartbeat_body.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 7. Agent 完成任务
    let complete_body = json!({
        "task_id": task_id,
        "status": "completed",
        "output": {
            "result": "Workflow completed successfully"
        }
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/agent/{}/complete", service_id))
        .header("Content-Type", "application/json")
        .header("X-Service-ID", &service_id)
        .header("X-API-Key", &api_key)
        .body(Body::from(complete_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 验证最终任务状态
    let row: (String, Option<String>) =
        sqlx::query_as("SELECT status, output FROM tasks WHERE id = ?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row.0, "completed");
    assert!(row.1.is_some());
}
