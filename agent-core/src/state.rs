//! 本地状态管理（SQLite）

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StateError {
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),
}

/// 本地任务状态
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LocalTask {
    pub task_id: String,
    pub server_task_id: String,
    pub status: String,
    pub container_id: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub output_path: Option<String>,
    pub error_message: Option<String>,
}

/// 状态管理器
pub struct StateManager {
    pool: Pool<Sqlite>,
}

impl StateManager {
    /// 初始化数据库 schema
    async fn init_schema(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS local_tasks (
                task_id TEXT PRIMARY KEY,
                server_task_id TEXT NOT NULL,
                status TEXT NOT NULL,
                container_id TEXT,
                started_at TIMESTAMP,
                completed_at TIMESTAMP,
                output_path TEXT,
                error_message TEXT
            );
            
            CREATE TABLE IF NOT EXISTS executor_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                last_heartbeat TIMESTAMP,
                current_load INTEGER DEFAULT 0
            );
            
            INSERT OR IGNORE INTO executor_state (id) VALUES (1);
            "#,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// 初始化状态管理器（使用指定数据库路径）
    pub async fn init(database_path: impl AsRef<Path>) -> Result<Self, StateError> {
        // 确保目录存在
        if let Some(parent) = database_path.as_ref().parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let path = database_path.as_ref();
        let path_str = path
            .to_str()
            .expect("database path should be valid UTF-8")
            .replace('\\', "/");
        let connection_string = if path.is_absolute() {
            format!("sqlite:///{}?mode=rwc", path_str)
        } else {
            format!("sqlite:{}?mode=rwc", path_str)
        };

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await?;

        Self::init_schema(&pool).await?;

        Ok(Self { pool })
    }

    /// 添加或更新任务
    pub async fn upsert_task(&self, task: &LocalTask) -> Result<(), StateError> {
        sqlx::query(
            r#"
            INSERT INTO local_tasks (task_id, server_task_id, status, container_id, started_at, completed_at, output_path, error_message)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(task_id) DO UPDATE SET
                status = excluded.status,
                container_id = excluded.container_id,
                started_at = excluded.started_at,
                completed_at = excluded.completed_at,
                output_path = excluded.output_path,
                error_message = excluded.error_message
            "#
        )
        .bind(&task.task_id)
        .bind(&task.server_task_id)
        .bind(&task.status)
        .bind(&task.container_id)
        .bind(task.started_at)
        .bind(task.completed_at)
        .bind(&task.output_path)
        .bind(&task.error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 获取运行中的任务
    pub async fn get_running_tasks(&self) -> Result<Vec<LocalTask>, StateError> {
        let tasks =
            sqlx::query_as::<_, LocalTask>("SELECT * FROM local_tasks WHERE status = 'running'")
                .fetch_all(&self.pool)
                .await?;

        Ok(tasks)
    }

    /// 按 task_id 查询单个任务（仅用于测试）
    #[cfg(any(test, feature = "test-utils"))]
    #[doc(hidden)]
    pub async fn get_task(&self, task_id: &str) -> Result<Option<LocalTask>, StateError> {
        let task = sqlx::query_as::<_, LocalTask>("SELECT * FROM local_tasks WHERE task_id = ?1")
            .bind(task_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(task)
    }

    /// 更新任务状态
    pub async fn update_task_status(
        &self,
        task_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<(), StateError> {
        sqlx::query("UPDATE local_tasks SET status = ?1, error_message = ?2 WHERE task_id = ?3")
            .bind(status)
            .bind(error_message)
            .bind(task_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// 更新心跳
    pub async fn update_heartbeat(&self) -> Result<(), StateError> {
        sqlx::query("UPDATE executor_state SET last_heartbeat = ?1 WHERE id = 1")
            .bind(Utc::now())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// 使用内存数据库初始化（仅用于测试）
    #[cfg(any(test, feature = "test-utils"))]
    #[doc(hidden)]
    pub async fn init_in_memory() -> Result<Self, StateError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await?;

        Self::init_schema(&pool).await?;

        Ok(Self { pool })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建测试任务
    fn create_test_task(id: &str, status: &str) -> LocalTask {
        LocalTask {
            task_id: id.to_string(),
            server_task_id: format!("server_{}", id),
            status: status.to_string(),
            container_id: Some(format!("container_{}", id)),
            started_at: Some(Utc::now()),
            completed_at: None,
            output_path: Some(
                std::env::temp_dir()
                    .join(format!("output_{}.zip", id))
                    .to_str()
                    .expect("temp dir path should be valid UTF-8")
                    .to_string(),
            ),
            error_message: None,
        }
    }

    #[tokio::test]
    async fn test_init_creates_tables() {
        let manager = StateManager::init_in_memory().await.unwrap();
        let running = manager.get_running_tasks().await.unwrap();
        assert!(running.is_empty());
    }

    #[tokio::test]
    async fn test_upsert_task() {
        let manager = StateManager::init_in_memory().await.unwrap();

        let task = create_test_task("task_001", "running");
        manager.upsert_task(&task).await.unwrap();

        let running = manager.get_running_tasks().await.unwrap();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].task_id, "task_001");
        assert_eq!(running[0].server_task_id, "server_task_001");
    }

    #[tokio::test]
    async fn test_upsert_updates_existing_task() {
        let manager = StateManager::init_in_memory().await.unwrap();

        let mut task = create_test_task("task_002", "running");
        manager.upsert_task(&task).await.unwrap();

        let running = manager.get_running_tasks().await.unwrap();
        assert_eq!(
            running[0].container_id,
            Some("container_task_002".to_string())
        );

        task.container_id = Some("new_container".to_string());
        manager.upsert_task(&task).await.unwrap();

        let running = manager.get_running_tasks().await.unwrap();
        assert_eq!(running[0].container_id, Some("new_container".to_string()));
    }

    #[tokio::test]
    async fn test_get_running_tasks() {
        let manager = StateManager::init_in_memory().await.unwrap();

        let pending_task = create_test_task("task_p1", "pending");
        let running_task1 = create_test_task("task_r1", "running");
        let running_task2 = create_test_task("task_r2", "running");
        let completed_task = create_test_task("task_c1", "completed");

        manager.upsert_task(&pending_task).await.unwrap();
        manager.upsert_task(&running_task1).await.unwrap();
        manager.upsert_task(&running_task2).await.unwrap();
        manager.upsert_task(&completed_task).await.unwrap();

        let running = manager.get_running_tasks().await.unwrap();
        assert_eq!(running.len(), 2);

        let task_ids: Vec<_> = running.iter().map(|t| t.task_id.clone()).collect();
        assert!(task_ids.contains(&"task_r1".to_string()));
        assert!(task_ids.contains(&"task_r2".to_string()));
    }

    #[tokio::test]
    async fn test_update_task_status() {
        let manager = StateManager::init_in_memory().await.unwrap();

        let task = create_test_task("task_003", "running");
        manager.upsert_task(&task).await.unwrap();

        let running_before = manager.get_running_tasks().await.unwrap();
        assert_eq!(running_before.len(), 1);

        manager
            .update_task_status("task_003", "failed", Some("Out of memory"))
            .await
            .unwrap();

        let running_after = manager.get_running_tasks().await.unwrap();
        assert!(running_after.is_empty());
    }

    #[tokio::test]
    async fn test_update_heartbeat() {
        let manager = StateManager::init_in_memory().await.unwrap();

        manager.update_heartbeat().await.unwrap();
    }

    #[tokio::test]
    async fn test_multiple_databases_isolation() {
        let manager1 = StateManager::init_in_memory().await.unwrap();
        let manager2 = StateManager::init_in_memory().await.unwrap();

        let task1 = create_test_task("isolated_task", "running");
        manager1.upsert_task(&task1).await.unwrap();

        let retrieved1 = manager1.get_running_tasks().await.unwrap();
        assert_eq!(retrieved1.len(), 1);

        let retrieved2 = manager2.get_running_tasks().await.unwrap();
        assert!(retrieved2.is_empty());
    }
}
