//! 测试工具（共享 MockExecutor）

use crate::executor::{Executor, ExecutorError, Task, TaskResult};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;

/// 用于测试的 MockExecutor
#[derive(Clone)]
pub struct MockExecutor {
    execute_count: Arc<AtomicUsize>,
    cancel_count: Arc<AtomicUsize>,
    execute_result: Arc<RwLock<HashMap<String, Result<TaskResult, ExecutorError>>>>,
    cancel_result: Arc<RwLock<Result<(), ExecutorError>>>,
    capacity: Arc<AtomicUsize>,
    current_load: Arc<AtomicUsize>,
}

impl MockExecutor {
    /// 创建新的 MockExecutor
    pub fn new() -> Self {
        Self {
            execute_count: Arc::new(AtomicUsize::new(0)),
            cancel_count: Arc::new(AtomicUsize::new(0)),
            execute_result: Arc::new(RwLock::new(HashMap::new())),
            cancel_result: Arc::new(RwLock::new(Ok(()))),
            capacity: Arc::new(AtomicUsize::new(5)),
            current_load: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// 设置预设执行结果
    pub async fn set_execute_result(
        &self,
        task_id: &str,
        result: Result<TaskResult, ExecutorError>,
    ) {
        self.execute_result
            .write()
            .await
            .insert(task_id.to_string(), result);
    }

    /// 设置预设取消结果
    pub async fn set_cancel_result(&self, result: Result<(), ExecutorError>) {
        *self.cancel_result.write().await = result;
    }

    /// 设置容量
    pub fn set_capacity(&self, capacity: usize) {
        self.capacity.store(capacity, Ordering::SeqCst);
    }

    /// 设置当前负载
    pub fn set_current_load(&self, load: usize) {
        self.current_load.store(load, Ordering::SeqCst);
    }

    /// 获取 execute 调用次数
    pub fn execute_call_count(&self) -> usize {
        self.execute_count.load(Ordering::SeqCst)
    }

    /// 获取 cancel 调用次数
    pub fn cancel_call_count(&self) -> usize {
        self.cancel_count.load(Ordering::SeqCst)
    }
}

impl Default for MockExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor for MockExecutor {
    async fn execute(
        &self,
        task: Task,
        _docker_mounts: Vec<String>,
    ) -> Result<TaskResult, ExecutorError> {
        self.execute_count.fetch_add(1, Ordering::SeqCst);
        let result = self.execute_result.read().await.get(&task.task_id).cloned();
        match result {
            Some(res) => res,
            None => Ok(TaskResult {
                task_id: task.task_id,
                exit_code: 0,
                stdout: "Mock execution successful".to_string(),
                stderr: String::new(),
                output_files: vec![],
            }),
        }
    }

    async fn cancel(&self, _task_id: &str) -> Result<(), ExecutorError> {
        self.cancel_count.fetch_add(1, Ordering::SeqCst);
        self.cancel_result.read().await.clone()
    }

    fn capacity(&self) -> usize {
        self.capacity.load(Ordering::SeqCst)
    }

    fn current_load(&self) -> usize {
        self.current_load.load(Ordering::SeqCst)
    }
}
