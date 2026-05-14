use std::time::Duration;
use tokio::sync::watch;
use open_aaas_server::state::AppState;

pub fn spawn_heartbeat_task(state: AppState, shutdown_tx: watch::Sender<()>) -> tokio::task::JoinHandle<()> {
    let mut shutdown_rx = shutdown_tx.subscribe();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
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

pub fn spawn_cleanup_task(state: AppState, shutdown_tx: watch::Sender<bool>, retention_days: i64) -> tokio::task::JoinHandle<()> {
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

/// 清理过期任务
/// 删除 completed/failed/cancelled 状态且超过保留期限的任务
async fn cleanup_expired_tasks(state: &AppState, retention_days: i64) {
    if retention_days <= 0 {
        tracing::debug!("Task cleanup skipped: retention_days is {}", retention_days);
        return;
    }

    let cutoff_date = chrono::Utc::now() - chrono::Duration::days(retention_days);

    // 先获取要删除的任务ID列表（用于后续清理文件）
    let tasks_to_delete: Vec<String> = sqlx::query_scalar(
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

    // 清理这些任务的文件
    for task_id in &tasks_to_delete {
        if let Err(e) = cleanup_task_files(state, task_id).await {
            tracing::error!("Failed to cleanup files for task {}: {}", task_id, e);
        }
    }

    match sqlx::query(
        r#"
        DELETE FROM tasks 
        WHERE status IN ('completed', 'failed', 'cancelled') 
        AND completed_at < ?
        "#,
    )
    .bind(cutoff_date.to_rfc3339())
    .execute(state.db.pool())
    .await
    {
        Ok(result) => {
            let deleted = result.rows_affected();
            if deleted > 0 {
                tracing::info!(
                    "Cleaned up {} expired task(s) older than {} days",
                    deleted,
                    retention_days
                );
            } else {
                tracing::debug!(
                    "No expired tasks to cleanup (retention: {} days)",
                    retention_days
                );
            }
        }
        Err(e) => {
            tracing::error!("Failed to cleanup expired tasks: {}", e);
        }
    }
}

/// 清理任务的文件
/// 删除数据库记录和对应的磁盘文件
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
