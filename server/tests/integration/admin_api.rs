//! Admin API 集成测试
//!
//! 测试用户管理、角色修改、服务权限等功能（需要管理员权限）

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use open_aaas_server::models::user::UserRole;
use serde_json::json;
use tower::ServiceExt;

use super::{
    TestApp, auth_header, create_test_service, create_test_task, create_test_user,
    grant_service_permission,
};

// ============================================================================
// 列出所有用户测试
// ============================================================================

#[tokio::test]
async fn test_admin_list_users_success() {
    let app = TestApp::new().await;

    // 创建管理员用户
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    // 创建普通用户
    let (_, _, _) = create_test_user(app.pool(), "user1", UserRole::Client).await;
    let (_, _, _) = create_test_user(app.pool(), "user2", UserRole::Client).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/users")
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    if status != StatusCode::OK {
        eprintln!("DEBUG RESPONSE: {:?}", String::from_utf8_lossy(&body_bytes));
    }
    assert_eq!(status, StatusCode::OK);
    let users: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();

    // 至少应该有 3 个用户
    assert!(users.len() >= 3);

    // 验证字段包含 api_key（admin 接口返回完整 key）
    let user1 = users.iter().find(|u| u["name"] == "user1");
    assert!(user1.is_some());
    assert!(user1.unwrap()["api_key"].is_string());
    assert!(user1.unwrap()["role"] == "client");

    app.cleanup().await;
}

#[tokio::test]
async fn test_client_list_users_forbidden() {
    let app = TestApp::new().await;

    let (_, client_api_key, _) = create_test_user(app.pool(), "clientuser", UserRole::Client).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/users")
                .header(
                    auth_header(&client_api_key).0,
                    auth_header(&client_api_key).1,
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_list_tasks_includes_all_users() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (user1_id, _, _) = create_test_user(app.pool(), "user1", UserRole::Client).await;
    let (user2_id, _, _) = create_test_user(app.pool(), "user2", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test-service", "Test Service", true).await;
    let user1_task = create_test_task(app.pool(), &user1_id, &service_id, "pending").await;
    let user2_task = create_test_task(app.pool(), &user2_id, &service_id, "running").await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/tasks")
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let tasks: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();

    assert!(tasks.iter().any(|t| t["id"] == user1_task));
    assert!(tasks.iter().any(|t| t["id"] == user2_task));
    assert!(tasks.iter().any(|t| t["user_name"] == "user1"));
    assert!(tasks.iter().any(|t| t["user_name"] == "user2"));

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_list_tasks_filters_by_user() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (user1_id, _, _) = create_test_user(app.pool(), "user1", UserRole::Client).await;
    let (user2_id, _, _) = create_test_user(app.pool(), "user2", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test-service", "Test Service", true).await;
    let user1_task = create_test_task(app.pool(), &user1_id, &service_id, "pending").await;
    let user2_task = create_test_task(app.pool(), &user2_id, &service_id, "running").await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/admin/tasks?user_id={}", user1_id))
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let tasks: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();

    assert!(tasks.iter().any(|t| t["id"] == user1_task));
    assert!(!tasks.iter().any(|t| t["id"] == user2_task));
    assert!(tasks.iter().all(|t| t["user_id"] == user1_id));

    app.cleanup().await;
}

#[tokio::test]
async fn test_client_list_admin_tasks_forbidden() {
    let app = TestApp::new().await;

    let (_, client_api_key, _) = create_test_user(app.pool(), "clientuser", UserRole::Client).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/tasks")
                .header(
                    auth_header(&client_api_key).0,
                    auth_header(&client_api_key).1,
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    app.cleanup().await;
}

// ============================================================================
// 删除用户测试
// ============================================================================

#[tokio::test]
async fn test_admin_delete_user_success() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (user_id, _, _) = create_test_user(app.pool(), "delete_me", UserRole::Client).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/admin/users/{}", user_id))
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["deleted"].as_bool(), Some(true));

    // 验证用户已被删除
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(app.pool())
        .await
        .unwrap();
    assert_eq!(count.0, 0);

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_delete_self_forbidden() {
    let app = TestApp::new().await;

    let (admin_id, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/admin/users/{}", admin_id))
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 验证管理员仍然存在
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id = ?")
        .bind(&admin_id)
        .fetch_one(app.pool())
        .await
        .unwrap();
    assert_eq!(count.0, 1);

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_delete_user_not_found() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/admin/users/non-existent-user")
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    app.cleanup().await;
}

// ============================================================================
// 修改用户角色测试
// ============================================================================

#[tokio::test]
async fn test_admin_update_user_role_success() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (user_id, _, _) = create_test_user(app.pool(), "normaluser", UserRole::Client).await;

    let request_body = json!({"role": "admin"});

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/admin/users/{}/role", user_id))
                .header("Content-Type", "application/json")
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["id"].as_str(), Some(user_id.as_str()));
    assert_eq!(json["role"].as_str(), Some("admin"));

    // 验证数据库中角色已更新
    let role: (String,) = sqlx::query_as("SELECT role FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(app.pool())
        .await
        .unwrap();
    assert_eq!(role.0, "admin");

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_update_self_role_forbidden() {
    let app = TestApp::new().await;

    let (admin_id, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;

    let request_body = json!({"role": "client"});

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/admin/users/{}/role", admin_id))
                .header("Content-Type", "application/json")
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 验证管理员角色未变
    let role: (String,) = sqlx::query_as("SELECT role FROM users WHERE id = ?")
        .bind(&admin_id)
        .fetch_one(app.pool())
        .await
        .unwrap();
    assert_eq!(role.0, "admin");

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_update_user_role_not_found() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;

    let request_body = json!({"role": "admin"});

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/admin/users/non-existent-user/role")
                .header("Content-Type", "application/json")
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    app.cleanup().await;
}

// ============================================================================
// 查看用户服务权限测试
// ============================================================================

#[tokio::test]
async fn test_admin_list_user_permissions_success() {
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

    // 授予权限
    grant_service_permission(app.pool(), &user_id, &service_id).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/admin/users/{}/permissions", user_id))
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let permissions: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(permissions.len(), 1);
    assert_eq!(
        permissions[0]["service_id"].as_str(),
        Some("restricted_service")
    );
    assert_eq!(
        permissions[0]["service_name"].as_str(),
        Some("Restricted Service")
    );
    assert!(permissions[0]["granted_at"].is_string());

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_list_user_permissions_empty() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (user_id, _, _) = create_test_user(app.pool(), "normaluser", UserRole::Client).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/admin/users/{}/permissions", user_id))
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let permissions: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();
    assert!(permissions.is_empty());

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_list_user_permissions_not_found() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/users/non-existent-user/permissions")
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    app.cleanup().await;
}

// ============================================================================
// 撤销用户服务权限测试
// ============================================================================

#[tokio::test]
async fn test_admin_revoke_service_permission_success() {
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

    // 先授予权限
    grant_service_permission(app.pool(), &user_id, &service_id).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/admin/services/{}/users/{}",
                    service_id, user_id
                ))
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["revoked"].as_bool(), Some(true));

    // 验证权限已被删除
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM user_service_permissions WHERE user_id = ? AND service_id = ?",
    )
    .bind(&user_id)
    .bind(&service_id)
    .fetch_one(app.pool())
    .await
    .unwrap();
    assert_eq!(count.0, 0);

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_revoke_service_permission_idempotent() {
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

    // 不授予权限，直接撤销
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/admin/services/{}/users/{}",
                    service_id, user_id
                ))
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 幂等：即使不存在也返回 200
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["revoked"].as_bool(), Some(true));

    app.cleanup().await;
}

// ============================================================================
// Client 权限拒绝测试
// ============================================================================

#[tokio::test]
async fn test_client_delete_user_forbidden() {
    let app = TestApp::new().await;

    let (_, _admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (user_id, _, _) = create_test_user(app.pool(), "targetuser", UserRole::Client).await;
    let (_, client_api_key, _) = create_test_user(app.pool(), "clientuser", UserRole::Client).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/admin/users/{}", user_id))
                .header(
                    auth_header(&client_api_key).0,
                    auth_header(&client_api_key).1,
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // 验证用户未被删除
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(app.pool())
        .await
        .unwrap();
    assert_eq!(count.0, 1);

    app.cleanup().await;
}

#[tokio::test]
async fn test_client_update_user_role_forbidden() {
    let app = TestApp::new().await;

    let (user_id, _, _) = create_test_user(app.pool(), "targetuser", UserRole::Client).await;
    let (_, client_api_key, _) = create_test_user(app.pool(), "clientuser", UserRole::Client).await;

    let request_body = json!({"role": "admin"});

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/admin/users/{}/role", user_id))
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

    // 验证角色未变
    let role: (String,) = sqlx::query_as("SELECT role FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(app.pool())
        .await
        .unwrap();
    assert_eq!(role.0, "client");

    app.cleanup().await;
}

#[tokio::test]
async fn test_client_revoke_service_permission_forbidden() {
    let app = TestApp::new().await;

    let (user_id, _, _) = create_test_user(app.pool(), "normaluser", UserRole::Client).await;
    let (service_id, _, _) = create_test_service(
        app.pool(),
        "restricted_service",
        "Restricted Service",
        false,
    )
    .await;
    let (_, client_api_key, _) = create_test_user(app.pool(), "clientuser", UserRole::Client).await;

    grant_service_permission(app.pool(), &user_id, &service_id).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/admin/services/{}/users/{}",
                    service_id, user_id
                ))
                .header(
                    auth_header(&client_api_key).0,
                    auth_header(&client_api_key).1,
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // 验证权限未被撤销
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM user_service_permissions WHERE user_id = ? AND service_id = ?",
    )
    .bind(&user_id)
    .bind(&service_id)
    .fetch_one(app.pool())
    .await
    .unwrap();
    assert_eq!(count.0, 1);

    app.cleanup().await;
}

// ============================================================================
// 非法输入测试
// ============================================================================

#[tokio::test]
async fn test_admin_update_user_role_invalid_role_unprocessable() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (user_id, _, _) = create_test_user(app.pool(), "normaluser", UserRole::Client).await;

    let request_body = json!({"role": "superadmin"});

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/admin/users/{}/role", user_id))
                .header("Content-Type", "application/json")
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 验证角色未变
    let role: (String,) = sqlx::query_as("SELECT role FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_one(app.pool())
        .await
        .unwrap();
    assert_eq!(role.0, "client");

    app.cleanup().await;
}

// ============================================================================
// Admin 对 Admin 操作测试
// ============================================================================

#[tokio::test]
async fn test_admin_delete_other_admin_allowed() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin1", UserRole::Admin).await;
    let (admin2_id, _, _) = create_test_user(app.pool(), "admin2", UserRole::Admin).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/admin/users/{}", admin2_id))
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["deleted"].as_bool(), Some(true));

    // 验证 admin2 已被删除
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id = ?")
        .bind(&admin2_id)
        .fetch_one(app.pool())
        .await
        .unwrap();
    assert_eq!(count.0, 0);

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_update_other_admin_role_allowed() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin1", UserRole::Admin).await;
    let (admin2_id, _, _) = create_test_user(app.pool(), "admin2", UserRole::Admin).await;

    let request_body = json!({"role": "client"});

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/admin/users/{}/role", admin2_id))
                .header("Content-Type", "application/json")
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["id"].as_str(), Some(admin2_id.as_str()));
    assert_eq!(json["role"].as_str(), Some("client"));

    // 验证数据库中角色已更新
    let role: (String,) = sqlx::query_as("SELECT role FROM users WHERE id = ?")
        .bind(&admin2_id)
        .fetch_one(app.pool())
        .await
        .unwrap();
    assert_eq!(role.0, "client");

    app.cleanup().await;
}

// ============================================================================
// 强制删除服务测试
// ============================================================================

#[tokio::test]
async fn test_admin_force_delete_service_with_active_tasks() {
    let app = TestApp::new().await;

    let (admin_id, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (service_id, _, _) = create_test_service(
        app.pool(),
        "force_delete_service",
        "Force Delete Service",
        false,
    )
    .await;

    // 授予一个用户权限
    let (user_id, _, _) = create_test_user(app.pool(), "normaluser", UserRole::Client).await;
    grant_service_permission(app.pool(), &user_id, &service_id).await;

    // 创建各种状态的任务
    let pending_task = create_test_task(app.pool(), &admin_id, &service_id, "pending").await;
    let running_task = create_test_task(app.pool(), &admin_id, &service_id, "running").await;
    let cancelling_task = create_test_task(app.pool(), &admin_id, &service_id, "cancelling").await;
    let completed_task = create_test_task(app.pool(), &admin_id, &service_id, "completed").await;
    let failed_task = create_test_task(app.pool(), &admin_id, &service_id, "failed").await;
    let cancelled_task = create_test_task(app.pool(), &admin_id, &service_id, "cancelled").await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/services/{}?force=true", service_id))
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["deleted"].as_bool(), Some(true));
    assert_eq!(json["tasks_cancelled"].as_i64(), Some(3));
    assert_eq!(json["tasks_retained"].as_i64(), Some(3));

    // 验证服务已被删除
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM services WHERE id = ?")
        .bind(&service_id)
        .fetch_one(app.pool())
        .await
        .unwrap();
    assert_eq!(count.0, 0);

    // 验证活跃任务已被取消
    for task_id in [&pending_task, &running_task, &cancelling_task] {
        let (status, error_message, completed_at): (String, Option<String>, Option<String>) =
            sqlx::query_as("SELECT status, error_message, completed_at FROM tasks WHERE id = ?")
                .bind(task_id)
                .fetch_one(app.pool())
                .await
                .unwrap();
        assert_eq!(status, "cancelled");
        assert_eq!(
            error_message.as_deref(),
            Some("Service was forcefully deleted by admin")
        );
        assert!(completed_at.is_some());
    }

    // 验证已结束任务仍然保留
    for task_id in [&completed_task, &failed_task, &cancelled_task] {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_one(app.pool())
            .await
            .unwrap();
        assert_eq!(count.0, 1);
    }

    // 验证权限已被删除
    let perm_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM user_service_permissions WHERE service_id = ?")
            .bind(&service_id)
            .fetch_one(app.pool())
            .await
            .unwrap();
    assert_eq!(perm_count.0, 0);

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_force_delete_service_without_tasks() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "force_delete_empty", "Force Delete Empty", true).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/services/{}?force=true", service_id))
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["deleted"].as_bool(), Some(true));
    assert_eq!(json["tasks_cancelled"].as_i64(), Some(0));
    assert_eq!(json["tasks_retained"].as_i64(), Some(0));

    // 验证服务已被删除
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM services WHERE id = ?")
        .bind(&service_id)
        .fetch_one(app.pool())
        .await
        .unwrap();
    assert_eq!(count.0, 0);

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_delete_service_with_tasks_no_force_returns_bad_request() {
    let app = TestApp::new().await;

    let (admin_id, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "no_force_delete", "No Force Delete", true).await;

    // 创建关联任务
    create_test_task(app.pool(), &admin_id, &service_id, "pending").await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/services/{}", service_id))
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 验证服务仍然存在
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM services WHERE id = ?")
        .bind(&service_id)
        .fetch_one(app.pool())
        .await
        .unwrap();
    assert_eq!(count.0, 1);

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_delete_service_not_found() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/services/non-existent-service")
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_force_delete_service_not_found() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/services/non-existent-service?force=true")
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    app.cleanup().await;
}

// ============================================================================
// 认证测试
// ============================================================================

#[tokio::test]
async fn test_admin_api_missing_auth() {
    let app = TestApp::new().await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_api_invalid_auth() {
    let app = TestApp::new().await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/users")
                .header("Authorization", "Bearer invalid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    app.cleanup().await;
}

// ============================================================================
// 列出服务授权用户测试
// ============================================================================

#[tokio::test]
async fn test_list_service_users_not_found() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/services/non-existent-service/users")
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    app.cleanup().await;
}

#[tokio::test]
async fn test_list_service_users_public_empty() {
    let app = TestApp::new().await;

    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "public_service", "Public Service", true).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/admin/services/{}/users", service_id))
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["is_public"].as_bool(), Some(true));
    assert!(json["users"].as_array().unwrap().is_empty());

    app.cleanup().await;
}

#[tokio::test]
async fn test_list_service_users_restricted_with_users() {
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

    grant_service_permission(app.pool(), &user_id, &service_id).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/admin/services/{}/users", service_id))
                .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["is_public"].as_bool(), Some(false));
    let users = json["users"].as_array().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0]["user_id"].as_str(), Some(user_id.as_str()));
    assert_eq!(users[0]["user_name"].as_str(), Some("normaluser"));
    assert_eq!(users[0]["role"].as_str(), Some("client"));
    assert!(users[0]["granted_at"].is_string());

    app.cleanup().await;
}
