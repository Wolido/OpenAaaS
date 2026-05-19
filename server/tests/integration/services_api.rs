//! 服务管理 API 集成测试
//!
//! 测试服务列表、创建、删除等功能（需要管理员权限）

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use open_aaas_server::models::user::UserRole;
use serde_json::json;

use super::{
    auth_header, create_test_service, create_test_task, create_test_user,
    grant_service_permission, TestApp,
};
use tower::ServiceExt;

// ============================================================================
// 服务列表测试
// ============================================================================

#[tokio::test]
async fn test_admin_list_services_success() {
    let app = TestApp::new().await;
    
    // 创建管理员用户
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    
    // 创建多个服务
    create_test_service(app.pool(), "service1", "Service 1", true).await;
    create_test_service(app.pool(), "service2", "Service 2", false).await;
    create_test_service(app.pool(), "service3", "Service 3", true).await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri("/api/v1/services")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let services: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();
    
    // 至少应该有 3 个服务（migrations 中可能已经有一些）
    assert!(services.len() >= 3);
    
    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_list_services_empty() {
    let app = TestApp::new().await;
    
    // 创建管理员用户
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri("/api/v1/services")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let services: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();
    
    // 新创建的数据库应该没有服务
    assert!(services.is_empty());
    
    app.cleanup().await;
}

#[tokio::test]
async fn test_client_list_services_forbidden() {
    let app = TestApp::new().await;
    
    // 创建普通用户
    let (_, client_api_key, _) = create_test_user(app.pool(), "clientuser", UserRole::Client).await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri("/api/v1/services")
            .header(auth_header(&client_api_key).0, auth_header(&client_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    // 普通用户无权访问服务管理接口
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    
    app.cleanup().await;
}

#[tokio::test]
async fn test_list_services_with_public_and_restricted() {
    let app = TestApp::new().await;
    
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    
    // 创建公开服务
    create_test_service(app.pool(), "public_service", "Public Service", true).await;
    // 创建受限服务
    create_test_service(app.pool(), "restricted_service", "Restricted Service", false).await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri("/api/v1/services")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let services: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();
    
    // 查找我们创建的服务
    let public_svc = services.iter().find(|s| s["id"] == "public_service");
    let restricted_svc = services.iter().find(|s| s["id"] == "restricted_service");
    
    assert!(public_svc.is_some());
    assert!(restricted_svc.is_some());
    
    // 验证 access_type
    assert_eq!(public_svc.unwrap()["access_type"].as_str(), Some("public"));
    assert_eq!(restricted_svc.unwrap()["access_type"].as_str(), Some("restricted"));
    
    // 管理员视角应该有所有权限
    assert_eq!(public_svc.unwrap()["has_permission"].as_bool(), Some(true));
    assert_eq!(restricted_svc.unwrap()["has_permission"].as_bool(), Some(true));
    
    app.cleanup().await;
}

// ============================================================================
// 创建服务测试
// ============================================================================

#[tokio::test]
async fn test_admin_create_service_success() {
    let app = TestApp::new().await;
    
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    
    let request_body = json!({
        "id": "new_service",
        "name": "New Test Service",
        "description": "A service created by test",
        "usage": "Service usage description",
        "is_public": true
    });
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/v1/services")
            .header("Content-Type", "application/json")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::from(request_body.to_string()))
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    
    assert_eq!(json["id"].as_str(), Some("new_service"));
    assert_eq!(json["name"].as_str(), Some("New Test Service"));
    assert!(json["registration_token"].as_str().is_some());
    assert!(json["registration_token"].as_str().unwrap().starts_with("rt_"));
    assert_eq!(json["registration_status"].as_str(), Some("pending"));
    
    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_create_service_auto_id() {
    let app = TestApp::new().await;
    
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    
    let request_body = json!({
        "name": "Auto ID Service",
        "description": "Service without explicit ID",
        "usage": "Test usage"
    });
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/v1/services")
            .header("Content-Type", "application/json")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::from(request_body.to_string()))
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    
    // 验证自动生成的 ID 是有效的 UUID
    let id = json["id"].as_str().unwrap();
    assert!(uuid::Uuid::parse_str(id).is_ok());
    
    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_create_service_duplicate_id() {
    let app = TestApp::new().await;
    
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    
    // 先创建一个服务
    let request_body = json!({
        "id": "duplicate_service",
        "name": "First Service",
        "description": "First service",
        "usage": "Test usage"
    });
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/v1/services")
            .header("Content-Type", "application/json")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::from(request_body.to_string()))
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // 尝试创建相同 ID 的服务
    let request_body2 = json!({
        "id": "duplicate_service",
        "name": "Second Service",
        "description": "Second service",
        "usage": "Test usage"
    });
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/v1/services")
            .header("Content-Type", "application/json")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::from(request_body2.to_string()))
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(json["message"].as_str().unwrap().contains("duplicate_service"));
    
    app.cleanup().await;
}

#[tokio::test]
async fn test_client_create_service_forbidden() {
    let app = TestApp::new().await;
    
    let (_, client_api_key, _) = create_test_user(app.pool(), "clientuser", UserRole::Client).await;
    
    let request_body = json!({
        "id": "unauthorized_service",
        "name": "Unauthorized Service",
        "description": "This should not be created",
        "usage": "Test usage"
    });
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/v1/services")
            .header("Content-Type", "application/json")
            .header(auth_header(&client_api_key).0, auth_header(&client_api_key).1)
            .body(Body::from(request_body.to_string()))
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    
    app.cleanup().await;
}

#[tokio::test]
async fn test_create_service_restricted() {
    let app = TestApp::new().await;
    
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    
    let request_body = json!({
        "id": "restricted_service",
        "name": "Restricted Service",
        "description": "A restricted service",
        "usage": "Test usage",
        "is_public": false
    });
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/v1/services")
            .header("Content-Type", "application/json")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::from(request_body.to_string()))
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    
    // 验证服务已创建
    assert_eq!(json["id"].as_str(), Some("restricted_service"));
    
    // 验证在数据库中是受限服务
    let is_public: (bool,) = sqlx::query_as("SELECT is_public FROM services WHERE id = ?")
        .bind("restricted_service")
        .fetch_one(app.pool())
        .await
        .unwrap();
    
    assert!(!is_public.0);
    
    app.cleanup().await;
}

// ============================================================================
// 获取单个服务详情测试
// ============================================================================

#[tokio::test]
async fn test_admin_get_service_success() {
    let app = TestApp::new().await;
    
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (service_id, _, _) = create_test_service(app.pool(), "test_service", "Test Service", true).await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri(&format!("/api/v1/services/{}", service_id))
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    
    assert_eq!(json["id"].as_str(), Some(service_id.as_str()));
    
    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_get_service_not_found() {
    let app = TestApp::new().await;
    
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri("/api/v1/services/non-existent-service")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    
    app.cleanup().await;
}

#[tokio::test]
async fn test_client_get_service_forbidden() {
    let app = TestApp::new().await;
    
    let (_, client_api_key, _) = create_test_user(app.pool(), "clientuser", UserRole::Client).await;
    let (service_id, _, _) = create_test_service(app.pool(), "test_service", "Test Service", true).await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri(&format!("/api/v1/services/{}", service_id))
            .header(auth_header(&client_api_key).0, auth_header(&client_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    
    app.cleanup().await;
}

// ============================================================================
// 删除服务测试
// ============================================================================

#[tokio::test]
async fn test_admin_delete_service_success() {
    let app = TestApp::new().await;
    
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (service_id, _, _) = create_test_service(app.pool(), "delete_me", "Delete Me Service", true).await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("DELETE")
            .uri(&format!("/api/v1/services/{}", service_id))
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    
    assert_eq!(json["deleted"].as_bool(), Some(true));
    
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
async fn test_admin_delete_service_not_found() {
    let app = TestApp::new().await;
    
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("DELETE")
            .uri("/api/v1/services/non-existent-service")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    // 删除不存在的服务应该返回 200（幂等性）或 404
    // 根据实现，这里是返回 200 且 deleted 为 true
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND);
    
    app.cleanup().await;
}

#[tokio::test]
async fn test_admin_delete_service_with_tasks() {
    let app = TestApp::new().await;
    
    let (admin_id, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    let (service_id, _, _) = create_test_service(app.pool(), "service_with_tasks", "Service With Tasks", true).await;
    
    // 创建关联的任务
    create_test_task(app.pool(), &admin_id, &service_id, "pending").await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("DELETE")
            .uri(&format!("/api/v1/services/{}", service_id))
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    // 有关联任务的服务不应该被删除
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    
    assert!(json["message"].as_str().unwrap().contains("任务"));
    
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
async fn test_client_delete_service_forbidden() {
    let app = TestApp::new().await;
    
    let (_, client_api_key, _) = create_test_user(app.pool(), "clientuser", UserRole::Client).await;
    let (service_id, _, _) = create_test_service(app.pool(), "test_service", "Test Service", true).await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("DELETE")
            .uri(&format!("/api/v1/services/{}", service_id))
            .header(auth_header(&client_api_key).0, auth_header(&client_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    
    // 验证服务仍然存在
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM services WHERE id = ?")
        .bind(&service_id)
        .fetch_one(app.pool())
        .await
        .unwrap();
    
    assert_eq!(count.0, 1);
    
    app.cleanup().await;
}

// ============================================================================
// 认证测试
// ============================================================================

#[tokio::test]
async fn test_services_api_missing_auth() {
    let app = TestApp::new().await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri("/api/v1/services")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    
    app.cleanup().await;
}

#[tokio::test]
async fn test_services_api_invalid_auth() {
    let app = TestApp::new().await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri("/api/v1/services")
            .header("Authorization", "Bearer invalid_token")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    
    app.cleanup().await;
}

// ============================================================================
// 完整工作流测试
// ============================================================================

#[tokio::test]
async fn test_service_lifecycle() {
    let app = TestApp::new().await;
    
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    
    // 1. 创建服务
    let create_body = json!({
        "id": "lifecycle_service",
        "name": "Lifecycle Service",
        "description": "A service for testing lifecycle",
        "usage": "Test usage",
        "is_public": false
    });
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/v1/services")
            .header("Content-Type", "application/json")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::from(create_body.to_string()))
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let create_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let _registration_token = create_response["registration_token"].as_str().unwrap();
    
    // 2. 获取服务详情
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri("/api/v1/services/lifecycle_service")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // 3. 列出服务
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri("/api/v1/services")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let services: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();
    assert!(services.iter().any(|s| s["id"] == "lifecycle_service"));
    
    // 4. 删除服务
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("DELETE")
            .uri("/api/v1/services/lifecycle_service")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // 5. 验证服务已被删除
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri("/api/v1/services/lifecycle_service")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    
    app.cleanup().await;
}

// ============================================================================
// 边缘情况测试
// ============================================================================

#[tokio::test]
async fn test_create_service_with_unicode_name() {
    let app = TestApp::new().await;
    
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    
    let request_body = json!({
        "id": "unicode_service",
        "name": "中文服务名称 🎉",
        "description": "Description with unicode: 日本語",
        "usage": "Usage: español"
    });
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/v1/services")
            .header("Content-Type", "application/json")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::from(request_body.to_string()))
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    
    assert_eq!(json["name"].as_str(), Some("中文服务名称 🎉"));
    
    app.cleanup().await;
}

#[tokio::test]
async fn test_create_service_default_is_public() {
    let app = TestApp::new().await;
    
    let (_, admin_api_key, _) = create_test_user(app.pool(), "admin", UserRole::Admin).await;
    
    // 不指定 is_public，应该默认为 true
    let request_body = json!({
        "id": "default_public_service",
        "name": "Default Public Service",
        "description": "Description",
        "usage": "Test usage"
    });
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("POST")
            .uri("/api/v1/services")
            .header("Content-Type", "application/json")
            .header(auth_header(&admin_api_key).0, auth_header(&admin_api_key).1)
            .body(Body::from(request_body.to_string()))
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // 验证数据库中 is_public 为 true
    let is_public: (bool,) = sqlx::query_as("SELECT is_public FROM services WHERE id = ?")
        .bind("default_public_service")
        .fetch_one(app.pool())
        .await
        .unwrap();
    
    assert!(is_public.0);
    
    app.cleanup().await;
}

// ============================================================================
// 服务 usage 查询测试（Client API）
// ============================================================================

#[tokio::test]
async fn test_get_public_service_usage_success() {
    let app = TestApp::new().await;
    
    let (_, client_api_key, _) = create_test_user(app.pool(), "clientuser", UserRole::Client).await;
    let (service_id, _, _) = create_test_service(app.pool(), "public_svc", "Public Service", true).await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri(&format!("/api/v1/client/services/{}/usage", service_id))
            .header(auth_header(&client_api_key).0, auth_header(&client_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    
    assert_eq!(json["id"].as_str(), Some(service_id.as_str()));
    assert_eq!(json["name"].as_str(), Some("Public Service"));
    assert_eq!(json["usage"].as_str(), Some("Test service usage"));
    
    app.cleanup().await;
}

#[tokio::test]
async fn test_get_restricted_service_usage_forbidden() {
    let app = TestApp::new().await;
    
    let (_, client_api_key, _) = create_test_user(app.pool(), "client1", UserRole::Client).await;
    let (service_id, _, _) = create_test_service(app.pool(), "restricted_svc", "Restricted Service", false).await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri(&format!("/api/v1/client/services/{}/usage", service_id))
            .header(auth_header(&client_api_key).0, auth_header(&client_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    
    app.cleanup().await;
}

#[tokio::test]
async fn test_get_restricted_service_usage_with_permission_success() {
    let app = TestApp::new().await;
    
    let (user_id, client_api_key, _) = create_test_user(app.pool(), "clientuser", UserRole::Client).await;
    let (service_id, _, _) = create_test_service(app.pool(), "restricted_svc2", "Restricted Service 2", false).await;
    grant_service_permission(app.pool(), &user_id, &service_id).await;
    
    let response = app
        .router
        .clone()
        .oneshot(Request::builder()
            .method("GET")
            .uri(&format!("/api/v1/client/services/{}/usage", service_id))
            .header(auth_header(&client_api_key).0, auth_header(&client_api_key).1)
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    
    assert_eq!(json["id"].as_str(), Some(service_id.as_str()));
    assert_eq!(json["name"].as_str(), Some("Restricted Service 2"));
    assert_eq!(json["usage"].as_str(), Some("Test service usage"));
    
    app.cleanup().await;
}
