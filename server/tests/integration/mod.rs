//! 集成测试共享工具模块

pub mod admin_api;
pub mod client_api;
pub mod files_api;
pub mod services_api;

use axum::Router;
use open_aaas_server::{
    auth::hash_api_key,
    config::AppConfig, db::Database, handlers, models::user::UserRole, state::AppState,
};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

pub const TEST_SECRET_KEY: &str = "test-secret-key-for-integration-tests";

pub struct TestApp {
    pub router: Router,
    pub db_pool: SqlitePool,
    pub config: Arc<AppConfig>,
}

impl TestApp {
    pub async fn new() -> Self {
        let db_file = std::env::temp_dir()
            .join(format!("test_open_aaas_{}.db", Uuid::new_v4().simple()))
            .to_str()
            .expect("temp dir path should be valid UTF-8")
            .replace('\\', "/");
        let database_url = format!("sqlite:///{}?mode=rwc", db_file);

        let config = AppConfig {
            secret_key: Some(TEST_SECRET_KEY.to_string()),
            database: open_aaas_server::config::DatabaseConfig {
                url: database_url.clone(),
            },
            task: open_aaas_server::config::TaskConfig {
                file_storage_path: std::env::temp_dir()
                    .join(format!("test_files_{}", Uuid::new_v4().simple()))
                    .to_str()
                    .expect("temp dir path should be valid UTF-8")
                    .to_string(),
                max_file_size_mb: 10,
                ..Default::default()
            },
            ..Default::default()
        };

        let db = Database::new(&database_url).await.expect("Failed to create database");
        db.init_tables().await.expect("Failed to initialize tables");

        // 创建默认 admin 用户（供测试使用，存储 hash）
        let hashed_admin_key = hash_api_key(TEST_SECRET_KEY, "ak_admin_default_key");
        sqlx::query("INSERT OR IGNORE INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)")
            .bind("admin")
            .bind(&hashed_admin_key)
            .bind("Administrator")
            .bind("admin")
            .execute(db.pool())
            .await
            .expect("Failed to create admin user");

        let state = AppState {
            config: Arc::new(config.clone()),
            db: db.clone(),
        };

        let router = handlers::routes(state.clone()).with_state(state.clone());

        Self {
            router,
            db_pool: db.pool().clone(),
            config: Arc::new(config),
        }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.db_pool
    }

    pub async fn cleanup(&self) {
        self.db_pool.close().await;
        let db_url = &self.config.database.url;
        let db_file = db_url.strip_prefix("sqlite:").unwrap_or(db_url);
        let db_file = db_file.strip_prefix("///").unwrap_or(db_file);
        let db_file = db_file.split('?').next().unwrap_or(db_file);
        // Remove WAL/SHM first, then main db (defensive ordering)
        if let Err(e) = tokio::fs::remove_file(format!("{}-wal", db_file)).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("Warning: failed to remove test db wal file {}-wal: {}", db_file, e);
            }
        }
        if let Err(e) = tokio::fs::remove_file(format!("{}-shm", db_file)).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("Warning: failed to remove test db shm file {}-shm: {}", db_file, e);
            }
        }
        if let Err(e) = tokio::fs::remove_file(&db_file).await {
            eprintln!("Warning: failed to remove test db file {}: {}", db_file, e);
        }
        if let Err(e) = tokio::fs::remove_dir_all(&self.config.task.file_storage_path).await {
            eprintln!("Warning: failed to remove test files dir {}: {}", self.config.task.file_storage_path, e);
        }
    }
}

pub async fn create_test_user(pool: &SqlitePool, name: &str, role: UserRole) -> (String, String, String) {
    let user_id = Uuid::new_v4().to_string();
    let api_key = format!("ak_client_{}", Uuid::new_v4().to_string().replace("-", ""));
    let hashed_api_key = hash_api_key(TEST_SECRET_KEY, &api_key);
    let now = chrono::Utc::now();

    sqlx::query(
        "INSERT INTO users (id, api_key, name, role, created_at) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&user_id)
    .bind(&hashed_api_key)
    .bind(name)
    .bind(&role.to_string())
    .bind(now.to_rfc3339())
    .execute(pool)
    .await
    .expect("Failed to create test user");

    (user_id, api_key, name.to_string())
}

pub async fn create_test_service(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    is_public: bool,
) -> (String, String, String) {
    let agent_api_key = format!("ak_agent_{}", Uuid::new_v4().to_string().replace("-", ""));
    let hashed_agent_api_key = hash_api_key(TEST_SECRET_KEY, &agent_api_key);
    let registration_token = format!("rt_{}", Uuid::new_v4().to_string().replace("-", ""));
    let now = chrono::Utc::now();

    sqlx::query(
        r#"INSERT INTO services (
            id, name, description, usage, agent_api_key, agent_status, 
            agent_capacity, agent_current_load, agent_last_heartbeat, 
            registration_token, registration_status, is_public, created_at
        ) VALUES (?, ?, ?, ?, ?, 'offline', 10, 0, NULL, ?, 'active', ?, ?)"#
    )
    .bind(id)
    .bind(name)
    .bind("Test service description")
    .bind("Test service usage")
    .bind(&hashed_agent_api_key)
    .bind(&registration_token)
    .bind(is_public)
    .bind(now.to_rfc3339())
    .execute(pool)
    .await
    .expect("Failed to create test service");

    (id.to_string(), agent_api_key, registration_token)
}

pub async fn create_test_task(
    pool: &SqlitePool,
    user_id: &str,
    service_id: &str,
    status: &str,
) -> String {
    let task_id = Uuid::new_v4().to_string();
    let session_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    let input = serde_json::json!({
        "task_prompt": "Test task prompt",
        "output_prompt": "Test output prompt",
        "input_files": [],
    });

    sqlx::query(
        r#"INSERT INTO tasks (
            id, user_id, service_id, status, input, output, error_message,
            retry_count, session_id, created_at, assigned_at, started_at, completed_at
        ) VALUES (?, ?, ?, ?, ?, NULL, NULL, 0, ?, ?, NULL, NULL, NULL)"#
    )
    .bind(&task_id)
    .bind(user_id)
    .bind(service_id)
    .bind(status)
    .bind(&input.to_string())
    .bind(&session_id)
    .bind(now.to_rfc3339())
    .execute(pool)
    .await
    .expect("Failed to create test task");

    task_id
}

pub async fn grant_service_permission(pool: &SqlitePool, user_id: &str, service_id: &str) {
    let permission_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    sqlx::query(
        r#"INSERT INTO user_service_permissions (id, user_id, service_id, granted_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(user_id, service_id) DO UPDATE SET granted_at = excluded.granted_at"#
    )
    .bind(&permission_id)
    .bind(user_id)
    .bind(service_id)
    .bind(now.to_rfc3339())
    .execute(pool)
    .await
    .expect("Failed to grant service permission");
}

pub fn auth_header(api_key: &str) -> (&str, String) {
    ("Authorization", format!("Bearer {}", api_key))
}

#[derive(Debug, serde::Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub name: String,
    pub api_key: String,
    pub role: String,
    pub created_at: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct TaskResponse {
    pub id: String,
    pub service_id: String,
    pub status: String,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub session_id: String,
    pub retry_count: i64,
    pub created_at: String,
    pub assigned_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ServiceListItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub agent_status: String,
    pub registration_status: String,
    pub agent_last_heartbeat: Option<String>,
    pub access_type: String,
    pub has_permission: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct ServiceStatusResponse {
    pub healthy: bool,
    pub version: String,
    pub timestamp: String,
    pub components: serde_json::Value,
    pub stats: serde_json::Value,
    pub alerts: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct GrantPermissionResponse {
    pub granted: bool,
    pub user_id: String,
    pub service_id: String,
    pub service_name: String,
}
