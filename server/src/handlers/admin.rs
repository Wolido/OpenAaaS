//! Admin API处理器
//!
//! 提供管理员功能：
//! - 列出所有用户
//! - 删除用户
//! - 修改用户角色
//! - 查看用户服务权限
//! - 撤销用户对服务的权限

use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    middleware,
    routing::{delete, get, put},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, QueryBuilder, Sqlite};

use crate::{
    auth::{AuthUser, require_admin},
    error::{AppError, Result},
    models::{
        service::UserPermissionResponse,
        user::{UpdateUserRoleRequest, User, UserResponse},
    },
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct AdminListTasksQuery {
    pub user_id: Option<String>,
    pub status: Option<String>,
    pub service_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AdminTaskResponse {
    pub id: String,
    pub user_id: String,
    pub user_name: Option<String>,
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

/// Admin 路由
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
        .route("/tasks", get(list_tasks))
        .route("/users/{id}", delete(delete_user))
        .route("/users/{id}/role", put(update_user_role))
        .route("/users/{id}/permissions", get(list_user_permissions))
        .route(
            "/services/{service_id}/users/{user_id}",
            delete(revoke_service_permission),
        )
        .layer(middleware::from_fn(require_admin))
        .layer(middleware::from_fn_with_state(
            state,
            crate::auth::require_auth,
        ))
}

/// 列出所有用户
pub async fn list_users(State(state): State<AppState>) -> Result<Json<Vec<UserResponse>>> {
    let users: Vec<User> =
        sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at DESC")
            .fetch_all(state.db.pool())
            .await
            .map_err(AppError::Database)?;

    let responses: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();

    Ok(Json(responses))
}

/// 管理员任务列表：可跨用户查看，并按用户/状态/服务过滤
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(query): Query<AdminListTasksQuery>,
) -> Result<Json<Vec<AdminTaskResponse>>> {
    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let offset = query.offset.unwrap_or(0).max(0);

    let mut builder = QueryBuilder::<Sqlite>::new(
        r#"
        SELECT
            t.id,
            t.user_id,
            u.name AS user_name,
            t.service_id,
            t.status,
            t.input,
            t.output,
            t.error_message,
            t.session_id,
            t.retry_count,
            t.created_at,
            t.assigned_at,
            t.started_at,
            t.completed_at
        FROM tasks t
        LEFT JOIN users u ON t.user_id = u.id
        "#,
    );

    let mut has_where = false;
    if let Some(user_id) = query.user_id.as_deref().filter(|v| !v.trim().is_empty()) {
        builder.push(if has_where { " AND " } else { " WHERE " });
        has_where = true;
        builder.push("t.user_id = ");
        builder.push_bind(user_id);
    }
    if let Some(status) = query.status.as_deref().filter(|v| !v.trim().is_empty()) {
        builder.push(if has_where { " AND " } else { " WHERE " });
        has_where = true;
        builder.push("t.status = ");
        builder.push_bind(status);
    }
    if let Some(service_id) = query.service_id.as_deref().filter(|v| !v.trim().is_empty()) {
        builder.push(if has_where { " AND " } else { " WHERE " });
        builder.push("t.service_id = ");
        builder.push_bind(service_id);
    }

    builder.push(" ORDER BY t.created_at DESC LIMIT ");
    builder.push_bind(limit);
    builder.push(" OFFSET ");
    builder.push_bind(offset);

    let tasks = builder
        .build_query_as::<AdminTaskResponse>()
        .fetch_all(state.db.pool())
        .await
        .map_err(AppError::Database)?;

    Ok(Json(tasks))
}

async fn user_exists(pool: &sqlx::SqlitePool, id: &str) -> Result<bool> {
    let exists: bool = sqlx::query_scalar::<_, i32>("SELECT 1 FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?
        .is_some();
    Ok(exists)
}

async fn service_exists(pool: &sqlx::SqlitePool, id: &str) -> Result<bool> {
    let exists: bool = sqlx::query_scalar::<_, i32>("SELECT 1 FROM services WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?
        .is_some();
    Ok(exists)
}

/// 删除用户
pub async fn delete_user(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    // 不能删除自己
    if auth_user.user_id == id {
        return Err(AppError::BadRequest(
            "不能删除当前登录的管理员账户".to_string(),
        ));
    }

    if !user_exists(state.db.pool(), &id).await? {
        return Err(AppError::NotFound);
    }

    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&id)
        .execute(state.db.pool())
        .await
        .map_err(AppError::Database)?;

    tracing::info!("删除用户: user_id={}", id);

    Ok(Json(json!({"deleted": true})))
}

/// 修改用户角色
pub async fn update_user_role(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserRoleRequest>,
) -> Result<Json<UserResponse>> {
    // 不能修改自己的角色
    if auth_user.user_id == id {
        return Err(AppError::BadRequest(
            "不能修改当前登录管理员的角色".to_string(),
        ));
    }

    let role_str = req.role.to_string();

    if !user_exists(state.db.pool(), &id).await? {
        return Err(AppError::NotFound);
    }

    sqlx::query("UPDATE users SET role = ? WHERE id = ?")
        .bind(&role_str)
        .bind(&id)
        .execute(state.db.pool())
        .await
        .map_err(AppError::Database)?;

    let user: User = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&id)
        .fetch_one(state.db.pool())
        .await
        .map_err(AppError::Database)?;

    tracing::info!("更新用户角色: user_id={}, role={}", id, role_str);

    Ok(Json(UserResponse::from(user)))
}

/// 查看用户的服务权限列表
pub async fn list_user_permissions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<UserPermissionResponse>>> {
    if !user_exists(state.db.pool(), &id).await? {
        return Err(AppError::NotFound);
    }

    let permissions: Vec<UserPermissionResponse> = sqlx::query_as::<_, UserPermissionResponse>(
        r#"
        SELECT 
            s.id as service_id,
            s.name as service_name,
            p.granted_at
        FROM user_service_permissions p
        JOIN services s ON p.service_id = s.id
        WHERE p.user_id = ?
        ORDER BY p.granted_at DESC
        "#,
    )
    .bind(&id)
    .fetch_all(state.db.pool())
    .await
    .map_err(AppError::Database)?;

    Ok(Json(permissions))
}

/// 撤销用户对服务的权限
pub async fn revoke_service_permission(
    State(state): State<AppState>,
    Path((service_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>> {
    if !user_exists(state.db.pool(), &user_id).await? {
        return Err(AppError::NotFound);
    }

    if !service_exists(state.db.pool(), &service_id).await? {
        return Err(AppError::NotFound);
    }

    sqlx::query("DELETE FROM user_service_permissions WHERE service_id = ? AND user_id = ?")
        .bind(&service_id)
        .bind(&user_id)
        .execute(state.db.pool())
        .await
        .map_err(AppError::Database)?;

    tracing::info!(
        "撤销服务权限: user_id={}, service_id={}",
        user_id,
        service_id
    );

    Ok(Json(json!({"revoked": true})))
}
