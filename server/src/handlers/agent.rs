//! Agent API处理器
//!
//! 基于服务的一对一Agent操作：
//! - POST /register - 注册为服务的Agent（需要 registration_token）
//! - POST /:service_id/poll - 轮询任务
//! - POST /:service_id/heartbeat - 心跳
//! - POST /:service_id/complete - 完成任务

use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    middleware::{self},
    routing::post,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    auth::{AuthAgent, agent_auth_middleware},
    error::{AppError, Result},
    models::{
        file::TaskFile,
        service::{AgentStatus, Service},
        task::{Task, TaskInput, TaskStatus},
    },
    state::AppState,
};
use serde_json::json;

/// Agent路由
///
/// 注意：register是公开的，其他路由需要Agent认证
pub fn routes(state: AppState) -> Router<AppState> {
    // 公开路由
    let public_routes = Router::new().route("/register", post(register_service_agent));

    // 需要认证的路由 - 使用 from_fn_with_state 添加 Agent 鉴权中间件
    let authenticated_routes = Router::new()
        .route("/{service_id}/poll", post(poll_handler))
        .route("/{service_id}/accept", post(accept_handler))
        .route("/{service_id}/complete", post(complete_handler))
        .route("/{service_id}/heartbeat", post(heartbeat_handler))
        .layer(middleware::from_fn_with_state(state, agent_auth_middleware));

    public_routes.merge(authenticated_routes)
}

// ============================================================================
// 请求/响应结构
// ============================================================================

/// Agent心跳请求
#[derive(Debug, Deserialize)]
pub struct ServiceHeartbeatRequest {
    pub status: Option<String>,
    pub current_load: Option<i64>,
    pub capacity: Option<i64>,
    pub available_capacity: Option<i64>,
}

/// Agent心跳响应
#[derive(Debug, Serialize)]
pub struct ServiceHeartbeatResponse {
    pub acknowledged: bool,
    pub service_id: String,
    pub timestamp: String,
}

/// 注册Agent请求（使用 registration_token）
#[derive(Debug, Deserialize)]
pub struct RegisterAgentApiRequest {
    pub registration_token: String, // 必填：管理员预创建的注册令牌
    #[serde(default)]
    pub capacity: Option<i64>, // 改为可选
}

/// 注册Agent响应
#[derive(Debug, Serialize)]
pub struct RegisterAgentResponse {
    pub service_id: String,
    pub name: String,
    pub api_key: String,
    pub status: String,
    pub created_at: String,
}

/// 接受任务请求
#[derive(Debug, Deserialize)]
pub struct AcceptTaskRequest {
    pub task_id: String,
}

/// 轮询任务请求
#[derive(Debug, Deserialize)]
pub struct PollRequest {
    pub current_load: Option<i64>,
    pub available_capacity: Option<i64>,
}

/// Agent 轮询响应中的任务结构（扁平化）
#[derive(Debug, Serialize)]
pub struct AgentInputFileResponse {
    pub id: String,
    pub filename: String,
    pub mime_type: Option<String>,
    pub size_bytes: i64,
}

impl From<TaskFile> for AgentInputFileResponse {
    fn from(file: TaskFile) -> Self {
        Self {
            id: file.id,
            filename: file.filename,
            mime_type: file.mime_type,
            size_bytes: file.size_bytes,
        }
    }
}

/// Agent 轮询响应中的任务结构（扁平化）
#[derive(Debug, Serialize)]
pub struct AgentTaskResponse {
    pub id: String,
    pub task_prompt: String,
    pub output_prompt: Option<String>,
    pub session_id: Option<String>,
    pub input_files: Vec<AgentInputFileResponse>,
}

impl AgentTaskResponse {
    /// 从 Task 和 TaskInput 创建 AgentTaskResponse
    pub fn from_task(
        task: Task,
        input: TaskInput,
        input_files: Vec<AgentInputFileResponse>,
    ) -> Self {
        Self {
            id: task.id,
            task_prompt: input.task_prompt,
            output_prompt: Some(input.output_prompt).filter(|s| !s.is_empty()),
            session_id: Some(task.session_id).filter(|s| !s.is_empty()),
            input_files,
        }
    }
}

/// 轮询响应
#[derive(Debug, Serialize)]
pub struct PollResponse {
    pub has_task: bool,
    pub task: Option<AgentTaskResponse>,
    pub should_cancel: bool,
    pub cancel_task_id: Option<String>,
}

/// 接受任务响应
#[derive(Debug, Serialize)]
pub struct AcceptTaskResponse {
    pub success: bool,
    pub task_id: String,
    pub message: String,
}

/// 完成任务请求
#[derive(Debug, Deserialize)]
pub struct CompleteTaskRequest {
    pub task_id: String,
    pub output: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub status: TaskCompleteStatus,
    #[serde(default)]
    pub file_ids: Vec<String>, // 新增：上传的文件ID列表
}

/// 任务完成状态
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskCompleteStatus {
    Completed,
    Failed,
    Cancelled,
}

impl TaskCompleteStatus {
    pub fn to_task_status(&self) -> TaskStatus {
        match self {
            TaskCompleteStatus::Completed => TaskStatus::Completed,
            TaskCompleteStatus::Failed => TaskStatus::Failed,
            TaskCompleteStatus::Cancelled => TaskStatus::Cancelled,
        }
    }
}

/// 完成任务响应
#[derive(Debug, Serialize)]
pub struct CompleteTaskResponse {
    pub success: bool,
    pub task_id: String,
    pub status: String,
}

// ============================================================================
// 处理器
// ============================================================================

/// 注册为服务的Agent（需要有效的 registration_token）
/// POST /api/v1/agent/register
///
/// 流程：
/// 1. 查找 service_id 对应的服务
/// 2. 验证 registration_token 是否匹配
/// 3. 检查服务状态是否为 pending
/// 4. 生成 agent_api_key
/// 5. 更新服务为 active 状态，清除 registration_token
pub async fn register_service_agent(
    State(state): State<AppState>,
    Json(req): Json<RegisterAgentApiRequest>,
) -> Result<Json<RegisterAgentResponse>> {
    // 1. 查找服务（通过 registration_token）
    let service: Option<Service> =
        sqlx::query_as::<_, Service>("SELECT * FROM services WHERE registration_token = ?")
            .bind(&req.registration_token)
            .fetch_optional(state.db.pool())
            .await
            .map_err(AppError::Database)?;

    // 2. 服务必须存在
    let service = service.ok_or_else(|| AppError::NotFound)?;

    // 3. 检查注册状态
    if service.registration_status == "active" {
        return Err(AppError::Conflict("该服务已注册，无法重复注册".to_string()));
    }

    if service.registration_status == "revoked" {
        return Err(AppError::Forbidden);
    }

    // 4. 生成 agent_api_key
    let api_key = format!(
        "ak_agent_{}",
        uuid::Uuid::new_v4().to_string().replace("-", "")
    );
    let now = Utc::now();

    // 从配置中获取 secret_key 并对 api_key 进行 HMAC 哈希
    let secret_key = state
        .config
        .secret_key
        .as_ref()
        .ok_or_else(|| AppError::Internal("Secret key not configured".to_string()))?;
    let hashed_api_key = crate::auth::hash_api_key(secret_key, &api_key);

    // 6. 更新服务：设置 hash 后的 api_key，状态改为 active，清除 registration_token
    sqlx::query(
        r#"
        UPDATE services
        SET agent_api_key = ?, 
            agent_status = 'online', 
            agent_capacity = ?, 
            agent_current_load = 0, 
            agent_last_heartbeat = ?,
            registration_status = 'active',
            registration_token = NULL
        WHERE id = ?
        "#,
    )
    .bind(&hashed_api_key)
    .bind(req.capacity.unwrap_or(1)) // capacity 不传则默认 1
    .bind(now.to_rfc3339())
    .bind(&service.id)
    .execute(state.db.pool())
    .await
    .map_err(AppError::Database)?;

    tracing::info!(
        "Agent 注册服务: service_id={}, name={}, capacity={}",
        service.id,
        service.name,
        req.capacity.unwrap_or(1)
    );

    Ok(Json(RegisterAgentResponse {
        service_id: service.id,
        name: service.name,
        api_key,
        status: "online".to_string(),
        created_at: now.to_rfc3339(),
    }))
}

/// 短轮询获取任务
/// POST /api/v1/agent/{service_id}/poll
pub async fn poll_handler(
    State(state): State<AppState>,
    Extension(auth_agent): Extension<AuthAgent>,
    Path(service_id): Path<String>,
) -> Result<Json<PollResponse>> {
    // 验证 service_id 与认证信息匹配
    if auth_agent.agent_id != service_id {
        return Err(AppError::Forbidden);
    }

    // ===== 1. 优先检查是否有 running 的任务需要取消（最高优先级） =====
    let cancelling_task = sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks WHERE service_id = ? AND status = 'cancelling' LIMIT 1",
    )
    .bind(&service_id)
    .fetch_optional(state.db.pool())
    .await
    .map_err(AppError::Database)?;

    if let Some(task) = cancelling_task {
        return Ok(Json(PollResponse {
            has_task: false,
            should_cancel: true,
            cancel_task_id: Some(task.id),
            task: None,
        }));
    }

    // ===== 2. 再检查 pending 任务 =====
    if let Some(task) = find_pending_tasks_by_service(state.db.pool(), &service_id)
        .await?
        .into_iter()
        .next()
    {
        // 解析 input JSON
        let task_input: TaskInput = if let Some(input_json) = &task.input {
            serde_json::from_value(input_json.clone()).unwrap_or_default()
        } else {
            TaskInput::default()
        };

        let input_files = if task_input.input_files.is_empty() {
            Vec::new()
        } else {
            let files: Vec<TaskFile> = sqlx::query_as::<_, TaskFile>(
                "SELECT * FROM task_files WHERE task_id = ? AND created_by = 'client' ORDER BY created_at ASC"
            )
            .bind(&task.id)
            .fetch_all(state.db.pool())
            .await
            .map_err(AppError::Database)?;

            let mut ordered_files = Vec::new();
            for file_id in &task_input.input_files {
                if let Some(file) = files.iter().find(|file| &file.id == file_id) {
                    ordered_files.push(AgentInputFileResponse::from(file.clone()));
                }
            }
            ordered_files
        };

        // 转换为 AgentTaskResponse
        let agent_task = AgentTaskResponse::from_task(task, task_input, input_files);

        return Ok(Json(PollResponse {
            has_task: true,
            should_cancel: false,
            cancel_task_id: None,
            task: Some(agent_task),
        }));
    }

    // 无任务，立即返回
    Ok(Json(PollResponse {
        has_task: false,
        should_cancel: false,
        cancel_task_id: None,
        task: None,
    }))
}

/// 接受任务
/// POST /api/v1/agent/{service_id}/accept
pub async fn accept_handler(
    State(state): State<AppState>,
    Extension(auth_agent): Extension<AuthAgent>,
    Path(service_id): Path<String>,
    Json(req): Json<AcceptTaskRequest>,
) -> Result<Json<AcceptTaskResponse>> {
    // 验证 service_id 与认证信息匹配
    if auth_agent.agent_id != service_id {
        return Err(AppError::Forbidden);
    }

    let updated = accept_task(state.db.pool(), &req.task_id, &service_id).await?;

    if updated {
        tracing::info!(
            "Agent 接受任务: task_id={}, service_id={}",
            req.task_id,
            service_id
        );
        Ok(Json(AcceptTaskResponse {
            success: true,
            task_id: req.task_id,
            message: "Task accepted".to_string(),
        }))
    } else {
        Err(AppError::BadRequest(
            "Task not found or not in pending status".to_string(),
        ))
    }
}

/// 完成任务
/// POST /api/v1/agent/{service_id}/complete
pub async fn complete_handler(
    State(state): State<AppState>,
    Extension(auth_agent): Extension<AuthAgent>,
    Path(service_id): Path<String>,
    Json(req): Json<CompleteTaskRequest>,
) -> Result<Json<CompleteTaskResponse>> {
    // 验证 service_id 与认证信息匹配
    if auth_agent.agent_id != service_id {
        return Err(AppError::Forbidden);
    }

    let task_status = req.status.to_task_status();

    // 如果有 file_ids，合并到 output 中
    let output = if !req.file_ids.is_empty() {
        match req.output {
            Some(mut output) => {
                if let Some(obj) = output.as_object_mut() {
                    obj.insert("file_ids".to_string(), json!(req.file_ids));
                }
                Some(output)
            }
            None => {
                // 如果 output 为 null 但有 file_ids，创建一个包含 file_ids 的对象
                Some(json!({"file_ids": req.file_ids}))
            }
        }
    } else {
        req.output
    };

    let updated = complete_task(
        state.db.pool(),
        &req.task_id,
        &service_id,
        task_status,
        output,
        req.error_message,
    )
    .await?;

    if updated {
        tracing::info!(
            "Agent 完成任务: task_id={}, service_id={}, status={:?}",
            req.task_id,
            service_id,
            req.status
        );
        Ok(Json(CompleteTaskResponse {
            success: true,
            task_id: req.task_id,
            status: task_status.to_string(),
        }))
    } else {
        Err(AppError::BadRequest(
            "Task not found or invalid status transition".to_string(),
        ))
    }
}

/// 心跳
/// POST /api/v1/agent/{service_id}/heartbeat
pub async fn heartbeat_handler(
    State(state): State<AppState>,
    Extension(auth_agent): Extension<AuthAgent>,
    Path(service_id): Path<String>,
    Json(req): Json<ServiceHeartbeatRequest>,
) -> Result<Json<ServiceHeartbeatResponse>> {
    // 验证 service_id 与认证信息匹配
    if auth_agent.agent_id != service_id {
        return Err(AppError::Forbidden);
    }

    // 1. 更新心跳时间戳
    update_service_heartbeat(state.db.pool(), &service_id).await?;

    // 2. 更新状态（如果提供）
    if let Some(status_str) = req.status {
        let status = match status_str.as_str() {
            "online" => AgentStatus::Online,
            "offline" => AgentStatus::Offline,
            "busy" => AgentStatus::Busy,
            _ => {
                return Err(AppError::BadRequest(format!(
                    "Invalid status: {}",
                    status_str
                )));
            }
        };
        update_service_status(state.db.pool(), &service_id, status).await?;
    }

    // 3. 【新增】更新 current_load 和 capacity（如果提供）
    if req.current_load.is_some() || req.capacity.is_some() {
        update_service_load(state.db.pool(), &service_id, req.current_load, req.capacity).await?;
    }

    Ok(Json(ServiceHeartbeatResponse {
        acknowledged: true,
        service_id: service_id.to_string(),
        timestamp: Utc::now().to_rfc3339(),
    }))
}

// ============================================================================
// 数据库操作
// ============================================================================

/// 查找指定服务的待处理任务
pub async fn find_pending_tasks_by_service(
    pool: &SqlitePool,
    service_id: &str,
) -> Result<Vec<Task>> {
    let tasks = sqlx::query_as::<_, Task>(
        r#"
        SELECT * FROM tasks 
        WHERE status = 'pending' AND service_id = ?
        ORDER BY created_at ASC
        LIMIT 10
        "#,
    )
    .bind(service_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(tasks)
}

/// 查找指定服务和多个状态的任务
pub async fn find_tasks_by_service_and_status(
    pool: &SqlitePool,
    service_id: &str,
    statuses: &[TaskStatus],
) -> Result<Vec<Task>> {
    // 将状态转换为字符串
    let status_strings: Vec<String> = statuses.iter().map(|s| s.to_string()).collect();

    // 构建动态查询，使用 IN 子句
    let placeholders: Vec<String> = status_strings
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 2)) // ?2, ?3, ... (因为 service_id 是 ?1)
        .collect();

    let sql = format!(
        r#"
        SELECT * FROM tasks 
        WHERE service_id = ?1 AND status IN ({})
        ORDER BY created_at ASC
        LIMIT 10
        "#,
        placeholders.join(", ")
    );

    // 构建查询
    let mut query = sqlx::query_as::<_, Task>(&sql).bind(service_id);
    for status in &status_strings {
        query = query.bind(status);
    }

    let tasks = query.fetch_all(pool).await.map_err(AppError::Database)?;
    Ok(tasks)
}

/// 接受任务
pub async fn accept_task(pool: &SqlitePool, task_id: &str, service_id: &str) -> Result<bool> {
    let now = Utc::now();

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    let result = sqlx::query(
        r#"
        UPDATE tasks 
        SET status = 'running', assigned_at = ?, started_at = ?
        WHERE id = ? AND status = 'pending' AND service_id = ?
        "#,
    )
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .bind(task_id)
    .bind(service_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    if result.rows_affected() == 0 {
        let _ = tx.rollback().await;
        return Ok(false);
    }

    sqlx::query(
        r#"
        UPDATE services 
        SET agent_current_load = agent_current_load + 1,
            agent_status = CASE
                WHEN agent_current_load + 1 >= agent_capacity THEN 'busy'
                WHEN agent_status = 'offline' THEN 'offline'
                ELSE agent_status
            END
        WHERE id = ?
        "#,
    )
    .bind(service_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    Ok(true)
}

/// 完成任务
pub async fn complete_task(
    pool: &SqlitePool,
    task_id: &str,
    service_id: &str,
    status: TaskStatus,
    result: Option<serde_json::Value>,
    error_message: Option<String>,
) -> Result<bool> {
    let now = Utc::now();

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    let result_rows = sqlx::query(
        r#"
        UPDATE tasks 
        SET status = ?, output = ?, error_message = ?, completed_at = ?
        WHERE id = ? AND service_id = ? AND (status = 'running' OR status = 'cancelling')
        "#,
    )
    .bind(status.to_string())
    .bind(result)
    .bind(error_message)
    .bind(now.to_rfc3339())
    .bind(task_id)
    .bind(service_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    if result_rows.rows_affected() == 0 {
        let _ = tx.rollback().await;
        return Ok(false);
    }

    sqlx::query(
        r#"
        UPDATE services 
        SET agent_current_load = CASE WHEN agent_current_load > 0 THEN agent_current_load - 1 ELSE 0 END,
            agent_status = CASE
                WHEN agent_current_load > 0 AND agent_current_load - 1 < agent_capacity AND agent_status = 'busy' THEN 'online'
                WHEN agent_status = 'offline' THEN 'offline'
                ELSE agent_status
            END
        WHERE id = ?
        "#,
    )
    .bind(service_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    tx.commit().await.map_err(AppError::Database)?;

    Ok(true)
}

/// 更新服务心跳
/// 只更新心跳时间戳，并发控制权完全交给 Agent
pub async fn update_service_heartbeat(pool: &SqlitePool, service_id: &str) -> Result<bool> {
    let now = Utc::now();

    let result = sqlx::query(
        r#"
        UPDATE services 
        SET agent_last_heartbeat = ?, 
            agent_status = CASE WHEN agent_status = 'offline' THEN 'online' ELSE agent_status END
        WHERE id = ?
        "#,
    )
    .bind(now.to_rfc3339())
    .bind(service_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(result.rows_affected() > 0)
}

/// 更新服务状态
async fn update_service_status(
    pool: &SqlitePool,
    service_id: &str,
    status: AgentStatus,
) -> Result<bool> {
    let result = sqlx::query("UPDATE services SET agent_status = ? WHERE id = ?")
        .bind(status.to_string())
        .bind(service_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

    Ok(result.rows_affected() > 0)
}

/// 更新服务负载
async fn update_service_load(
    pool: &SqlitePool,
    service_id: &str,
    current_load: Option<i64>,
    capacity: Option<i64>,
) -> Result<bool> {
    // 验证非负
    if let Some(load) = current_load {
        if load < 0 {
            return Err(AppError::BadRequest(
                "current_load cannot be negative".to_string(),
            ));
        }
    }
    if let Some(cap) = capacity {
        if cap <= 0 {
            return Err(AppError::BadRequest(
                "capacity must be positive".to_string(),
            ));
        }
    }
    // 验证 current_load <= capacity（如果两者都提供）
    if let (Some(load), Some(cap)) = (current_load, capacity) {
        if load > cap {
            return Err(AppError::BadRequest(format!(
                "current_load ({}) cannot exceed capacity ({})",
                load, cap
            )));
        }
    }

    let mut query_parts = vec![];
    if current_load.is_some() {
        query_parts.push("agent_current_load = ?");
    }
    if capacity.is_some() {
        query_parts.push("agent_capacity = ?");
    }

    if query_parts.is_empty() {
        return Ok(false);
    }

    let query = format!(
        "UPDATE services SET {} WHERE id = ?",
        query_parts.join(", ")
    );

    let mut sql = sqlx::query(&query);
    if let Some(load) = current_load {
        sql = sql.bind(load);
    }
    if let Some(cap) = capacity {
        sql = sql.bind(cap);
    }
    sql = sql.bind(service_id);

    sql.execute(pool).await.map_err(AppError::Database)?;
    Ok(true)
}

// ============================================================================
// 测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::service::{AgentStatus, Service};
    use crate::test_utils::setup_test_db;
    use chrono::Utc;

    /// 创建测试服务
    async fn create_test_service(pool: &SqlitePool) -> (String, String, String) {
        let service_id = format!("test-service-{}", uuid::Uuid::new_v4());
        let registration_token = format!("rt_{}", uuid::Uuid::new_v4());
        let api_key = format!(
            "ak_agent_{}",
            uuid::Uuid::new_v4().to_string().replace("-", "")
        );

        sqlx::query(
            r#"
            INSERT INTO services (id, name, description, usage, agent_api_key, registration_token,
                                  registration_status, agent_status, agent_capacity, agent_current_load, is_public, created_at)
            VALUES (?, ?, ?, ?, ?, ?, 'active', 'online', 1, 0, true, ?)
            "#
        )
        .bind(&service_id)
        .bind("Test Service")
        .bind("A test service")
        .bind("Test usage")
        .bind(&api_key)
        .bind(&registration_token)
        .bind(Utc::now())
        .execute(pool)
        .await
        .expect("Failed to create test service");

        (service_id, api_key, registration_token)
    }

    // ==================== 心跳上报负载测试 ====================

    /// 测试心跳上报正常负载
    #[sqlx::test]
    async fn test_heartbeat_with_load() {
        let pool = setup_test_db().await;
        let (service_id, _api_key, _) = create_test_service(&pool).await;

        // 发送心跳带上 current_load=2, capacity=5
        let result = update_service_load(&pool, &service_id, Some(2), Some(5)).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // 验证数据库中的 agent_current_load=2, agent_capacity=5
        let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
            .bind(&service_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(service.agent_current_load, 2);
        assert_eq!(service.agent_capacity, 5);
    }

    /// 测试心跳上报 busy 状态
    #[sqlx::test]
    async fn test_heartbeat_busy_status() {
        let pool = setup_test_db().await;
        let (service_id, _api_key, _) = create_test_service(&pool).await;

        // 发送心跳：current_load=5, capacity=5（满载）
        let result = update_service_load(&pool, &service_id, Some(5), Some(5)).await;
        assert!(result.is_ok());

        // 更新状态为 busy
        let result = update_service_status(&pool, &service_id, AgentStatus::Busy).await;
        assert!(result.is_ok());

        // 验证 agent_status 变为 busy
        let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
            .bind(&service_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(service.agent_status, AgentStatus::Busy);
        assert_eq!(service.agent_current_load, 5);
        assert_eq!(service.agent_capacity, 5);
    }

    /// 测试无效负载值被拒绝 - current_load = -1
    #[sqlx::test]
    async fn test_heartbeat_invalid_load_negative() {
        let pool = setup_test_db().await;
        let (service_id, _, _) = create_test_service(&pool).await;

        // 测试 current_load = -1 返回 BadRequest
        let result = update_service_load(&pool, &service_id, Some(-1), None).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::BadRequest(msg) => {
                assert!(msg.contains("negative"), "错误消息应包含 negative: {}", msg);
            }
            _ => panic!("期望是 BadRequest 错误类型"),
        }
    }

    /// 测试无效负载值被拒绝 - capacity = 0
    #[sqlx::test]
    async fn test_heartbeat_invalid_capacity_zero() {
        let pool = setup_test_db().await;
        let (service_id, _, _) = create_test_service(&pool).await;

        // 测试 capacity = 0 返回 BadRequest
        let result = update_service_load(&pool, &service_id, None, Some(0)).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::BadRequest(msg) => {
                assert!(msg.contains("positive"), "错误消息应包含 positive: {}", msg);
            }
            _ => panic!("期望是 BadRequest 错误类型"),
        }
    }

    /// 测试无效负载值被拒绝 - current_load > capacity
    #[sqlx::test]
    async fn test_heartbeat_invalid_load_exceeds_capacity() {
        let pool = setup_test_db().await;
        let (service_id, _, _) = create_test_service(&pool).await;

        // 测试 current_load > capacity 返回 BadRequest
        let result = update_service_load(&pool, &service_id, Some(10), Some(5)).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::BadRequest(msg) => {
                assert!(msg.contains("exceed"), "错误消息应包含 exceed: {}", msg);
            }
            _ => panic!("期望是 BadRequest 错误类型"),
        }
    }

    /// 测试心跳不带负载字段（向后兼容）
    #[sqlx::test]
    async fn test_heartbeat_without_load() {
        let pool = setup_test_db().await;
        let (service_id, _, _) = create_test_service(&pool).await;

        // 先设置初始负载值
        update_service_load(&pool, &service_id, Some(3), Some(10))
            .await
            .unwrap();

        // 只发送状态更新（不提供负载字段）
        let result = update_service_status(&pool, &service_id, AgentStatus::Online).await;
        assert!(result.is_ok());

        // 验证成功，不改变负载值
        let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
            .bind(&service_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(service.agent_status, AgentStatus::Online);
        assert_eq!(service.agent_current_load, 3); // 保持不变
        assert_eq!(service.agent_capacity, 10); // 保持不变
    }

    // ==================== 更新服务心跳测试 ====================

    #[sqlx::test]
    async fn test_update_service_heartbeat() {
        let pool = setup_test_db().await;
        let (service_id, _, _) = create_test_service(&pool).await;

        // 先将状态设置为 offline
        sqlx::query("UPDATE services SET agent_status = 'offline' WHERE id = ?")
            .bind(&service_id)
            .execute(&pool)
            .await
            .unwrap();

        // 更新心跳
        let result = update_service_heartbeat(&pool, &service_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // 验证状态已从 offline 变为 online
        let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
            .bind(&service_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(service.agent_status, AgentStatus::Online);
        assert!(service.agent_last_heartbeat.is_some());
    }

    // ==================== 任务相关数据库操作测试 ====================

    #[sqlx::test]
    async fn test_find_pending_tasks_by_service() {
        let pool = setup_test_db().await;
        let (service_id, _, _) = create_test_service(&pool).await;

        // 创建 pending 任务
        let task_id = format!("task-{}", uuid::Uuid::new_v4());
        let session_id = format!("session-{}", uuid::Uuid::new_v4());
        let input = serde_json::json!({
            "task_prompt": "Test task",
            "output_prompt": "Test output"
        });

        sqlx::query(
            r#"
            INSERT INTO tasks (id, user_id, service_id, status, input, session_id, created_at)
            VALUES (?, 'admin', ?, 'pending', ?, ?, datetime('now'))
            "#,
        )
        .bind(&task_id)
        .bind(&service_id)
        .bind(&input)
        .bind(&session_id)
        .execute(&pool)
        .await
        .unwrap();

        // 查找 pending 任务
        let tasks = find_pending_tasks_by_service(&pool, &service_id)
            .await
            .unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, task_id);
    }

    #[sqlx::test]
    async fn test_accept_and_complete_task() {
        let pool = setup_test_db().await;
        let (service_id, _, _) = create_test_service(&pool).await;

        // 创建 pending 任务
        let task_id = format!("task-{}", uuid::Uuid::new_v4());
        let session_id = format!("session-{}", uuid::Uuid::new_v4());
        let input = serde_json::json!({
            "task_prompt": "Test task",
            "output_prompt": "Test output"
        });

        sqlx::query(
            r#"
            INSERT INTO tasks (id, user_id, service_id, status, input, session_id, created_at)
            VALUES (?, 'admin', ?, 'pending', ?, ?, datetime('now'))
            "#,
        )
        .bind(&task_id)
        .bind(&service_id)
        .bind(&input)
        .bind(&session_id)
        .execute(&pool)
        .await
        .unwrap();

        // 接受任务
        let accepted = accept_task(&pool, &task_id, &service_id).await.unwrap();
        assert!(accepted);

        // 验证任务状态为 running
        let status: (String,) = sqlx::query_as("SELECT status FROM tasks WHERE id = ?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status.0, "running");

        // 完成任务
        let completed = complete_task(
            &pool,
            &task_id,
            &service_id,
            TaskStatus::Completed,
            Some(serde_json::json!({"result": "success"})),
            None,
        )
        .await
        .unwrap();
        assert!(completed);

        // 验证任务状态为 completed
        let status: (String,) = sqlx::query_as("SELECT status FROM tasks WHERE id = ?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status.0, "completed");
    }

    /// 创建指定容量的测试服务
    async fn create_test_service_with_capacity(pool: &SqlitePool, capacity: i64) -> (String, String, String) {
        let service_id = format!("test-service-{}", uuid::Uuid::new_v4());
        let registration_token = format!("rt_{}", uuid::Uuid::new_v4());
        let api_key = format!(
            "ak_agent_{}",
            uuid::Uuid::new_v4().to_string().replace("-", "")
        );

        sqlx::query(
            r#"
            INSERT INTO services (id, name, description, usage, agent_api_key, registration_token,
                                  registration_status, agent_status, agent_capacity, agent_current_load, is_public, created_at)
            VALUES (?, ?, ?, ?, ?, ?, 'active', 'online', ?, 0, true, ?)
            "#
        )
        .bind(&service_id)
        .bind("Test Service")
        .bind("A test service")
        .bind("Test usage")
        .bind(&api_key)
        .bind(&registration_token)
        .bind(capacity)
        .bind(Utc::now())
        .execute(pool)
        .await
        .expect("Failed to create test service");

        (service_id, api_key, registration_token)
    }

    /// 创建 pending 任务辅助函数
    async fn create_pending_task(pool: &SqlitePool, service_id: &str) -> String {
        let task_id = format!("task-{}", uuid::Uuid::new_v4());
        let session_id = format!("session-{}", uuid::Uuid::new_v4());
        let input = serde_json::json!({
            "task_prompt": "Test task",
            "output_prompt": "Test output"
        });

        sqlx::query(
            r#"
            INSERT INTO tasks (id, user_id, service_id, status, input, session_id, created_at)
            VALUES (?, 'admin', ?, 'pending', ?, ?, datetime('now'))
            "#,
        )
        .bind(&task_id)
        .bind(service_id)
        .bind(&input)
        .bind(&session_id)
        .execute(pool)
        .await
        .unwrap();

        task_id
    }

    #[sqlx::test]
    async fn test_accept_task_duplicate_fails() {
        let pool = setup_test_db().await;
        let (service_id, _, _) = create_test_service(&pool).await;
        let task_id = create_pending_task(&pool, &service_id).await;

        // 第一次 accept
        let accepted = accept_task(&pool, &task_id, &service_id).await.unwrap();
        assert!(accepted);

        // 验证 load = 1
        let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
            .bind(&service_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(service.agent_current_load, 1);

        // 第二次 accept 同一任务，应返回 Ok(false)
        let accepted2 = accept_task(&pool, &task_id, &service_id).await.unwrap();
        assert!(!accepted2);

        // 验证 load 没有继续增加
        let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
            .bind(&service_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(service.agent_current_load, 1);
    }

    #[sqlx::test]
    async fn test_complete_non_running_task_fails() {
        let pool = setup_test_db().await;
        let (service_id, _, _) = create_test_service(&pool).await;
        let task_id = create_pending_task(&pool, &service_id).await;

        // 直接 complete，不先 accept
        let completed = complete_task(
            &pool,
            &task_id,
            &service_id,
            TaskStatus::Completed,
            Some(serde_json::json!({"result": "success"})),
            None,
        )
        .await
        .unwrap();
        assert!(!completed);

        // 验证任务状态仍为 pending
        let status: (String,) = sqlx::query_as("SELECT status FROM tasks WHERE id = ?")
            .bind(&task_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status.0, "pending");
    }

    #[sqlx::test]
    async fn test_accept_task_increments_load() {
        let pool = setup_test_db().await;
        let (service_id, _, _) = create_test_service_with_capacity(&pool, 5).await;

        // accept 1 个任务
        let task_id1 = create_pending_task(&pool, &service_id).await;
        let accepted = accept_task(&pool, &task_id1, &service_id).await.unwrap();
        assert!(accepted);

        let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
            .bind(&service_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(service.agent_current_load, 1);
        assert_eq!(service.agent_status, AgentStatus::Online);

        // 再 accept 4 个任务
        for i in 2..=5 {
            let task_id = create_pending_task(&pool, &service_id).await;
            let accepted = accept_task(&pool, &task_id, &service_id).await.unwrap();
            assert!(accepted, "第 {} 个任务应 accept 成功", i);
        }

        let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
            .bind(&service_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(service.agent_current_load, 5);
        assert_eq!(service.agent_status, AgentStatus::Busy);
    }

    #[sqlx::test]
    async fn test_complete_task_decrements_load_and_status() {
        let pool = setup_test_db().await;
        let (service_id, _, _) = create_test_service_with_capacity(&pool, 5).await;

        // 1. 创建并 accept 5 个任务，验证 status=busy，load=5
        let mut task_ids = Vec::new();
        for _ in 0..5 {
            let task_id = create_pending_task(&pool, &service_id).await;
            let accepted = accept_task(&pool, &task_id, &service_id).await.unwrap();
            assert!(accepted);
            task_ids.push(task_id);
        }

        let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
            .bind(&service_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(service.agent_current_load, 5);
        assert_eq!(service.agent_status, AgentStatus::Busy);

        // 2. complete 其中 2 个任务，验证 status=online（因为 3 < 5），load=3
        for task_id in task_ids.iter().take(2) {
            let completed = complete_task(
                &pool,
                task_id,
                &service_id,
                TaskStatus::Completed,
                Some(serde_json::json!({"result": "success"})),
                None,
            )
            .await
            .unwrap();
            assert!(completed);
        }

        let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
            .bind(&service_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(service.agent_current_load, 3);
        assert_eq!(service.agent_status, AgentStatus::Online);

        // 3. complete 剩余 3 个任务，验证 status=online，load=0
        for task_id in task_ids.iter().skip(2) {
            let completed = complete_task(
                &pool,
                task_id,
                &service_id,
                TaskStatus::Completed,
                Some(serde_json::json!({"result": "success"})),
                None,
            )
            .await
            .unwrap();
            assert!(completed);
        }

        let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
            .bind(&service_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(service.agent_current_load, 0);
        assert_eq!(service.agent_status, AgentStatus::Online);
    }

    // ==================== 辅助函数测试 ====================

    #[test]
    fn test_task_complete_status_to_task_status() {
        assert_eq!(
            TaskCompleteStatus::Completed.to_task_status(),
            TaskStatus::Completed
        );
        assert_eq!(
            TaskCompleteStatus::Failed.to_task_status(),
            TaskStatus::Failed
        );
        assert_eq!(
            TaskCompleteStatus::Cancelled.to_task_status(),
            TaskStatus::Cancelled
        );
    }

    #[test]
    fn test_poll_request_deserialization() {
        let json = r#"{"current_load": 2, "available_capacity": 3}"#;
        let req: PollRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.current_load, Some(2));
        assert_eq!(req.available_capacity, Some(3));
    }

    #[test]
    fn test_complete_task_request_deserialization() {
        let json = r#"{
            "task_id": "task-123",
            "status": "completed",
            "output": {"result": "success"},
            "error_message": null,
            "file_ids": ["file-1", "file-2"]
        }"#;
        let req: CompleteTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.task_id, "task-123");
        assert!(req.output.is_some());
        assert_eq!(req.file_ids.len(), 2);
    }
}
