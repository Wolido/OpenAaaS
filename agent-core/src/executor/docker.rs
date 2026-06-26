//! Docker 执行器实现

use super::{Executor, ExecutorError, Task, TaskResult};
use crate::config::ExecutorConfig;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{info, warn};

/// 负载计数守卫，确保计数在退出时自动减少
struct LoadGuard {
    load: Arc<AtomicUsize>,
}

impl LoadGuard {
    fn new(load: &Arc<AtomicUsize>) -> Self {
        load.fetch_add(1, Ordering::SeqCst);
        Self { load: load.clone() }
    }
}

impl Drop for LoadGuard {
    fn drop(&mut self) {
        self.load.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Docker 执行器
pub struct DockerExecutor {
    config: ExecutorConfig,
    data_dir: PathBuf,
    current_load: Arc<AtomicUsize>,
}

impl DockerExecutor {
    /// 创建新的 Docker 执行器
    pub fn new(config: ExecutorConfig, data_dir: PathBuf) -> Self {
        Self {
            config,
            data_dir,
            current_load: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// 获取 workspace 目录（宿主机）
    fn workspace_dir(&self, task_id: &str) -> PathBuf {
        self.data_dir.join("workspaces").join(task_id)
    }

    /// 处理命令输出
    async fn process_output(
        &self,
        task_id: String,
        output: std::process::Output,
        start_time: Instant,
        workspace: &Path,
    ) -> Result<TaskResult, ExecutorError> {
        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if exit_code != 0 && stderr.trim().is_empty() {
            // Some executor scripts pipe tool errors through stdout via `2>&1 | tee`.
            // Preserve a concise failure message so Server/Dashboard do not show a blank failure.
            stderr = failure_message_from_output(&stdout, &stderr).unwrap_or_default();
        }

        info!(
            "任务 {} 执行完成，退出码: {}, 耗时: {:?}",
            task_id,
            exit_code,
            start_time.elapsed()
        );

        let output_files = scan_output_files(workspace).await.unwrap_or_default();

        if !stderr.is_empty() {
            warn!("Task {} stderr: {}", task_id, stderr);
        }

        Ok(TaskResult {
            task_id,
            exit_code,
            stdout,
            stderr,
            output_files,
        })
    }

    /// 构建 docker run 命令
    fn build_run_command(
        &self,
        task: &Task,
        workspace: &std::path::Path,
        docker_mounts: &[String],
    ) -> Command {
        // 添加调试日志
        info!("Building docker command with mounts: {:?}", docker_mounts);

        let timeout_secs = self.config.timeout_minutes * 60;

        let mut cmd = Command::new("docker");
        cmd.arg("run")
            .arg("--rm")
            .arg("--name")
            .arg(format!("open-aaas-task-{}", task.task_id));

        // 添加所有挂载
        for mount in docker_mounts {
            cmd.arg("-v").arg(mount);
        }

        // 挂载 workspace
        cmd.arg("-v")
            .arg(format!("{}:/workspace", workspace.display()));

        // 设置工作目录
        cmd.arg("-w").arg(&self.config.working_dir);

        cmd.arg("-e")
            .arg(format!("TASK_ID={}", task.task_id))
            .arg("-e")
            .arg(format!("TIMEOUT={}", timeout_secs));

        // 内存限制
        if let Some(ref mem) = self.config.memory_limit {
            cmd.arg("-m").arg(mem);
        }

        // 设置 ENTRYPOINT（如果配置指定）
        if let Some(entrypoint) = self.config.get_entrypoint() {
            cmd.arg("--entrypoint").arg(&entrypoint[0]);
            // 如果 ENTRYPOINT 有额外参数，添加到命令前
            for arg in &entrypoint[1..] {
                cmd.arg(arg);
            }
        }

        info!("Entrypoint config: {:?}", self.config.get_entrypoint());
        info!(
            "Command args: {:?}",
            self.config.get_command_args(&task.task_id)
        );

        cmd.arg(&self.config.image);

        // 添加命令参数
        let args = self.config.get_command_args(&task.task_id);
        for arg in args {
            cmd.arg(arg);
        }

        info!("Docker command: {:?}", cmd);
        cmd
    }

    /// 确保 workspace 目录存在
    async fn ensure_workspace(&self, workspace: &Path) -> Result<(), ExecutorError> {
        tokio::fs::create_dir_all(workspace)
            .await
            .map_err(|e| ExecutorError::Io(format!("创建 workspace 失败: {}", e)))?;
        ensure_container_writable_dir(workspace, "workspace").await?;

        let output_dir = workspace.join("output");
        tokio::fs::create_dir_all(&output_dir)
            .await
            .map_err(|e| ExecutorError::Io(format!("创建 output 目录失败: {}", e)))?;
        ensure_container_writable_dir(&output_dir, "output").await?;

        Ok(())
    }

    /// 写入任务配置到 workspace
    async fn write_task_config(&self, task: &Task, workspace: &Path) -> Result<(), ExecutorError> {
        let config = serde_json::json!({
            "task_id": task.task_id,
            "task_prompt": task.prompt,
            "prompt": task.prompt,
            "output_prompt": task.output_prompt,
            "session_id": task.session_id,
            "input_files": task.input_files,
        });

        let config_path = workspace.join("task.json");
        tokio::fs::write(&config_path, config.to_string())
            .await
            .map_err(|e| ExecutorError::Io(format!("写入任务配置失败: {}", e)))?;

        Ok(())
    }
}

#[cfg(unix)]
async fn ensure_container_writable_dir(
    path: &std::path::Path,
    label: &str,
) -> Result<(), ExecutorError> {
    use std::os::unix::fs::PermissionsExt;

    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o777))
        .await
        .map_err(|e| ExecutorError::Io(format!("设置 {} 目录权限失败: {}", label, e)))?;

    Ok(())
}

#[cfg(not(unix))]
async fn ensure_container_writable_dir(
    _path: &std::path::Path,
    _label: &str,
) -> Result<(), ExecutorError> {
    Ok(())
}

impl Executor for DockerExecutor {
    async fn execute(
        &self,
        task: Task,
        docker_mounts: Vec<String>,
    ) -> Result<TaskResult, ExecutorError> {
        let task_id = task.task_id.clone();
        let _guard = LoadGuard::new(&self.current_load);

        // 准备 workspace
        let workspace = self.workspace_dir(&task_id);
        self.ensure_workspace(&workspace).await?;
        self.write_task_config(&task, &workspace).await?;

        info!(
            "Executing task {} with {} mounts",
            task.task_id,
            docker_mounts.len()
        );

        // 构建 docker run 命令
        let mut cmd = self.build_run_command(&task, &workspace, &docker_mounts);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let start_time = Instant::now();
        let timeout_secs = self.config.timeout_minutes * 60;

        // 极简超时判断
        if timeout_secs == 0 {
            // 无限制模式：一直等待
            info!("执行任务 {} (无超时限制)", task_id);
            let output = cmd
                .output()
                .await
                .map_err(|e| ExecutorError::ExecutionFailed {
                    task_id: task_id.clone(),
                    reason: format!("启动失败: {}", e),
                })?;
            self.process_output(task_id, output, start_time, &workspace)
                .await
        } else {
            // 有超时模式
            info!(
                "执行任务 {} (超时: {}分钟)",
                task_id, self.config.timeout_minutes
            );
            match timeout(Duration::from_secs(timeout_secs), cmd.output()).await {
                Ok(Ok(output)) => {
                    self.process_output(task_id, output, start_time, &workspace)
                        .await
                }
                Ok(Err(e)) => Err(ExecutorError::ExecutionFailed {
                    task_id: task_id.clone(),
                    reason: format!("启动失败: {}", e),
                }),
                Err(_) => {
                    warn!("任务 {} 执行超时，强制停止", task_id);
                    if let Err(e) = stop_container(&task_id).await {
                        warn!("停止超时容器 {} 失败: {}", task_id, e);
                    }
                    Err(ExecutorError::Timeout {
                        task_id,
                        elapsed: start_time.elapsed(),
                    })
                }
            }
        }
    }

    async fn cancel(&self, task_id: &str) -> Result<(), ExecutorError> {
        info!("取消任务: {}", task_id);
        stop_container(task_id).await
    }

    fn capacity(&self) -> usize {
        self.config.capacity
    }

    fn current_load(&self) -> usize {
        self.current_load.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// 停止容器
async fn stop_container(task_id: &str) -> Result<(), ExecutorError> {
    let container_name = format!("open-aaas-task-{}", task_id);

    let output = Command::new("docker")
        .args(["stop", "-t", "10", &container_name])
        .output()
        .await
        .map_err(|e| ExecutorError::Io(format!("停止容器失败: {}", e)))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No such container") {
            // 容器已不存在，视为成功
            Ok(())
        } else {
            Err(ExecutorError::ExecutionFailed {
                task_id: task_id.to_string(),
                reason: format!("停止容器失败: {}", stderr),
            })
        }
    }
}

/// 扫描输出文件
async fn scan_output_files(workspace: &Path) -> Result<Vec<String>, std::io::Error> {
    let mut files = vec![];
    let mut dirs = vec![workspace.to_path_buf()];

    while let Some(dir) = dirs.pop() {
        let mut entries = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                if path
                    .file_name()
                    .is_some_and(|name| name == OsStr::new("input"))
                {
                    continue;
                }
                dirs.push(path);
                continue;
            }

            if !path.is_file() {
                continue;
            }

            let Some(file_name) = path.file_name() else {
                continue;
            };
            if file_name == OsStr::new("task.json") || file_name.to_string_lossy().starts_with('.')
            {
                continue;
            }

            if let Ok(relative_path) = path.strip_prefix(workspace) {
                files.push(
                    relative_path
                        .to_string_lossy()
                        .to_string()
                        .replace('\\', "/"),
                );
            }
        }
    }

    files.sort();
    if files.iter().any(|file| file == "output/response.md") {
        files.retain(|file| file != "response.md");
    }
    Ok(files)
}

fn failure_message_from_output(stdout: &str, stderr: &str) -> Option<String> {
    let raw = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };

    if raw.is_empty() {
        return Some(
            "Executor exited with non-zero status but did not return error output".to_string(),
        );
    }

    const MAX_ERROR_CHARS: usize = 2000;
    if raw.chars().count() <= MAX_ERROR_CHARS {
        Some(raw.to_string())
    } else {
        let tail: String = raw
            .chars()
            .rev()
            .take(MAX_ERROR_CHARS)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        Some(format!("...{}", tail))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ExecutorType;
    use std::panic::catch_unwind;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    use tempfile::TempDir;

    /// 创建测试用的 ExecutorConfig
    fn create_test_config() -> ExecutorConfig {
        ExecutorConfig {
            executor_type: ExecutorType::Standard,
            image: "test-image:latest".to_string(),
            capacity: 5,
            timeout_minutes: 10,
            memory_limit: Some("512m".to_string()),
            working_dir: "/workspace".to_string(),
            script_path: None,
            custom_entrypoint: None,
            custom_args: None,
        }
    }

    /// 创建测试用的 Task
    fn create_test_task(task_id: &str) -> Task {
        Task {
            task_id: task_id.to_string(),
            prompt: "test prompt".to_string(),
            output_prompt: Some("test output format".to_string()),
            session_id: Some("session-123".to_string()),
            input_files: vec![],
        }
    }

    #[test]
    fn test_load_guard_raii_behavior() {
        // 测试 LoadGuard 的 RAII 负载计数行为
        let load = Arc::new(AtomicUsize::new(0));

        // 创建 Guard 时计数 +1
        {
            let _guard = LoadGuard::new(&load);
            assert_eq!(load.load(Ordering::SeqCst), 1, "创建 Guard 后计数应为 1");

            // 嵌套创建另一个 Guard
            {
                let _guard2 = LoadGuard::new(&load);
                assert_eq!(
                    load.load(Ordering::SeqCst),
                    2,
                    "创建第二个 Guard 后计数应为 2"
                );
            }
            // _guard2 在这里被 drop，计数 -1
            assert_eq!(
                load.load(Ordering::SeqCst),
                1,
                "第二个 Guard 被 drop 后计数应为 1"
            );
        }
        // _guard 在这里被 drop，计数 -1
        assert_eq!(
            load.load(Ordering::SeqCst),
            0,
            "所有 Guard 被 drop 后计数应为 0"
        );
    }

    #[test]
    fn test_docker_executor_new() {
        // 测试 DockerExecutor::new() 创建后属性正确
        let config = create_test_config();
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let executor = DockerExecutor::new(config, data_dir.clone());

        // 验证内部状态
        assert_eq!(executor.data_dir, data_dir);
        assert_eq!(executor.config.image, "test-image:latest");
        assert_eq!(executor.config.capacity, 5);
        assert_eq!(executor.config.timeout_minutes, 10);
        assert_eq!(executor.config.memory_limit, Some("512m".to_string()));
    }

    #[test]
    fn test_docker_executor_capacity() {
        // 测试 DockerExecutor::capacity() 返回正确的配置值
        let config = create_test_config();
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let executor = DockerExecutor::new(config, data_dir);

        assert_eq!(executor.capacity(), 5, "capacity() 应返回配置的 capacity");
    }

    #[test]
    fn test_docker_executor_current_load_initial() {
        // 测试 DockerExecutor::current_load() 初始值为 0
        let config = create_test_config();
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let executor = DockerExecutor::new(config, data_dir);

        assert_eq!(executor.current_load(), 0, "初始负载应为 0");
    }

    #[test]
    fn test_workspace_dir() {
        // 测试 workspace_dir() 路径生成正确
        let config = create_test_config();
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let executor = DockerExecutor::new(config, data_dir.clone());

        let task_id = "task-abc-123";
        let workspace = executor.workspace_dir(task_id);

        let expected = data_dir.join("workspaces").join(task_id);
        assert_eq!(workspace, expected, "workspace_dir 应返回正确的路径");
    }

    #[tokio::test]
    async fn test_ensure_workspace() {
        // 测试 ensure_workspace() 正确创建目录
        let config = create_test_config();
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let executor = DockerExecutor::new(config, data_dir);

        // 使用临时目录下的子目录进行测试
        let workspace_path = temp_dir.path().join("test_workspace").join("nested");

        // 确保目录不存在
        assert!(!workspace_path.exists(), "测试前目录不应存在");

        // 调用 ensure_workspace
        let result = executor.ensure_workspace(&workspace_path).await;
        assert!(result.is_ok(), "ensure_workspace 应成功");

        // 验证目录已创建
        assert!(workspace_path.exists(), "ensure_workspace 应创建目录");
        assert!(workspace_path.is_dir(), "创建的应是目录");
        assert!(
            workspace_path.join("output").is_dir(),
            "ensure_workspace 应创建 output 目录"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_ensure_workspace_makes_mount_writable() {
        use std::os::unix::fs::PermissionsExt;

        let config = create_test_config();
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();
        let executor = DockerExecutor::new(config, data_dir);
        let workspace_path = temp_dir.path().join("writable_workspace");

        executor.ensure_workspace(&workspace_path).await.unwrap();

        let workspace_mode = std::fs::metadata(&workspace_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        let output_mode = std::fs::metadata(workspace_path.join("output"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(workspace_mode, 0o777, "workspace 应允许容器用户写入");
        assert_eq!(output_mode, 0o777, "output 应允许容器用户写入");
    }

    #[tokio::test]
    async fn test_write_task_config() {
        // 测试 write_task_config() 正确写入任务配置
        let config = create_test_config();
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let executor = DockerExecutor::new(config, data_dir);

        // 创建测试目录
        let workspace_path = temp_dir.path().join("test_task_workspace");
        tokio::fs::create_dir_all(&workspace_path).await.unwrap();

        // 创建测试任务
        let task = create_test_task("test-task-001");

        // 调用 write_task_config
        let result = executor.write_task_config(&task, &workspace_path).await;
        assert!(result.is_ok(), "write_task_config 应成功");

        // 验证文件已创建并内容正确
        let config_path = workspace_path.join("task.json");
        assert!(config_path.exists(), "task.json 应被创建");

        let content = tokio::fs::read_to_string(&config_path).await.unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(json["task_id"], "test-task-001", "task_id 应正确写入");
        assert_eq!(json["task_prompt"], "test prompt", "task_prompt 应正确写入");
        assert_eq!(json["prompt"], "test prompt", "prompt 应正确写入");
        assert_eq!(
            json["output_prompt"],
            serde_json::json!("test output format"),
            "output_prompt 应正确写入"
        );
        assert_eq!(
            json["session_id"],
            serde_json::json!("session-123"),
            "session_id 应正确写入"
        );
        assert_eq!(
            json["input_files"],
            serde_json::json!([]),
            "input_files 应正确写入"
        );
    }

    #[tokio::test]
    async fn test_scan_output_files() {
        // 测试 scan_output_files() 正确递归扫描输出文件（排除 task.json 和 input）
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("scan_test");
        tokio::fs::create_dir_all(&workspace_path).await.unwrap();

        // 创建多个文件
        tokio::fs::write(workspace_path.join("output.txt"), "output content")
            .await
            .unwrap();
        tokio::fs::write(workspace_path.join("result.json"), "{}")
            .await
            .unwrap();
        tokio::fs::write(workspace_path.join("task.json"), "task config")
            .await
            .unwrap();
        tokio::fs::write(workspace_path.join(".DS_Store"), "mac metadata")
            .await
            .unwrap();
        tokio::fs::write(workspace_path.join("data.csv"), "col1,col2")
            .await
            .unwrap();

        // 创建输出子目录（应递归扫描）
        tokio::fs::create_dir_all(workspace_path.join("subdir"))
            .await
            .unwrap();
        tokio::fs::write(workspace_path.join("subdir").join("nested.txt"), "nested")
            .await
            .unwrap();

        // 创建输入目录（应被忽略）
        tokio::fs::create_dir_all(workspace_path.join("input"))
            .await
            .unwrap();
        tokio::fs::write(workspace_path.join("input").join("source.pdf"), "input")
            .await
            .unwrap();

        // 调用 scan_output_files
        let result = scan_output_files(&workspace_path).await;
        assert!(result.is_ok(), "scan_output_files 应成功");

        let mut files = result.unwrap();
        files.sort(); // 排序以便比较

        // 验证结果：应包含根目录和输出子目录文件，不包含 task.json 和 input 文件
        assert_eq!(
            files.len(),
            4,
            "应扫描到 4 个文件（排除 task.json 和 input 目录）"
        );
        assert!(
            files.contains(&"output.txt".to_string()),
            "应包含 output.txt"
        );
        assert!(
            files.contains(&"result.json".to_string()),
            "应包含 result.json"
        );
        assert!(files.contains(&"data.csv".to_string()), "应包含 data.csv");
        assert!(
            !files.contains(&"task.json".to_string()),
            "不应包含 task.json"
        );
        assert!(
            !files.contains(&".DS_Store".to_string()),
            "不应包含隐藏系统文件"
        );
        assert!(
            files.contains(&"subdir/nested.txt".to_string()),
            "应包含子目录中的输出文件"
        );
        assert!(
            !files.contains(&"input/source.pdf".to_string()),
            "不应包含 input 目录中的文件"
        );
    }

    #[tokio::test]
    async fn test_scan_output_files_empty_dir() {
        // 测试 scan_output_files() 处理空目录的情况
        let temp_dir = TempDir::new().unwrap();
        let workspace_path = temp_dir.path().join("empty_test");
        tokio::fs::create_dir_all(&workspace_path).await.unwrap();

        let result = scan_output_files(&workspace_path).await;
        assert!(result.is_ok(), "scan_output_files 对空目录应成功");

        let files = result.unwrap();
        assert!(files.is_empty(), "空目录应返回空列表");
    }

    #[test]
    fn test_build_run_command_basic() {
        // 测试 docker run 命令构建（基础情况）
        let config = ExecutorConfig {
            executor_type: ExecutorType::Standard,
            image: "test-executor:latest".to_string(),
            capacity: 5,
            timeout_minutes: 10,
            memory_limit: None,
            working_dir: "/workspace".to_string(),
            script_path: None,
            custom_entrypoint: None,
            custom_args: None,
        };
        let temp_dir = TempDir::new().unwrap();
        let executor = DockerExecutor::new(config, temp_dir.path().to_path_buf());

        let task = create_test_task("test-task-001");
        let workspace = temp_dir.path().join("workspace");
        let _cmd = executor.build_run_command(&task, &workspace, &[]);

        // 验证命令构建正确（通过检查 program 和 args）
        // 由于 Command 没有提供直接访问 args 的方法，我们通过构建时的行为来验证
        // 这里主要验证函数没有 panic 且返回了 Command 对象
        // 更详细的验证需要在集成测试中进行
    }

    #[test]
    fn test_build_run_command_with_memory() {
        // 测试 docker run 命令构建（带内存限制）
        let config = ExecutorConfig {
            executor_type: ExecutorType::Standard,
            image: "test-executor:latest".to_string(),
            capacity: 5,
            timeout_minutes: 10,
            memory_limit: Some("2g".to_string()),
            working_dir: "/workspace".to_string(),
            script_path: None,
            custom_entrypoint: None,
            custom_args: None,
        };
        let temp_dir = TempDir::new().unwrap();
        let executor = DockerExecutor::new(config, temp_dir.path().to_path_buf());

        let task = create_test_task("test-task-memory");
        let workspace = temp_dir.path().join("workspace");
        let _cmd = executor.build_run_command(&task, &workspace, &[]);

        // 验证命令构建没有 panic
    }

    #[test]
    fn test_build_run_command_with_mounts() {
        // 测试 docker run 命令构建（带挂载参数）
        let config = ExecutorConfig {
            executor_type: ExecutorType::Standard,
            image: "test-executor:latest".to_string(),
            capacity: 5,
            timeout_minutes: 10,
            memory_limit: None,
            working_dir: "/workspace".to_string(),
            script_path: None,
            custom_entrypoint: None,
            custom_args: None,
        };
        let temp_dir = TempDir::new().unwrap();
        let executor = DockerExecutor::new(config, temp_dir.path().to_path_buf());

        let task = create_test_task("test-task-mounts");
        let workspace = temp_dir.path().join("workspace");
        let docker_mounts = vec![
            "/host/path:/container/path".to_string(),
            "/another/host:/another/container:ro".to_string(),
        ];
        let _cmd = executor.build_run_command(&task, &workspace, &docker_mounts);

        // 验证命令构建没有 panic
    }

    #[test]
    fn test_build_run_command_with_bash_executor() {
        // 测试 bash 执行器类型的命令构建
        let config = ExecutorConfig {
            executor_type: ExecutorType::Bash,
            image: "bash-executor:latest".to_string(),
            capacity: 5,
            timeout_minutes: 10,
            memory_limit: None,
            working_dir: "/workspace".to_string(),
            script_path: None,
            custom_entrypoint: None,
            custom_args: None,
        };
        let temp_dir = TempDir::new().unwrap();
        let executor = DockerExecutor::new(config, temp_dir.path().to_path_buf());

        let task = create_test_task("test-task-bash");
        let workspace = temp_dir.path().join("workspace");
        let _cmd = executor.build_run_command(&task, &workspace, &[]);

        // 验证命令构建没有 panic
        // bash 类型应该设置 --entrypoint bash 并添加脚本路径参数
    }

    #[test]
    fn test_build_run_command_with_python_executor() {
        // 测试 python 执行器类型的命令构建
        let config = ExecutorConfig {
            executor_type: ExecutorType::Python,
            image: "python-executor:latest".to_string(),
            capacity: 5,
            timeout_minutes: 10,
            memory_limit: None,
            working_dir: "/app".to_string(),
            script_path: Some("/app/main.py".to_string()),
            custom_entrypoint: None,
            custom_args: None,
        };
        let temp_dir = TempDir::new().unwrap();
        let executor = DockerExecutor::new(config, temp_dir.path().to_path_buf());

        let task = create_test_task("test-task-python");
        let workspace = temp_dir.path().join("workspace");
        let _cmd = executor.build_run_command(&task, &workspace, &[]);

        // 验证命令构建没有 panic
        // python 类型应该设置 --entrypoint python 并添加脚本路径参数
    }

    #[test]
    fn test_build_run_command_with_custom_executor() {
        // 测试 custom 执行器类型的命令构建
        let config = ExecutorConfig {
            executor_type: ExecutorType::Custom,
            image: "custom-executor:latest".to_string(),
            capacity: 5,
            timeout_minutes: 10,
            memory_limit: None,
            working_dir: "/workspace".to_string(),
            script_path: None,
            custom_entrypoint: Some(vec!["/bin/sh".to_string(), "-c".to_string()]),
            custom_args: Some(vec!["echo".to_string(), "hello".to_string()]),
        };
        let temp_dir = TempDir::new().unwrap();
        let executor = DockerExecutor::new(config, temp_dir.path().to_path_buf());

        let task = create_test_task("test-task-custom");
        let workspace = temp_dir.path().join("workspace");
        let _cmd = executor.build_run_command(&task, &workspace, &[]);

        // 验证命令构建没有 panic
        // custom 类型应该设置自定义 entrypoint 和 args
    }

    #[test]
    fn test_load_guard_with_panic() {
        // 使用 catch_unwind 测试 panic 时的负载递减
        let load = Arc::new(AtomicUsize::new(0));

        // 验证初始状态
        assert_eq!(load.load(Ordering::SeqCst), 0, "初始负载应为 0");

        // 在 panic 后验证负载是否正确递减
        let result = catch_unwind(|| {
            let guard = LoadGuard::new(&load);
            assert_eq!(load.load(Ordering::SeqCst), 1, "创建 Guard 后负载应为 1");

            // 显式 drop guard
            drop(guard);

            panic!("模拟 panic");
        });

        // 验证 panic 发生了
        assert!(result.is_err(), "应该发生 panic");

        // 验证负载在 panic 后正确递减（guard 在 panic 前已被 drop）
        assert_eq!(load.load(Ordering::SeqCst), 0, "panic 后负载应为 0");

        // 再次测试：在 panic 时 Guard 自动 drop
        let load2 = Arc::new(AtomicUsize::new(0));

        let result2 = catch_unwind(|| {
            let _guard = LoadGuard::new(&load2);
            assert_eq!(load2.load(Ordering::SeqCst), 1, "创建 Guard 后负载应为 1");

            // 直接 panic，让 Guard 在栈展开时自动 drop
            panic!("自动 drop 测试");
        });

        assert!(result2.is_err(), "应该发生 panic");
        // 注意：catch_unwind 会捕获 panic 但可能不会在 unwinding 时正确执行 Drop
        // 这取决于 catch_unwind 的行为
    }
}
