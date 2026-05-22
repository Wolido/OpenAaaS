//! 客户端API处理器
//!
//! 提供给前端/客户端使用的API

use axum::{
    extract::{Extension, Multipart, Path, Query, State},
    http::StatusCode,
    middleware,
    routing::{get, post, put},
    Json, Router,
};
use tokio::io::AsyncWriteExt;
use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, Result},
    models::{
        file::{FileCreatedBy, TaskFile},
        service::{Service, ServiceListItem, ServiceUsageResponse},
        task::{ListTasksQuery, Task, TaskResponse, TaskStatus},
        user::{CreateUserRequest, UserResponse},
    },
    state::AppState,
};

/// 授权请求
#[derive(Debug, serde::Deserialize)]
struct GrantPermissionRequest {
    user_id: String,
}

/// 更新用户资料请求
#[derive(Debug, serde::Deserialize)]
struct UpdateProfileRequest {
    name: String,
}

/// 验证用户名格式
fn validate_username(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("用户名不能为空".to_string()));
    }
    if name.len() > 64 {
        return Err(AppError::BadRequest("用户名不能超过64个字符".to_string()));
    }
    // 特殊字符检查（参考 sanitize_filename）
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(AppError::BadRequest("用户名包含非法字符".to_string()));
    }
    if name.contains('\0') {
        return Err(AppError::BadRequest("用户名包含空字节".to_string()));
    }
    Ok(name)
}

/// 验证并净化文件名
fn sanitize_filename(name: &str) -> Result<String> {
    let name = name.trim();
    
    if name.is_empty() {
        return Err(AppError::BadRequest("文件名不能为空".to_string()));
    }
    if name.len() > 255 {
        return Err(AppError::BadRequest("文件名过长".to_string()));
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(AppError::BadRequest("文件名包含非法字符".to_string()));
    }
    if name.contains('\0') {
        return Err(AppError::BadRequest("文件名包含空字节".to_string()));
    }
    
    // 获取文件名部分（不含路径）
    let filename = std::path::Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| AppError::BadRequest("无效的文件名".to_string()))?;
    
    Ok(filename.to_string())
}

/// 服务负载响应
#[derive(Debug, Serialize)]
pub struct ServiceLoadResponse {
    pub service_id: String,
    pub name: String,
    pub agent_status: String,
    pub capacity: i64,
    pub current_load: i64,
    pub available_slots: i64,
    pub pending_tasks: i64,
    pub running_tasks: i64,
    pub last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
}

/// 客户端路由
pub fn routes(state: AppState) -> Router<AppState> {
    // 需要 Client 鉴权的路由
    let authenticated_routes = Router::new()
        // 任务管理 - 需要 Client 鉴权
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/{id}", get(get_task))
        .route("/tasks/{id}/cancel", post(cancel_task))
        // 服务列表 - 需要 Client 鉴权
        .route("/services", get(list_services))
        // 服务 usage 详情 - 需要 Client 鉴权
        .route("/services/{id}/usage", get(get_service_usage))
        // 服务负载查询 - 需要 Client 鉴权
        .route("/services/{service_id}/load", get(get_service_load_handler))
        // 服务授权 - 需要管理员权限
        .route("/services/{service_id}/grant", post(grant_service_permission))
        // 用户资料管理
        .route("/profile", put(update_profile))
        .layer(middleware::from_fn_with_state(state, crate::auth::require_auth));

    // 公开路由
    let public_routes = Router::new()
        // 用户认证 - 公开
        .route("/auth/register", post(register))
        // 健康检查 - 公开
        .route("/health", get(health_check))
        // 服务状态 - 公开
        .route("/status", get(service_status));

    authenticated_routes.merge(public_routes)
}

/// 健康检查
async fn health_check(State(_state): State<AppState>) -> Result<Json<serde_json::Value>> {
    Ok(Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

/// 创建任务（支持 multipart/form-data 上传文件）
async fn create_task(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    mut multipart: Multipart,
) -> Result<Json<TaskResponse>> {
    // 解析 multipart 表单字段
    let mut service_id: Option<String> = None;
    let mut task_prompt: Option<String> = None;
    let mut output_prompt: Option<String> = None;
    let mut session_id: Option<String> = None;

    // 收集上传的文件信息：(临时路径, 文件名, mime_type, 文件大小)
    let mut uploaded_files: Vec<(std::path::PathBuf, String, Option<String>, usize)> = Vec::new();

    let max_size = state.max_file_size_bytes();
    let storage_base = state.file_storage_path();

    // 流式处理 multipart 表单
    while let Some(mut field) = multipart.next_field().await.map_err(|e| {
        AppError::BadRequest(format!("解析上传数据失败: {}", e))
    })? {
        let name = field.name().map(|s| s.to_string());

        match name.as_deref() {
            Some("service_id") => {
                service_id = Some(field.text().await.map_err(|e| {
                    AppError::BadRequest(format!("读取 service_id 失败: {}", e))
                })?);
            }
            Some("task_prompt") => {
                task_prompt = Some(field.text().await.map_err(|e| {
                    AppError::BadRequest(format!("读取 task_prompt 失败: {}", e))
                })?);
            }
            Some("output_prompt") => {
                output_prompt = Some(field.text().await.map_err(|e| {
                    AppError::BadRequest(format!("读取 output_prompt 失败: {}", e))
                })?);
            }
            Some("session_id") => {
                let id = field.text().await.map_err(|e| {
                    AppError::BadRequest(format!("读取 session_id 失败: {}", e))
                })?;
                // 验证格式：长度不超过64，只允许字母数字和下划线、连字符
                if !id.trim().is_empty() 
                    && !id.contains("..") 
                    && !id.contains('/') 
                    && id.len() <= 64
                    && id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                    session_id = Some(id);
                } else {
                    tracing::warn!("Invalid session_id '{}', generating new one", id);
                }
            }
            Some("files") => {
                // 处理文件上传
                let raw_filename = field.file_name().map(|s| s.to_string());
                let filename = match raw_filename {
                    Some(name) => sanitize_filename(&name)?,
                    None => "unnamed".to_string(),
                };
                let mime_type = field.content_type().map(|s| s.to_string());

                // 生成临时任务ID（用于存储路径）
                let temp_task_id = format!("create_task_{}", Uuid::new_v4());

                // 生成文件ID并创建临时文件
                let file_id = Uuid::new_v4().to_string();
                let tmp_dir = std::path::PathBuf::from(storage_base)
                    .join(".tmp")
                    .join(&temp_task_id);
                let temp_path = tmp_dir.join(&file_id);

                // 创建临时目录
                tokio::fs::create_dir_all(&tmp_dir).await.map_err(|e| {
                    AppError::Internal(format!("创建临时目录失败: {}", e))
                })?;

                // 创建临时文件
                let mut file = tokio::fs::File::create(&temp_path).await.map_err(|e| {
                    AppError::Internal(format!("创建临时文件失败: {}", e))
                })?;

                let mut total_size: usize = 0;

                // 流式读取并写入
                while let Some(chunk) = field.chunk().await.map_err(|e| {
                    AppError::BadRequest(format!("读取上传数据失败: {}", e))
                })? {
                    let chunk_size = chunk.len();
                    total_size += chunk_size;

                    // 边读边检查大小
                    if total_size > max_size {
                        let _ = tokio::fs::remove_file(&temp_path).await;
                        let _ = tokio::fs::remove_dir_all(&tmp_dir).await;
                        return Err(AppError::BadRequest(format!(
                            "文件大小超过限制: {} > {} MB",
                            total_size / 1024 / 1024,
                            max_size / 1024 / 1024
                        )));
                    }

                    file.write_all(&chunk).await.map_err(|e| {
                        AppError::Internal(format!("写入文件失败: {}", e))
                    })?;
                }

                file.flush().await.map_err(|e| {
                    AppError::Internal(format!("刷新文件失败: {}", e))
                })?;

                // 保存文件信息，等待任务创建后再处理
                uploaded_files.push((temp_path, filename, mime_type, total_size));
            }
            _ => {
                // 消耗其他字段
                while let Some(_) = field.chunk().await.map_err(|e| {
                    AppError::BadRequest(format!("读取字段失败: {}", e))
                })? {}
            }
        }
    }

    // 验证必需字段
    let service_id = service_id.ok_or_else(|| {
        AppError::BadRequest("缺少 service_id 字段".to_string())
    })?;
    let task_prompt = task_prompt.ok_or_else(|| {
        AppError::BadRequest("缺少 task_prompt 字段".to_string())
    })?;
    let output_prompt = output_prompt.ok_or_else(|| {
        AppError::BadRequest("缺少 output_prompt 字段".to_string())
    })?;

    // 1. 验证 service_id 是否存在
    let _service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&service_id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::BadRequest(format!("服务 '{}' 不存在", service_id)))?;

    // 2. 验证用户是否有权限使用该服务（公开服务、admin、或有授权）
    let has_permission = _service.is_public
        || auth_user.role == "admin"
        || sqlx::query_scalar::<_, i64>(
            "SELECT 1 FROM user_service_permissions WHERE user_id = ? AND service_id = ?"
        )
        .bind(&auth_user.user_id)
        .bind(&service_id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .is_some();
    
    if !has_permission {
        return Err(AppError::Forbidden);
    }

    // 3. 处理 session_id：验证或自动生成
    let session_id = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());

    // 4. 开启事务
    let mut tx = state.db.pool().begin().await.map_err(AppError::Database)?;

    // 5. 创建任务（始终为 pending 状态，等待 Agent 轮询领取）
    let mut task = Task::new(&auth_user.user_id, &service_id, Some(session_id));

    // 构建初始 input JSON（先不包含 file_ids，避免 task_files 外键先于 tasks 插入）
    task.input = Some(serde_json::json!({
        "task_prompt": task_prompt,
        "output_prompt": output_prompt,
        "input_files": [],
    }));
    task.status = TaskStatus::Pending;

    // 将 input/output 序列化为 JSON 字符串
    let input_json = task.input.as_ref().map(|v| v.to_string());
    let output_json = task.output.as_ref().map(|v| v.to_string());

    // 先插入任务记录（task_files 有 task_id 外键，必须先有 tasks）
    let task_result = sqlx::query(
        r#"
        INSERT INTO tasks 
            (id, user_id, service_id, status, input, output, session_id, output_format, created_at, assigned_at, started_at, completed_at, error_message, retry_count)
        VALUES 
            (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&task.id)
    .bind(&task.user_id)
    .bind(&task.service_id)
    .bind(&task.status.to_string())
    .bind(&input_json)
    .bind(&output_json)
    .bind(&task.session_id)
    .bind(&task.output_format.as_ref().unwrap_or(&String::new()))
    .bind(&task.created_at.to_rfc3339())
    .bind(task.assigned_at.as_ref().map(|d| d.to_rfc3339()))
    .bind(task.started_at.as_ref().map(|d| d.to_rfc3339()))
    .bind(task.completed_at.as_ref().map(|d| d.to_rfc3339()))
    .bind(&task.error_message)
    .bind(task.retry_count)
    .execute(&mut *tx)
    .await;

    if let Err(e) = task_result {
        let _ = tx.rollback().await;
        return Err(AppError::Database(e));
    }

    // 再处理上传文件：插入 task_files 并移动到最终存储路径
    let mut file_ids: Vec<String> = Vec::new();
    for (temp_path, filename, mime_type, size) in &uploaded_files {
        let file_id = Uuid::new_v4().to_string();
        let final_storage_path = format!("{}/{}", task.id, file_id);
        let final_path = std::path::PathBuf::from(storage_base).join(&final_storage_path);

        if let Some(parent) = final_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                AppError::Internal(format!("创建目录失败: {}", e))
            })?;
        }

        let file_record = TaskFile::new(
            &task.id,
            filename.clone(),
            mime_type.clone(),
            *size as i64,
            &final_storage_path,
            FileCreatedBy::Client,
        );

        let result = sqlx::query(
            r#"
            INSERT INTO task_files (id, task_id, filename, mime_type, size_bytes, storage_path, created_by, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&file_record.id)
        .bind(&file_record.task_id)
        .bind(&file_record.filename)
        .bind(&file_record.mime_type)
        .bind(file_record.size_bytes)
        .bind(&file_record.storage_path)
        .bind(&file_record.created_by.to_string())
        .bind(&file_record.created_at.to_rfc3339())
        .execute(&mut *tx)
        .await;

        match result {
            Ok(_) => match tokio::fs::rename(&temp_path, &final_path).await {
                Ok(_) => {
                    file_ids.push(file_record.id.clone());
                    tracing::info!(
                        "上传文件成功: task_id={}, file_id={}, filename={}, size={}",
                        task.id,
                        file_record.id,
                        filename,
                        size
                    );
                }
                Err(e) => {
                    let _ = tx.rollback().await;
                    let _ = tokio::fs::remove_file(&temp_path).await;
                    if let Some(parent) = temp_path.parent() {
                        let _ = tokio::fs::remove_dir_all(parent).await;
                    }
                    return Err(AppError::Internal(format!("移动文件失败: {}", e)));
                }
            },
            Err(e) => {
                let _ = tx.rollback().await;
                let _ = tokio::fs::remove_file(&temp_path).await;
                if let Some(parent) = temp_path.parent() {
                    let _ = tokio::fs::remove_dir_all(parent).await;
                }
                return Err(AppError::Database(e));
            }
        }
    }

    // 如果有上传文件，回写 tasks.input 中的 input_files
    if !file_ids.is_empty() {
        let updated_input = serde_json::json!({
            "task_prompt": task_prompt,
            "output_prompt": output_prompt,
            "input_files": file_ids,
        });
        task.input = Some(updated_input.clone());

        sqlx::query("UPDATE tasks SET input = ? WHERE id = ?")
            .bind(updated_input.to_string())
            .bind(&task.id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
    }

    // 清理临时目录
    for (temp_path, _, _, _) in &uploaded_files {
        if let Some(parent) = temp_path.parent() {
            let _ = tokio::fs::remove_dir_all(parent).await;
        }
    }

    // 提交事务
    tx.commit().await.map_err(AppError::Database)?;

    tracing::info!("创建任务: task_id={}, service_id={}", task.id, service_id);

    // 返回 TaskResponse
    Ok(Json(TaskResponse::from(task)))
}

/// 获取任务列表
async fn list_tasks(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<Vec<TaskResponse>>> {
    // 设置默认值
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    // 只返回当前用户的任务
    let tasks: Vec<Task> = if let Some(status) = query.status {
        sqlx::query_as::<_, Task>(
            r#"
            SELECT * FROM tasks 
            WHERE user_id = ? AND status = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(&auth_user.user_id)
        .bind(&status)
        .bind(limit)
        .bind(offset)
        .fetch_all(state.db.pool())
        .await
        .map_err(AppError::Database)?
    } else if let Some(service_id) = query.service_id {
        sqlx::query_as::<_, Task>(
            r#"
            SELECT * FROM tasks 
            WHERE user_id = ? AND service_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(&auth_user.user_id)
        .bind(&service_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(state.db.pool())
        .await
        .map_err(AppError::Database)?
    } else {
        sqlx::query_as::<_, Task>(
            r#"
            SELECT * FROM tasks 
            WHERE user_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(&auth_user.user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(state.db.pool())
        .await
        .map_err(AppError::Database)?
    };

    // 转换为 TaskResponse 列表
    let responses: Vec<TaskResponse> = tasks.into_iter().map(TaskResponse::from).collect();

    Ok(Json(responses))
}

/// 获取单个任务
async fn get_task(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<TaskResponse>> {
    // 根据 task_id 查询
    let task: Task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
        .bind(&id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // 验证任务属于当前用户（admin 可查看所有任务）
    if task.user_id != auth_user.user_id && auth_user.role != "admin" {
        return Err(AppError::Forbidden);
    }

    Ok(Json(TaskResponse::from(task)))
}

/// 取消任务
async fn cancel_task(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<TaskResponse>> {
    use sqlx::Row;

    // 1. 验证任务存在且属于当前用户（只读检查）
    let task: Task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
        .bind(&id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    if task.user_id != auth_user.user_id && auth_user.role != "admin" {
        return Err(AppError::Forbidden);
    }

    // 2. 检查是否可以取消（提前拒绝不可取消的状态）
    if !task.status.can_cancel() {
        return Err(AppError::BadRequest(format!(
            "当前状态 {:?} 的任务无法取消",
            task.status
        )));
    }

    let now = Utc::now();

    // 3. 原子更新：根据当前状态决定新状态
    let result = sqlx::query(
        r#"
        UPDATE tasks 
        SET status = CASE 
                WHEN status = 'pending' THEN 'cancelled'
                WHEN status = 'running' THEN 'cancelling'
            END,
            completed_at = CASE 
                WHEN status = 'pending' THEN ?
                ELSE NULL 
            END
        WHERE id = ? AND status IN ('pending', 'running')
        RETURNING status, completed_at
        "#
    )
    .bind(now.to_rfc3339())
    .bind(&id)
    .fetch_optional(state.db.pool())
    .await
    .map_err(AppError::Database)?;

    // 4. 检查更新是否成功
    let (new_status, completed_at) = match result {
        Some(row) => {
            let status: String = row.try_get("status").map_err(AppError::Database)?;
            let completed: Option<String> = row.try_get("completed_at").ok();
            (status, completed)
        }
        None => {
            // 更新失败，任务状态可能已改变
            return Err(AppError::BadRequest(
                "任务状态已改变，无法取消".to_string()
            ));
        }
    };

    tracing::info!("取消任务: task_id={}, new_status={}", id, new_status);

    // 5. 构造响应
    let mut response_task = task;
    response_task.status = match new_status.as_str() {
        "cancelled" => TaskStatus::Cancelled,
        "cancelling" => TaskStatus::Cancelling,
        _ => return Err(AppError::Internal("未知状态".to_string())),
    };
    response_task.completed_at = completed_at.and_then(|s| 
        chrono::DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))
    );

    Ok(Json(TaskResponse::from(response_task)))
}

/// 获取服务列表（含Agent状态和权限信息）
async fn list_services(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<Vec<ServiceListItem>>> {
    // 1. 查询所有服务
    let services: Vec<Service> = sqlx::query_as::<_, Service>(
        "SELECT * FROM services ORDER BY created_at DESC"
    )
    .fetch_all(state.db.pool())
    .await
    .map_err(AppError::Database)?;
    
    // 2. 查询当前用户有权限的受限服务
    let permitted_services: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT service_id FROM user_service_permissions WHERE user_id = ?"
    )
    .bind(&auth_user.user_id)
    .fetch_all(state.db.pool())
    .await
    .map_err(AppError::Database)?;
    
    // 3. 构建返回列表，包含权限信息
    let items: Vec<ServiceListItem> = services
        .into_iter()
        .map(|s| {
            let access_type = if s.is_public { "public" } else { "restricted" }.to_string();
            let has_permission = s.is_public || auth_user.role == "admin" || permitted_services.contains(&s.id);
            
            ServiceListItem {
                id: s.id,
                name: s.name,
                description: s.description,
                agent_status: s.agent_status,
                registration_status: s.registration_status,
                agent_last_heartbeat: s.agent_last_heartbeat,
                access_type,
                has_permission,
            }
        })
        .collect();
    
    Ok(Json(items))
}

/// 服务状态
pub async fn service_status(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    use std::time::Instant;

    // 1. 数据库健康检查
    let db_check_start = Instant::now();
    let db_result: std::result::Result<(), sqlx::Error> = sqlx::query_scalar("SELECT 1")
        .fetch_one(state.db.pool())
        .await
        .map(|_: i64| ());
    let db_response_ms = db_check_start.elapsed().as_millis() as u64;

    let db_healthy = db_result.is_ok();
    let mut db_component = json!({
        "healthy": db_healthy,
        "response_ms": db_response_ms,
    });
    if let Err(ref e) = db_result {
        db_component["error"] = json!(e.to_string());
    }

    // 2. 统计信息（无论数据库是否健康都尝试查询，失败则返回 null）
    let mut alerts: Vec<String> = Vec::new();
    let stats = if db_healthy {
        let services_total: std::result::Result<i64, _> = sqlx::query_scalar("SELECT COUNT(*) FROM services")
            .fetch_one(state.db.pool())
            .await;
        let services_online: std::result::Result<i64, _> =
            sqlx::query_scalar("SELECT COUNT(*) FROM services WHERE agent_status = 'online'")
                .fetch_one(state.db.pool())
                .await;
        let services_busy: std::result::Result<i64, _> =
            sqlx::query_scalar("SELECT COUNT(*) FROM services WHERE agent_status = 'busy'")
                .fetch_one(state.db.pool())
                .await;
        let services_offline: std::result::Result<i64, _> =
            sqlx::query_scalar("SELECT COUNT(*) FROM services WHERE agent_status = 'offline'")
                .fetch_one(state.db.pool())
                .await;

        let tasks_total: std::result::Result<i64, _> = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(state.db.pool())
            .await;
        let tasks_pending: std::result::Result<i64, _> =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status = 'pending'")
                .fetch_one(state.db.pool())
                .await;
        let tasks_running: std::result::Result<i64, _> =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status = 'running'")
                .fetch_one(state.db.pool())
                .await;
        let tasks_completed: std::result::Result<i64, _> =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status = 'completed'")
                .fetch_one(state.db.pool())
                .await;
        let tasks_failed: std::result::Result<i64, _> =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status = 'failed'")
                .fetch_one(state.db.pool())
                .await;

        let total = services_total.unwrap_or(0);
        let online = services_online.unwrap_or(0);
        let busy = services_busy.unwrap_or(0);
        let offline = services_offline.unwrap_or(0);
        let pending_tasks = tasks_pending.unwrap_or(0);
        let failed_tasks = tasks_failed.unwrap_or(0);

        if offline > 0 {
            alerts.push(format!("有 {} 个服务已离线", offline));
        }
        if pending_tasks > 0 && online == 0 && busy == 0 {
            alerts.push("有 pending 任务但无可用服务".to_string());
        }
        if failed_tasks > 0 {
            alerts.push(format!("有 {} 个任务失败", failed_tasks));
        }

        json!({
            "services": {
                "total": total,
                "online": online,
                "busy": busy,
                "offline": offline
            },
            "tasks": {
                "total": tasks_total.unwrap_or(0),
                "pending": pending_tasks,
                "running": tasks_running.unwrap_or(0),
                "completed": tasks_completed.unwrap_or(0),
                "failed": failed_tasks
            }
        })
    } else {
        serde_json::Value::Null
    };

    let healthy = db_healthy;
    let status_code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = json!({
        "healthy": healthy,
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "components": {
            "database": db_component
        },
        "stats": stats,
        "alerts": alerts
    });

    Ok((status_code, Json(body)))
}

/// 用户注册
async fn register(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<UserResponse>> {
    // 0. 验证用户名格式
    let name = validate_username(&req.name)?;

    // 1. 检查用户名是否已存在
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM users WHERE name = ? LIMIT 1"
    )
        .bind(name)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?;
    
    if existing.is_some() {
        return Err(AppError::Conflict(format!("用户名 '{}' 已存在", name)));
    }
    
    // 生成用户ID和API Key
    let user_id = uuid::Uuid::new_v4().to_string();
    let api_key = format!("ak_client_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
    let now = Utc::now();
    
    let secret_key = state.config.secret_key.as_deref()
        .ok_or(AppError::Internal("secret_key 未配置".to_string()))?;
    let api_key_hash = crate::auth::hash_api_key(secret_key, &api_key);
    
    // 插入数据库
    sqlx::query(
        r#"
        INSERT INTO users (id, api_key, name, role, created_at)
        VALUES (?, ?, ?, 'client', ?)
        "#
    )
    .bind(&user_id)
    .bind(&api_key_hash)
    .bind(name)
    .bind(now.to_rfc3339())
    .execute(state.db.pool())
    .await
    .map_err(AppError::Database)?;

    tracing::info!("用户注册: user_id={}, name={}", user_id, name);
    
    Ok(Json(UserResponse {
        id: user_id,
        name: name.to_string(),
        api_key,
        role: "client".to_string(),
        created_at: now.to_rfc3339(),
    }))
}

/// 更新用户资料（改名）
/// PUT /api/v1/client/profile
async fn update_profile(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UserResponse>> {
    // 0. 验证用户名格式
    let name = validate_username(&req.name)?;

    // 1. 检查新用户名是否已被其他用户使用
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM users WHERE name = ? AND id != ? LIMIT 1"
    )
    .bind(name)
    .bind(&auth_user.user_id)
    .fetch_optional(state.db.pool())
    .await
    .map_err(AppError::Database)?;

    if existing.is_some() {
        return Err(AppError::Conflict(format!("用户名 '{}' 已被使用", name)));
    }

    // 2. 更新用户名
    sqlx::query("UPDATE users SET name = ? WHERE id = ?")
        .bind(name)
        .bind(&auth_user.user_id)
        .execute(state.db.pool())
        .await
        .map_err(AppError::Database)?;

    // 3. 查询更新后的用户信息
    let user: crate::models::user::User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&auth_user.user_id)
        .fetch_one(state.db.pool())
        .await
        .map_err(AppError::Database)?;

    Ok(Json(UserResponse {
        id: user.id,
        name: user.name,
        api_key: String::new(),
        role: user.role.to_string(),
        created_at: user.created_at.to_rfc3339(),
    }))
}

/// 获取服务负载详情
/// GET /api/v1/client/services/{service_id}/load
pub async fn get_service_load_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(service_id): Path<String>,
) -> Result<Json<ServiceLoadResponse>> {
    // 1. 检查服务是否存在
    let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&service_id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // 2. 检查用户是否有权限使用该服务（公开服务、admin、或有授权）
    let has_permission = service.is_public
        || auth_user.role == "admin"
        || sqlx::query_scalar::<_, i64>(
            "SELECT 1 FROM user_service_permissions WHERE user_id = ? AND service_id = ?"
        )
        .bind(&auth_user.user_id)
        .bind(&service_id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .is_some();

    if !has_permission {
        return Err(AppError::Forbidden);
    }

    // 3. 查询 pending 任务数
    let pending_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE service_id = ? AND status = 'pending'"
    )
    .bind(&service_id)
    .fetch_one(state.db.pool())
    .await
    .map_err(AppError::Database)?;

    // 4. 查询活跃任务数（running + cancelling 都占用执行槽位）
    let running_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tasks WHERE service_id = ? AND status IN ('running', 'cancelling')"
    )
    .bind(&service_id)
    .fetch_one(state.db.pool())
    .await
    .map_err(AppError::Database)?;

    // 5. 计算可用槽位和预估等待时间
    let capacity = service.agent_capacity;
    let current_load = service.agent_current_load;
    let available_slots = if capacity >= current_load {
        capacity - current_load
    } else {
        tracing::warn!(
            "Data inconsistency: capacity({}) < current_load({}) for service {}",
            capacity, current_load, service_id
        );
        0
    };
    Ok(Json(ServiceLoadResponse {
        service_id: service.id,
        name: service.name,
        agent_status: service.agent_status.to_string(),
        capacity,
        current_load,
        available_slots,
        pending_tasks: pending_count,
        running_tasks: running_count,
        last_heartbeat: service.agent_last_heartbeat,
    }))
}

/// 获取单个服务的 usage（长正文）
async fn get_service_usage(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(service_id): Path<String>,
) -> Result<Json<ServiceUsageResponse>> {
    // 1. 查询服务是否存在
    let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&service_id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // 2. 检查用户权限（公开服务、admin、或有授权）
    let has_permission = service.is_public
        || auth_user.role == "admin"
        || sqlx::query_scalar::<_, i64>(
            "SELECT 1 FROM user_service_permissions WHERE user_id = ? AND service_id = ?"
        )
        .bind(&auth_user.user_id)
        .bind(&service_id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .is_some();

    if !has_permission {
        return Err(AppError::Forbidden);
    }

    Ok(Json(ServiceUsageResponse {
        id: service.id,
        name: service.name,
        usage: service.usage,
    }))
}

/// 管理员给 Client 授权使用受限服务
/// POST /api/v1/client/services/{service_id}/grant
async fn grant_service_permission(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(service_id): Path<String>,
    Json(req): Json<GrantPermissionRequest>,
) -> Result<Json<serde_json::Value>> {
    // 1. 验证当前用户是管理员
    if auth_user.role != "admin" {
        return Err(AppError::Forbidden);
    }
    
    // 2. 验证服务存在且是受限服务
    let service: Service = sqlx::query_as::<_, Service>(
        "SELECT * FROM services WHERE id = ?"
    )
    .bind(&service_id)
    .fetch_optional(state.db.pool())
    .await
    .map_err(AppError::Database)?
    .ok_or(AppError::NotFound)?;
    
    if service.is_public {
        return Err(AppError::BadRequest("公开服务无需授权".to_string()));
    }
    
    // 3. 验证目标用户存在
    let _target_user: crate::models::user::User = sqlx::query_as::<_, crate::models::user::User>(
        "SELECT * FROM users WHERE id = ?"
    )
    .bind(&req.user_id)
    .fetch_optional(state.db.pool())
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::BadRequest("目标用户不存在".to_string()))?;
    
    // 4. 插入或更新权限记录
    let permission_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();
    
    sqlx::query(
        r#"
        INSERT INTO user_service_permissions (id, user_id, service_id, granted_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(user_id, service_id) DO UPDATE SET
            granted_at = excluded.granted_at
        "#
    )
    .bind(&permission_id)
    .bind(&req.user_id)
    .bind(&service_id)
    .bind(now.to_rfc3339())
    .execute(state.db.pool())
    .await
    .map_err(AppError::Database)?;

    tracing::info!("授权服务权限: user_id={}, service_id={}", req.user_id, service_id);
    
    Ok(Json(serde_json::json!({
        "granted": true,
        "user_id": req.user_id,
        "service_id": service_id,
        "service_name": service.name
    })))
}

// ============================================================================
// 测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::setup_test_db;
    use crate::models::service::Service;
    use chrono::Utc;
    use sqlx::SqlitePool;

    #[cfg(test)]
    fn test_hash(key: &str) -> String {
        crate::auth::hash_api_key("test-secret-key-for-unit-tests-only", key)
    }

    /// 创建已注册的服务（Agent 已注册）
    async fn create_registered_service(pool: &SqlitePool) -> (String, String) {
        let service_id = format!("test-service-{}", uuid::Uuid::new_v4());
        let api_key = format!("ak_agent_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        
        sqlx::query(
            r#"
            INSERT INTO services (id, name, description, usage, agent_api_key, registration_status,
                                  agent_status, agent_capacity, agent_current_load, is_public, created_at)
            VALUES (?, ?, ?, ?, ?, 'active', 'online', 5, 0, true, ?)
            "#
        )
        .bind(&service_id)
        .bind("Test Service")
        .bind("A test service")
        .bind("Test usage")
        .bind(test_hash(&api_key))
        .bind(Utc::now())
        .execute(pool)
        .await
        .expect("Failed to create registered service");
        
        (service_id, api_key)
    }

    /// 创建受限服务（非公开）
    async fn create_restricted_service(pool: &SqlitePool) -> (String, String) {
        let service_id = format!("test-service-{}", uuid::Uuid::new_v4());
        let api_key = format!("ak_agent_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        
        sqlx::query(
            r#"
            INSERT INTO services (id, name, description, usage, agent_api_key, registration_status,
                                  agent_status, agent_capacity, agent_current_load, is_public, created_at)
            VALUES (?, ?, ?, ?, ?, 'active', 'online', 5, 0, false, ?)
            "#
        )
        .bind(&service_id)
        .bind("Restricted Service")
        .bind("A restricted service")
        .bind("Test usage")
        .bind(test_hash(&api_key))
        .bind(Utc::now())
        .execute(pool)
        .await
        .expect("Failed to create restricted service");
        
        (service_id, api_key)
    }

    /// 创建测试用户
    async fn create_test_user(pool: &SqlitePool, role: &str) -> (String, String) {
        let user_id = format!("user-{}", uuid::Uuid::new_v4());
        let api_key = format!("ak_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        
        sqlx::query(
            "INSERT INTO users (id, api_key, name, role, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&user_id)
        .bind(test_hash(&api_key))
        .bind("Test User")
        .bind(role)
        .bind(Utc::now())
        .execute(pool)
        .await
        .expect("Failed to create test user");
        
        (user_id, api_key)
    }

    /// 创建测试任务
    async fn create_test_task(
        pool: &SqlitePool,
        service_id: &str,
        user_id: &str,
        status: &str,
    ) -> String {
        let task_id = format!("task-{}", uuid::Uuid::new_v4());
        let session_id = format!("session-{}", uuid::Uuid::new_v4());
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
        .bind(&input)
        .bind(&session_id)
        .execute(pool)
        .await
        .expect("Failed to create test task");
        
        task_id
    }

    /// 授权用户访问服务
    async fn grant_permission(pool: &SqlitePool, user_id: &str, service_id: &str) {
        let permission_id = format!("perm-{}", uuid::Uuid::new_v4());
        
        sqlx::query(
            "INSERT INTO user_service_permissions (id, user_id, service_id, granted_at) VALUES (?, ?, ?, ?)"
        )
        .bind(&permission_id)
        .bind(user_id)
        .bind(service_id)
        .bind(Utc::now())
        .execute(pool)
        .await
        .expect("Failed to grant permission");
    }

    // ==================== 服务负载查询测试 ====================

    /// 测试获取服务负载详情
    #[sqlx::test]
    async fn test_get_service_load_success() {
        let pool = setup_test_db().await;
        let (service_id, _) = create_registered_service(&pool).await;
        let (user_id, _) = create_test_user(&pool, "client").await;
        
        // 更新服务负载
        sqlx::query("UPDATE services SET agent_capacity = 5, agent_current_load = 2 WHERE id = ?")
            .bind(&service_id)
            .execute(&pool)
            .await
            .unwrap();
        
        // 创建一些 pending 和 running 任务
        create_test_task(&pool, &service_id, &user_id, "pending").await;
        create_test_task(&pool, &service_id, &user_id, "pending").await;
        create_test_task(&pool, &service_id, &user_id, "running").await;
        
        // 查询服务
        let service: Service = sqlx::query_as::<_, Service>(
            "SELECT * FROM services WHERE id = ?"
        )
        .bind(&service_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        
        // 查询 pending 任务数
        let pending_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE service_id = ? AND status = 'pending'"
        )
        .bind(&service_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        
        // 查询 running 任务数
        let running_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE service_id = ? AND status = 'running'"
        )
        .bind(&service_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        
        // 计算可用槽位
        let capacity = service.agent_capacity;
        let current_load = service.agent_current_load;
        let available_slots = capacity - current_load;
        
        // 验证返回：capacity=5, current_load=2, pending_tasks=2, running_tasks=1
        assert_eq!(capacity, 5);
        assert_eq!(current_load, 2);
        assert_eq!(available_slots, 3);
        assert_eq!(pending_count, 2);
        assert_eq!(running_count, 1);
    }

    /// 测试负载为0的情况
    #[sqlx::test]
    async fn test_get_service_load_zero_capacity() {
        let pool = setup_test_db().await;
        let (service_id, _) = create_registered_service(&pool).await;
        
        // Agent 上报 capacity=0
        sqlx::query("UPDATE services SET agent_capacity = 0, agent_current_load = 0 WHERE id = ?")
            .bind(&service_id)
            .execute(&pool)
            .await
            .unwrap();
        
        // 查询服务
        let service: Service = sqlx::query_as::<_, Service>(
            "SELECT * FROM services WHERE id = ?"
        )
        .bind(&service_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        
        // 验证 available_slots=0
        let available_slots = if service.agent_capacity >= service.agent_current_load {
            service.agent_capacity - service.agent_current_load
        } else {
            0
        };
        assert_eq!(available_slots, 0);
        assert_eq!(service.agent_capacity, 0);
    }

    /// 测试服务负载查询权限控制 - 公开服务无需权限
    #[sqlx::test]
    async fn test_get_service_load_public_service() {
        let pool = setup_test_db().await;
        let (service_id, _) = create_registered_service(&pool).await;
        
        // 公开服务
        let service: Service = sqlx::query_as::<_, Service>(
            "SELECT * FROM services WHERE id = ?"
        )
        .bind(&service_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        
        assert!(service.is_public);
    }

    /// 测试服务负载查询权限控制 - 受限服务需要权限
    #[sqlx::test]
    async fn test_get_service_load_restricted_service() {
        let pool = setup_test_db().await;
        let (service_id, _) = create_restricted_service(&pool).await;
        let (user_id, _) = create_test_user(&pool, "client").await;
        
        // 受限服务
        let service: Service = sqlx::query_as::<_, Service>(
            "SELECT * FROM services WHERE id = ?"
        )
        .bind(&service_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        
        assert!(!service.is_public);
        
        // 用户无权限时
        let has_permission: bool = sqlx::query_scalar::<_, i64>(
            "SELECT 1 FROM user_service_permissions WHERE user_id = ? AND service_id = ?"
        )
        .bind(&user_id)
        .bind(&service_id)
        .fetch_optional(&pool)
        .await
        .unwrap()
        .is_some();
        
        assert!(!has_permission);
        
        // 授权用户
        grant_permission(&pool, &user_id, &service_id).await;
        
        // 用户现在有权限
        let has_permission: bool = sqlx::query_scalar::<_, i64>(
            "SELECT 1 FROM user_service_permissions WHERE user_id = ? AND service_id = ?"
        )
        .bind(&user_id)
        .bind(&service_id)
        .fetch_optional(&pool)
        .await
        .unwrap()
        .is_some();
        
        assert!(has_permission);
    }

    // ==================== 用户名验证测试 ====================

    #[test]
    fn test_validate_username_success() {
        let result = validate_username("valid_user");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "valid_user");
    }

    #[test]
    fn test_validate_username_empty() {
        let result = validate_username("");
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(msg) => assert!(msg.contains("不能为空")),
            _ => panic!("期望是 BadRequest 错误类型"),
        }
    }

    #[test]
    fn test_validate_username_whitespace() {
        let result = validate_username("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_username_too_long() {
        let long_name = "a".repeat(65);
        let result = validate_username(&long_name);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(msg) => assert!(msg.contains("64")),
            _ => panic!("期望是 BadRequest 错误类型"),
        }
    }

    #[test]
    fn test_validate_username_invalid_chars() {
        let result = validate_username("user/../name");
        assert!(result.is_err());
        
        let result = validate_username("user/name");
        assert!(result.is_err());
        
        let result = validate_username("user\\name");
        assert!(result.is_err());
    }

    // ==================== 文件名净化测试 ====================

    #[test]
    fn test_sanitize_filename_success() {
        let result = sanitize_filename("valid_file.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "valid_file.txt");
    }

    #[test]
    fn test_sanitize_filename_empty() {
        let result = sanitize_filename("");
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_filename_path_traversal() {
        let result = sanitize_filename("../etc/passwd");
        assert!(result.is_err());
        
        let result = sanitize_filename("file/../../../etc/passwd");
        assert!(result.is_err());
    }

    // ==================== 结构体序列化测试 ====================

    #[test]
    fn test_service_load_response_serialization() {
        let now = Utc::now();
        let response = ServiceLoadResponse {
            service_id: "service-123".to_string(),
            name: "Test Service".to_string(),
            agent_status: "online".to_string(),
            capacity: 5,
            current_load: 2,
            available_slots: 3,
            pending_tasks: 10,
            running_tasks: 2,
            last_heartbeat: Some(now),
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("service-123"));
        assert!(json.contains("Test Service"));
        assert!(json.contains("online"));
        assert!(json.contains("5"));
        assert!(json.contains("2"));
        assert!(json.contains("3"));
        assert!(json.contains("10"));
    }

    #[test]
    fn test_grant_permission_request_deserialization() {
        let json = r#"{"user_id": "user-123"}"#;
        let req: GrantPermissionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.user_id, "user-123");
    }

    #[test]
    fn test_update_profile_request_deserialization() {
        let json = r#"{"name": "New Name"}"#;
        let req: UpdateProfileRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "New Name");
    }
}
