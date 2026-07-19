//! 任务模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 任务状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TaskStatus {
    /// 等待中
    #[default]
    Pending,
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
    /// 取消中
    Cancelling,
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
        }
    }
}

impl TaskStatus {
    /// 检查状态是否为终止状态
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        )
    }

    /// 检查状态是否允许取消
    pub fn can_cancel(&self) -> bool {
        matches!(self, TaskStatus::Pending | TaskStatus::Running)
    }

    /// 检查状态是否为取消中
    pub fn is_cancelling(&self) -> bool {
        matches!(self, TaskStatus::Cancelling)
    }
}

/// 任务模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Task {
    /// 任务ID
    pub id: String,
    /// 创建者用户ID
    pub user_id: String,
    /// 对应的服务ID（一对一模型下，service_id就是agent_id）
    pub service_id: String,
    /// 任务状态
    pub status: TaskStatus,
    /// 输入数据 (JSON)
    pub input: Option<serde_json::Value>,
    /// 输出数据 (JSON)
    pub output: Option<serde_json::Value>,
    /// 错误信息
    pub error_message: Option<String>,
    /// 重试次数
    pub retry_count: i64,
    /// Session ID（必需，用于任务分组或执行器会话标识）
    pub session_id: String,
    /// 输出格式（描述回复的格式和内容）
    pub output_format: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 分配时间
    pub assigned_at: Option<DateTime<Utc>>,
    /// 开始时间
    pub started_at: Option<DateTime<Utc>>,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
}

impl Task {
    /// 创建新任务
    pub fn new(
        user_id: impl Into<String>,
        service_id: impl Into<String>,
        session_id: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            service_id: service_id.into(),
            status: TaskStatus::Pending,
            input: None,
            output: None,
            error_message: None,
            retry_count: 0,
            session_id: session_id.unwrap_or_else(|| Uuid::now_v7().to_string()),
            output_format: None,
            created_at: now,
            assigned_at: None,
            started_at: None,
            completed_at: None,
        }
    }

    /// 设置输入数据
    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.input = Some(input);
        self
    }

    /// 分配给Agent（一对一模型下，service_id就是agent_id）
    pub fn assign_to_service(&mut self, service_id: impl Into<String>) {
        self.service_id = service_id.into();
        self.status = TaskStatus::Running;
        self.assigned_at = Some(Utc::now());
    }

    /// 开始执行
    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// 完成任务
    pub fn complete(&mut self, output: Option<serde_json::Value>) {
        self.status = TaskStatus::Completed;
        self.output = output;
        self.completed_at = Some(Utc::now());
    }

    /// 标记为失败
    pub fn fail(&mut self, error_message: impl Into<String>) {
        self.status = TaskStatus::Failed;
        self.error_message = Some(error_message.into());
        self.completed_at = Some(Utc::now());
    }

    /// 取消任务
    pub fn cancel(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    /// 增加重试次数
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// 任务输入（强制包含两个提示词）
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct TaskInput {
    /// 任务提示词（干活的）
    pub task_prompt: String,
    /// 输出格式提示词（规定返回格式）
    pub output_prompt: String,
    /// 上传的输入文件 ID 列表
    #[serde(default)]
    pub input_files: Vec<String>,
}

/// 任务响应
#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub service_id: String,
    pub status: String,
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

impl From<Task> for TaskResponse {
    fn from(task: Task) -> Self {
        Self {
            id: task.id,
            service_id: task.service_id,
            status: task.status.to_string(),
            input: task.input,
            output: task.output,
            error_message: task.error_message,
            session_id: task.session_id,
            retry_count: task.retry_count,
            created_at: task.created_at,
            assigned_at: task.assigned_at,
            started_at: task.started_at,
            completed_at: task.completed_at,
        }
    }
}

/// 任务列表查询参数
#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    pub status: Option<String>,
    pub service_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ==================== TaskStatus 枚举测试 ====================

    #[test]
    fn test_task_status_default() {
        let status: TaskStatus = Default::default();
        assert_eq!(status, TaskStatus::Pending);
    }

    #[test]
    fn test_task_status_to_string() {
        assert_eq!(TaskStatus::Pending.to_string(), "pending");
        assert_eq!(TaskStatus::Running.to_string(), "running");
        assert_eq!(TaskStatus::Completed.to_string(), "completed");
        assert_eq!(TaskStatus::Failed.to_string(), "failed");
        assert_eq!(TaskStatus::Cancelled.to_string(), "cancelled");
        assert_eq!(TaskStatus::Cancelling.to_string(), "cancelling");
    }

    #[test]
    fn test_task_status_is_terminal() {
        // 终止状态
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());

        // 非终止状态
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
        assert!(!TaskStatus::Cancelling.is_terminal());
    }

    #[test]
    fn test_task_status_can_cancel() {
        // 可取消状态
        assert!(TaskStatus::Pending.can_cancel());
        assert!(TaskStatus::Running.can_cancel());

        // 不可取消状态
        assert!(!TaskStatus::Completed.can_cancel());
        assert!(!TaskStatus::Failed.can_cancel());
        assert!(!TaskStatus::Cancelled.can_cancel());
        assert!(!TaskStatus::Cancelling.can_cancel());
    }

    #[test]
    fn test_task_status_is_cancelling() {
        assert!(TaskStatus::Cancelling.is_cancelling());
        assert!(!TaskStatus::Pending.is_cancelling());
        assert!(!TaskStatus::Running.is_cancelling());
        assert!(!TaskStatus::Completed.is_cancelling());
        assert!(!TaskStatus::Failed.is_cancelling());
        assert!(!TaskStatus::Cancelled.is_cancelling());
    }

    #[test]
    fn test_task_status_clone_and_eq() {
        let status = TaskStatus::Running;
        let cloned = status;
        assert_eq!(status, cloned);
        assert_eq!(status, TaskStatus::Running);
        assert_ne!(status, TaskStatus::Pending);
    }

    #[test]
    fn test_task_status_serialization() {
        let status = TaskStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"completed\"");

        let deserialized: TaskStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, TaskStatus::Completed);
    }

    // ==================== Task 结构体测试 ====================

    #[test]
    fn test_task_new() {
        let task = Task::new("user_123", "service_456", None);

        assert!(!task.id.is_empty());
        assert_eq!(task.user_id, "user_123");
        assert_eq!(task.service_id, "service_456");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.input.is_none());
        assert!(task.output.is_none());
        assert!(task.error_message.is_none());
        assert_eq!(task.retry_count, 0);
        assert!(!task.session_id.is_empty());
        assert!(task.output_format.is_none());
        assert!(task.assigned_at.is_none());
        assert!(task.started_at.is_none());
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn test_task_new_with_session_id() {
        let task = Task::new(
            "user_123",
            "service_456",
            Some("custom_session".to_string()),
        );

        assert_eq!(task.session_id, "custom_session");
    }

    #[test]
    fn test_task_new_generates_session_id_v7() {
        // Arrange
        use uuid::Uuid;

        // Act
        let task = Task::new("user_123", "service_456", None);
        let session_uuid =
            Uuid::parse_str(&task.session_id).expect("session_id should be a valid UUID");

        // Assert
        let version = session_uuid.as_bytes()[6] >> 4;
        assert_eq!(
            version, 7,
            "expected UUID v7 session_id, got version {}",
            version
        );
    }

    #[test]
    fn test_task_with_input() {
        let input = json!({"prompt": "test task"});
        let task = Task::new("user_123", "service_456", None).with_input(input.clone());

        assert!(task.input.is_some());
        assert_eq!(task.input.unwrap(), input);
    }

    #[test]
    fn test_task_assign_to_service() {
        let mut task = Task::new("user_123", "initial_service", None);

        task.assign_to_service("new_service");

        assert_eq!(task.service_id, "new_service");
        assert_eq!(task.status, TaskStatus::Running);
        assert!(task.assigned_at.is_some());
    }

    #[test]
    fn test_task_start() {
        let mut task = Task::new("user_123", "service_456", None);

        task.start();

        assert_eq!(task.status, TaskStatus::Running);
        assert!(task.started_at.is_some());
    }

    #[test]
    fn test_task_complete() {
        let mut task = Task::new("user_123", "service_456", None);
        let output = json!({"result": "success"});

        task.complete(Some(output.clone()));

        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.output, Some(output));
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_complete_without_output() {
        let mut task = Task::new("user_123", "service_456", None);

        task.complete(None);

        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.output.is_none());
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_fail() {
        let mut task = Task::new("user_123", "service_456", None);

        task.fail("Something went wrong");

        assert_eq!(task.status, TaskStatus::Failed);
        assert_eq!(task.error_message, Some("Something went wrong".to_string()));
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_fail_with_string() {
        let mut task = Task::new("user_123", "service_456", None);
        let error = String::from("Error from string");

        task.fail(error);

        assert_eq!(task.status, TaskStatus::Failed);
        assert_eq!(task.error_message, Some("Error from string".to_string()));
    }

    #[test]
    fn test_task_cancel() {
        let mut task = Task::new("user_123", "service_456", None);

        task.cancel();

        assert_eq!(task.status, TaskStatus::Cancelled);
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_increment_retry() {
        let mut task = Task::new("user_123", "service_456", None);

        assert_eq!(task.retry_count, 0);

        task.increment_retry();
        assert_eq!(task.retry_count, 1);

        task.increment_retry();
        assert_eq!(task.retry_count, 2);

        task.increment_retry();
        assert_eq!(task.retry_count, 3);
    }

    #[test]
    fn test_task_clone() {
        let task = Task::new("user_123", "service_456", None).with_input(json!({"test": true}));

        let cloned = task.clone();

        assert_eq!(cloned.id, task.id);
        assert_eq!(cloned.user_id, task.user_id);
        assert_eq!(cloned.service_id, task.service_id);
        assert_eq!(cloned.status, task.status);
        assert_eq!(cloned.session_id, task.session_id);
        assert_eq!(cloned.input, task.input);
    }

    #[test]
    fn test_task_serialization() {
        let task = Task::new("user_123", "service_456", Some("session_789".to_string()))
            .with_input(json!({"key": "value"}));

        let json = serde_json::to_string(&task).unwrap();

        assert!(json.contains("user_123"));
        assert!(json.contains("service_456"));
        assert!(json.contains("session_789"));
        assert!(json.contains("pending"));

        let deserialized: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, task.id);
        assert_eq!(deserialized.user_id, task.user_id);
        assert_eq!(deserialized.session_id, task.session_id);
    }

    // ==================== TaskInput 结构体测试 ====================

    #[test]
    fn test_task_input_creation() {
        let input = TaskInput {
            task_prompt: "Do something".to_string(),
            output_prompt: "Return as JSON".to_string(),
            input_files: vec![],
        };

        assert_eq!(input.task_prompt, "Do something");
        assert_eq!(input.output_prompt, "Return as JSON");
    }

    #[test]
    fn test_task_input_serialization() {
        let input = TaskInput {
            task_prompt: "Do something".to_string(),
            output_prompt: "Return as JSON".to_string(),
            input_files: vec![],
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("Do something"));
        assert!(json.contains("Return as JSON"));

        let deserialized: TaskInput = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.task_prompt, input.task_prompt);
        assert_eq!(deserialized.output_prompt, input.output_prompt);
    }

    #[test]
    fn test_task_input_clone() {
        let input = TaskInput {
            task_prompt: "Do something".to_string(),
            output_prompt: "Return as JSON".to_string(),
            input_files: vec![],
        };

        let cloned = input.clone();
        assert_eq!(cloned.task_prompt, input.task_prompt);
        assert_eq!(cloned.output_prompt, input.output_prompt);
    }

    // ==================== TaskResponse 结构体测试 ====================

    #[test]
    fn test_task_response_from_task() {
        let task = Task::new("user_123", "service_456", Some("session_789".to_string()))
            .with_input(json!({"key": "value"}));

        let response: TaskResponse = task.clone().into();

        assert_eq!(response.id, task.id);
        assert_eq!(response.service_id, task.service_id);
        assert_eq!(response.status, "pending");
        assert_eq!(response.session_id, task.session_id);
        assert_eq!(response.retry_count, task.retry_count);
        assert_eq!(response.created_at, task.created_at);
        assert_eq!(response.input, task.input);
        assert_eq!(response.output, task.output);
        assert_eq!(response.error_message, task.error_message);
        assert_eq!(response.assigned_at, task.assigned_at);
        assert_eq!(response.started_at, task.started_at);
        assert_eq!(response.completed_at, task.completed_at);
    }

    #[test]
    fn test_task_response_serialization() {
        let task = Task::new("user_123", "service_456", None);
        let response: TaskResponse = task.into();

        let json = serde_json::to_string(&response).unwrap();
        // TaskResponse 没有 user_id 字段，只检查存在的字段
        assert!(json.contains("service_456"));
        assert!(json.contains("pending"));
        assert!(json.contains("retry_count"));
    }

    // ==================== ListTasksQuery 结构体测试 ====================

    #[test]
    fn test_list_tasks_query_deserialization_full() {
        let json = r#"{
            "status": "running",
            "service_id": "service_123",
            "limit": 10,
            "offset": 20
        }"#;

        let query: ListTasksQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.status, Some("running".to_string()));
        assert_eq!(query.service_id, Some("service_123".to_string()));
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(20));
    }

    #[test]
    fn test_list_tasks_query_deserialization_partial() {
        let json = r#"{
            "status": "pending"
        }"#;

        let query: ListTasksQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.status, Some("pending".to_string()));
        assert!(query.service_id.is_none());
        assert!(query.limit.is_none());
        assert!(query.offset.is_none());
    }

    #[test]
    fn test_list_tasks_query_deserialization_empty() {
        let json = r#"{}"#;

        let query: ListTasksQuery = serde_json::from_str(json).unwrap();
        assert!(query.status.is_none());
        assert!(query.service_id.is_none());
        assert!(query.limit.is_none());
        assert!(query.offset.is_none());
    }

    // ==================== 数据库集成测试 ====================

    use crate::test_utils::setup_test_db;

    #[sqlx::test]
    async fn test_task_insert_and_fetch() {
        let pool = setup_test_db().await;

        // 创建外键引用的服务和用户
        sqlx::query("INSERT INTO services (id, name, description, usage) VALUES (?, ?, ?, ?)")
            .bind("service_456")
            .bind("Test Service")
            .bind("")
            .bind("")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)")
            .bind("user_123")
            .bind("ak_test_key")
            .bind("Test User")
            .bind("client")
            .execute(&pool)
            .await
            .unwrap();

        // 创建任务
        let task = Task::new("user_123", "service_456", Some("session_789".to_string()))
            .with_input(json!({"test": true}));

        // 插入数据库
        sqlx::query(
            r#"
            INSERT INTO tasks (id, user_id, service_id, status, input, session_id, retry_count, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&task.id)
        .bind(&task.user_id)
        .bind(&task.service_id)
        .bind("pending")
        .bind(task.input.as_ref().map(|v| v.to_string()))
        .bind(&task.session_id)
        .bind(task.retry_count)
        .bind(task.created_at)
        .execute(&pool)
        .await
        .unwrap();

        // 查询任务
        let fetched: Task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
            .bind(&task.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(fetched.id, task.id);
        assert_eq!(fetched.user_id, task.user_id);
        assert_eq!(fetched.service_id, task.service_id);
        assert_eq!(fetched.session_id, task.session_id);
        assert_eq!(fetched.status, TaskStatus::Pending);
    }

    #[sqlx::test]
    async fn test_task_update_status() {
        let pool = setup_test_db().await;

        // 创建外键引用的服务和用户
        sqlx::query("INSERT INTO services (id, name, description, usage) VALUES (?, ?, ?, ?)")
            .bind("service_456")
            .bind("Test Service")
            .bind("")
            .bind("")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)")
            .bind("user_123")
            .bind("ak_test_key")
            .bind("Test User")
            .bind("client")
            .execute(&pool)
            .await
            .unwrap();

        let task = Task::new("user_123", "service_456", None);

        // 插入
        sqlx::query(
            r#"
            INSERT INTO tasks (id, user_id, service_id, status, session_id, retry_count, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&task.id)
        .bind(&task.user_id)
        .bind(&task.service_id)
        .bind("pending")
        .bind(&task.session_id)
        .bind(task.retry_count)
        .bind(task.created_at)
        .execute(&pool)
        .await
        .unwrap();

        // 更新状态
        sqlx::query("UPDATE tasks SET status = ? WHERE id = ?")
            .bind("running")
            .bind(&task.id)
            .execute(&pool)
            .await
            .unwrap();

        // 查询验证
        let fetched: Task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
            .bind(&task.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(fetched.status, TaskStatus::Running);
    }

    #[sqlx::test]
    async fn test_task_fetch_by_status() {
        let pool = setup_test_db().await;

        // 创建外键引用的服务和用户
        for svc in ["service_1", "service_2"] {
            sqlx::query("INSERT INTO services (id, name, description, usage) VALUES (?, ?, ?, ?)")
                .bind(svc)
                .bind("Test Service")
                .bind("")
                .bind("")
                .execute(&pool)
                .await
                .unwrap();
        }

        for (uid, key) in [("user_1", "ak_key1"), ("user_2", "ak_key2")] {
            sqlx::query("INSERT INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)")
                .bind(uid)
                .bind(key)
                .bind("Test User")
                .bind("client")
                .execute(&pool)
                .await
                .unwrap();
        }

        // 创建两个不同状态的任务
        let task1 = Task::new("user_1", "service_1", None);
        let task2 = Task::new("user_2", "service_2", None);

        // 插入第一个
        sqlx::query(
            r#"
            INSERT INTO tasks (id, user_id, service_id, status, session_id, retry_count, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&task1.id)
        .bind(&task1.user_id)
        .bind(&task1.service_id)
        .bind("pending")
        .bind(&task1.session_id)
        .bind(task1.retry_count)
        .bind(task1.created_at)
        .execute(&pool)
        .await
        .unwrap();

        // 插入第二个
        sqlx::query(
            r#"
            INSERT INTO tasks (id, user_id, service_id, status, session_id, retry_count, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&task2.id)
        .bind(&task2.user_id)
        .bind(&task2.service_id)
        .bind("running")
        .bind(&task2.session_id)
        .bind(task2.retry_count)
        .bind(task2.created_at)
        .execute(&pool)
        .await
        .unwrap();

        // 按状态查询
        let pending_tasks: Vec<Task> =
            sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE status = ?")
                .bind("pending")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert_eq!(pending_tasks.len(), 1);
        assert_eq!(pending_tasks[0].id, task1.id);
    }
}
