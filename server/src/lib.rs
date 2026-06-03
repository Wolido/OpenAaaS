//! OpenAaaS Server Library
//!
//! 提供异步Agent服务的服务端实现 - 一对一服务模型

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod main_support;
pub mod models;
pub mod state;

#[cfg(test)]
pub mod test_utils;

// 导出配置相关
pub use config::{AgentConfig, AppConfig, DatabaseConfig, ServerConfig, TaskConfig};

// 导出数据库
pub use db::Database;

// 导出错误类型
pub use error::{AppError, Result};

// 导出应用状态
pub use state::AppState;

// 导出模型
pub use models::{AgentStatus, Service, Task, TaskStatus, User, UserRole};
