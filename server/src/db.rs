//! 数据库连接和初始化管理

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;
use std::time::Duration;

/// 数据库连接池包装
#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// 创建新的数据库连接
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(30))
            .connect_with(options)
            .await?;
        
        Ok(Self { pool })
    }
    
    /// 获取连接池
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
    
    /// 初始化数据库表（首次运行时自动创建）
    pub async fn init_tables(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS services (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                usage TEXT NOT NULL DEFAULT '',
                agent_api_key TEXT UNIQUE,
                agent_status TEXT DEFAULT 'offline',
                agent_capacity INTEGER DEFAULT 1,
                agent_current_load INTEGER DEFAULT 0,
                agent_last_heartbeat TIMESTAMP,
                registration_token TEXT UNIQUE,
                registration_status TEXT DEFAULT 'pending',
                is_public INTEGER DEFAULT 1,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL,
                service_id TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                input TEXT,
                output TEXT,
                error_message TEXT,
                retry_count INTEGER NOT NULL DEFAULT 0,
                session_id TEXT NOT NULL,
                output_format TEXT,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                assigned_at TIMESTAMP,
                started_at TIMESTAMP,
                completed_at TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );

            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY NOT NULL,
                api_key TEXT UNIQUE NOT NULL,
                name TEXT NOT NULL,
                role TEXT DEFAULT 'client',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS task_files (
                id TEXT PRIMARY KEY NOT NULL,
                task_id TEXT NOT NULL,
                filename TEXT NOT NULL,
                mime_type TEXT,
                size_bytes INTEGER NOT NULL,
                storage_path TEXT NOT NULL,
                created_by TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS user_service_permissions (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL,
                service_id TEXT NOT NULL,
                granted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, service_id)
            );

            CREATE INDEX IF NOT EXISTS idx_services_agent_status ON services(agent_status);
            CREATE INDEX IF NOT EXISTS idx_tasks_service_id ON tasks(service_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_tasks_user_id ON tasks(user_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_session_id ON tasks(session_id);
            CREATE INDEX IF NOT EXISTS idx_users_api_key ON users(api_key);
            CREATE INDEX IF NOT EXISTS idx_task_files_task_id ON task_files(task_id);
            CREATE INDEX IF NOT EXISTS idx_user_service_permissions_user_id ON user_service_permissions(user_id);
            CREATE INDEX IF NOT EXISTS idx_user_service_permissions_service_id ON user_service_permissions(service_id);
            "#
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
    
    /// 关闭连接池
    pub async fn close(&self) {
        self.pool.close().await;
    }
}


