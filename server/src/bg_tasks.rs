use open_aaas_server::state::AppState;
use std::time::Duration;
use tokio::sync::watch;

pub fn spawn_heartbeat_task(
    state: AppState,
    shutdown_tx: watch::Sender<()>,
) -> tokio::task::JoinHandle<()> {
    let mut shutdown_rx = shutdown_tx.subscribe();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let timeout_secs = state.config.agent.heartbeat_timeout_secs;
                    let threshold = chrono::Utc::now() - chrono::Duration::seconds(timeout_secs as i64);

                    // 更新离线服务状态
                    match sqlx::query(
                        "UPDATE services SET agent_status = 'offline', agent_current_load = 0 WHERE agent_status != 'offline' AND agent_last_heartbeat < ?"
                    )
                    .bind(threshold.to_rfc3339())
                    .execute(state.db.pool())
                    .await {
                        Ok(result) => {
                            let rows = result.rows_affected();
                            if rows > 0 {
                                tracing::warn!("{} service(s) marked as offline due to heartbeat timeout", rows);

                                // 任务自动迁移：将 offline 服务的 running 任务改回 pending
                                match migrate_tasks_from_offline_services(&state, threshold.to_rfc3339()).await {
                                    Ok(migrated_count) => {
                                        if migrated_count > 0 {
                                            tracing::info!("{} task(s) migrated back to pending queue", migrated_count);
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to migrate tasks from offline services: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to check service heartbeats: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    tracing::info!("Heartbeat checker shutting down gracefully...");
                    break;
                }
            }
        }
    })
}

pub fn spawn_cleanup_task(
    state: AppState,
    shutdown_tx: watch::Sender<bool>,
    retention_days: i64,
) -> tokio::task::JoinHandle<()> {
    let mut shutdown_rx = shutdown_tx.subscribe();
    tokio::spawn(async move {
        // 每天执行一次清理
        let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        // 启动时立即执行一次清理
        cleanup_expired_tasks(&state, retention_days).await;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    cleanup_expired_tasks(&state, retention_days).await;
                }
                _ = shutdown_rx.changed() => {
                    tracing::info!("Cleanup task shutting down gracefully...");
                    break;
                }
            }
        }
    })
}

/// 将 offline 服务的活跃任务收敛到终态或重新排队
/// - running: 改回 pending（或重试超限后 failed）
/// - cancelling: 直接标记为 cancelled，避免永久卡住
async fn migrate_tasks_from_offline_services(
    state: &AppState,
    threshold: String,
) -> anyhow::Result<u64> {
    use open_aaas_server::models::task::Task;

    // 1. 查询需要迁移的任务
    let tasks_to_migrate: Vec<Task> = sqlx::query_as::<_, Task>(
        r#"
        SELECT * FROM tasks 
        WHERE service_id IN (
            SELECT id FROM services 
            WHERE agent_status = 'offline' AND agent_last_heartbeat < ?
        ) AND status IN ('running', 'cancelling')
        "#,
    )
    .bind(&threshold)
    .fetch_all(state.db.pool())
    .await?;

    let mut migrated_count = 0u64;
    let mut failed_count = 0u64;
    let mut cancelled_count = 0u64;
    let now = chrono::Utc::now();

    for task in tasks_to_migrate {
        if task.status == open_aaas_server::models::task::TaskStatus::Cancelling {
            sqlx::query(
                r#"
                UPDATE tasks
                SET status = 'cancelled', error_message = ?, completed_at = ?
                WHERE id = ?
                "#,
            )
            .bind("Agent 离线，任务取消完成")
            .bind(now.to_rfc3339())
            .bind(&task.id)
            .execute(state.db.pool())
            .await?;

            cancelled_count += 1;
            continue;
        }

        if task.retry_count >= 3 {
            // 重试次数超限，标记为失败
            sqlx::query(
                r#"
                UPDATE tasks 
                SET status = 'failed', error_message = ?, completed_at = ?
                WHERE id = ?
                "#,
            )
            .bind("任务重试次数超过上限，无可用服务")
            .bind(now.to_rfc3339())
            .bind(&task.id)
            .execute(state.db.pool())
            .await?;

            failed_count += 1;
        } else {
            // 重试次数未超限，改回 pending 并增加计数
            sqlx::query(
                r#"
                UPDATE tasks 
                SET status = 'pending', assigned_at = NULL, started_at = NULL, retry_count = retry_count + 1
                WHERE id = ?
                "#
            )
            .bind(&task.id)
            .execute(state.db.pool())
            .await?;

            migrated_count += 1;
        }
    }

    if failed_count > 0 {
        tracing::warn!(
            "{} task(s) marked as failed due to retry limit exceeded",
            failed_count
        );
    }
    if cancelled_count > 0 {
        tracing::info!(
            "{} cancelling task(s) finalized as cancelled after agent offline",
            cancelled_count
        );
    }

    Ok(migrated_count + cancelled_count)
}

/// 清理过期任务的本地磁盘文件（数据库记录保留）
/// 删除 completed/failed/cancelled 状态且超过保留期限的任务
async fn cleanup_expired_tasks(state: &AppState, retention_days: i64) {
    if retention_days <= 0 {
        tracing::debug!("Task cleanup skipped: retention_days is {}", retention_days);
        return;
    }

    let cutoff_date = chrono::Utc::now() - chrono::Duration::days(retention_days);

    // 获取已过期任务ID列表（用于清理本地磁盘文件，数据库记录保留）
    let expired_tasks: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT id FROM tasks 
        WHERE status IN ('completed', 'failed', 'cancelled') 
        AND completed_at < ?
        "#,
    )
    .bind(cutoff_date.to_rfc3339())
    .fetch_all(state.db.pool())
    .await
    .unwrap_or_default();

    // 清理这些任务的本地磁盘文件（数据库记录保留）
    for task_id in &expired_tasks {
        if let Err(e) = cleanup_task_files(state, task_id).await {
            tracing::error!("Failed to cleanup files for task {}: {}", task_id, e);
        }
    }

    let cleaned_count = expired_tasks.len();
    if cleaned_count > 0 {
        tracing::info!(
            "Attempted cleanup for {} expired task(s) older than {} days (database records retained)",
            cleaned_count,
            retention_days
        );
    } else {
        tracing::debug!(
            "No expired tasks to cleanup (retention: {} days)",
            retention_days
        );
    }
}

/// 清理任务的本地磁盘文件
/// 仅删除磁盘文件和空目录，不删除数据库记录
async fn cleanup_task_files(state: &AppState, task_id: &str) -> anyhow::Result<()> {
    use open_aaas_server::models::file::TaskFile;

    // 查询该任务的所有文件
    let files: Vec<TaskFile> = sqlx::query_as("SELECT * FROM task_files WHERE task_id = ?")
        .bind(task_id)
        .fetch_all(state.db.pool())
        .await?;

    let storage_path = state.file_storage_path();
    let mut deleted_count = 0;
    let mut failed_count = 0;

    for file in files {
        let full_path = match file.full_storage_path(storage_path) {
            Ok(path) => path,
            Err(e) => {
                tracing::warn!("Invalid file path for file {}: {}", file.id, e);
                failed_count += 1;
                continue;
            }
        };

        // 删除磁盘文件（如果存在）
        if full_path.exists() {
            match tokio::fs::remove_file(&full_path).await {
                Ok(_) => {
                    deleted_count += 1;
                    tracing::debug!("Deleted file: {}", full_path.display());
                }
                Err(e) => {
                    failed_count += 1;
                    tracing::warn!("Failed to delete file {}: {}", full_path.display(), e);
                }
            }
        }
    }

    // 删除空目录（任务目录）
    let task_dir = std::path::PathBuf::from(storage_path).join(task_id);
    if task_dir.exists() {
        match tokio::fs::remove_dir(&task_dir).await {
            Ok(_) => tracing::debug!("Deleted task directory: {}", task_dir.display()),
            Err(e) => tracing::debug!(
                "Failed to delete task directory {} (may not be empty): {}",
                task_dir.display(),
                e
            ),
        }
    }

    if deleted_count > 0 || failed_count > 0 {
        tracing::info!(
            "Task {} file cleanup: {} deleted, {} failed",
            task_id,
            deleted_count,
            failed_count
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use open_aaas_server::{
        config::AppConfig,
        models::file::{FileCreatedBy, TaskFile},
        state::AppState,
    };
    use uuid::Uuid;

    async fn setup_test_state() -> (AppState, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = AppConfig::default();
        config.database.url = "sqlite::memory:".to_string();
        config.task.file_storage_path = temp_dir.path().to_str().unwrap().to_string();
        let state = AppState::new(config).await.unwrap();
        state.db.init_tables().await.unwrap();
        (state, temp_dir)
    }

    async fn create_test_user_and_service(state: &AppState) -> (String, String) {
        let user_id = format!("user-{}", Uuid::new_v4());
        let service_id = format!("service-{}", Uuid::new_v4());

        sqlx::query("INSERT INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)")
            .bind(&user_id)
            .bind("ak_test")
            .bind("Test User")
            .bind("client")
            .execute(state.db.pool())
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO services (id, name, description, usage, agent_api_key, registration_status, agent_status, agent_capacity, agent_current_load, is_public, created_at) VALUES (?, ?, ?, ?, ?, 'active', 'online', 1, 0, 1, ?)"
        )
        .bind(&service_id)
        .bind("Test Service")
        .bind("Description")
        .bind("Usage")
        .bind("ak_agent")
        .bind(Utc::now())
        .execute(state.db.pool())
        .await
        .unwrap();

        (user_id, service_id)
    }

    async fn create_test_task(
        state: &AppState,
        user_id: &str,
        service_id: &str,
        status: &str,
        completed_at: Option<chrono::DateTime<Utc>>,
    ) -> String {
        let task_id = format!("task-{}", Uuid::new_v4());
        let session_id = format!("session-{}", Uuid::new_v4());
        let input = serde_json::json!({"test": "input"});
        let created_at = Utc::now() - chrono::Duration::days(10);

        sqlx::query(
            r#"
            INSERT INTO tasks (id, user_id, service_id, status, input, session_id, retry_count, created_at, completed_at)
            VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?)
            "#
        )
        .bind(&task_id)
        .bind(user_id)
        .bind(service_id)
        .bind(status)
        .bind(&input)
        .bind(&session_id)
        .bind(created_at)
        .bind(completed_at.map(|d| d.to_rfc3339()))
        .execute(state.db.pool())
        .await
        .unwrap();

        task_id
    }

    async fn create_test_file(state: &AppState, task_id: &str) -> std::path::PathBuf {
        let file_id = Uuid::new_v4().to_string();
        let storage_path = format!("{}/{}", task_id, file_id);
        let full_path = std::path::PathBuf::from(state.file_storage_path()).join(&storage_path);

        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await.unwrap();
        }
        tokio::fs::write(&full_path, b"test content").await.unwrap();

        let file = TaskFile::new(
            task_id,
            "test.txt",
            Some("text/plain".to_string()),
            12,
            &storage_path,
            FileCreatedBy::Client,
        );

        sqlx::query(
            r#"
            INSERT INTO task_files (id, task_id, filename, mime_type, size_bytes, storage_path, created_by, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&file.id)
        .bind(&file.task_id)
        .bind(&file.filename)
        .bind(&file.mime_type)
        .bind(file.size_bytes)
        .bind(&file.storage_path)
        .bind("client")
        .bind(file.created_at)
        .execute(state.db.pool())
        .await
        .unwrap();

        full_path
    }

    /// 测试1：retention_days = 0 时跳过清理
    #[tokio::test]
    async fn test_cleanup_skipped_when_retention_disabled() {
        let (state, _temp_dir) = setup_test_state().await;
        let (user_id, service_id) = create_test_user_and_service(&state).await;
        let completed_at = Some(Utc::now() - chrono::Duration::days(10));
        let task_id =
            create_test_task(&state, &user_id, &service_id, "completed", completed_at).await;
        let file_path = create_test_file(&state, &task_id).await;

        cleanup_expired_tasks(&state, 0).await;

        assert!(
            file_path.exists(),
            "File should still exist when retention_days = 0"
        );
    }

    /// 测试2：核心测试——仅删除文件，保留数据库记录
    #[tokio::test]
    async fn test_cleanup_deletes_only_files_retains_records() {
        let (state, _temp_dir) = setup_test_state().await;
        let (user_id, service_id) = create_test_user_and_service(&state).await;
        let completed_at = Some(Utc::now() - chrono::Duration::days(10));
        let task_id =
            create_test_task(&state, &user_id, &service_id, "completed", completed_at).await;
        let file_path = create_test_file(&state, &task_id).await;

        cleanup_expired_tasks(&state, 7).await;

        assert!(
            !file_path.exists(),
            "Expired task file should be deleted from disk"
        );

        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE id = ?")
            .bind(&task_id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
        assert_eq!(task_count, 1, "Task database record should be retained");

        let file_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM task_files WHERE task_id = ?")
                .bind(&task_id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
        assert_eq!(
            file_count, 1,
            "Task file database record should be retained"
        );
    }

    /// 测试3：状态过滤——pending 和 running 任务不被清理
    #[tokio::test]
    async fn test_cleanup_respects_status_filter() {
        let (state, _temp_dir) = setup_test_state().await;
        let (user_id, service_id) = create_test_user_and_service(&state).await;

        let pending_task_id =
            create_test_task(&state, &user_id, &service_id, "pending", None).await;
        let pending_file_path = create_test_file(&state, &pending_task_id).await;

        let running_task_id =
            create_test_task(&state, &user_id, &service_id, "running", None).await;
        let running_file_path = create_test_file(&state, &running_task_id).await;

        cleanup_expired_tasks(&state, 7).await;

        assert!(
            pending_file_path.exists(),
            "Pending task file should not be deleted"
        );
        assert!(
            running_file_path.exists(),
            "Running task file should not be deleted"
        );
    }

    /// 测试4：时间过滤——未过期的任务不被清理
    #[tokio::test]
    async fn test_cleanup_respects_time_filter() {
        let (state, _temp_dir) = setup_test_state().await;
        let (user_id, service_id) = create_test_user_and_service(&state).await;
        let completed_at = Some(Utc::now() - chrono::Duration::days(3));
        let task_id =
            create_test_task(&state, &user_id, &service_id, "completed", completed_at).await;
        let file_path = create_test_file(&state, &task_id).await;

        cleanup_expired_tasks(&state, 7).await;

        assert!(
            file_path.exists(),
            "File should still exist for task within retention period"
        );
    }

    /// 测试5：幂等性——重复调用 cleanup_task_files 不应报错
    #[tokio::test]
    async fn test_cleanup_task_files_idempotent() {
        let (state, _temp_dir) = setup_test_state().await;
        let (user_id, service_id) = create_test_user_and_service(&state).await;
        let completed_at = Some(Utc::now() - chrono::Duration::days(10));
        let task_id =
            create_test_task(&state, &user_id, &service_id, "completed", completed_at).await;
        let file_path = create_test_file(&state, &task_id).await;

        let result1 = cleanup_task_files(&state, &task_id).await;
        assert!(result1.is_ok(), "First cleanup should succeed");

        let result2 = cleanup_task_files(&state, &task_id).await;
        assert!(
            result2.is_ok(),
            "Second cleanup should succeed (idempotent)"
        );

        assert!(
            !file_path.exists(),
            "File should still not exist after second cleanup"
        );

        let file_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM task_files WHERE task_id = ?")
                .bind(&task_id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
        assert_eq!(
            file_count, 1,
            "Task file database record should be retained"
        );
    }

    /// 测试6：验证 cleanup_expired_tasks 不会漏掉任何一个过期任务
    #[tokio::test]
    async fn test_cleanup_batch_no_leak() {
        let (state, _temp_dir) = setup_test_state().await;
        let (user_id, service_id) = create_test_user_and_service(&state).await;

        let task1_id = create_test_task(
            &state,
            &user_id,
            &service_id,
            "completed",
            Some(Utc::now() - chrono::Duration::days(10)),
        )
        .await;
        let file1_path = create_test_file(&state, &task1_id).await;

        let task2_id = create_test_task(
            &state,
            &user_id,
            &service_id,
            "completed",
            Some(Utc::now() - chrono::Duration::days(10)),
        )
        .await;
        let file2_path = create_test_file(&state, &task2_id).await;

        cleanup_expired_tasks(&state, 7).await;

        assert!(
            !file1_path.exists(),
            "First expired task file should be deleted"
        );
        assert!(
            !file2_path.exists(),
            "Second expired task file should be deleted"
        );

        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE id IN (?, ?)")
            .bind(&task1_id)
            .bind(&task2_id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
        assert_eq!(
            task_count, 2,
            "Both task database records should be retained"
        );

        let file_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM task_files WHERE task_id IN (?, ?)")
                .bind(&task1_id)
                .bind(&task2_id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
        assert_eq!(
            file_count, 2,
            "Both task file database records should be retained"
        );
    }

    /// 测试7：failed 和 cancelled 状态的过期任务文件也应被清理
    #[tokio::test]
    async fn test_cleanup_deletes_failed_and_cancelled_files() {
        let (state, _temp_dir) = setup_test_state().await;
        let (user_id, service_id) = create_test_user_and_service(&state).await;
        let completed_at = Some(Utc::now() - chrono::Duration::days(10));

        let failed_task_id =
            create_test_task(&state, &user_id, &service_id, "failed", completed_at).await;
        let failed_file_path = create_test_file(&state, &failed_task_id).await;

        let cancelled_task_id =
            create_test_task(&state, &user_id, &service_id, "cancelled", completed_at).await;
        let cancelled_file_path = create_test_file(&state, &cancelled_task_id).await;

        cleanup_expired_tasks(&state, 7).await;

        assert!(
            !failed_file_path.exists(),
            "Failed task file should be deleted"
        );
        assert!(
            !cancelled_file_path.exists(),
            "Cancelled task file should be deleted"
        );

        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE id IN (?, ?)")
            .bind(&failed_task_id)
            .bind(&cancelled_task_id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
        assert_eq!(
            task_count, 2,
            "Both task database records should be retained"
        );

        let file_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM task_files WHERE task_id IN (?, ?)")
                .bind(&failed_task_id)
                .bind(&cancelled_task_id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
        assert_eq!(
            file_count, 2,
            "Both task file database records should be retained"
        );
    }
}
