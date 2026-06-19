//! 文件上传下载处理器

use axum::{
    Json, Router,
    body::Body,
    extract::{Extension, Multipart, Path, State},
    http::{StatusCode, header},
    middleware::{self},
    response::Response,
    routing::{get, post},
};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::{
    auth::{AuthAgent, AuthUser, agent_auth_middleware, require_auth},
    error::{AppError, Result},
    models::file::{
        FileCreatedBy, FileInfoResponse, FileListResponse, TaskFile, UploadFileResponse,
    },
    models::task::Task,
    rate_limit::{agent_rate_limit_middleware, client_rate_limit_middleware},
    state::AppState,
};

/// Client 文件路由 - 需要 Client 鉴权
/// 路由路径: /files/list/{task_id}, /files/{id}/download, /files/{id}
pub fn client_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/files/list/{task_id}", get(client_list_files))
        .route("/files/{id}/download", get(client_download_file))
        .route("/files/{id}", get(client_get_file_info))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .layer(middleware::from_fn_with_state(
            state,
            client_rate_limit_middleware,
        ))
}

/// Agent 文件路由 - 需要 Agent 鉴权
/// 路由路径: /{service_id}/files/upload, /{service_id}/files/list/{task_id}, etc.
pub fn agent_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{service_id}/files/upload", post(agent_upload_file))
        .route("/{service_id}/files/list/{task_id}", get(agent_list_files))
        .route(
            "/{service_id}/files/{id}/download",
            get(agent_download_file),
        )
        .route("/{service_id}/files/{id}", get(agent_get_file_info))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            agent_auth_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state,
            agent_rate_limit_middleware,
        ))
}

// ============================================================================
// 内部公共函数
// ============================================================================

/// 流式处理文件上传（内部公共函数）
/// 在一个循环中处理 multipart，支持流式写入和边读边检查大小
async fn handle_stream_upload(
    state: &AppState,
    mut multipart: Multipart,
    created_by: FileCreatedBy,
    expected_service_id: Option<&str>,
) -> Result<UploadFileResponse> {
    let mut task_id: Option<String> = None;
    let mut filename: Option<String> = None;
    let mut mime_type: Option<String> = None;
    let mut tmp_path: Option<std::path::PathBuf> = None;
    let mut total_size: usize = 0;
    let max_size = state.max_file_size_bytes();
    let storage_base = state.file_storage_path();

    // 流式处理 multipart 表单
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("解析上传数据失败: {}", e)))?
    {
        let name = field.name().map(|s| s.to_string());

        match name.as_deref() {
            Some("task_id") => {
                // 先读取 task_id
                task_id = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| AppError::BadRequest(format!("读取 task_id 失败: {}", e)))?,
                );
            }
            Some("file") => {
                filename = field.file_name().map(|s| s.to_string());
                mime_type = field.content_type().map(|s| s.to_string());

                // 此时必须有 task_id 才能创建临时文件路径
                let tid = task_id.as_ref().ok_or_else(|| {
                    AppError::BadRequest("字段顺序错误：task_id 必须在 file 之前".to_string())
                })?;

                // 验证任务存在并检查权限（在写入文件前验证）
                let task: Task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
                    .bind(tid)
                    .fetch_optional(state.db.pool())
                    .await
                    .map_err(AppError::Database)?
                    .ok_or(AppError::NotFound)?;

                if let Some(service_id) = expected_service_id {
                    if task.service_id != service_id {
                        return Err(AppError::Forbidden);
                    }
                }

                // 如果是 Client 上传，还需要验证用户权限
                if created_by == FileCreatedBy::Client {
                    // Client 权限验证需要在 handler 中进行，这里无法获取 auth_user
                    // 所以我们假设调用方已经验证了权限，或者通过其他方式验证
                }

                // 生成文件ID并创建临时文件
                let file_id = Uuid::new_v4().to_string();
                let tmp_dir = std::path::PathBuf::from(storage_base)
                    .join(".tmp")
                    .join(tid);
                let temp_path = tmp_dir.join(&file_id);

                // 清理该任务可能存在的旧临时文件（防止上传中断导致残留）
                if tmp_dir.exists() {
                    if let Ok(mut entries) = tokio::fs::read_dir(&tmp_dir).await {
                        while let Ok(Some(entry)) = entries.next_entry().await {
                            let path = entry.path();
                            if path.is_file() {
                                let _ = tokio::fs::remove_file(&path).await;
                            }
                        }
                    }
                }

                // 创建临时目录
                tokio::fs::create_dir_all(&tmp_dir)
                    .await
                    .map_err(|e| AppError::Internal(format!("创建临时目录失败: {}", e)))?;

                // 创建临时文件
                let mut file = tokio::fs::File::create(&temp_path)
                    .await
                    .map_err(|e| AppError::Internal(format!("创建临时文件失败: {}", e)))?;

                // 流式读取并写入
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("读取上传数据失败: {}", e)))?
                {
                    let chunk_size = chunk.len();
                    total_size += chunk_size;

                    // 边读边检查大小
                    if total_size > max_size {
                        // 删除临时文件
                        let _ = tokio::fs::remove_file(&temp_path).await;
                        return Err(AppError::BadRequest(format!(
                            "文件大小超过限制: {} > {} MB",
                            total_size / 1024 / 1024,
                            max_size / 1024 / 1024
                        )));
                    }

                    file.write_all(&chunk)
                        .await
                        .map_err(|e| AppError::Internal(format!("写入文件失败: {}", e)))?;
                }

                file.flush()
                    .await
                    .map_err(|e| AppError::Internal(format!("刷新文件失败: {}", e)))?;

                // 关闭文件句柄
                drop(file);

                tmp_path = Some(temp_path);
            }
            _ => {
                // 消耗其他字段
                while let Some(_) = field
                    .chunk()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("读取字段失败: {}", e)))?
                {
                }
            }
        }
    }

    // 验证必需字段
    let task_id = task_id.ok_or_else(|| AppError::BadRequest("缺少 task_id 字段".to_string()))?;
    let tmp_path = tmp_path.ok_or_else(|| AppError::BadRequest("缺少 file 字段".to_string()))?;
    let filename = filename.unwrap_or_else(|| "unnamed".to_string());

    // 生成最终文件ID和路径
    let file_id = Uuid::new_v4().to_string();
    let final_storage_path = format!("{}/{}", task_id, file_id);

    // 创建目标目录
    let final_path = std::path::PathBuf::from(storage_base).join(&final_storage_path);
    if let Some(parent) = final_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| AppError::Internal(format!("创建目录失败: {}", e)))?;
    }

    // 创建文件记录
    let file = TaskFile::new(
        &task_id,
        &filename,
        mime_type.clone(),
        total_size as i64,
        &final_storage_path,
        created_by,
    );

    // 先插入数据库记录
    let result = sqlx::query(
        r#"
        INSERT INTO task_files (id, task_id, filename, mime_type, size_bytes, storage_path, created_by, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&file.id)
    .bind(&file.task_id)
    .bind(&file.filename)
    .bind(&file.mime_type)
    .bind(file.size_bytes)
    .bind(&file.storage_path)
    .bind(&file.created_by.to_string())
    .bind(&file.created_at.to_rfc3339())
    .execute(state.db.pool())
    .await;

    match result {
        Ok(_) => {
            // 数据库插入成功，原子移动临时文件到最终位置
            tokio::fs::rename(&tmp_path, &final_path)
                .await
                .map_err(|e| {
                    // 尝试删除数据库记录
                    let pool = state.db.pool().clone();
                    let file_id = file.id.clone();
                    tokio::spawn(async move {
                        let _ = sqlx::query("DELETE FROM task_files WHERE id = ?")
                            .bind(&file_id)
                            .execute(&pool)
                            .await;
                    });
                    AppError::Internal(format!("移动文件失败: {}", e))
                })?;

            tracing::info!(
                "上传文件成功: task_id={}, file_id={}, filename={}, size={}, created_by={:?}",
                task_id,
                file.id,
                filename,
                total_size,
                created_by
            );

            Ok(UploadFileResponse::from(file))
        }
        Err(e) => {
            // 数据库插入失败，删除临时文件
            let _ = tokio::fs::remove_file(&tmp_path).await;
            Err(AppError::Database(e))
        }
    }
}

/// 内部下载处理逻辑（流式处理）
async fn handle_download_internal(state: &AppState, file: &TaskFile) -> Result<Response> {
    // 打开文件用于流式读取
    let file_handle = tokio::fs::File::open(file.full_storage_path(state.file_storage_path())?)
        .await
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                tracing::error!("文件不存在: path={}, error={}", file.storage_path, e);
                AppError::NotFound
            }
            _ => {
                tracing::error!("打开文件失败: path={}, error={}", file.storage_path, e);
                AppError::Internal(format!("打开文件失败: {}", e))
            }
        })?;

    // 创建流
    let stream = ReaderStream::new(file_handle);
    let body = Body::from_stream(stream);

    // 构建响应
    let mut response_builder = Response::builder().status(StatusCode::OK).header(
        header::CONTENT_TYPE,
        file.mime_type
            .as_deref()
            .unwrap_or("application/octet-stream"),
    );

    // 添加 Content-Disposition 头部，触发下载
    let filename = &file.filename;
    let encoded_filename = urlencoding::encode(filename);
    response_builder = response_builder.header(
        header::CONTENT_DISPOSITION,
        format!(
            r#"attachment; filename="{}"; filename*=UTF-8''{}"#,
            filename, encoded_filename
        ),
    );

    Ok(response_builder
        .body(body)
        .map_err(|e| AppError::Internal(format!("构建响应失败: {}", e)))?)
}

/// 验证 Client 对任务的权限
async fn verify_client_task_access(
    state: &AppState,
    task_id: &str,
    auth_user: &AuthUser,
) -> Result<Task> {
    let task: Task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
        .bind(task_id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    if task.user_id != auth_user.user_id && auth_user.role != "admin" {
        return Err(AppError::Forbidden);
    }

    Ok(task)
}

/// 验证 Agent 对文件的权限
async fn verify_agent_file_access(
    state: &AppState,
    file_id: &str,
    service_id: &str,
) -> Result<(TaskFile, Task)> {
    let file: TaskFile = sqlx::query_as::<_, TaskFile>("SELECT * FROM task_files WHERE id = ?")
        .bind(file_id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let task: Task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
        .bind(&file.task_id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    if task.service_id != service_id {
        return Err(AppError::Forbidden);
    }

    Ok((file, task))
}

// ============================================================================
// Client API
// ============================================================================

/// Client 下载文件
/// GET /api/v1/client/files/{id}/download
async fn client_download_file(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(file_id): Path<String>,
) -> Result<Response> {
    // 获取文件记录
    let file: TaskFile = sqlx::query_as::<_, TaskFile>("SELECT * FROM task_files WHERE id = ?")
        .bind(&file_id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // 验证任务属于当前用户
    verify_client_task_access(&state, &file.task_id, &auth_user).await?;

    // 使用流式下载
    handle_download_internal(&state, &file).await
}

/// Client 获取文件信息
/// GET /api/v1/client/files/{id}
async fn client_get_file_info(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(file_id): Path<String>,
) -> Result<Json<FileInfoResponse>> {
    // 获取文件记录
    let file: TaskFile = sqlx::query_as::<_, TaskFile>("SELECT * FROM task_files WHERE id = ?")
        .bind(&file_id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // 验证任务属于当前用户
    verify_client_task_access(&state, &file.task_id, &auth_user).await?;

    Ok(Json(FileInfoResponse::from(file)))
}

/// Client 列出任务的所有文件
/// GET /api/v1/client/files/list/{task_id}
async fn client_list_files(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(task_id): Path<String>,
) -> Result<Json<FileListResponse>> {
    // 验证任务存在且属于当前用户
    verify_client_task_access(&state, &task_id, &auth_user).await?;

    // 查询文件列表
    let files: Vec<TaskFile> = sqlx::query_as::<_, TaskFile>(
        "SELECT * FROM task_files WHERE task_id = ? ORDER BY created_at DESC",
    )
    .bind(&task_id)
    .fetch_all(state.db.pool())
    .await
    .map_err(AppError::Database)?;

    Ok(Json(FileListResponse {
        files: files.into_iter().map(FileInfoResponse::from).collect(),
    }))
}

// ============================================================================
// Agent API
// ============================================================================

/// Agent 上传文件
/// POST /api/v1/agent/{service_id}/files/upload
async fn agent_upload_file(
    State(state): State<AppState>,
    Extension(auth_agent): Extension<AuthAgent>,
    Path(service_id): Path<String>,
    multipart: Multipart,
) -> Result<Json<UploadFileResponse>> {
    // 验证 service_id 与认证信息匹配
    if auth_agent.agent_id != service_id {
        return Err(AppError::Forbidden);
    }

    // 使用流式上传处理器，Agent 权限在 handle_stream_upload 内部验证
    let result =
        handle_stream_upload(&state, multipart, FileCreatedBy::Agent, Some(&service_id)).await?;

    Ok(Json(result))
}

/// Agent 下载文件
/// GET /api/v1/agent/{service_id}/files/{id}/download
async fn agent_download_file(
    State(state): State<AppState>,
    Extension(auth_agent): Extension<AuthAgent>,
    Path((service_id, file_id)): Path<(String, String)>,
) -> Result<Response> {
    // 验证 service_id 与认证信息匹配
    if auth_agent.agent_id != service_id {
        return Err(AppError::Forbidden);
    }

    // 验证权限并获取文件
    let (file, _) = verify_agent_file_access(&state, &file_id, &service_id).await?;

    // 使用流式下载
    handle_download_internal(&state, &file).await
}

/// Agent 获取文件信息
/// GET /api/v1/agent/{service_id}/files/{id}
async fn agent_get_file_info(
    State(state): State<AppState>,
    Extension(auth_agent): Extension<AuthAgent>,
    Path((service_id, file_id)): Path<(String, String)>,
) -> Result<Json<FileInfoResponse>> {
    // 验证 service_id 与认证信息匹配
    if auth_agent.agent_id != service_id {
        return Err(AppError::Forbidden);
    }

    // 验证权限并获取文件
    let (file, _) = verify_agent_file_access(&state, &file_id, &service_id).await?;

    Ok(Json(FileInfoResponse::from(file)))
}

/// Agent 列出任务的所有文件
/// GET /api/v1/agent/{service_id}/files/list/{task_id}
async fn agent_list_files(
    State(state): State<AppState>,
    Extension(auth_agent): Extension<AuthAgent>,
    Path((service_id, task_id)): Path<(String, String)>,
) -> Result<Json<FileListResponse>> {
    // 验证 service_id 与认证信息匹配
    if auth_agent.agent_id != service_id {
        return Err(AppError::Forbidden);
    }

    // 验证任务存在且分配给当前 Agent
    let task: Task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
        .bind(&task_id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    if task.service_id != service_id {
        return Err(AppError::Forbidden);
    }

    // 查询文件列表
    let files: Vec<TaskFile> = sqlx::query_as::<_, TaskFile>(
        "SELECT * FROM task_files WHERE task_id = ? ORDER BY created_at DESC",
    )
    .bind(&task_id)
    .fetch_all(state.db.pool())
    .await
    .map_err(AppError::Database)?;

    Ok(Json(FileListResponse {
        files: files.into_iter().map(FileInfoResponse::from).collect(),
    }))
}
