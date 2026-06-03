//! 文件 API 集成测试
//!
//! 测试文件上传、下载、列表等功能

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use open_aaas_server::models::user::UserRole;

use super::{TestApp, auth_header, create_test_service, create_test_task, create_test_user};
use tower::ServiceExt;

// ============================================================================
// 辅助函数
// ============================================================================

// 辅助函数：创建测试文件记录并写入文件系统

/// 创建测试文件记录并写入文件系统
async fn create_test_file(app: &TestApp, task_id: &str, filename: &str, content: &[u8]) -> String {
    use open_aaas_server::models::file::{FileCreatedBy, TaskFile};

    let file_id = uuid::Uuid::new_v4().to_string();
    let storage_path = format!("{}/{}", task_id, file_id);

    // 创建文件记录
    let file = TaskFile::new(
        task_id,
        filename,
        Some("text/plain".to_string()),
        content.len() as i64,
        &storage_path,
        FileCreatedBy::Agent,
    );

    // 插入数据库
    sqlx::query(
        r#"
        INSERT INTO task_files (id, task_id, filename, mime_type, size_bytes, storage_path, created_by, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&file.id)
    .bind(&file.task_id)
    .bind(&file.filename)
    .bind(&file.mime_type)
    .bind(file.size_bytes)
    .bind(&file.storage_path)
    .bind(&file.created_by.to_string())
    .bind(&file.created_at.to_rfc3339())
    .execute(app.pool())
    .await
    .expect("Failed to create test file record");

    // 写入实际文件
    let full_path =
        std::path::PathBuf::from(&app.config.task.file_storage_path).join(&storage_path);
    if let Some(parent) = full_path.parent() {
        tokio::fs::create_dir_all(parent).await.unwrap();
    }
    tokio::fs::write(&full_path, content).await.unwrap();

    file.id
}

// ============================================================================
// Agent 文件上传测试
// ============================================================================

#[tokio::test]
async fn test_agent_upload_file_success() {
    let app = TestApp::new().await;

    // 创建测试服务和任务
    let (service_id, api_key, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let task_id = create_test_task(app.pool(), "admin", &service_id, "pending").await;

    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!(
        "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"task_id\"\r\n\r\n\
        {}\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
        Content-Type: text/plain\r\n\r\n\
        Hello, World!\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n",
        task_id
    );

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/v1/files/agent/{}/files/upload", service_id))
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &api_key)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(json["file_id"].as_str().is_some());
    assert_eq!(json["filename"].as_str(), Some("test.txt"));
    assert_eq!(json["size_bytes"].as_i64(), Some(13)); // "Hello, World!" length

    app.cleanup().await;
}

#[tokio::test]
async fn test_agent_upload_file_wrong_service_id() {
    let app = TestApp::new().await;

    // 创建测试服务和任务
    let (service_id, api_key, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let (other_service_id, _, _) =
        create_test_service(app.pool(), "other_service", "Other Service", true).await;
    let task_id = create_test_task(app.pool(), "admin", &other_service_id, "pending").await;

    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let _content = b"Hello, World!";
    let body = format!(
        "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"task_id\"\r\n\r\n\
        {}\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
        Content-Type: text/plain\r\n\r\n\
        Hello, World!\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n",
        task_id
    );

    // 使用 service_id 认证但上传到其他服务的任务
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/v1/files/agent/{}/files/upload", service_id))
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &api_key)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // 应该返回 Forbidden，因为任务不属于该服务
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    app.cleanup().await;
}

#[tokio::test]
async fn test_agent_upload_file_missing_task_id() {
    let app = TestApp::new().await;

    let (service_id, api_key, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;

    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!(
        "------{}\r\n\
        Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
        Content-Type: text/plain\r\n\r\n\
        Hello\r\n\
        ------{}--\r\n",
        boundary, boundary
    );

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/v1/files/agent/{}/files/upload", service_id))
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &api_key)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    app.cleanup().await;
}

#[tokio::test]
async fn test_agent_upload_file_invalid_auth() {
    let app = TestApp::new().await;

    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;

    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!(
        "------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"task_id\"\r\n\r\n\
        task-123\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
        Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
        Content-Type: text/plain\r\n\r\n\
        Hello\r\n\
        ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n"
    );

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/v1/files/agent/{}/files/upload", service_id))
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", "invalid_api_key")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    app.cleanup().await;
}

#[tokio::test]
async fn test_agent_upload_file_size_limit() {
    let app = TestApp::new().await;

    // 创建测试服务和任务
    let (service_id, api_key, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let task_id = create_test_task(app.pool(), "admin", &service_id, "pending").await;

    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    // 创建一个超过 10MB 限制的内容（使用不同的字符避免 boundary 混淆）
    let large_content: Vec<u8> = (0..11 * 1024 * 1024)
        .map(|i| (i % 200 + 32) as u8)
        .collect();

    // 构建 multipart 请求体
    let mut body = Vec::new();
    body.extend_from_slice(format!("------{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"task_id\"\r\n\r\n");
    body.extend_from_slice(task_id.as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("------{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"large.bin\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(&large_content);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("------{}--\r\n", boundary).as_bytes());

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/v1/files/agent/{}/files/upload", service_id))
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &api_key)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    app.cleanup().await;
}

// ============================================================================
// Client 文件上传测试（通过创建任务时的文件上传）
// 注意：此测试与 client_api.rs 中的 test_create_task_success_public_service 类似
// ============================================================================

// ============================================================================
// 文件下载测试
// ============================================================================

#[tokio::test]
async fn test_agent_download_file_success() {
    let app = TestApp::new().await;

    let (service_id, api_key, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let task_id = create_test_task(app.pool(), "admin", &service_id, "pending").await;
    let file_content = b"Downloadable file content";
    let file_id = create_test_file(&app, &task_id, "download.txt", file_content).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!(
                    "/api/v1/files/agent/{}/files/{}/download",
                    service_id, file_id
                ))
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // 验证 Content-Disposition header
    let content_disposition = response.headers().get("content-disposition");
    assert!(content_disposition.is_some());

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body_bytes.as_ref(), file_content.as_slice());

    app.cleanup().await;
}

#[tokio::test]
async fn test_agent_download_file_not_found() {
    let app = TestApp::new().await;

    let (service_id, api_key, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!(
                    "/api/v1/files/agent/{}/files/non-existent-file/download",
                    service_id
                ))
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    app.cleanup().await;
}

#[tokio::test]
async fn test_agent_download_file_wrong_service() {
    let app = TestApp::new().await;

    let (service_id, api_key, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let (other_service_id, _, _) =
        create_test_service(app.pool(), "other_service", "Other Service", true).await;
    let task_id = create_test_task(app.pool(), "admin", &other_service_id, "pending").await;
    let file_id = create_test_file(&app, &task_id, "test.txt", b"content").await;

    // 尝试用 service_id 访问 other_service 的文件
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!(
                    "/api/v1/files/agent/{}/files/{}/download",
                    service_id, file_id
                ))
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    app.cleanup().await;
}

#[tokio::test]
async fn test_client_download_file_success() {
    let app = TestApp::new().await;

    let (user_id, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let task_id = create_test_task(app.pool(), &user_id, &service_id, "pending").await;
    let file_content = b"Client downloadable content";
    let file_id = create_test_file(&app, &task_id, "client_file.txt", file_content).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/client/files/{}/download", file_id))
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body_bytes.as_ref(), file_content.as_slice());

    app.cleanup().await;
}

#[tokio::test]
async fn test_client_download_file_forbidden() {
    let app = TestApp::new().await;

    let (user1_id, _, _) = create_test_user(app.pool(), "user1", UserRole::Client).await;
    let (_, api_key2, _) = create_test_user(app.pool(), "user2", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let task_id = create_test_task(app.pool(), &user1_id, &service_id, "pending").await;
    let file_id = create_test_file(&app, &task_id, "private.txt", b"private content").await;

    // user2 尝试下载 user1 的文件
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/client/files/{}/download", file_id))
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
// 文件信息获取测试
// ============================================================================

#[tokio::test]
async fn test_agent_get_file_info_success() {
    let app = TestApp::new().await;

    let (service_id, api_key, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let task_id = create_test_task(app.pool(), "admin", &service_id, "pending").await;
    let file_id = create_test_file(&app, &task_id, "info.txt", b"content for info").await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!(
                    "/api/v1/files/agent/{}/files/{}",
                    service_id, file_id
                ))
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["id"].as_str(), Some(file_id.as_str()));
    assert_eq!(json["filename"].as_str(), Some("info.txt"));
    assert_eq!(json["task_id"].as_str(), Some(task_id.as_str()));

    app.cleanup().await;
}

#[tokio::test]
async fn test_client_get_file_info_success() {
    let app = TestApp::new().await;

    let (user_id, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let task_id = create_test_task(app.pool(), &user_id, &service_id, "pending").await;
    let file_id = create_test_file(&app, &task_id, "client_info.txt", b"content").await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/client/files/{}", file_id))
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["id"].as_str(), Some(file_id.as_str()));

    app.cleanup().await;
}

// ============================================================================
// 文件列表测试
// ============================================================================

#[tokio::test]
async fn test_agent_list_task_files() {
    let app = TestApp::new().await;

    let (service_id, api_key, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let task_id = create_test_task(app.pool(), "admin", &service_id, "pending").await;

    // 创建多个文件
    create_test_file(&app, &task_id, "file1.txt", b"content1").await;
    create_test_file(&app, &task_id, "file2.txt", b"content2").await;
    create_test_file(&app, &task_id, "file3.txt", b"content3").await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!(
                    "/api/v1/files/agent/{}/files/list/{}",
                    service_id, task_id
                ))
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let files = json["files"].as_array().unwrap();
    assert_eq!(files.len(), 3);

    app.cleanup().await;
}

#[tokio::test]
async fn test_agent_list_task_files_wrong_service() {
    let app = TestApp::new().await;

    let (service_id, api_key, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let (other_service_id, _, _) =
        create_test_service(app.pool(), "other_service", "Other Service", true).await;
    let task_id = create_test_task(app.pool(), "admin", &other_service_id, "pending").await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!(
                    "/api/v1/files/agent/{}/files/list/{}",
                    service_id, task_id
                ))
                .header("X-Service-ID", &service_id)
                .header("X-API-Key", &api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    app.cleanup().await;
}

#[tokio::test]
async fn test_client_list_task_files() {
    let app = TestApp::new().await;

    let (user_id, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;
    let (service_id, _, _) =
        create_test_service(app.pool(), "test_service", "Test Service", true).await;
    let task_id = create_test_task(app.pool(), &user_id, &service_id, "pending").await;

    // 创建多个文件
    create_test_file(&app, &task_id, "client_file1.txt", b"content1").await;
    create_test_file(&app, &task_id, "client_file2.txt", b"content2").await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/client/files/list/{}", task_id))
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let files = json["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);

    app.cleanup().await;
}

#[tokio::test]
async fn test_client_list_task_files_forbidden() {
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
                .uri(&format!("/api/v1/client/files/list/{}", task_id))
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
// 路径遍历防护测试
// 注意：路径遍历防护已在 src/models/file.rs 的单元测试中覆盖
// ============================================================================

// ============================================================================
// 流式上传处理测试
// 注意：流式上传的核心功能已通过 test_agent_upload_file_success 测试覆盖
// ============================================================================

// ============================================================================
// 文件不存在测试
// ============================================================================

#[tokio::test]
async fn test_client_get_file_info_not_found() {
    let app = TestApp::new().await;

    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/client/files/non-existent-file-id")
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
async fn test_client_download_file_not_found() {
    let app = TestApp::new().await;

    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/client/files/non-existent-file-id/download")
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
async fn test_client_list_files_task_not_found() {
    let app = TestApp::new().await;

    let (_, api_key, _) = create_test_user(app.pool(), "testuser", UserRole::Client).await;

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/client/files/list/non-existent-task-id")
                .header(auth_header(&api_key).0, auth_header(&api_key).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    app.cleanup().await;
}
