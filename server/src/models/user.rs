//! 用户模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 用户角色
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// 客户端用户
    Client,
    /// 平台管理员
    Admin,
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::Client
    }
}

impl ToString for UserRole {
    fn to_string(&self) -> String {
        match self {
            UserRole::Client => "client".to_string(),
            UserRole::Admin => "admin".to_string(),
        }
    }
}

/// 用户模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    /// 用户ID
    pub id: String,
    /// API密钥 (用于认证)
    pub api_key: String,
    /// 用户名称
    pub name: String,
    /// 用户角色
    pub role: UserRole,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

impl User {
    /// 创建新用户
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().simple().to_string(),
            api_key: format!("ak_{}", Uuid::new_v4().simple()),
            name: name.into(),
            role: UserRole::Client,
            created_at: now,
        }
    }

    /// 设置API密钥
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = api_key.into();
        self
    }

    /// 设置角色
    pub fn with_role(mut self, role: UserRole) -> Self {
        self.role = role;
        self
    }

    /// 检查是否是管理员
    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }

    /// 检查是否是客户端用户
    pub fn is_client(&self) -> bool {
        self.role == UserRole::Client
    }
}

/// 创建用户请求
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub role: Option<UserRole>,
}

/// 更新用户角色请求
#[derive(Debug, Deserialize)]
pub struct UpdateUserRoleRequest {
    pub role: UserRole,
}

/// 用户响应
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub name: String,
    pub api_key: String,
    pub role: String,
    pub created_at: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            api_key: String::new(),
            role: user.role.to_string(),
            created_at: user.created_at.to_rfc3339(),
        }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== UserRole 枚举测试 ====================

    #[test]
    fn test_user_role_default() {
        let role: UserRole = Default::default();
        assert_eq!(role, UserRole::Client);
    }

    #[test]
    fn test_user_role_to_string() {
        assert_eq!(UserRole::Client.to_string(), "client");
        assert_eq!(UserRole::Admin.to_string(), "admin");
    }

    #[test]
    fn test_user_role_clone_and_eq() {
        let role = UserRole::Admin;
        let cloned = role.clone();
        assert_eq!(role, cloned);
        assert_eq!(role, UserRole::Admin);
        assert_ne!(role, UserRole::Client);
    }

    #[test]
    fn test_user_role_serialization() {
        let role = UserRole::Admin;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"admin\"");

        let deserialized: UserRole = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, UserRole::Admin);
    }

    #[test]
    fn test_user_role_deserialization() {
        let json = "\"client\"";
        let role: UserRole = serde_json::from_str(json).unwrap();
        assert_eq!(role, UserRole::Client);

        let json = "\"admin\"";
        let role: UserRole = serde_json::from_str(json).unwrap();
        assert_eq!(role, UserRole::Admin);
    }

    // ==================== User 结构体测试 ====================

    #[test]
    fn test_user_new() {
        let user = User::new("Test User");

        assert!(!user.id.is_empty());
        assert!(user.api_key.starts_with("ak_"));
        assert_eq!(user.name, "Test User");
        assert_eq!(user.role, UserRole::Client);
        // 验证 ID 是有效的 UUID 格式（去除简单格式后的）
        assert_eq!(user.id.len(), 32); // UUID simple format is 32 chars
    }

    #[test]
    fn test_user_new_with_string() {
        let name = String::from("Another User");
        let user = User::new(name);
        assert_eq!(user.name, "Another User");
    }

    #[test]
    fn test_user_with_api_key() {
        let user = User::new("Test User").with_api_key("custom_api_key");

        assert_eq!(user.api_key, "custom_api_key");
    }

    #[test]
    fn test_user_with_role_admin() {
        let user = User::new("Admin User").with_role(UserRole::Admin);

        assert_eq!(user.role, UserRole::Admin);
        assert!(user.is_admin());
        assert!(!user.is_client());
    }

    #[test]
    fn test_user_with_role_client() {
        let user = User::new("Client User").with_role(UserRole::Client);

        assert_eq!(user.role, UserRole::Client);
        assert!(!user.is_admin());
        assert!(user.is_client());
    }

    #[test]
    fn test_user_is_admin() {
        let admin = User::new("Admin").with_role(UserRole::Admin);
        let client = User::new("Client").with_role(UserRole::Client);
        let default_user = User::new("Default");

        assert!(admin.is_admin());
        assert!(!client.is_admin());
        assert!(!default_user.is_admin());
    }

    #[test]
    fn test_user_is_client() {
        let admin = User::new("Admin").with_role(UserRole::Admin);
        let client = User::new("Client").with_role(UserRole::Client);
        let default_user = User::new("Default");

        assert!(!admin.is_client());
        assert!(client.is_client());
        assert!(default_user.is_client());
    }

    #[test]
    fn test_user_chaining_methods() {
        let user = User::new("Chained User")
            .with_api_key("chained_key")
            .with_role(UserRole::Admin);

        assert_eq!(user.name, "Chained User");
        assert_eq!(user.api_key, "chained_key");
        assert_eq!(user.role, UserRole::Admin);
    }

    #[test]
    fn test_user_clone() {
        let user = User::new("Original")
            .with_api_key("original_key")
            .with_role(UserRole::Admin);

        let cloned = user.clone();

        assert_eq!(cloned.id, user.id);
        assert_eq!(cloned.name, user.name);
        assert_eq!(cloned.api_key, user.api_key);
        assert_eq!(cloned.role, user.role);
        assert_eq!(cloned.created_at, user.created_at);
    }

    #[test]
    fn test_user_serialization() {
        let user = User::new("Serialize Test")
            .with_api_key("serialize_key")
            .with_role(UserRole::Admin);

        let json = serde_json::to_string(&user).unwrap();

        assert!(json.contains("Serialize Test"));
        assert!(json.contains("serialize_key"));
        assert!(json.contains("admin"));

        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, user.id);
        assert_eq!(deserialized.name, user.name);
        assert_eq!(deserialized.api_key, user.api_key);
        assert_eq!(deserialized.role, user.role);
    }

    #[test]
    fn test_user_unique_ids() {
        // 确保生成的用户 ID 是唯一的
        let user1 = User::new("User 1");
        let user2 = User::new("User 2");

        assert_ne!(user1.id, user2.id);
        assert_ne!(user1.api_key, user2.api_key);
    }

    // ==================== CreateUserRequest 测试 ====================

    #[test]
    fn test_create_user_request_full() {
        let json = r#"{
            "name": "New User",
            "role": "admin"
        }"#;

        let request: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "New User");
        assert_eq!(request.role, Some(UserRole::Admin));
    }

    #[test]
    fn test_create_user_request_without_role() {
        let json = r#"{"name": "New User"}"#;

        let request: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "New User");
        assert!(request.role.is_none());
    }

    #[test]
    fn test_create_user_request_client_role() {
        let json = r#"{
            "name": "New User",
            "role": "client"
        }"#;

        let request: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.role, Some(UserRole::Client));
    }

    // ==================== UserResponse 测试 ====================

    #[test]
    fn test_user_response_from_user() {
        let user = User::new("Test User")
            .with_api_key("test_key")
            .with_role(UserRole::Admin);

        let response: UserResponse = user.clone().into();

        assert_eq!(response.id, user.id);
        assert_eq!(response.name, user.name);
        assert_eq!(response.api_key, "");
        assert_eq!(response.role, "admin");
        assert_eq!(response.created_at, user.created_at.to_rfc3339());
    }

    #[test]
    fn test_user_response_from_user_client() {
        let user = User::new("Client User").with_api_key("client_key");

        let response: UserResponse = user.into();

        assert_eq!(response.api_key, "");
        assert_eq!(response.role, "client");
    }

    #[test]
    fn test_user_response_serialization() {
        let user = User::new("Test")
            .with_api_key("key")
            .with_role(UserRole::Admin);
        let response: UserResponse = user.into();

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("key"));
        assert!(json.contains("admin"));
    }

    // ==================== 数据库集成测试 ====================

    use crate::test_utils::setup_test_db;

    #[sqlx::test]
    async fn test_user_insert_and_fetch() {
        let pool = setup_test_db().await;
        let user = User::new("DB Test User").with_api_key("db_test_key");

        // 插入用户
        sqlx::query(
            "INSERT INTO users (id, api_key, name, role, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&user.id)
        .bind(&user.api_key)
        .bind(&user.name)
        .bind("client")
        .bind(user.created_at)
        .execute(&pool)
        .await
        .unwrap();

        // 查询用户
        let fetched: User = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(&user.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(fetched.id, user.id);
        assert_eq!(fetched.name, user.name);
        assert_eq!(fetched.api_key, user.api_key);
        assert_eq!(fetched.role, UserRole::Client);
    }

    #[sqlx::test]
    async fn test_user_fetch_by_api_key() {
        let pool = setup_test_db().await;
        let user = User::new("API Key User").with_api_key("lookup_key");

        // 插入用户
        sqlx::query(
            "INSERT INTO users (id, api_key, name, role, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&user.id)
        .bind(&user.api_key)
        .bind(&user.name)
        .bind("client")
        .bind(user.created_at)
        .execute(&pool)
        .await
        .unwrap();

        // 通过 api_key 查询
        let fetched: User = sqlx::query_as::<_, User>("SELECT * FROM users WHERE api_key = ?")
            .bind("lookup_key")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(fetched.name, "API Key User");
        assert_eq!(fetched.api_key, "lookup_key");
    }

    #[sqlx::test]
    async fn test_user_update_name() {
        let pool = setup_test_db().await;
        let user = User::new("Original Name").with_api_key("update_key");

        // 插入用户
        sqlx::query(
            "INSERT INTO users (id, api_key, name, role, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&user.id)
        .bind(&user.api_key)
        .bind(&user.name)
        .bind("client")
        .bind(user.created_at)
        .execute(&pool)
        .await
        .unwrap();

        // 更新名称
        sqlx::query("UPDATE users SET name = ? WHERE id = ?")
            .bind("Updated Name")
            .bind(&user.id)
            .execute(&pool)
            .await
            .unwrap();

        // 查询验证
        let fetched: User = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(&user.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(fetched.name, "Updated Name");
    }

    #[sqlx::test]
    async fn test_admin_user_in_db() {
        let pool = setup_test_db().await;
        let admin = User::new("Admin User")
            .with_api_key("admin_key")
            .with_role(UserRole::Admin);

        // 插入管理员用户
        sqlx::query(
            "INSERT INTO users (id, api_key, name, role, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&admin.id)
        .bind(&admin.api_key)
        .bind(&admin.name)
        .bind("admin")
        .bind(admin.created_at)
        .execute(&pool)
        .await
        .unwrap();

        // 查询验证
        let fetched: User = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(&admin.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(fetched.role, UserRole::Admin);
        assert!(fetched.is_admin());
    }

    #[sqlx::test]
    async fn test_user_count() {
        let pool = setup_test_db().await;

        // 插入多个用户（迁移中已经有一个 admin 用户）
        for i in 0..5 {
            let user = User::new(format!("User {}", i)).with_api_key(format!("key_{}", i));
            sqlx::query(
                "INSERT INTO users (id, api_key, name, role, created_at) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&user.id)
            .bind(&user.api_key)
            .bind(&user.name)
            .bind("client")
            .bind(user.created_at)
            .execute(&pool)
            .await
            .unwrap();
        }

        // 查询用户数量（包括迁移中的 admin 用户）
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(count.0, 6); // 5 个新用户 + 1 个迁移中的 admin 用户
    }
}
