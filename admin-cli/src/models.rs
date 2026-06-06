use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Service Models
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ServiceListItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub agent_status: String,
    pub registration_status: String,
    pub agent_last_heartbeat: Option<DateTime<Utc>>,
    pub access_type: String,
    pub has_permission: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ServiceResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub usage: String,
    pub agent_status: String,
    pub registration_status: String,
    pub agent_capacity: i64,
    pub agent_current_load: i64,
    pub agent_last_heartbeat: Option<DateTime<Utc>>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ServiceUsageResponse {
    pub id: String,
    pub name: String,
    pub usage: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateServiceRequest {
    pub name: String,
    pub description: String,
    pub usage: String,
    #[serde(default = "default_true")]
    pub is_public: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CreateServiceResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub usage: String,
    pub registration_status: String,
    pub registration_token: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateServiceRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_public: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteServiceResponse {
    pub deleted: bool,
    pub tasks_cancelled: i64,
    pub tasks_retained: i64,
}

#[allow(dead_code)]
fn default_true() -> bool {
    true
}

// ============================================================================
// User Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub name: String,
    #[serde(skip)]
    pub api_key: String,
    pub role: String,
    pub created_at: String,
}

// ============================================================================
// Permission Models
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct UserPermissionResponse {
    pub service_id: String,
    pub service_name: String,
    pub granted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceUser {
    pub user_id: String,
    pub user_name: Option<String>,
    pub role: String,
    pub granted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceUsersList {
    pub is_public: bool,
    pub users: Vec<ServiceUser>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrantPermissionRequest {
    pub user_id: String,
}

// ============================================================================
// Task Models
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct AdminTaskResponse {
    pub id: String,
    pub user_id: String,
    pub user_name: Option<String>,
    pub service_id: String,
    pub status: TaskStatus,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub session_id: String,
    pub retry_count: i64,
    pub created_at: DateTime<Utc>,
    pub assigned_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Health Models
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub timestamp: String,
}

// ============================================================================
// Generic API Error Response
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ApiErrorResponse {
    pub message: Option<String>,
    pub detail: Option<String>,
    pub error: Option<String>,
}

// ============================================================================
// Task Status
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Cancelling,
    Unknown(String),
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
            TaskStatus::Cancelling => write!(f, "cancelling"),
            TaskStatus::Unknown(s) => write!(f, "{}", s),
        }
    }
}

impl Serialize for TaskStatus {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for TaskStatus {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "pending" => TaskStatus::Pending,
            "running" => TaskStatus::Running,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            "cancelling" => TaskStatus::Cancelling,
            _ => TaskStatus::Unknown(s),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_roundtrip() {
        let cases = vec![
            (TaskStatus::Pending, "pending"),
            (TaskStatus::Running, "running"),
            (TaskStatus::Completed, "completed"),
            (TaskStatus::Failed, "failed"),
            (TaskStatus::Cancelled, "cancelled"),
            (TaskStatus::Cancelling, "cancelling"),
        ];
        for (variant, json_str) in cases {
            let serialized = serde_json::to_string(&variant).unwrap();
            assert_eq!(serialized, format!("\"{}\"", json_str));
            let deserialized: TaskStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn test_task_status_unknown() {
        let deserialized: TaskStatus = serde_json::from_str("\"retrying\"").unwrap();
        assert_eq!(deserialized, TaskStatus::Unknown("retrying".to_string()));
        assert_eq!(deserialized.to_string(), "retrying");
    }

    #[test]
    fn test_task_status_display() {
        assert_eq!(TaskStatus::Pending.to_string(), "pending");
        assert_eq!(TaskStatus::Running.to_string(), "running");
        assert_eq!(TaskStatus::Completed.to_string(), "completed");
        assert_eq!(TaskStatus::Failed.to_string(), "failed");
        assert_eq!(TaskStatus::Cancelled.to_string(), "cancelled");
        assert_eq!(TaskStatus::Cancelling.to_string(), "cancelling");
        assert_eq!(
            TaskStatus::Unknown("retrying".to_string()).to_string(),
            "retrying"
        );
    }

    #[test]
    fn test_user_roundtrip() {
        let user = UserResponse {
            id: "u1".to_string(),
            name: "Alice".to_string(),
            api_key: "ak_abc".to_string(),
            role: "admin".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&user).unwrap();
        assert!(!json.contains("ak_abc"));
        let parsed: UserResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, user.id);
        assert_eq!(parsed.name, user.name);
        assert_eq!(parsed.role, user.role);
        // api_key is skipped on serialize, so it will be empty after deserialize
        assert_eq!(parsed.api_key, "");
    }

    #[test]
    fn test_service_roundtrip() {
        let svc = ServiceResponse {
            id: "svc-1".to_string(),
            name: "Test".to_string(),
            description: "Desc".to_string(),
            usage: "Usage".to_string(),
            agent_status: "online".to_string(),
            registration_status: "active".to_string(),
            agent_capacity: 10,
            agent_current_load: 2,
            agent_last_heartbeat: None,
            is_public: true,
            created_at: "2024-01-01T00:00:00Z".parse().unwrap(),
        };
        let json = serde_json::to_string(&svc).unwrap();
        let parsed: ServiceResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, svc.id);
        assert_eq!(parsed.name, svc.name);
        assert_eq!(parsed.agent_capacity, svc.agent_capacity);
        assert!(parsed.is_public);
    }
}
