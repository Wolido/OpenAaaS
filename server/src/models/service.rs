//! Service 模型（服务与Agent一对一）

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum AgentStatus {
    Online,
    #[default]
    Offline,
    Busy,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Online => write!(f, "online"),
            AgentStatus::Offline => write!(f, "offline"),
            AgentStatus::Busy => write!(f, "busy"),
        }
    }
}

/// 注册状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum RegistrationStatus {
    #[default]
    Pending, // 待注册（只有registration_token，没有agent_api_key）
    Active,  // 已注册（有agent_api_key）
    Revoked, // 已吊销
}

impl std::fmt::Display for RegistrationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistrationStatus::Pending => write!(f, "pending"),
            RegistrationStatus::Active => write!(f, "active"),
            RegistrationStatus::Revoked => write!(f, "revoked"),
        }
    }
}

/// Service 模型（包含唯一的Agent信息）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Service {
    pub id: String, // 服务ID，同时也是Agent标识
    pub name: String,
    pub description: String,
    pub usage: String, // 服务使用说明/用法

    pub agent_api_key: Option<String>, // Agent认证密钥
    pub agent_status: AgentStatus,
    pub agent_capacity: i64,
    pub agent_current_load: i64,
    pub agent_last_heartbeat: Option<DateTime<Utc>>,
    pub registration_token: Option<String>, // 注册令牌（一次性使用）
    pub registration_status: String,        // pending/active/revoked
    pub is_public: bool,                    // 是否为公开服务
    pub created_at: DateTime<Utc>,
}

/// 服务列表项
#[derive(Debug, Serialize)]
pub struct ServiceListItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub agent_status: AgentStatus,
    pub registration_status: String,
    pub agent_last_heartbeat: Option<DateTime<Utc>>,
    pub access_type: String,  // "public" 或 "restricted"
    pub has_permission: bool, // 当前用户是否有权限
}

/// 创建服务请求（admin）
#[derive(Debug, Deserialize)]
pub struct CreateServiceRequest {
    pub id: Option<String>,
    pub name: String,
    pub description: String, // 必需
    pub usage: String,       // 必需
    #[serde(default = "default_true")] // 默认 true
    pub is_public: bool,
}

fn default_true() -> bool {
    true
}

/// 更新服务请求（admin）
#[derive(Debug, Deserialize)]
pub struct UpdateServiceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub usage: Option<String>,
    pub is_public: Option<bool>,
}

/// 创建服务响应（admin）- 包含 registration_token
#[derive(Debug, Serialize)]
pub struct CreateServiceResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub usage: String,
    pub registration_status: String,
    pub registration_token: String, // 用于 Agent 注册的令牌
    pub created_at: DateTime<Utc>,
}

/// 服务响应
#[derive(Debug, Serialize)]
pub struct ServiceResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub usage: String,
    pub agent_status: AgentStatus,
    pub registration_status: String,
    pub agent_capacity: i64,
    pub agent_current_load: i64,
    pub agent_last_heartbeat: Option<DateTime<Utc>>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
}

impl From<Service> for ServiceResponse {
    fn from(service: Service) -> Self {
        Self {
            id: service.id,
            name: service.name,
            description: service.description,
            usage: service.usage,
            agent_status: service.agent_status,
            registration_status: service.registration_status,
            agent_capacity: service.agent_capacity,
            agent_current_load: service.agent_current_load,
            agent_last_heartbeat: service.agent_last_heartbeat,
            is_public: service.is_public,
            created_at: service.created_at,
        }
    }
}

/// 服务 usage 响应
#[derive(Debug, Serialize)]
pub struct ServiceUsageResponse {
    pub id: String,
    pub name: String,
    pub usage: String,
}

/// 用户服务权限（受限服务的授权记录）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserServicePermission {
    pub id: String,
    pub user_id: String,
    pub service_id: String,
    pub granted_at: DateTime<Utc>,
}

/// 用户服务权限响应（Admin API）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserPermissionResponse {
    pub service_id: String,
    pub service_name: String,
    pub granted_at: DateTime<Utc>,
}

/// 删除服务响应
#[derive(Debug, Serialize)]
pub struct DeleteServiceResponse {
    pub deleted: bool,
    pub tasks_cancelled: i64,
    pub tasks_retained: i64,
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== AgentStatus 枚举测试 ====================

    #[test]
    fn test_agent_status_default() {
        let status: AgentStatus = Default::default();
        assert_eq!(status, AgentStatus::Offline);
    }

    #[test]
    fn test_agent_status_to_string() {
        assert_eq!(AgentStatus::Online.to_string(), "online");
        assert_eq!(AgentStatus::Offline.to_string(), "offline");
        assert_eq!(AgentStatus::Busy.to_string(), "busy");
    }

    #[test]
    fn test_agent_status_clone_and_eq() {
        let status = AgentStatus::Online;
        let cloned = status;
        assert_eq!(status, cloned);
        assert_eq!(status, AgentStatus::Online);
        assert_ne!(status, AgentStatus::Offline);
    }

    #[test]
    fn test_agent_status_serialization() {
        let status = AgentStatus::Online;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"online\"");

        let deserialized: AgentStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, AgentStatus::Online);
    }

    // ==================== RegistrationStatus 枚举测试 ====================

    #[test]
    fn test_registration_status_default() {
        let status: RegistrationStatus = Default::default();
        assert_eq!(status, RegistrationStatus::Pending);
    }

    #[test]
    fn test_registration_status_to_string() {
        assert_eq!(RegistrationStatus::Pending.to_string(), "pending");
        assert_eq!(RegistrationStatus::Active.to_string(), "active");
        assert_eq!(RegistrationStatus::Revoked.to_string(), "revoked");
    }

    #[test]
    fn test_registration_status_clone_and_eq() {
        let status = RegistrationStatus::Active;
        let cloned = status;
        assert_eq!(status, cloned);
        assert_eq!(status, RegistrationStatus::Active);
        assert_ne!(status, RegistrationStatus::Pending);
    }

    #[test]
    fn test_registration_status_serialization() {
        let status = RegistrationStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"active\"");

        let deserialized: RegistrationStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, RegistrationStatus::Active);
    }

    // ==================== Service 结构体测试 ====================

    #[test]
    fn test_service_creation() {
        let now = Utc::now();
        let service = Service {
            id: "service_123".to_string(),
            name: "Test Service".to_string(),
            description: "A test service".to_string(),
            usage: "Service usage description".to_string(),
            agent_api_key: Some("ak_agent_key".to_string()),
            agent_status: AgentStatus::Online,
            agent_capacity: 5,
            agent_current_load: 2,
            agent_last_heartbeat: Some(now),
            registration_token: Some("reg_token_123".to_string()),
            registration_status: "active".to_string(),
            is_public: true,
            created_at: now,
        };

        assert_eq!(service.id, "service_123");
        assert_eq!(service.name, "Test Service");
        assert_eq!(service.description, "A test service");
        assert_eq!(service.usage, "Service usage description");
        assert_eq!(service.agent_api_key, Some("ak_agent_key".to_string()));
        assert_eq!(service.agent_status, AgentStatus::Online);
        assert_eq!(service.agent_capacity, 5);
        assert_eq!(service.agent_current_load, 2);
        assert!(service.agent_last_heartbeat.is_some());
        assert_eq!(
            service.registration_token,
            Some("reg_token_123".to_string())
        );
        assert_eq!(service.registration_status, "active");
        assert!(service.is_public);
    }

    #[test]
    fn test_service_clone() {
        let now = Utc::now();
        let service = Service {
            id: "service_123".to_string(),
            name: "Test Service".to_string(),
            description: "A test service".to_string(),
            usage: "Instructions".to_string(),
            agent_api_key: Some("ak_key".to_string()),
            agent_status: AgentStatus::Offline,
            agent_capacity: 1,
            agent_current_load: 0,
            agent_last_heartbeat: None,
            registration_token: None,
            registration_status: "pending".to_string(),
            is_public: false,
            created_at: now,
        };

        let cloned = service.clone();
        assert_eq!(cloned.id, service.id);
        assert_eq!(cloned.name, service.name);
        assert_eq!(cloned.agent_status, service.agent_status);
        assert_eq!(cloned.is_public, service.is_public);
    }

    #[test]
    fn test_service_serialization() {
        let now = Utc::now();
        let service = Service {
            id: "service_123".to_string(),
            name: "Test Service".to_string(),
            description: "Description".to_string(),
            usage: "Instructions".to_string(),
            agent_api_key: Some("ak_key".to_string()),
            agent_status: AgentStatus::Online,
            agent_capacity: 3,
            agent_current_load: 1,
            agent_last_heartbeat: Some(now),
            registration_token: Some("token".to_string()),
            registration_status: "active".to_string(),
            is_public: true,
            created_at: now,
        };

        let json = serde_json::to_string(&service).unwrap();
        assert!(json.contains("service_123"));
        assert!(json.contains("Test Service"));
        assert!(json.contains("online"));
        assert!(json.contains("active"));
    }

    // ==================== ServiceListItem 测试 ====================

    #[test]
    fn test_service_list_item_serialization() {
        let item = ServiceListItem {
            id: "service_123".to_string(),
            name: "Test Service".to_string(),
            description: "Description".to_string(),
            agent_status: AgentStatus::Online,
            registration_status: "active".to_string(),
            agent_last_heartbeat: None,
            access_type: "public".to_string(),
            has_permission: true,
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("service_123"));
        assert!(json.contains("Test Service"));
        assert!(json.contains("online"));
        assert!(json.contains("public"));
        assert!(json.contains("true")); // has_permission
    }

    // ==================== CreateServiceRequest 测试 ====================

    #[test]
    fn test_create_service_request_full() {
        let json = r#"{
            "id": "custom_id",
            "name": "New Service",
            "description": "Service description",
            "usage": "How to use",
            "is_public": false
        }"#;

        let request: CreateServiceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, Some("custom_id".to_string()));
        assert_eq!(request.name, "New Service");
        assert_eq!(request.description, "Service description");
        assert_eq!(request.usage, "How to use");
        assert!(!request.is_public);
    }

    #[test]
    fn test_create_service_request_default_is_public() {
        let json = r#"{
            "name": "New Service",
            "description": "Description",
            "usage": "Instructions"
        }"#;

        let request: CreateServiceRequest = serde_json::from_str(json).unwrap();
        assert!(request.is_public); // 默认为 true
    }

    #[test]
    fn test_default_true() {
        assert!(default_true());
    }

    // ==================== CreateServiceResponse 测试 ====================

    #[test]
    fn test_create_service_response_serialization() {
        let now = Utc::now();
        let response = CreateServiceResponse {
            id: "service_123".to_string(),
            name: "Test Service".to_string(),
            description: "Description".to_string(),
            usage: "Instructions".to_string(),
            registration_status: "pending".to_string(),
            registration_token: "token_123".to_string(),
            created_at: now,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("service_123"));
        assert!(json.contains("Test Service"));
        assert!(json.contains("pending"));
        assert!(json.contains("token_123"));
    }

    // ==================== ServiceResponse 测试 ====================

    #[test]
    fn test_service_response_from_service() {
        let now = Utc::now();
        let service = Service {
            id: "service_123".to_string(),
            name: "Test Service".to_string(),
            description: "Description".to_string(),
            usage: "Instructions".to_string(),
            agent_api_key: None,
            agent_status: AgentStatus::Busy,
            agent_capacity: 10,
            agent_current_load: 5,
            agent_last_heartbeat: Some(now),
            registration_token: None,
            registration_status: "active".to_string(),
            is_public: false,
            created_at: now,
        };

        let response: ServiceResponse = service.clone().into();

        assert_eq!(response.id, service.id);
        assert_eq!(response.name, service.name);
        assert_eq!(response.description, service.description);
        assert_eq!(response.usage, service.usage);
        assert_eq!(response.agent_status, service.agent_status);
        assert_eq!(response.registration_status, service.registration_status);
        assert_eq!(response.agent_capacity, service.agent_capacity);
        assert_eq!(response.agent_current_load, service.agent_current_load);
        assert_eq!(response.agent_last_heartbeat, service.agent_last_heartbeat);
        assert_eq!(response.is_public, service.is_public);
        assert_eq!(response.created_at, service.created_at);
    }

    #[test]
    fn test_service_response_serialization() {
        let now = Utc::now();
        let response = ServiceResponse {
            id: "service_123".to_string(),
            name: "Test Service".to_string(),
            description: "Description".to_string(),
            usage: "Instructions".to_string(),
            agent_status: AgentStatus::Online,
            registration_status: "active".to_string(),
            agent_capacity: 5,
            agent_current_load: 2,
            agent_last_heartbeat: None,
            is_public: true,
            created_at: now,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("service_123"));
        assert!(json.contains("online"));
        assert!(json.contains("active"));
    }

    // ==================== UserServicePermission 测试 ====================

    #[test]
    fn test_user_service_permission_creation() {
        let now = Utc::now();
        let permission = UserServicePermission {
            id: "perm_123".to_string(),
            user_id: "user_456".to_string(),
            service_id: "service_789".to_string(),
            granted_at: now,
        };

        assert_eq!(permission.id, "perm_123");
        assert_eq!(permission.user_id, "user_456");
        assert_eq!(permission.service_id, "service_789");
    }

    #[test]
    fn test_user_service_permission_clone() {
        let now = Utc::now();
        let permission = UserServicePermission {
            id: "perm_123".to_string(),
            user_id: "user_456".to_string(),
            service_id: "service_789".to_string(),
            granted_at: now,
        };

        let cloned = permission.clone();
        assert_eq!(cloned.id, permission.id);
        assert_eq!(cloned.user_id, permission.user_id);
        assert_eq!(cloned.service_id, permission.service_id);
    }

    // ==================== 数据库集成测试 ====================

    use crate::test_utils::setup_test_db;

    #[sqlx::test]
    async fn test_service_insert_and_fetch() {
        let pool = setup_test_db().await;
        let now = Utc::now();

        // 插入服务
        sqlx::query(
            r#"
            INSERT INTO services (
                id, name, description, usage, agent_api_key, agent_status,
                agent_capacity, agent_current_load, registration_status, is_public, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind("service_123")
        .bind("Test Service")
        .bind("Description")
        .bind("Test usage")
        .bind("ak_agent_key")
        .bind("online")
        .bind(5i64)
        .bind(2i64)
        .bind("active")
        .bind(1)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        // 查询服务
        let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
            .bind("service_123")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(service.id, "service_123");
        assert_eq!(service.name, "Test Service");
        assert_eq!(service.agent_status, AgentStatus::Online);
        assert_eq!(service.agent_capacity, 5);
    }

    #[sqlx::test]
    async fn test_service_update_status() {
        let pool = setup_test_db().await;
        let now = Utc::now();

        // 插入服务
        sqlx::query(
            r#"
            INSERT INTO services (id, name, description, usage, agent_status, agent_capacity, is_public, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind("service_123")
        .bind("Test Service")
        .bind("")
        .bind("")
        .bind("offline")
        .bind(1i64)
        .bind(1)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        // 更新状态
        sqlx::query("UPDATE services SET agent_status = ?, agent_current_load = ? WHERE id = ?")
            .bind("busy")
            .bind(5i64)
            .bind("service_123")
            .execute(&pool)
            .await
            .unwrap();

        // 查询验证
        let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
            .bind("service_123")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(service.agent_status, AgentStatus::Busy);
        assert_eq!(service.agent_current_load, 5);
    }

    #[sqlx::test]
    async fn test_user_service_permission_insert_and_fetch() {
        let pool = setup_test_db().await;
        let now = Utc::now();

        // 插入权限
        sqlx::query(
            "INSERT INTO user_service_permissions (id, user_id, service_id, granted_at) VALUES (?, ?, ?, ?)"
        )
        .bind("perm_123")
        .bind("user_456")
        .bind("service_789")
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        // 查询权限
        let permission: UserServicePermission = sqlx::query_as::<_, UserServicePermission>(
            "SELECT * FROM user_service_permissions WHERE id = ?",
        )
        .bind("perm_123")
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(permission.id, "perm_123");
        assert_eq!(permission.user_id, "user_456");
        assert_eq!(permission.service_id, "service_789");
    }

    #[sqlx::test]
    async fn test_service_fetch_by_status() {
        let pool = setup_test_db().await;
        let now = Utc::now();

        // 插入多个服务
        sqlx::query(
            r#"
            INSERT INTO services (id, name, description, usage, agent_status, agent_capacity, is_public, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind("service_1")
        .bind("Service 1")
        .bind("")
        .bind("")
        .bind("online")
        .bind(1i64)
        .bind(1)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            INSERT INTO services (id, name, description, usage, agent_status, agent_capacity, is_public, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind("service_2")
        .bind("Service 2")
        .bind("")
        .bind("")
        .bind("offline")
        .bind(1i64)
        .bind(1)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        // 按状态查询
        let online_services: Vec<Service> =
            sqlx::query_as::<_, Service>("SELECT * FROM services WHERE agent_status = ?")
                .bind("online")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert_eq!(online_services.len(), 1);
        assert_eq!(online_services[0].id, "service_1");
    }
}
