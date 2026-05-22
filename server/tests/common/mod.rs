//! 集成测试工具模块

#![allow(dead_code)]

use axum::Router;
use open_aaas_server::{
    auth::hash_api_key,
    config::AppConfig,
    db::Database,
    handlers,
    state::AppState,
};
use sqlx::SqlitePool;
use std::sync::Arc;

/// 测试用 Secret Key
pub const TEST_SECRET_KEY: &str = "test-secret-key-for-integration-tests";

/// 创建测试应用状态和路由
pub async fn create_test_app() -> (Router, AppState, SqlitePool) {
    // 创建内存数据库配置（带 secret_key）
    let config = AppConfig {
        secret_key: Some(TEST_SECRET_KEY.to_string()),
        ..AppConfig::default()
    };
    
    // 创建内存数据库连接
    let db = Database::new("sqlite::memory:").await.expect("Failed to create in-memory database");
    let pool = db.pool().clone();
    
    // 初始化数据库表
    db.init_tables().await.expect("Failed to initialize tables");
    
    // 创建默认 admin 用户（供测试使用，使用 hash_api_key）
    let admin_api_key = "ak_admin_default_key";
    let hashed_admin_key = hash_api_key(TEST_SECRET_KEY, admin_api_key);
    sqlx::query("INSERT OR IGNORE INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)")
        .bind("admin")
        .bind(&hashed_admin_key)
        .bind("Administrator")
        .bind("admin")
        .execute(&pool)
        .await
        .expect("Failed to create admin user");
    
    // 创建应用状态
    let state = AppState {
        config: Arc::new(config),
        db,
    };
    
    // 创建路由
    let app = handlers::routes(state.clone());
    let app = app.with_state(state.clone());
    
    (app, state, pool)
}

/// 创建测试服务（返回 service_id 和 registration_token）
pub async fn create_test_service(pool: &SqlitePool) -> (String, String) {
    let service_id = format!("test-service-{}", uuid::Uuid::new_v4());
    let registration_token = format!("rt_{}", uuid::Uuid::new_v4());
    
    sqlx::query(
        r#"
        INSERT INTO services (id, name, description, usage, registration_token, registration_status, is_public)
        VALUES (?, ?, ?, ?, ?, 'pending', true)
        "#
    )
    .bind(&service_id)
    .bind("Test Service")
    .bind("A test service")
    .bind("Test usage")
    .bind(&registration_token)
    .execute(pool)
    .await
    .expect("Failed to create test service");
    
    (service_id, registration_token)
}

/// 创建已注册的服务（Agent 已注册）
pub async fn create_registered_service(pool: &SqlitePool) -> (String, String, String) {
    let service_id = format!("test-service-{}", uuid::Uuid::new_v4());
    let registration_token = format!("rt_{}", uuid::Uuid::new_v4());
    let api_key = format!("ak_agent_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
    // 存储 hash 后的 api_key
    let hashed_api_key = hash_api_key(TEST_SECRET_KEY, &api_key);
    
    sqlx::query(
        r#"
        INSERT INTO services (id, name, description, usage, agent_api_key, registration_status, 
                              agent_status, agent_capacity, agent_current_load, is_public)
        VALUES (?, ?, ?, ?, ?, 'active', 'online', 1, 0, true)
        "#
    )
    .bind(&service_id)
    .bind("Test Service")
    .bind("A test service")
    .bind("Test usage")
    .bind(&hashed_api_key)
    .execute(pool)
    .await
    .expect("Failed to create registered service");
    
    (service_id, api_key, registration_token)
}

/// 创建测试任务
/// 使用默认 admin 用户 (id: 'admin')
pub async fn create_test_task(
    pool: &SqlitePool,
    service_id: &str,
    status: &str,
) -> String {
    let task_id = format!("task-{}", uuid::Uuid::new_v4());
    let session_id = format!("session-{}", uuid::Uuid::new_v4());
    let user_id = "admin"; // 使用 migrations 中创建的默认 admin 用户
    
    let input = serde_json::json!({
        "task_prompt": "Test task prompt",
        "output_prompt": "Test output prompt"
    });
    
    sqlx::query(
        r#"
        INSERT INTO tasks (id, user_id, service_id, status, input, session_id, created_at)
        VALUES (?, ?, ?, ?, ?, ?, datetime('now'))
        "#
    )
    .bind(&task_id)
    .bind(user_id)
    .bind(service_id)
    .bind(status)
    .bind(input)
    .bind(session_id)
    .execute(pool)
    .await
    .expect("Failed to create test task");
    
    task_id
}

/// 创建测试用户
pub async fn create_test_user(pool: &SqlitePool, role: &str) -> (String, String) {
    let user_id = format!("user-{}", uuid::Uuid::new_v4());
    let api_key = format!("ak_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
    // 存储 hash 后的 api_key
    let hashed_api_key = hash_api_key(TEST_SECRET_KEY, &api_key);
    
    sqlx::query(
        "INSERT INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)"
    )
    .bind(&user_id)
    .bind(&hashed_api_key)
    .bind("Test User")
    .bind(role)
    .execute(pool)
    .await
    .expect("Failed to create test user");
    
    (user_id, api_key)
}


