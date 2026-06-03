//! 请求处理器模块

pub mod admin;
pub mod agent;
pub mod client;
pub mod discovery;
pub mod files;
pub mod services;

use crate::state::AppState;
use axum::{Json, Router, routing::get};

/// 创建API路由
pub fn routes(state: AppState) -> Router<AppState> {
    let agent_routes = agent::routes(state.clone());
    let client_routes = client::routes(state.clone());
    // Client 文件路由单独获取
    let client_file_routes = files::client_routes(state.clone());
    // Agent 文件路由（保持原有）
    let agent_file_routes = files::agent_routes(state.clone());

    // services 路由 - 需要管理员权限
    let services_routes = services::routes(state.clone());
    // admin 路由 - 需要管理员权限
    let admin_routes = admin::routes(state.clone());

    // 公开路由
    let public_routes = Router::new()
        .route("/", get(discovery::discovery))
        .route("/status", get(crate::handlers::client::service_status))
        .route("/api/v1/discovery", get(discovery::discovery))
        .route("/health", get(health_check));

    // 需要认证的路由
    let auth_routes = Router::new()
        .nest("/api/v1/agent", agent_routes)
        .nest("/api/v1/client", client_routes.merge(client_file_routes))
        .nest("/api/v1/files/agent", agent_file_routes)
        .nest("/api/v1/services", services_routes)
        .nest("/api/v1/admin", admin_routes);

    public_routes.merge(auth_routes)
}

/// 健康检查处理器
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy"
    }))
}
