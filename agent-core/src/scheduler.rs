//! 调度器核心

use crate::client::{ApiClient, ServerTask, TaskCompleteStatus};
use crate::config::Config;
use crate::executor::{Executor, Task, TaskInputFile};
use crate::state::{LocalTask, StateManager};
use anyhow::Result;
use chrono::Utc;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

fn sanitize_input_filename(raw: &str) -> String {
    let candidate = Path::new(raw)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("input.bin");

    let sanitized = candidate
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>();

    if sanitized.trim().is_empty() {
        "input.bin".to_string()
    } else {
        sanitized
    }
}

/// 调度命令
#[derive(Debug)]
pub enum SchedulerCommand {
    /// 停止调度器
    Stop,
    /// 取消任务
    CancelTask(String),
}

/// 调度器
pub struct Scheduler<E: Executor> {
    config: Config,
    client: Arc<RwLock<ApiClient>>,
    executor: Arc<E>,
    state: Arc<StateManager>,
    running: Arc<RwLock<bool>>,
    cancelled_tasks: Arc<Mutex<HashSet<String>>>,
    command_tx: mpsc::Sender<SchedulerCommand>,
    command_rx: Arc<Mutex<mpsc::Receiver<SchedulerCommand>>>,
}

impl<E: Executor + 'static> Scheduler<E> {
    /// 创建新的调度器
    pub fn new(config: Config, client: ApiClient, executor: E, state: StateManager) -> Self {
        let (command_tx, command_rx) = mpsc::channel(10);

        Self {
            config,
            client: Arc::new(RwLock::new(client)),
            executor: Arc::new(executor),
            state: Arc::new(state),
            running: Arc::new(RwLock::new(false)),
            cancelled_tasks: Arc::new(Mutex::new(HashSet::new())),
            command_tx,
            command_rx: Arc::new(Mutex::new(command_rx)),
        }
    }

    /// 获取命令发送器
    pub fn command_sender(&self) -> mpsc::Sender<SchedulerCommand> {
        self.command_tx.clone()
    }

    /// 启动调度器
    pub async fn run(&self) -> Result<()> {
        info!("调度器启动");

        *self.running.write().await = true;

        // 恢复运行中的任务（崩溃恢复）
        self.recover_tasks().await?;

        // 创建心跳间隔
        let mut heartbeat_interval = interval(Duration::from_secs(20));

        // 创建轮询间隔
        let poll_interval = Duration::from_secs(self.config.server.poll_interval_secs);

        loop {
            if !*self.running.read().await {
                break;
            }

            tokio::select! {
                // 心跳
                _ = heartbeat_interval.tick() => {
                    if let Err(e) = self.heartbeat().await {
                        error!("心跳失败: {}", e);
                    }
                }

                // 轮询任务
                _ = sleep(poll_interval) => {
                    if let Err(e) = self.poll_and_execute().await {
                        error!("轮询执行失败: {}", e);
                        // 短暂退避
                        sleep(Duration::from_secs(5)).await;
                    }
                }

                // 处理命令
                cmd = async { self.command_rx.lock().await.recv().await } => {
                    if let Some(c) = cmd {
                        self.handle_command(c).await;
                    }
                }
            }
        }

        info!("调度器停止");
        Ok(())
    }

    /// 恢复运行中的任务（崩溃恢复）
    async fn recover_tasks(&self) -> Result<()> {
        let running_tasks = self.state.get_running_tasks().await?;

        if !running_tasks.is_empty() {
            warn!("发现 {} 个运行中的任务，标记为失败", running_tasks.len());

            for task in running_tasks {
                warn!("恢复任务: {} -> 标记为失败", task.task_id);

                // 更新本地状态
                self.state
                    .update_task_status(&task.task_id, "failed", Some("Agent 重启，任务中断"))
                    .await?;

                // 上报 Server
                let client = self.client.read().await;
                if let Err(e) = client
                    .complete_task(
                        &task.task_id,
                        TaskCompleteStatus::Failed,
                        None,
                        Some("Agent 重启，任务中断".to_string()),
                        vec![],
                    )
                    .await
                {
                    error!("上报任务失败状态失败: {}", e);
                }
            }
        }

        Ok(())
    }

    /// 发送心跳
    async fn heartbeat(&self) -> Result<()> {
        // 更新本地心跳
        self.state.update_heartbeat().await?;

        // 获取当前负载信息
        let current_load = self.executor.current_load();
        let capacity = self.executor.capacity();

        // 发送 Server 心跳（带负载信息）
        let client = self.client.read().await;
        client.heartbeat(current_load, capacity).await?;

        debug!("心跳完成: load={}/{}", current_load, capacity);
        Ok(())
    }

    /// 轮询并执行任务
    async fn poll_and_execute(&self) -> Result<()> {
        let current_load = self.executor.current_load();
        let capacity = self.executor.capacity();

        let client = self.client.read().await;
        let poll_outcome = client.poll(current_load, capacity).await?;
        drop(client); // 释放读锁

        if let Some(task_id) = poll_outcome.cancel_task_id {
            self.handle_command(SchedulerCommand::CancelTask(task_id))
                .await;
            return Ok(());
        }

        if current_load >= capacity {
            if poll_outcome.task.is_some() {
                debug!("负载已满 ({}/{}), 暂不领取新任务", current_load, capacity);
            } else {
                debug!(
                    "负载已满 ({}/{}), 本轮仅检查取消信号",
                    current_load, capacity
                );
            }
            return Ok(());
        }

        let Some(server_task) = poll_outcome.task else {
            debug!("没有新任务");
            return Ok(());
        };

        info!("获取到新任务: {}", server_task.id);

        // 接受任务
        let client = self.client.read().await;
        let accepted = client.accept_task(&server_task.id).await?;
        drop(client);

        if !accepted {
            warn!("任务 {} 接受失败，可能已被其他 Agent 接受", server_task.id);
            return Ok(());
        }

        // 执行任务（异步，不阻塞轮询）
        self.spawn_task(server_task).await;

        Ok(())
    }

    /// 启动任务执行（后台）
    async fn spawn_task(&self, server_task: ServerTask) {
        let task_id = server_task.id.clone();
        let executor = self.executor.clone();
        let state = self.state.clone();
        let client = self.client.clone();
        let config = self.config.clone();
        let cancelled_tasks = self.cancelled_tasks.clone();

        tokio::spawn(async move {
            info!("开始执行任务: {}", task_id);

            // 创建本地任务记录
            let local_task = LocalTask {
                task_id: task_id.clone(),
                server_task_id: task_id.clone(),
                status: "running".to_string(),
                container_id: None,
                started_at: Some(Utc::now()),
                completed_at: None,
                output_path: Some(config.workspace_dir(&task_id).to_string_lossy().to_string()),
                error_message: None,
            };

            if let Err(e) = state.upsert_task(&local_task).await {
                error!("保存任务状态失败: {}，终止任务 {}", e, task_id);
                let client = client.read().await;
                if let Err(report_err) = client
                    .complete_task(
                        &task_id,
                        TaskCompleteStatus::Failed,
                        None,
                        Some(format!("保存本地任务状态失败: {}", e)),
                        vec![],
                    )
                    .await
                {
                    error!("上报任务失败状态失败: {}", report_err);
                }
                return;
            }

            let workspace = config.workspace_dir(&task_id);
            let input_dir = workspace.join("input");
            if let Err(e) = tokio::fs::create_dir_all(&input_dir).await {
                error!("创建输入目录失败: task_id={}, error={}", task_id, e);
                if let Err(db_err) = state
                    .update_task_status(
                        &task_id,
                        "failed",
                        Some(&format!("创建输入目录失败: {}", e)),
                    )
                    .await
                {
                    error!("更新本地任务状态失败: {}", db_err);
                }
                let client = client.read().await;
                if let Err(report_err) = client
                    .complete_task(
                        &task_id,
                        TaskCompleteStatus::Failed,
                        None,
                        Some(format!("创建输入目录失败: {}", e)),
                        vec![],
                    )
                    .await
                {
                    error!("上报任务失败状态失败: {}", report_err);
                }
                return;
            }

            let mut local_input_files = Vec::new();
            {
                let client = client.read().await;
                for (index, input_file) in server_task.input_files.iter().enumerate() {
                    let safe_name = sanitize_input_filename(&input_file.filename);
                    let relative_path = format!("input/{:02}-{}", index + 1, safe_name);
                    let full_path = workspace.join(&relative_path);

                    match client.download_input_file(&input_file.id).await {
                        Ok(content) => {
                            if let Err(e) = tokio::fs::write(&full_path, content).await {
                                error!(
                                    "写入输入文件失败: task_id={}, file_id={}, path={}, error={}",
                                    task_id,
                                    input_file.id,
                                    full_path.display(),
                                    e
                                );
                                if let Err(db_err) = state
                                    .update_task_status(
                                        &task_id,
                                        "failed",
                                        Some(&format!("写入输入文件失败: {}", e)),
                                    )
                                    .await
                                {
                                    error!("更新本地任务状态失败: {}", db_err);
                                }
                                if let Err(report_err) = client
                                    .complete_task(
                                        &task_id,
                                        TaskCompleteStatus::Failed,
                                        None,
                                        Some(format!("写入输入文件失败: {}", e)),
                                        vec![],
                                    )
                                    .await
                                {
                                    error!("上报任务失败状态失败: {}", report_err);
                                }
                                return;
                            }

                            local_input_files.push(TaskInputFile {
                                id: input_file.id.clone(),
                                filename: input_file.filename.clone(),
                                mime_type: input_file.mime_type.clone(),
                                size_bytes: input_file.size_bytes,
                                local_path: relative_path,
                            });
                        }
                        Err(e) => {
                            error!(
                                "下载输入文件失败: task_id={}, file_id={}, error={}",
                                task_id, input_file.id, e
                            );
                            if let Err(db_err) = state
                                .update_task_status(
                                    &task_id,
                                    "failed",
                                    Some(&format!("下载输入文件失败: {}", e)),
                                )
                                .await
                            {
                                error!("更新本地任务状态失败: {}", db_err);
                            }
                            if let Err(report_err) = client
                                .complete_task(
                                    &task_id,
                                    TaskCompleteStatus::Failed,
                                    None,
                                    Some(format!("下载输入文件失败: {}", e)),
                                    vec![],
                                )
                                .await
                            {
                                error!("上报任务失败状态失败: {}", report_err);
                            }
                            return;
                        }
                    }
                }
            }

            // 执行转换
            let task = Task {
                task_id: task_id.clone(),
                prompt: server_task.task_prompt,
                output_prompt: server_task.output_prompt,
                session_id: Some(server_task.session_id),
                input_files: local_input_files,
            };

            // 获取 docker 挂载配置
            let docker_mounts = config.docker_mounts();

            // 记录任务执行信息
            info!(
                "Spawning task {} with {} mounts",
                task.task_id,
                docker_mounts.len()
            );
            for mount in &docker_mounts {
                info!("  Mount: {}", mount);
            }

            // 执行任务
            let result = executor.execute(task, docker_mounts).await;

            let was_cancelled = {
                let mut cancelled = cancelled_tasks.lock().await;
                cancelled.remove(&task_id)
            };

            if was_cancelled {
                info!("任务 {} 已按取消请求结束", task_id);

                if let Err(e) = state
                    .update_task_status(&task_id, "cancelled", Some("任务已取消"))
                    .await
                {
                    error!("更新任务取消状态失败: {}", e);
                }

                let client = client.read().await;
                if let Err(e) = client
                    .complete_task(
                        &task_id,
                        TaskCompleteStatus::Cancelled,
                        None,
                        Some("任务已取消".to_string()),
                        vec![],
                    )
                    .await
                {
                    error!("上报任务取消失败: {}", e);
                }
                return;
            }

            // 处理结果
            match result {
                Ok(task_result) => {
                    let status = task_result.status();
                    let status_str = format!("{}", status);

                    info!(
                        "任务 {} 执行完成: exit_code={}",
                        task_id, task_result.exit_code
                    );

                    // 更新本地状态
                    if let Err(e) = state
                        .update_task_status(
                            &task_id,
                            &status_str,
                            if task_result.exit_code != 0 {
                                Some(&task_result.stderr)
                            } else {
                                None
                            },
                        )
                        .await
                    {
                        error!("更新任务状态失败: {}", e);
                    }

                    // 上传结果文件到 Server，便于 Client/Dashboard 查看
                    let workspace = config.workspace_dir(&task_id);
                    let mut uploaded_file_ids = Vec::new();
                    {
                        let client = client.read().await;
                        for output_file in &task_result.output_files {
                            let file_path = workspace.join(output_file);
                            if !file_path.is_file() {
                                warn!("输出文件不存在，跳过上传: {}", file_path.display());
                                continue;
                            }

                            match client
                                .upload_file_as(&task_id, &file_path, Some(output_file))
                                .await
                            {
                                Ok(file_id) => uploaded_file_ids.push(file_id),
                                Err(e) => error!(
                                    "上传结果文件失败: task_id={}, file={}, error={}",
                                    task_id, output_file, e
                                ),
                            }
                        }
                    }

                    // 构建输出
                    let output = if !task_result.output_files.is_empty() {
                        Some(serde_json::json!({
                            "files": task_result.output_files,
                            "stdout": task_result.stdout,
                        }))
                    } else {
                        None
                    };

                    // 上报 Server
                    let client = client.read().await;
                    if let Err(e) = client
                        .complete_task(
                            &task_id,
                            status,
                            output,
                            if task_result.exit_code != 0 {
                                Some(task_result.stderr)
                            } else {
                                None
                            },
                            uploaded_file_ids,
                        )
                        .await
                    {
                        error!("上报任务完成失败: {}", e);
                    }
                }
                Err(e) => {
                    error!("任务 {} 执行失败: {}", task_id, e);

                    // 更新本地状态
                    if let Err(db_err) = state
                        .update_task_status(&task_id, "failed", Some(&e.to_string()))
                        .await
                    {
                        error!("更新本地任务状态失败: {}", db_err);
                    }

                    // 上报 Server
                    let client = client.read().await;
                    if let Err(e) = client
                        .complete_task(
                            &task_id,
                            TaskCompleteStatus::Failed,
                            None,
                            Some(e.to_string()),
                            vec![],
                        )
                        .await
                    {
                        error!("上报任务失败状态失败: {}", e);
                    }
                }
            }
        });
    }

    /// 处理命令
    async fn handle_command(&self, cmd: SchedulerCommand) {
        match cmd {
            SchedulerCommand::Stop => {
                info!("收到停止命令");
                *self.running.write().await = false;
            }
            SchedulerCommand::CancelTask(task_id) => {
                warn!("收到取消任务命令: {}", task_id);
                self.cancelled_tasks.lock().await.insert(task_id.clone());
                if let Err(e) = self.executor.cancel(&task_id).await {
                    error!("取消任务 {} 失败: {}", task_id, e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ServerConfig};
    use crate::test_utils::MockExecutor;

    /// 创建测试用的 Config
    fn create_test_config() -> Config {
        Config {
            server: ServerConfig {
                base_url: "http://127.0.0.1:8080".to_string(),
                poll_interval_secs: 30,
                use_system_proxy: false,
            },
            agent: crate::config::AgentConfig::default(),
            executor: crate::config::ExecutorConfig::default(),
            paths: crate::config::PathConfig::default(),
        }
    }

    /// 创建测试用的 ApiClient
    fn create_test_client() -> ApiClient {
        ApiClient::new(&ServerConfig {
            base_url: "http://127.0.0.1:8080".to_string(),
            poll_interval_secs: 30,
            use_system_proxy: false,
        })
    }

    // ==================== SchedulerCommand 测试 ====================

    #[test]
    fn test_scheduler_command_stop_creation() {
        let cmd = SchedulerCommand::Stop;
        match cmd {
            SchedulerCommand::Stop => {}
            _ => panic!("Expected Stop command"),
        }
    }

    #[test]
    fn test_scheduler_command_cancel_task_creation() {
        let task_id = "task-123".to_string();
        let cmd = SchedulerCommand::CancelTask(task_id.clone());
        match cmd {
            SchedulerCommand::CancelTask(id) => assert_eq!(id, task_id),
            _ => panic!("Expected CancelTask command"),
        }
    }

    #[test]
    fn test_scheduler_command_debug_format() {
        let stop_cmd = SchedulerCommand::Stop;
        let cancel_cmd = SchedulerCommand::CancelTask("task-456".to_string());

        let stop_debug = format!("{:?}", stop_cmd);
        let cancel_debug = format!("{:?}", cancel_cmd);

        assert!(stop_debug.contains("Stop"));
        assert!(cancel_debug.contains("CancelTask"));
        assert!(cancel_debug.contains("task-456"));
    }

    // ==================== Scheduler::new() 测试 ====================

    #[tokio::test]
    async fn test_scheduler_new_creation() {
        let config = create_test_config();
        let client = create_test_client();
        let executor = MockExecutor::new();
        let state = StateManager::init_in_memory().await.unwrap();

        let scheduler = Scheduler::new(config, client, executor, state);

        // 验证 command_sender 可以被获取
        let sender = scheduler.command_sender();
        assert!(sender.send(SchedulerCommand::Stop).await.is_ok());
    }

    #[tokio::test]
    async fn test_scheduler_command_sender_can_send_stop() {
        let config = create_test_config();
        let client = create_test_client();
        let executor = MockExecutor::new();
        let state = StateManager::init_in_memory().await.unwrap();

        let scheduler = Scheduler::new(config, client, executor, state);
        let sender = scheduler.command_sender();

        // 验证可以发送 Stop 命令
        let result = sender.send(SchedulerCommand::Stop).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_scheduler_command_sender_can_send_cancel_task() {
        let config = create_test_config();
        let client = create_test_client();
        let executor = MockExecutor::new();
        let state = StateManager::init_in_memory().await.unwrap();

        let scheduler = Scheduler::new(config, client, executor, state);
        let sender = scheduler.command_sender();

        // 验证可以发送 CancelTask 命令
        let result = sender
            .send(SchedulerCommand::CancelTask("task-123".to_string()))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_scheduler_multiple_command_senders() {
        let config = create_test_config();
        let client = create_test_client();
        let executor = MockExecutor::new();
        let state = StateManager::init_in_memory().await.unwrap();

        let scheduler = Scheduler::new(config, client, executor, state);

        // 获取多个 sender
        let sender1 = scheduler.command_sender();
        let sender2 = scheduler.command_sender();

        // 两个 sender 都应该能正常工作
        assert!(sender1.send(SchedulerCommand::Stop).await.is_ok());
        assert!(
            sender2
                .send(SchedulerCommand::CancelTask("task-456".to_string()))
                .await
                .is_ok()
        );
    }

    // ==================== 命令通道测试 ====================

    #[tokio::test]
    async fn test_command_channel_stop_received() {
        let config = create_test_config();
        let client = create_test_client();
        let executor = MockExecutor::new();
        let state = StateManager::init_in_memory().await.unwrap();

        let scheduler = Scheduler::new(config, client, executor, state);
        let sender = scheduler.command_sender();

        // 发送 Stop 命令
        sender.send(SchedulerCommand::Stop).await.unwrap();

        // 验证命令可以被接收
        let mut receiver = scheduler.command_rx.lock().await;
        let received = receiver.recv().await;
        assert!(matches!(received, Some(SchedulerCommand::Stop)));
    }

    #[tokio::test]
    async fn test_command_channel_cancel_task_received() {
        let config = create_test_config();
        let client = create_test_client();
        let executor = MockExecutor::new();
        let state = StateManager::init_in_memory().await.unwrap();

        let scheduler = Scheduler::new(config, client, executor, state);
        let sender = scheduler.command_sender();

        // 发送 CancelTask 命令
        sender
            .send(SchedulerCommand::CancelTask("task-to-cancel".to_string()))
            .await
            .unwrap();

        // 验证命令可以被接收
        let mut receiver = scheduler.command_rx.lock().await;
        let received = receiver.recv().await;
        assert!(matches!(received, Some(SchedulerCommand::CancelTask(_))));

        if let Some(SchedulerCommand::CancelTask(task_id)) = received {
            assert_eq!(task_id, "task-to-cancel");
        }
    }

    #[tokio::test]
    async fn test_command_channel_multiple_commands() {
        let config = create_test_config();
        let client = create_test_client();
        let executor = MockExecutor::new();
        let state = StateManager::init_in_memory().await.unwrap();

        let scheduler = Scheduler::new(config, client, executor, state);
        let sender = scheduler.command_sender();

        // 发送多个命令
        sender
            .send(SchedulerCommand::CancelTask("task-1".to_string()))
            .await
            .unwrap();
        sender.send(SchedulerCommand::Stop).await.unwrap();
        sender
            .send(SchedulerCommand::CancelTask("task-2".to_string()))
            .await
            .unwrap();

        // 验证所有命令按顺序被接收
        let mut receiver = scheduler.command_rx.lock().await;

        let cmd1 = receiver.recv().await;
        assert!(matches!(cmd1, Some(SchedulerCommand::CancelTask(ref id)) if id == "task-1"));

        let cmd2 = receiver.recv().await;
        assert!(matches!(cmd2, Some(SchedulerCommand::Stop)));

        let cmd3 = receiver.recv().await;
        assert!(matches!(cmd3, Some(SchedulerCommand::CancelTask(ref id)) if id == "task-2"));
    }
}
