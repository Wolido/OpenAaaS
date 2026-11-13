//! 客户端 API 集成测试

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use open_aaas_server::models::user::UserRole;
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

use super::{
    ErrorResponse, GrantPermissionResponse, ServiceStatusResponse, TaskResponse, TestApp,
    UserResponse, auth_header, create_test_service, create_test_task, create_test_user,
};

// ============================================================================
// 健康检查测试（公开端点）
// ============================================================================

#[tokio::test]
async fn test_health_check_success() {
    let app = TestApp::new().await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/client/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "ok");
    assert!(json["version"].is_string());
    assert!(json["timestamp"].is_string());

    app.cleanup().await;
}

// ============================================================================
// 用户注册测试
// ============================================================================

#[tokio::test]
async fn test_register_success() {
    let app = TestApp::new().await;

    let request_body = json!({"name": "testuser123"});

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let user: UserResponse = serde_json::from_slice(&body).unwrap();

    assert!(!user.id.is_empty());
    assert_eq!(user.name, "testuser123");
    assert!(!user.api_key.is_empty());
    assert!(user.api_key.starts_with("ak_client_"));
    assert_eq!(user.role, "client");

    app.cleanup().await;
}

#[tokio::test]
async fn test_register_duplicate_username() {
    let app = TestApp::new().await;

    let request_body = json!({"name": "duplicateuser"});

    // 创建第一个用户
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 尝试创建同名用户
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let error: ErrorResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(error.error, "Conflict");

    app.cleanup().await;
}

#[tokio::test]
async fn test_register_invalid_username_empty() {
    let app = TestApp::new().await;

    let request_body = json!({"name": ""});
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    app.cleanup().await;
}

// ============================================================================
// 服务状态测试（公开端点）
// ============================================================================

#[tokio::test]
async fn test_service_status_success() {
    let app = TestApp::new().await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/client/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let status: ServiceStatusResponse = serde_json::from_slice(&body).unwrap();

    assert!(status.healthy);

    app.cleanup().await;
}

// ============================================================================
// 认证失败测试
// ============================================================================

#[tokio::test]
async fn test_auth_missing_header() {
    let app = TestApp::new().await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/client/tasks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    app.cleanup().await;
}

#[tokio::test]
async fn test_auth_invalid_api_key() {
    let app = TestApp::new().await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/client/tasks")
                .header("Authorization", "Bearer invalid_api_key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    app.cleanup().await;
}

// ============================================================================
// 任务列表测试
// ============================================================================

#[tokio::test]
async fn test_list_tasks_success() {
    let app = TestApp::new().await;

    let (user_id, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    create_test_task(app.pool(), &user_id, &service_id, "pending").await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/client/tasks")
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let tasks: Vec<TaskResponse> = serde_json::from_slice(&body).unwrap();
    assert_eq!(tasks.len(), 1);

    app.cleanup().await;
}

// ============================================================================
// 获取单个任务测试
// ============================================================================

#[tokio::test]
async fn test_get_task_success() {
    let app = TestApp::new().await;

    let (user_id, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let task_id = create_test_task(app.pool(), &user_id, &service_id, "pending").await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/client/tasks/{}", task_id))
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: TaskResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(task.id, task_id);

    app.cleanup().await;
}

#[tokio::test]
async fn test_get_task_returns_live_progress_and_complete_clears_it() {
    let app = TestApp::new().await;

    let (user_id, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, agent_api_key, _) =
        create_test_service(app.pool(), "test_service_live", "Live Service", true).await;
    let task_id = create_test_task(app.pool(), &user_id, &service_id, "running").await;

    // Agent 在 poll 时上报执行器内进度
    let poll_body = json!({
        "current_load": 1,
        "available_capacity": 0,
        "snapshots": [{
            "task_id": task_id,
            "seq": 12,
            "text": "2026-11-12T10:00:01Z phase=step1 开始\n2026-11-12T10:00:03Z note=已解析完 12 个 CSV"
        }]
    });

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/agent/{}/poll", service_id))
                .header("Content-Type", "application/json")
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &agent_api_key)
                .body(Body::from(poll_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 1. get_task 应返回 live 视图
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/client/tasks/{}", task_id))
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // live 视图只含原文，不再有 facts 字段
    assert!(
        task["live"].get("facts").is_none(),
        "live 不应含 facts: {}",
        task["live"]
    );
    assert!(
        task["live"]["text"]
            .as_str()
            .unwrap_or_default()
            .contains("note=已解析完 12 个 CSV")
    );
    assert!(
        task["live"].get("source").is_none(),
        "live 不应含 source 键：{}",
        task["live"]
    );
    assert_eq!(task["live"]["stale"].as_bool(), Some(false));
    assert_eq!(task["live"]["truncated"].as_bool(), Some(false));
    // 时间维度已整体删除，只保留布尔 stale
    for key in ["updated_at", "idle_secs", "elapsed_secs"] {
        assert!(
            task["live"].get(key).is_none(),
            "live 不应含 {:?}: {}",
            key,
            task["live"]
        );
    }

    // 2. 任务完成后 live 字段应消失
    let complete_body = json!({"task_id": task_id, "status": "completed"});
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/agent/{}/complete", service_id))
                .header("Content-Type", "application/json")
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &agent_api_key)
                .body(Body::from(complete_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/client/tasks/{}", task_id))
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(task.get("live").is_none());

    app.cleanup().await;
}

#[tokio::test]
async fn test_poll_snapshot_after_task_terminal_is_ignored() {
    let app = TestApp::new().await;

    let (user_id, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, agent_api_key, _) = create_test_service(
        app.pool(),
        "test_service_terminal",
        "Terminal Service",
        true,
    )
    .await;
    let task_id = create_test_task(app.pool(), &user_id, &service_id, "running").await;

    // 1. 任务 running 时上报的进度可见
    let poll_body = json!({
        "snapshots": [{
            "task_id": task_id,
            "seq": 1,
            "text": "running progress"
        }]
    });
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/agent/{}/poll", service_id))
                .header("Content-Type", "application/json")
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &agent_api_key)
                .body(Body::from(poll_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/client/tasks/{}", task_id))
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(task["live"].get("facts").is_none());
    assert_eq!(task["live"]["text"].as_str(), Some("running progress"));

    // 2. 任务完成
    let complete_body = json!({"task_id": task_id, "status": "completed"});
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/agent/{}/complete", service_id))
                .header("Content-Type", "application/json")
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &agent_api_key)
                .body(Body::from(complete_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 3. Agent 再带一条更新的快照，不得让终态任务重新出现 live
    let late_poll_body = json!({
        "snapshots": [{
            "task_id": task_id,
            "seq": 2,
            "text": "late progress"
        }]
    });
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/agent/{}/poll", service_id))
                .header("Content-Type", "application/json")
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &agent_api_key)
                .body(Body::from(late_poll_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/client/tasks/{}", task_id))
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(task.get("live").is_none());

    app.cleanup().await;
}

#[tokio::test]
async fn test_get_task_not_found() {
    let app = TestApp::new().await;

    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/client/tasks/non-existent-task-id")
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    app.cleanup().await;
}

#[tokio::test]
async fn test_get_task_forbidden() {
    let app = TestApp::new().await;

    let (user1_id, _, _) = create_test_user(app.pool(), "user1", UserRole::Client).await;
    let (_, api_key2, _) = create_test_user(app.pool(), "user2", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let task_id = create_test_task(app.pool(), &user1_id, &service_id, "pending").await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/client/tasks/{}", task_id))
                .header(auth_header(&api_key2).0, auth_header(&api_key2).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    app.cleanup().await;
}

// ============================================================================
// 取消任务测试
// ============================================================================

#[tokio::test]
async fn test_cancel_task_pending_success() {
    let app = TestApp::new().await;

    let (user_id, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let task_id = create_test_task(app.pool(), &user_id, &service_id, "pending").await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/client/tasks/{}/cancel", task_id))
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: TaskResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(task.status, "cancelled");

    app.cleanup().await;
}

#[tokio::test]
async fn test_cancel_task_not_found() {
    let app = TestApp::new().await;

    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/tasks/non-existent-task/cancel")
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    app.cleanup().await;
}

#[tokio::test]
async fn test_cancel_task_already_completed() {
    let app = TestApp::new().await;

    let (user_id, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let task_id = create_test_task(app.pool(), &user_id, &service_id, "completed").await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/client/tasks/{}/cancel", task_id))
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    app.cleanup().await;
}

// ============================================================================
// 服务列表测试
// ============================================================================

#[tokio::test]
async fn test_list_services_success() {
    let app = TestApp::new().await;

    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    create_test_service(app.pool(), "public_service", "Public Service", true).await;
    create_test_service(
        app.pool(),
        "restricted_service",
        "Restricted Service",
        false,
    )
    .await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/client/services")
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let services: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(services.len(), 2);

    app.cleanup().await;
}

// ============================================================================
// 服务授权测试
// ============================================================================

#[tokio::test]
async fn test_grant_service_permission_success() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (user_id, _, _) = create_test_user(app.pool(), "normaluser", UserRole::Client).await;
    let (service_id, _, _) = create_test_service(
        app.pool(),
        "restricted_service",
        "Restricted Service",
        false,
    )
    .await;

    let request_body = json!({"user_id": user_id});

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/client/services/{}/grant", service_id))
                .header("Content-Type", "application/json")
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let result: GrantPermissionResponse = serde_json::from_slice(&body).unwrap();
    assert!(result.granted);

    app.cleanup().await;
}

#[tokio::test]
async fn test_grant_service_permission_forbidden_for_client() {
    let app = TestApp::new().await;

    let (_, client_api_key, _) = create_test_user(app.pool(), "client", UserRole::Client).await;
    let (target_user_id, _, _) = create_test_user(app.pool(), "targetuser", UserRole::Client).await;
    let (service_id, _, _) = create_test_service(
        app.pool(),
        "restricted_service",
        "Restricted Service",
        false,
    )
    .await;

    let request_body = json!({"user_id": target_user_id});

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/client/services/{}/grant", service_id))
                .header("Content-Type", "application/json")
                .header(
                    auth_header(&client_api_key).0,
                    auth_header(&client_api_key).1,
                )
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    app.cleanup().await;
}

// ============================================================================
// 创建任务测试
// ============================================================================

#[tokio::test]
async fn test_create_task_success_public_service() {
    let app = TestApp::new().await;

    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;

    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!(
        "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"service_id\"\r\n\r\n\
        {}\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"task_prompt\"\r\n\r\n\
        Test task prompt\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"output_prompt\"\r\n\r\n\
        Test output format\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n",
        service_id
    );

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/tasks")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: TaskResponse = serde_json::from_slice(&body).unwrap();
    assert!(!task.id.is_empty());

    app.cleanup().await;
}

#[tokio::test]
async fn test_create_task_missing_service_id() {
    let app = TestApp::new().await;

    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;

    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"task_prompt\"\r\n\r\n\
        Test task prompt\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n";

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/tasks")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    app.cleanup().await;
}

#[tokio::test]
async fn test_create_task_json_success_public_service() {
    let app = TestApp::new().await;

    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;

    let request_body = json!({
        "service_id": service_id,
        "task_prompt": "Test task prompt",
        "output_prompt": "Test output format"
    });

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/tasks")
                .header("Content-Type", "application/json")
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: TaskResponse = serde_json::from_slice(&body).unwrap();
    assert!(!task.id.is_empty());
    assert!(!task.session_id.is_empty());

    app.cleanup().await;
}

#[tokio::test]
async fn test_create_task_generates_v7_session_id() {
    let app = TestApp::new().await;

    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;

    let request_body = json!({
        "service_id": service_id,
        "task_prompt": "Test task prompt",
        "output_prompt": "Test output format"
    });

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/tasks")
                .header("Content-Type", "application/json")
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: TaskResponse = serde_json::from_slice(&body).unwrap();
    let session_uuid =
        Uuid::parse_str(&task.session_id).expect("session_id should be a valid UUID");
    let version = session_uuid.as_bytes()[6] >> 4;
    assert_eq!(
        version, 7,
        "expected UUID v7 session_id, got version {}",
        version
    );

    app.cleanup().await;
}

#[tokio::test]
async fn test_create_task_json_missing_service_id() {
    let app = TestApp::new().await;

    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;

    let request_body = json!({
        "task_prompt": "Test task prompt",
        "output_prompt": "Test output format"
    });

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/tasks")
                .header("Content-Type", "application/json")
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    app.cleanup().await;
}

#[tokio::test]
async fn test_create_task_json_invalid_session_id() {
    let app = TestApp::new().await;

    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;

    let request_body = json!({
        "service_id": service_id,
        "task_prompt": "Test task prompt",
        "output_prompt": "Test output format",
        "session_id": "../hack"
    });

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/tasks")
                .header("Content-Type", "application/json")
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: TaskResponse = serde_json::from_slice(&body).unwrap();
    assert_ne!(task.session_id, "../hack");
    assert!(!task.session_id.is_empty());

    app.cleanup().await;
}

#[tokio::test]
async fn test_create_task_json_body_too_large() {
    let app = TestApp::new().await;

    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;

    let large_string = "a".repeat(2 * 1024 * 1024);
    let request_body = json!({
        "service_id": "some-service-id",
        "task_prompt": large_string,
        "output_prompt": "Test output format"
    });

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/tasks")
                .header("Content-Type", "application/json")
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    app.cleanup().await;
}

#[tokio::test]
async fn test_create_task_json_invalid_session_id_empty() {
    let app = TestApp::new().await;
    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;

    let body = json!({
        "service_id": service_id,
        "task_prompt": "Test task prompt",
        "output_prompt": "Test output format",
        "session_id": ""
    });

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/tasks")
                .header("Content-Type", "application/json")
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: TaskResponse = serde_json::from_slice(&body).unwrap();
    assert!(!task.session_id.is_empty());
    app.cleanup().await;
}

#[tokio::test]
async fn test_create_task_json_invalid_session_id_with_slash() {
    let app = TestApp::new().await;
    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;

    let body = json!({
        "service_id": service_id,
        "task_prompt": "Test task prompt",
        "output_prompt": "Test output format",
        "session_id": "foo/bar"
    });

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/tasks")
                .header("Content-Type", "application/json")
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: TaskResponse = serde_json::from_slice(&body).unwrap();
    assert_ne!(task.session_id, "foo/bar");
    app.cleanup().await;
}

#[tokio::test]
async fn test_create_task_json_valid_session_id_preserved() {
    let app = TestApp::new().await;
    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;

    let body = json!({
        "service_id": service_id,
        "task_prompt": "Test task prompt",
        "output_prompt": "Test output format",
        "session_id": "my_session_123"
    });

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/client/tasks")
                .header("Content-Type", "application/json")
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let task: TaskResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(task.session_id, "my_session_123");
    app.cleanup().await;
}
