//! Agent API 集成测试 - 负载上报与查询
//!
//! 测试完整流程：心跳上报 + Client 查询

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    create_registered_service, create_test_app, create_test_service, create_test_task,
    create_test_user,
};
use serde_json::json;
use tower::ServiceExt;

// ============================================================================
// 心跳负载上报测试
// ============================================================================

/// 测试心跳上报正常负载
#[tokio::test]
async fn test_heartbeat_with_load_reporting() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    // 发送心跳带上 current_load=2, capacity=5
    let request_body = json!({
        "status": "online",
        "current_load": 2,
        "capacity": 5
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

    // 验证数据库中的 agent_current_load=2, agent_capacity=5
    let row: (i64, i64) =
        sqlx::query_as("SELECT agent_current_load, agent_capacity FROM services WHERE id = ?")
            .bind(&service_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(row.0, 2);
    assert_eq!(row.1, 5);
}

/// 测试心跳上报 busy 状态
#[tokio::test]
async fn test_heartbeat_busy_status() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    // 发送心跳：current_load=5, capacity=5（满载）
    let request_body = json!({
        "status": "busy",
        "current_load": 5,
        "capacity": 5
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

    // 验证 agent_status 变为 busy
    let status: (String,) = sqlx::query_as("SELECT agent_status FROM services WHERE id = ?")
        .bind(&service_id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(status.0, "busy");
}

/// 测试心跳上报 current_load = -1 返回 BadRequest
#[tokio::test]
async fn test_heartbeat_invalid_load_negative() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    let request_body = json!({
        "status": "online",
        "current_load": -1,
        "capacity": 5
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
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// 测试心跳上报 capacity = 0 返回 BadRequest
#[tokio::test]
async fn test_heartbeat_invalid_capacity_zero() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    let request_body = json!({
        "status": "online",
        "current_load": 0,
        "capacity": 0
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
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// 测试心跳上报 current_load > capacity 返回 BadRequest
#[tokio::test]
async fn test_heartbeat_invalid_load_exceeds_capacity() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    let request_body = json!({
        "status": "online",
        "current_load": 10,
        "capacity": 5
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
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// 测试心跳上报 status = "invalid" 返回 BadRequest
#[tokio::test]
async fn test_heartbeat_invalid_status_rejected() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    let request_body = json!({
        "status": "invalid_status",
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
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// 测试心跳不带负载字段（向后兼容）
#[tokio::test]
async fn test_heartbeat_without_load() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;

    // 先设置初始负载值
    sqlx::query("UPDATE services SET agent_current_load = 3, agent_capacity = 10 WHERE id = ?")
        .bind(&service_id)
        .execute(&pool)
        .await
        .unwrap();

    // 只发送 status 字段
    let request_body = json!({
        "status": "online"
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

    // 验证成功，不改变负载值
    let row: (i64, i64) =
        sqlx::query_as("SELECT agent_current_load, agent_capacity FROM services WHERE id = ?")
            .bind(&service_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(row.0, 3); // 保持不变
    assert_eq!(row.1, 10); // 保持不变
}

// ============================================================================
// 服务负载查询测试
// ============================================================================

/// 测试获取服务负载详情
#[tokio::test]
async fn test_get_service_load() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;
    let (_user_id, api_key_client) = create_test_user(&pool, "client").await;

    // Agent 心跳上报：current_load=2, capacity=5
    let heartbeat_body = json!({
        "status": "online",
        "current_load": 2,
        "capacity": 5
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

    // 创建一些 pending 和 running 任务
    create_test_task(&pool, &service_id, "pending").await;
    create_test_task(&pool, &service_id, "pending").await;
    create_test_task(&pool, &service_id, "running").await;

    // 调用 GET /client/services/{id}/load
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/client/services/{}/load", service_id))
        .header("Authorization", format!("Bearer {}", api_key_client))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // 验证返回：capacity=5, current_load=2, pending_tasks=2, running_tasks=1
    assert_eq!(json["capacity"].as_i64(), Some(5));
    assert_eq!(json["current_load"].as_i64(), Some(2));
    assert_eq!(json["available_slots"].as_i64(), Some(3));
    assert_eq!(json["pending_tasks"].as_i64(), Some(2));
    assert_eq!(json["running_tasks"].as_i64(), Some(1));
    assert_eq!(json["service_id"].as_str(), Some(service_id.as_str()));
    assert_eq!(json["agent_status"].as_str(), Some("online"));
}

/// 测试服务负载查询权限控制 - 受限服务无权限访问返回 403
#[tokio::test]
async fn test_get_service_load_permission_denied() {
    let (app, _state, pool) = create_test_app().await;

    // 创建受限服务（非公开）
    let service_id = format!("test-service-{}", uuid::Uuid::new_v4());
    let api_key = format!(
        "ak_agent_{}",
        uuid::Uuid::new_v4().to_string().replace("-", "")
    );

    let hashed_api_key = open_aaas_server::auth::hash_api_key(common::TEST_SECRET_KEY, &api_key);
    sqlx::query(
        r#"
        INSERT INTO services (id, name, description, usage, agent_api_key, registration_status, 
                              agent_status, agent_capacity, agent_current_load, is_public, created_at)
        VALUES (?, ?, ?, ?, ?, 'active', 'online', 5, 0, false, datetime('now'))
        "#
    )
    .bind(&service_id)
    .bind("Restricted Service")
    .bind("A restricted service")
    .bind("Test usage")
    .bind(&hashed_api_key)
    .execute(&pool)
    .await
    .unwrap();

    // 创建普通用户（无权限）
    let (_, api_key_client) = create_test_user(&pool, "client").await;

    // 调用 GET /client/services/{id}/load - 应该返回 403
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/client/services/{}/load", service_id))
        .header("Authorization", format!("Bearer {}", api_key_client))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// 测试负载为0的情况
#[tokio::test]
async fn test_get_service_load_zero_capacity() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, _api_key, _) = create_registered_service(&pool).await;
    let (_, api_key_client) = create_test_user(&pool, "client").await;

    // Agent 上报 capacity=0
    let _heartbeat_body = json!({
        "status": "online",
        "current_load": 0,
        "capacity": 0
    });

    // 先更新容量为0（绕过验证）
    sqlx::query("UPDATE services SET agent_capacity = 0, agent_current_load = 0 WHERE id = ?")
        .bind(&service_id)
        .execute(&pool)
        .await
        .unwrap();

    // 调用 GET /client/services/{id}/load
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/client/services/{}/load", service_id))
        .header("Authorization", format!("Bearer {}", api_key_client))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // 验证 available_slots=0
    assert_eq!(json["capacity"].as_i64(), Some(0));
    assert_eq!(json["current_load"].as_i64(), Some(0));
    assert_eq!(json["available_slots"].as_i64(), Some(0));
}

// ============================================================================
// 完整流程测试
// ============================================================================

/// 测试完整流程：心跳上报 + Client 查询
#[tokio::test]
async fn test_heartbeat_and_load_query() {
    let (app, _state, pool) = create_test_app().await;

    // 1. Admin 创建 Service（使用 create_test_service）
    let (service_id, registration_token) = create_test_service(&pool).await;

    // 2. Agent 注册
    let register_body = json!({
        "registration_token": registration_token,
        "capacity": 5
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

    // 3. 创建 Client 用户
    let (_, api_key_client) = create_test_user(&pool, "client").await;

    // 4. Agent 心跳上报负载
    let heartbeat_body = json!({
        "status": "online",
        "current_load": 3,
        "capacity": 5
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

    // 创建一些任务
    create_test_task(&pool, &service_id, "pending").await;
    create_test_task(&pool, &service_id, "pending").await;
    create_test_task(&pool, &service_id, "running").await;

    // 5. Client 查询负载详情
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/client/services/{}/load", service_id))
        .header("Authorization", format!("Bearer {}", api_key_client))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // 6. 验证数据一致性
    assert_eq!(json["service_id"].as_str(), Some(service_id.as_str()));
    assert_eq!(json["capacity"].as_i64(), Some(5));
    assert_eq!(json["current_load"].as_i64(), Some(3));
    assert_eq!(json["available_slots"].as_i64(), Some(2));
    assert_eq!(json["agent_status"].as_str(), Some("online"));
    assert_eq!(json["pending_tasks"].as_i64(), Some(2));
    assert_eq!(json["running_tasks"].as_i64(), Some(1));
}

/// 测试完整工作流：上报负载变化
#[tokio::test]
async fn test_load_changes_workflow() {
    let (app, _state, pool) = create_test_app().await;
    let (service_id, api_key, _) = create_registered_service(&pool).await;
    let (_, api_key_client) = create_test_user(&pool, "client").await;

    // 初始状态：空闲
    let heartbeat_body = json!({
        "status": "online",
        "current_load": 0,
        "capacity": 5
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

    // 查询负载
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/client/services/{}/load", service_id))
        .header("Authorization", format!("Bearer {}", api_key_client))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["current_load"].as_i64(), Some(0));
    assert_eq!(json["available_slots"].as_i64(), Some(5));

    // 开始处理任务：上报负载增加
    let heartbeat_body = json!({
        "status": "online",
        "current_load": 2,
        "capacity": 5
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

    // 再次查询负载
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/client/services/{}/load", service_id))
        .header("Authorization", format!("Bearer {}", api_key_client))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["current_load"].as_i64(), Some(2));
    assert_eq!(json["available_slots"].as_i64(), Some(3));

    // 满载：上报 busy
    let heartbeat_body = json!({
        "status": "busy",
        "current_load": 5,
        "capacity": 5
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

    // 最终查询
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/client/services/{}/load", service_id))
        .header("Authorization", format!("Bearer {}", api_key_client))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["current_load"].as_i64(), Some(5));
    assert_eq!(json["available_slots"].as_i64(), Some(0));
    assert_eq!(json["agent_status"].as_str(), Some("busy"));
}
