//! 应用状态

use crate::{config::AppConfig, db::Database, rate_limit::RateLimiter};
use std::sync::Arc;

/// 应用共享状态
#[derive(Debug, Clone)]
pub struct AppState {
    /// 应用配置
    pub config: Arc<AppConfig>,
    /// 数据库连接
    pub db: Database,
    /// API Key 限流器
    pub rate_limiter: Arc<RateLimiter>,
}

impl AppState {
    /// 创建新的应用状态
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        let db = Database::new(config.database_url()).await?;

        // 确保文件存储目录存在
        let file_storage_path = &config.task.file_storage_path;
        tokio::fs::create_dir_all(file_storage_path)
            .await
            .map_err(|e| anyhow::anyhow!("无法创建文件存储目录 '{}': {}", file_storage_path, e))?;

        tracing::info!("文件存储目录: {}", file_storage_path);

        Ok(Self {
            config: Arc::new(config),
            db,
            rate_limiter: Arc::new(RateLimiter::new()),
        })
    }

    /// 获取文件存储基础路径
    pub fn file_storage_path(&self) -> &str {
        &self.config.task.file_storage_path
    }

    /// 获取最大文件大小（字节）
    pub fn max_file_size_bytes(&self) -> usize {
        self.config.task.max_file_size_mb * 1024 * 1024
    }
}
