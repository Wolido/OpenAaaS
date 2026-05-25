//! 服务管理API处理器
//!
//! 提供服务管理功能：
//! - 创建服务（admin）- 生成 registration_token
//! - 列出所有服务（含Agent状态）
//! - 获取单个服务详情
//! - 删除服务（admin）

use axum::{
    extract::{Path, Query, State},
    middleware,
    routing::get,
    Json, Router,
};
use chrono::Utc;

use crate::{
    auth::require_admin,
    error::{AppError, Result},
    models::service::{CreateServiceRequest, DeleteServiceResponse, Service, ServiceListItem, ServiceResponse, CreateServiceResponse},
    state::AppState,
};

/// 服务管理路由
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_services).post(create_service))
        .route("/{id}", get(get_service).delete(delete_service))
        .layer(middleware::from_fn(require_admin))
        .layer(middleware::from_fn_with_state(state, crate::auth::require_auth))
}

/// 列出所有服务
pub async fn list_services(
    State(state): State<AppState>,
) -> Result<Json<Vec<ServiceListItem>>> {
    let services: Vec<Service> = sqlx::query_as::<_, Service>(
        "SELECT * FROM services ORDER BY created_at DESC"
    )
    .fetch_all(state.db.pool())
    .await
    .map_err(AppError::Database)?;

    let items: Vec<ServiceListItem> = services
        .into_iter()
        .map(|s| ServiceListItem {
            id: s.id,
            name: s.name,
            description: s.description,
            agent_status: s.agent_status,
            registration_status: s.registration_status,
            agent_last_heartbeat: s.agent_last_heartbeat,
            access_type: if s.is_public { "public" } else { "restricted" }.to_string(),
            has_permission: true, // Admin 接口默认有所有权限
        })
        .collect();

    Ok(Json(items))
}

/// 创建服务
/// 生成 registration_token，返回给管理员
pub async fn create_service(
    State(state): State<AppState>,
    Json(req): Json<CreateServiceRequest>,
) -> Result<Json<CreateServiceResponse>> {
    // 生成或获取ID
    let id = req.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // 检查ID是否已存在
    let exists: bool = sqlx::query_scalar::<_, i32>("SELECT 1 FROM services WHERE id = ?")
        .bind(&id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .is_some();

    if exists {
        return Err(AppError::BadRequest(format!("服务 '{}' 已存在", id)));
    }

    // 生成 registration_token（格式：rt_ + uuid）
    let registration_token = format!("rt_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
    let now = Utc::now();

    // 插入新服务
    sqlx::query(
        r#"
        INSERT INTO services (
            id, name, description, usage,
            agent_status, agent_capacity, agent_current_load,
            registration_token, registration_status, is_public, created_at
        )
        VALUES (?, ?, ?, ?, 'offline', 1, 0, ?, 'pending', ?, ?)
        "#
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.usage)
    .bind(&registration_token)
    .bind(req.is_public)
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(AppError::Database)?;

    tracing::info!("创建服务: service_id={}, name={}", id, req.name);

    // 返回创建的服务（包含 registration_token）
    Ok(Json(CreateServiceResponse {
        id,
        name: req.name,
        description: req.description,
        usage: req.usage,
        registration_status: "pending".to_string(),
        registration_token,  // 重要：管理员需要保存此令牌给 Agent 使用
        created_at: now,
    }))
}


/// 获取单个服务详情
pub async fn get_service(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ServiceResponse>> {
    let service: Service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    Ok(Json(ServiceResponse::from(service)))
}

/// 删除服务查询参数
#[derive(Debug, serde::Deserialize)]
pub struct DeleteServiceQuery {
    #[serde(default)]
    pub force: bool,
}

/// 删除服务
pub async fn delete_service(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DeleteServiceQuery>,
) -> Result<Json<DeleteServiceResponse>> {
    // 先检查服务是否存在
    let exists: bool = sqlx::query_scalar::<_, i32>("SELECT 1 FROM services WHERE id = ?")
        .bind(&id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(AppError::Database)?
        .is_some();

    if !exists {
        return Err(AppError::NotFound);
    }

    if query.force {
        // 强制删除：取消活跃任务，保留历史任务，删除服务和权限
        let mut tx = state.db.pool().begin().await.map_err(AppError::Database)?;
        let now = Utc::now();
        let error_message = "Service was forcefully deleted by admin";

        // 1. 统计已结束任务数量（completed/failed/cancelled）
        let tasks_retained: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE service_id = ? AND status IN ('completed', 'failed', 'cancelled')"
        )
        .bind(&id)
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        // 2. 将 pending / running / cancelling 任务标记为 cancelled
        let result = sqlx::query(
            r#"
            UPDATE tasks
            SET status = 'cancelled', error_message = ?, completed_at = ?
            WHERE service_id = ? AND status IN ('pending', 'running', 'cancelling')
            "#
        )
        .bind(error_message)
        .bind(now)
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        let tasks_cancelled = result.rows_affected() as i64;

        // 3. 删除 user_service_permissions 中该服务的权限记录
        sqlx::query("DELETE FROM user_service_permissions WHERE service_id = ?")
            .bind(&id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        // 4. 物理删除 services 记录
        sqlx::query("DELETE FROM services WHERE id = ?")
            .bind(&id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        tx.commit().await.map_err(AppError::Database)?;

        tracing::info!(
            "删除服务: service_id={}, mode=force, tasks_cancelled={}, tasks_retained={}",
            id, tasks_cancelled, tasks_retained
        );

        Ok(Json(DeleteServiceResponse {
            deleted: true,
            tasks_cancelled,
            tasks_retained,
        }))
    } else {
        // 非强制删除：保持现有逻辑
        let mut tx = state.db.pool().begin().await.map_err(AppError::Database)?;

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE service_id = ?")
            .bind(&id)
            .fetch_one(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        if count > 0 {
            let _ = tx.rollback().await;
            return Err(AppError::BadRequest(format!(
                "无法删除：还有 {} 个任务关联此服务", count
            )));
        }

        sqlx::query("DELETE FROM services WHERE id = ?")
            .bind(&id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        tx.commit().await.map_err(AppError::Database)?;

        tracing::info!(
            "删除服务: service_id={}, mode=normal, tasks_cancelled=0, tasks_retained=0",
            id
        );

        Ok(Json(DeleteServiceResponse {
            deleted: true,
            tasks_cancelled: 0,
            tasks_retained: 0,
        }))
    }
}
