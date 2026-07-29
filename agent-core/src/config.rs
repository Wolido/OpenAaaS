//! 配置管理

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// 执行器类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorType {
    /// 标准执行器（使用容器 ENTRYPOINT，默认行为）
    #[default]
    Standard,
    /// Bash 执行器
    Bash,
    /// Python 执行器
    Python,
    /// 自定义执行器
    Custom,
}

impl ExecutorType {
    /// 获取容器的 ENTRYPOINT
    pub fn get_entrypoint(&self, custom_entrypoint: Option<&Vec<String>>) -> Option<Vec<String>> {
        match self {
            ExecutorType::Standard => None, // 使用容器默认 ENTRYPOINT
            ExecutorType::Bash => Some(vec!["bash".to_string()]),
            ExecutorType::Python => Some(vec!["python".to_string()]),
            ExecutorType::Custom => custom_entrypoint.cloned(),
        }
    }

    /// 获取容器的命令参数
    pub fn get_command_args(
        &self,
        task_id: &str,
        working_dir: &str,
        script_path: Option<&String>,
        custom_args: Option<&Vec<String>>,
    ) -> Vec<String> {
        let workspace = format!("{}/{}", working_dir, task_id);

        match self {
            ExecutorType::Standard => vec![], // 使用容器默认 CMD
            ExecutorType::Bash => {
                let script = script_path
                    .cloned()
                    .unwrap_or_else(|| format!("{}/run.sh", workspace));
                vec![script]
            }
            ExecutorType::Python => {
                let script = script_path
                    .cloned()
                    .unwrap_or_else(|| format!("{}/run.py", workspace));
                vec![script]
            }
            ExecutorType::Custom => custom_args.cloned().unwrap_or_default(),
        }
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("解析错误: {0}")]
    Parse(String),
    #[error("目录错误: {0}")]
    Directory(String),
}

/// Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub executor: ExecutorConfig,
    #[serde(default)]
    pub paths: PathConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_server_url")]
    pub base_url: String,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    /// 是否使用系统代理
    #[serde(default = "default_use_system_proxy")]
    pub use_system_proxy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentConfig {
    /// 服务 ID（注册后填充）
    pub service_id: Option<String>,
    /// API Key（注册后填充）
    pub api_key: Option<String>,
    /// Agent 名称
    pub name: Option<String>,
}

impl AgentConfig {
    /// 将空字符串字段归一化为未设置，兼容旧配置模板。
    pub fn normalize(&mut self) {
        normalize_optional_string(&mut self.service_id);
        normalize_optional_string(&mut self.api_key);
        normalize_optional_string(&mut self.name);
    }

    pub fn has_credentials(&self) -> bool {
        self.service_id.as_deref().is_some_and(has_non_empty_value)
            && self.api_key.as_deref().is_some_and(has_non_empty_value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// 执行器类型
    #[serde(default)]
    pub executor_type: ExecutorType,

    /// Docker 镜像
    #[serde(default = "default_executor_image")]
    pub image: String,

    /// 并发任务数
    #[serde(default = "default_capacity")]
    pub capacity: usize,

    /// 任务超时（分钟）
    #[serde(default = "default_timeout")]
    pub timeout_minutes: u64,

    /// 内存限制
    pub memory_limit: Option<String>,

    /// 工作目录（容器内）
    #[serde(default = "default_working_dir")]
    pub working_dir: String,

    /// 脚本路径（用于 bash/python 类型）
    pub script_path: Option<String>,

    /// 自定义 ENTRYPOINT（仅用于 custom 类型）
    pub custom_entrypoint: Option<Vec<String>>,

    /// 自定义命令参数（仅用于 custom 类型）
    pub custom_args: Option<Vec<String>>,

    /// 是否允许任务容器访问宿主机服务（注入 host.docker.internal:host-gateway）
    #[serde(default)]
    pub enable_host_access: bool,
}

/// 单个挂载配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    /// 宿主机路径（相对二进制或绝对路径）
    pub host: String,
    /// 容器内路径
    pub container: String,
    /// 是否只读
    #[serde(default = "default_mount_readonly")]
    pub readonly: bool,
}

fn default_mount_readonly() -> bool {
    false
}

fn has_non_empty_value(value: &str) -> bool {
    !value.trim().is_empty()
}

fn normalize_optional_string(value: &mut Option<String>) {
    if value.as_deref().is_some_and(|item| item.trim().is_empty()) {
        *value = None;
    }
}

/// 路径配置（只保留数据目录）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    /// 自定义数据目录（用于 pidfile、数据库等）
    #[serde(default = "default_data_dir")]
    pub data_dir: Option<PathBuf>,
    /// 挂载列表
    #[serde(default = "default_mounts")]
    pub mounts: Vec<Mount>,
}

fn default_mounts() -> Vec<Mount> {
    Vec::new()
}

fn default_data_dir() -> Option<PathBuf> {
    Some(PathBuf::from("./data"))
}

// 默认值函数
fn default_server_url() -> String {
    "http://127.0.0.1:8080".to_string()
}

fn default_poll_interval() -> u64 {
    5
}

fn default_capacity() -> usize {
    2
}

fn default_executor_image() -> String {
    "open-aaas-executor:latest".to_string()
}

fn default_timeout() -> u64 {
    0 // 默认无超时
}

fn default_working_dir() -> String {
    "/workspace".to_string()
}

fn default_use_system_proxy() -> bool {
    false
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            base_url: default_server_url(),
            poll_interval_secs: default_poll_interval(),
            use_system_proxy: default_use_system_proxy(),
        }
    }
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            executor_type: ExecutorType::default(),
            image: default_executor_image(),
            capacity: default_capacity(),
            timeout_minutes: default_timeout(),
            memory_limit: None,
            working_dir: default_working_dir(),
            script_path: None,
            custom_entrypoint: None,
            custom_args: None,
            enable_host_access: false,
        }
    }
}

impl ExecutorConfig {
    /// 获取容器的 ENTRYPOINT
    pub fn get_entrypoint(&self) -> Option<Vec<String>> {
        self.executor_type
            .get_entrypoint(self.custom_entrypoint.as_ref())
    }

    /// 获取容器的命令参数
    pub fn get_command_args(&self, task_id: &str) -> Vec<String> {
        self.executor_type.get_command_args(
            task_id,
            &self.working_dir,
            self.script_path.as_ref(),
            self.custom_args.as_ref(),
        )
    }
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            data_dir: Some(PathBuf::from("./data")),
            mounts: default_mounts(),
        }
    }
}

impl Config {
    fn normalize(&mut self) {
        self.agent.normalize();
    }

    /// 从指定路径加载配置
    pub async fn load_from_path(config_path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let config_path = config_path.as_ref();

        if !config_path.exists() {
            // 创建默认配置
            let config = Config::default();
            config.save_to_path(config_path).await?;
            return Ok(config);
        }

        let content = tokio::fs::read_to_string(&config_path).await?;
        let mut config: Config =
            toml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))?;
        config.normalize();

        Ok(config)
    }

    fn toml_str(value: &str) -> String {
        toml::Value::String(value.to_string()).to_string()
    }

    /// 生成带注释的运行时 TOML 配置文本
    pub fn to_runtime_toml(&self) -> String {
        let mut lines = Vec::new();

        lines.push("# OpenAaaS Agent Core 配置文件".to_string());
        lines.push(
            "# 首次运行时自动生成，注册后 [agent] 节会自动填充 service_id 和 api_key".to_string(),
        );
        lines.push(String::new());

        // [server]
        lines.push("[server]".to_string());
        lines.push("# Server 地址".to_string());
        lines.push(format!(
            "base_url = {}",
            Self::toml_str(&self.server.base_url)
        ));
        lines.push("# 轮询间隔（秒）".to_string());
        lines.push(format!(
            "poll_interval_secs = {}",
            self.server.poll_interval_secs
        ));
        lines.push("# 是否使用系统代理".to_string());
        lines.push(format!(
            "use_system_proxy = {}",
            self.server.use_system_proxy
        ));
        lines.push(String::new());

        // [agent]
        lines.push("[agent]".to_string());
        if let Some(ref id) = self.agent.service_id {
            lines.push("# 服务 ID（注册后自动填充）".to_string());
            lines.push(format!("service_id = {}", Self::toml_str(id)));
        } else {
            lines.push("# 服务 ID（注册后自动填充）".to_string());
            lines.push(r#"# service_id = "svc-xxx""#.to_string());
        }
        if let Some(ref key) = self.agent.api_key {
            lines.push("# API Key（注册后自动填充）".to_string());
            lines.push(format!("api_key = {}", Self::toml_str(key)));
        } else {
            lines.push("# API Key（注册后自动填充）".to_string());
            lines.push(r#"# api_key = "ak_xxx""#.to_string());
        }
        let name = self
            .agent
            .name
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("agent-core");
        lines.push("# Agent 名称".to_string());
        lines.push(format!("name = {}", Self::toml_str(name)));
        lines.push(String::new());

        // [executor]
        lines.push("[executor]".to_string());
        lines.push("# 执行器类型：standard / bash / python / custom".to_string());
        let exec_type_str = match self.executor.executor_type {
            ExecutorType::Standard => "standard",
            ExecutorType::Bash => "bash",
            ExecutorType::Python => "python",
            ExecutorType::Custom => "custom",
        };
        lines.push(format!("executor_type = {}", Self::toml_str(exec_type_str)));
        lines.push("# Docker 镜像".to_string());
        lines.push(format!("image = {}", Self::toml_str(&self.executor.image)));
        lines.push("# 并发任务数".to_string());
        lines.push(format!("capacity = {}", self.executor.capacity));
        lines.push("# 任务超时时间（分钟），0 表示不限制".to_string());
        lines.push(format!(
            "timeout_minutes = {}",
            self.executor.timeout_minutes
        ));
        if let Some(ref mem) = self.executor.memory_limit {
            lines.push("# 内存限制".to_string());
            lines.push(format!("memory_limit = {}", Self::toml_str(mem)));
        }
        lines.push(
            "# 是否允许任务容器访问宿主机服务（可选，true = 注入 host.docker.internal 指向宿主机）"
                .to_string(),
        );
        lines.push(
            "# 安全风险：开启后任务容器可访问宿主机上监听 0.0.0.0 的所有服务，请谨慎开启"
                .to_string(),
        );
        if self.executor.enable_host_access {
            lines.push("enable_host_access = true".to_string());
        } else {
            lines.push("# enable_host_access = false".to_string());
        }
        lines.push("# 工作目录（容器内）".to_string());
        lines.push(format!(
            "working_dir = {}",
            Self::toml_str(&self.executor.working_dir)
        ));
        if let Some(ref script) = self.executor.script_path {
            lines.push("# 脚本路径（仅 bash / python 类型使用）".to_string());
            lines.push(format!("script_path = {}", Self::toml_str(script)));
        }
        if let Some(ref entrypoint) = self.executor.custom_entrypoint {
            lines.push("# 自定义 ENTRYPOINT（仅 custom 类型使用）".to_string());
            let arr = entrypoint
                .iter()
                .map(|s| Self::toml_str(s))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("custom_entrypoint = [{}]", arr));
        }
        if let Some(ref args) = self.executor.custom_args {
            lines.push("# 自定义命令参数（仅 custom 类型使用）".to_string());
            let arr = args
                .iter()
                .map(|s| Self::toml_str(s))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("custom_args = [{}]", arr));
        }
        lines.push(String::new());

        // [paths]
        lines.push("[paths]".to_string());
        lines.push("# 数据目录".to_string());
        let data_dir = self
            .paths
            .data_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "./data".to_string());
        lines.push(format!("data_dir = {}", Self::toml_str(&data_dir)));
        if self.paths.mounts.is_empty() {
            lines.push("mounts = []".to_string());
        } else {
            for mount in &self.paths.mounts {
                lines.push(String::new());
                lines.push("[[paths.mounts]]".to_string());
                lines.push(format!("host = {}", Self::toml_str(&mount.host)));
                lines.push(format!("container = {}", Self::toml_str(&mount.container)));
                lines.push(format!("readonly = {}", mount.readonly));
            }
        }

        lines.join("\n") + "\n"
    }

    /// 保存配置到指定路径
    pub async fn save_to_path(&self, config_path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let config_path = config_path.as_ref();

        // 确保配置目录存在
        if let Some(parent) = config_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut sanitized = self.clone();
        sanitized.normalize();

        let content = sanitized.to_runtime_toml();

        tokio::fs::write(&config_path, content).await?;
        Ok(())
    }

    /// 获取配置文件路径（使用当前工作目录）
    #[cfg(not(test))]
    pub fn config_path() -> PathBuf {
        // 使用当前工作目录
        std::env::current_dir()
            .expect("无法获取当前工作目录")
            .join("config.toml")
    }

    /// 测试时使用临时目录
    #[cfg(test)]
    pub fn config_path() -> PathBuf {
        let temp_dir = std::env::temp_dir().join("open-aaas-test").join(format!(
            "config-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        // 确保目录存在
        let _ = std::fs::create_dir_all(&temp_dir);
        temp_dir.join("config.toml")
    }

    /// 获取数据目录（跨平台）
    pub fn data_dir(&self) -> PathBuf {
        if let Some(ref custom) = self.paths.data_dir {
            return custom.clone();
        }

        // 尝试获取标准数据目录，失败时使用当前目录
        dirs::data_local_dir()
            .map(|p| p.join("open-aaas"))
            .unwrap_or_else(|| {
                tracing::warn!("无法获取标准数据目录，使用当前目录");
                std::env::current_dir()
                    .map(|p| p.join(".open-aaas-data"))
                    .unwrap_or_else(|_| PathBuf::from(".open-aaas-data"))
            })
    }

    /// 获取任务工作目录
    pub fn workspace_dir(&self, task_id: &str) -> PathBuf {
        self.data_dir().join("workspaces").join(task_id)
    }

    /// 获取数据库路径
    pub fn database_path(&self) -> PathBuf {
        self.data_dir().join("agent.db")
    }

    /// 解析挂载路径（相对路径转为绝对路径）
    #[cfg(not(test))]
    pub fn resolve_mount_path(&self, host: &str) -> PathBuf {
        if let Some(rest) = host.strip_prefix("~/")
            && let Some(home) = dirs::home_dir()
        {
            return home.join(rest);
        }

        if host.starts_with("/") || host.starts_with("\\") {
            PathBuf::from(host)
        } else {
            std::env::current_dir()
                .expect("无法获取当前工作目录")
                .join(host)
        }
    }

    /// 测试时使用临时目录作为基准
    #[cfg(test)]
    pub fn resolve_mount_path(&self, host: &str) -> PathBuf {
        if let Some(rest) = host.strip_prefix("~/")
            && let Some(home) = dirs::home_dir()
        {
            return home.join(rest);
        }

        if host.starts_with("/") || host.starts_with("\\") {
            PathBuf::from(host)
        } else {
            let test_base = std::env::temp_dir().join("open-aaas-test").join("mounts");
            let _ = std::fs::create_dir_all(&test_base);
            test_base.join(host)
        }
    }

    /// 获取所有挂载的 docker -v 参数
    pub fn docker_mounts(&self) -> Vec<String> {
        self.paths
            .mounts
            .iter()
            .map(|m| {
                let host_abs = self.resolve_mount_path(&m.host);
                let readonly = if m.readonly { ":ro" } else { "" };
                format!("{}:{}{}", host_abs.display(), m.container, readonly)
            })
            .collect()
    }

    /// 确保所有挂载的宿主机目录存在
    pub async fn ensure_mount_dirs(&self) -> Result<(), ConfigError> {
        for mount in &self.paths.mounts {
            let host_abs = self.resolve_mount_path(&mount.host);
            if host_abs.exists() {
                continue;
            }

            if looks_like_file_mount(&host_abs, &mount.container) {
                if let Some(parent) = host_abs.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                return Err(ConfigError::Directory(format!(
                    "挂载文件不存在: {} -> {}",
                    host_abs.display(),
                    mount.container
                )));
            }

            tokio::fs::create_dir_all(&host_abs).await?;
        }
        Ok(())
    }
}

fn looks_like_file_mount(host: &std::path::Path, container: &str) -> bool {
    host.extension().is_some() || std::path::Path::new(container).extension().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// 创建临时目录用于测试
    fn temp_dir() -> TempDir {
        TempDir::new().expect("Failed to create temp dir")
    }

    /// RAII 模式保护当前目录
    struct CurrentDirGuard(std::path::PathBuf);

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.0);
        }
    }

    /// 创建临时配置目录并返回 (TempDir, 配置路径) 用于测试
    fn setup_test_config_dir() -> (TempDir, std::path::PathBuf) {
        let temp = temp_dir();
        let config_path = temp.path().join("config.toml");

        // 如果配置文件已存在，先删除它
        if config_path.exists() {
            std::fs::remove_file(&config_path).unwrap();
        }

        (temp, config_path)
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();

        // 验证 ServerConfig 默认值
        assert_eq!(config.server.base_url, "http://127.0.0.1:8080");
        assert_eq!(config.server.poll_interval_secs, 5);
        assert!(!config.server.use_system_proxy);

        // 验证 AgentConfig 默认值
        assert!(config.agent.service_id.is_none());
        assert!(config.agent.api_key.is_none());
        assert!(config.agent.name.is_none());

        // 验证 ExecutorConfig 默认值
        assert_eq!(config.executor.executor_type, ExecutorType::Standard);
        assert_eq!(config.executor.image, "open-aaas-executor:latest");
        assert_eq!(config.executor.capacity, 2);
        assert_eq!(config.executor.timeout_minutes, 0);
        assert!(config.executor.memory_limit.is_none());
        assert_eq!(config.executor.working_dir, "/workspace");
        assert!(config.executor.script_path.is_none());
        assert!(config.executor.custom_entrypoint.is_none());
        assert!(config.executor.custom_args.is_none());
        assert!(!config.executor.enable_host_access);

        // 验证 PathConfig 默认值
        assert_eq!(config.paths.data_dir, Some(PathBuf::from("./data")));
        assert!(config.paths.mounts.is_empty());
    }

    #[test]
    fn test_server_config_default() {
        let server = ServerConfig::default();
        assert_eq!(server.base_url, "http://127.0.0.1:8080");
        assert_eq!(server.poll_interval_secs, 5);
        assert!(!server.use_system_proxy);
    }

    #[test]
    fn test_executor_config_default() {
        let executor = ExecutorConfig::default();
        assert_eq!(executor.executor_type, ExecutorType::Standard);
        assert_eq!(executor.image, "open-aaas-executor:latest");
        assert_eq!(executor.capacity, 2);
        assert_eq!(executor.timeout_minutes, 0);
        assert!(executor.memory_limit.is_none());
        assert_eq!(executor.working_dir, "/workspace");
        assert!(executor.script_path.is_none());
        assert!(executor.custom_entrypoint.is_none());
        assert!(executor.custom_args.is_none());
        assert!(!executor.enable_host_access);
    }

    #[test]
    fn test_agent_config_default() {
        let agent = AgentConfig::default();
        assert!(agent.service_id.is_none());
        assert!(agent.api_key.is_none());
        assert!(agent.name.is_none());
        assert!(!agent.has_credentials());
    }

    #[test]
    fn test_agent_config_normalize_empty_credentials() {
        let mut agent = AgentConfig {
            service_id: Some("   ".to_string()),
            api_key: Some(String::new()),
            name: Some(" test-agent ".to_string()),
        };

        agent.normalize();

        assert!(agent.service_id.is_none());
        assert!(agent.api_key.is_none());
        assert_eq!(agent.name, Some(" test-agent ".to_string()));
        assert!(!agent.has_credentials());
    }

    #[test]
    fn test_path_config_default() {
        let paths = PathConfig::default();
        assert_eq!(paths.data_dir, Some(PathBuf::from("./data")));
        assert!(paths.mounts.is_empty());
    }

    #[test]
    fn test_toml_serialization() {
        let config = Config::default();
        let toml_str = config.to_runtime_toml();

        // 验证关键字段在序列化后的字符串中存在
        assert!(toml_str.contains("base_url"));
        assert!(toml_str.contains("http://127.0.0.1:8080"));
        assert!(toml_str.contains("poll_interval_secs"));
        assert!(toml_str.contains("image"));
        assert!(toml_str.contains("open-aaas-executor:latest"));
        assert!(toml_str.contains("capacity"));
        assert!(toml_str.contains("timeout_minutes"));
        assert!(toml_str.contains("# OpenAaaS Agent Core 配置文件"));
    }

    #[test]
    fn test_toml_deserialization() {
        let toml_str = r#"
[server]
base_url = "http://example.com:8080"
poll_interval_secs = 5
use_system_proxy = false

[agent]
service_id = "test-service"
api_key = "test-api-key"
name = "Test Agent"

[executor]
image = "custom-executor:v1"
capacity = 4
timeout_minutes = 60
memory_limit = "2g"

[paths]
data_dir = "/custom/data"

[[paths.mounts]]
host = "./data"
container = "/data"
readonly = true

[[paths.mounts]]
host = "/absolute/path"
container = "/mount"
readonly = false
"#;

        let config: Config = toml::from_str(toml_str).unwrap();

        // 验证服务器配置
        assert_eq!(config.server.base_url, "http://example.com:8080");
        assert_eq!(config.server.poll_interval_secs, 5);
        assert!(!config.server.use_system_proxy);

        // 验证 Agent 配置
        assert_eq!(config.agent.service_id, Some("test-service".to_string()));
        assert_eq!(config.agent.api_key, Some("test-api-key".to_string()));
        assert_eq!(config.agent.name, Some("Test Agent".to_string()));

        // 验证执行器配置
        assert_eq!(config.executor.image, "custom-executor:v1");
        assert_eq!(config.executor.capacity, 4);
        assert_eq!(config.executor.timeout_minutes, 60);
        assert_eq!(config.executor.memory_limit, Some("2g".to_string()));

        // 验证路径配置
        assert_eq!(config.paths.data_dir, Some(PathBuf::from("/custom/data")));
        assert_eq!(config.paths.mounts.len(), 2);
        assert_eq!(config.paths.mounts[0].host, "./data");
        assert_eq!(config.paths.mounts[0].container, "/data");
        assert!(config.paths.mounts[0].readonly);
        assert_eq!(config.paths.mounts[1].host, "/absolute/path");
        assert_eq!(config.paths.mounts[1].container, "/mount");
        assert!(!config.paths.mounts[1].readonly);
    }

    #[test]
    fn test_toml_deserialization_normalizes_blank_agent_fields() {
        let toml_str = r#"
[agent]
service_id = ""
api_key = "   "
name = "my-agent"
"#;

        let mut config: Config = toml::from_str(toml_str).unwrap();
        config.normalize();

        assert!(config.agent.service_id.is_none());
        assert!(config.agent.api_key.is_none());
        assert_eq!(config.agent.name, Some("my-agent".to_string()));
        assert!(!config.agent.has_credentials());
    }

    #[test]
    fn test_toml_roundtrip() {
        let original = Config {
            server: ServerConfig {
                base_url: "http://test.example.com".to_string(),
                poll_interval_secs: 15,
                use_system_proxy: false,
            },
            agent: AgentConfig {
                service_id: Some("roundtrip-test".to_string()),
                api_key: Some("secret-key".to_string()),
                name: Some("Roundtrip Test".to_string()),
            },
            executor: ExecutorConfig {
                executor_type: ExecutorType::Bash,
                image: "test-image:latest".to_string(),
                capacity: 8,
                timeout_minutes: 45,
                memory_limit: Some("4g".to_string()),
                working_dir: "/custom/workspace".to_string(),
                script_path: Some("/scripts/run.sh".to_string()),
                custom_entrypoint: None,
                custom_args: None,
                enable_host_access: false,
            },
            paths: PathConfig {
                data_dir: Some(PathBuf::from("/test/data")),
                mounts: vec![Mount {
                    host: "./test".to_string(),
                    container: "/test".to_string(),
                    readonly: true,
                }],
            },
        };

        let toml_str = original.to_runtime_toml();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(original.server.base_url, deserialized.server.base_url);
        assert_eq!(
            original.server.poll_interval_secs,
            deserialized.server.poll_interval_secs
        );
        assert_eq!(
            original.server.use_system_proxy,
            deserialized.server.use_system_proxy
        );
        assert_eq!(original.agent.service_id, deserialized.agent.service_id);
        assert_eq!(original.agent.api_key, deserialized.agent.api_key);
        assert_eq!(original.agent.name, deserialized.agent.name);
        assert_eq!(
            original.executor.executor_type,
            deserialized.executor.executor_type
        );
        assert_eq!(original.executor.image, deserialized.executor.image);
        assert_eq!(original.executor.capacity, deserialized.executor.capacity);
        assert_eq!(
            original.executor.timeout_minutes,
            deserialized.executor.timeout_minutes
        );
        assert_eq!(
            original.executor.memory_limit,
            deserialized.executor.memory_limit
        );
        assert_eq!(
            original.executor.working_dir,
            deserialized.executor.working_dir
        );
        assert_eq!(
            original.executor.script_path,
            deserialized.executor.script_path
        );
        assert_eq!(original.paths.data_dir, deserialized.paths.data_dir);
        assert_eq!(original.paths.mounts.len(), deserialized.paths.mounts.len());
        assert_eq!(
            original.paths.mounts[0].host,
            deserialized.paths.mounts[0].host
        );
        assert_eq!(
            original.paths.mounts[0].container,
            deserialized.paths.mounts[0].container
        );
        assert_eq!(
            original.paths.mounts[0].readonly,
            deserialized.paths.mounts[0].readonly
        );
    }

    #[test]
    fn test_data_dir_with_custom_path() {
        let config = Config {
            paths: PathConfig {
                data_dir: Some(PathBuf::from("/custom/data/dir")),
                mounts: vec![],
            },
            ..Config::default()
        };

        assert_eq!(config.data_dir(), PathBuf::from("/custom/data/dir"));
    }

    #[test]
    fn test_workspace_dir() {
        let config = Config {
            paths: PathConfig {
                data_dir: Some(PathBuf::from("/data")),
                mounts: vec![],
            },
            ..Config::default()
        };

        let workspace = config.workspace_dir("task-123");
        assert_eq!(workspace, PathBuf::from("/data/workspaces/task-123"));
    }

    #[test]
    fn test_database_path() {
        let config = Config {
            paths: PathConfig {
                data_dir: Some(PathBuf::from("/data")),
                mounts: vec![],
            },
            ..Config::default()
        };

        let db_path = config.database_path();
        assert_eq!(db_path, PathBuf::from("/data/agent.db"));
    }

    #[test]
    fn test_resolve_mount_path_absolute_unix() {
        let config = Config::default();
        let path = config.resolve_mount_path("/absolute/path/to/mount");
        assert_eq!(path, PathBuf::from("/absolute/path/to/mount"));
    }

    #[test]
    fn test_resolve_mount_path_absolute_windows() {
        let config = Config::default();
        let path = config.resolve_mount_path("\\absolute\\windows\\path");
        assert_eq!(path, PathBuf::from("\\absolute\\windows\\path"));
    }

    #[test]
    fn test_mount_default_readonly() {
        // 测试默认 readonly 值
        let mount: Mount =
            serde_json::from_str(r#"{"host":"./test","container":"/test"}"#).unwrap();
        assert!(!mount.readonly);
    }

    #[test]
    fn test_docker_mounts_generation() {
        let temp = TempDir::new().unwrap();
        let exe_dir = temp.path().join("exe_dir");
        std::fs::create_dir_all(&exe_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        let _guard = CurrentDirGuard(original_dir);

        std::env::set_current_dir(&exe_dir).unwrap();

        // 创建一个模拟的可执行文件环境
        let _mock_exe = exe_dir.join(if cfg!(windows) { "test.exe" } else { "test" });

        // 由于 resolve_mount_path 依赖 current_exe，我们在测试中创建一个模拟的配置
        let config = Config {
            paths: PathConfig {
                data_dir: None,
                mounts: vec![
                    Mount {
                        host: "./data".to_string(),
                        container: "/data".to_string(),
                        readonly: false,
                    },
                    Mount {
                        host: "/absolute/volume".to_string(),
                        container: "/volume".to_string(),
                        readonly: true,
                    },
                ],
            },
            ..Config::default()
        };

        let mounts = config.docker_mounts();

        // 验证有两个挂载
        assert_eq!(mounts.len(), 2);

        // 验证绝对路径挂载（带 readonly 标记）
        assert!(
            mounts
                .iter()
                .any(|m| m.contains("/absolute/volume:/volume:ro"))
        );

        // 验证相对路径被解析为绝对路径（由于 current_exe 在测试环境的不确定性，
        // 我们主要检查格式正确性）
        let relative_mount = mounts
            .iter()
            .find(|m| m.contains(":/data") && !m.contains(":ro"))
            .unwrap();
        assert!(relative_mount.contains(":/data"));
        assert!(!relative_mount.ends_with(":ro"));

        // 目录会在 _guard drop 时自动恢复
    }

    #[test]
    fn test_docker_mounts_readonly_flag() {
        let config = Config {
            paths: PathConfig {
                data_dir: None,
                mounts: vec![
                    Mount {
                        host: "/readonly/path".to_string(),
                        container: "/readonly".to_string(),
                        readonly: true,
                    },
                    Mount {
                        host: "/writable/path".to_string(),
                        container: "/writable".to_string(),
                        readonly: false,
                    },
                ],
            },
            ..Config::default()
        };

        let mounts = config.docker_mounts();

        // 验证 readonly 挂载带有 :ro 后缀
        let readonly_mount = mounts.iter().find(|m| m.contains("/readonly")).unwrap();
        assert!(readonly_mount.ends_with(":ro"));

        // 验证非 readonly 挂载没有 :ro 后缀
        let writable_mount = mounts.iter().find(|m| m.contains("/writable")).unwrap();
        assert!(!writable_mount.ends_with(":ro"));
    }

    #[test]
    fn test_default_mounts_count() {
        let config = Config::default();
        assert_eq!(config.paths.mounts.len(), 0);
    }

    #[test]
    fn test_config_error_display() {
        let io_error = ConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(io_error.to_string().contains("IO错误"));

        let parse_error = ConfigError::Parse("invalid toml".to_string());
        assert!(parse_error.to_string().contains("解析错误"));

        let dir_error = ConfigError::Directory("bad dir".to_string());
        assert!(dir_error.to_string().contains("目录错误"));
    }

    #[tokio::test]
    async fn test_save_and_load_config() {
        // 创建独立的临时目录
        let (_temp, config_path) = setup_test_config_dir();

        // 创建一个临时配置目录用于存储测试数据
        let data_temp = temp_dir();

        // 创建一个自定义配置，使用绝对路径作为 data_dir
        let config = Config {
            server: ServerConfig {
                base_url: "http://test-server:9090".to_string(),
                poll_interval_secs: 20,
                use_system_proxy: false,
            },
            agent: AgentConfig {
                service_id: Some("test-service-id".to_string()),
                api_key: Some("test-api-key-123".to_string()),
                name: Some("Test Agent Name".to_string()),
            },
            executor: ExecutorConfig {
                executor_type: ExecutorType::Standard,
                image: "test-executor:v2".to_string(),
                capacity: 5,
                timeout_minutes: 120,
                memory_limit: Some("8g".to_string()),
                working_dir: "/workspace".to_string(),
                script_path: None,
                custom_entrypoint: None,
                custom_args: None,
                enable_host_access: false,
            },
            paths: PathConfig {
                data_dir: Some(data_temp.path().join("custom_data")),
                mounts: vec![Mount {
                    host: "./test_share".to_string(),
                    container: "/test_share".to_string(),
                    readonly: false,
                }],
            },
        };

        // 保存配置到临时目录
        config.save_to_path(&config_path).await.unwrap();

        // 验证配置文件存在
        assert!(config_path.exists(), "配置文件应该存在于 {:?}", config_path);

        // 从临时目录加载配置
        let loaded_config = Config::load_from_path(&config_path).await.unwrap();

        // 验证加载的配置与原始配置一致
        assert_eq!(loaded_config.server.base_url, config.server.base_url);
        assert_eq!(
            loaded_config.server.poll_interval_secs,
            config.server.poll_interval_secs
        );
        assert_eq!(loaded_config.agent.service_id, config.agent.service_id);
        assert_eq!(loaded_config.agent.api_key, config.agent.api_key);
        assert_eq!(loaded_config.executor.image, config.executor.image);
        assert_eq!(loaded_config.executor.capacity, config.executor.capacity);
    }

    #[tokio::test]
    async fn test_load_creates_default_config_when_missing() {
        // 创建独立的临时目录
        let (_temp, config_path) = setup_test_config_dir();

        // 确保配置文件不存在
        assert!(!config_path.exists(), "测试前配置文件不应存在");

        // 手动创建默认配置并保存
        let config = Config::default();
        config.save_to_path(&config_path).await.unwrap();

        // 加载配置
        let loaded_config = Config::load_from_path(&config_path).await.unwrap();

        // 验证是默认配置
        assert_eq!(loaded_config.server.base_url, "http://127.0.0.1:8080");
        assert_eq!(loaded_config.executor.image, "open-aaas-executor:latest");

        // 验证配置文件已创建
        assert!(config_path.exists(), "默认配置文件应该被创建");
    }

    #[tokio::test]
    async fn test_ensure_mount_dirs() {
        let temp = temp_dir();
        let mount_dir1 = temp.path().join("mount1");
        let mount_dir2 = temp.path().join("mount2");

        let config = Config {
            paths: PathConfig {
                data_dir: None,
                mounts: vec![
                    Mount {
                        host: mount_dir1.to_string_lossy().to_string(),
                        container: "/mount1".to_string(),
                        readonly: false,
                    },
                    Mount {
                        host: mount_dir2.to_string_lossy().to_string(),
                        container: "/mount2".to_string(),
                        readonly: true,
                    },
                ],
            },
            ..Config::default()
        };

        // 确保目录不存在
        let _ = std::fs::remove_dir(&mount_dir1);
        let _ = std::fs::remove_dir(&mount_dir2);
        assert!(!mount_dir1.exists());
        assert!(!mount_dir2.exists());

        // 调用 ensure_mount_dirs
        config.ensure_mount_dirs().await.unwrap();

        // 验证目录已创建
        assert!(mount_dir1.exists());
        assert!(mount_dir2.exists());
    }

    #[test]
    fn test_serde_skip_defaults() {
        // 测试反序列化时缺失的字段会使用默认值
        let toml_str = r#"
[server]
base_url = "http://custom.com"

[executor]
capacity = 10
"#;

        let config: Config = toml::from_str(toml_str).unwrap();

        // 自定义值
        assert_eq!(config.server.base_url, "http://custom.com");
        assert_eq!(config.executor.capacity, 10);

        // 默认值
        assert_eq!(config.server.poll_interval_secs, 5);
        assert_eq!(config.executor.image, "open-aaas-executor:latest");
        assert_eq!(config.executor.timeout_minutes, 0);
    }

    #[test]
    fn test_empty_mounts_list() {
        let toml_str = r#"
[paths]
mounts = []
"#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.paths.mounts.is_empty());
    }

    #[test]
    fn test_multiple_mounts() {
        let config = Config {
            paths: PathConfig {
                data_dir: None,
                mounts: vec![
                    Mount {
                        host: "./share".to_string(),
                        container: "/share".to_string(),
                        readonly: false,
                    },
                    Mount {
                        host: "./config".to_string(),
                        container: "/config".to_string(),
                        readonly: true,
                    },
                    Mount {
                        host: "/var/log".to_string(),
                        container: "/logs".to_string(),
                        readonly: true,
                    },
                ],
            },
            ..Config::default()
        };

        assert_eq!(config.paths.mounts.len(), 3);

        let mounts = config.docker_mounts();
        assert_eq!(mounts.len(), 3);
    }

    #[test]
    fn test_partial_executor_config() {
        let toml_str = r#"
[executor]
image = "custom:latest"
memory_limit = "1g"
"#;

        let config: Config = toml::from_str(toml_str).unwrap();

        // 自定义值
        assert_eq!(config.executor.image, "custom:latest");
        assert_eq!(config.executor.memory_limit, Some("1g".to_string()));

        // 其他字段使用默认值
        assert_eq!(config.executor.executor_type, ExecutorType::Standard);
        assert_eq!(config.executor.capacity, 2);
        assert_eq!(config.executor.timeout_minutes, 0);
        assert_eq!(config.executor.working_dir, "/workspace");
    }

    // ========== ExecutorType 测试 ==========

    #[test]
    fn test_executor_type_default() {
        let executor_type: ExecutorType = Default::default();
        assert_eq!(executor_type, ExecutorType::Standard);
    }

    #[test]
    fn test_executor_type_get_entrypoint_standard() {
        let executor_type = ExecutorType::Standard;
        assert_eq!(executor_type.get_entrypoint(None), None);
        assert_eq!(
            executor_type.get_entrypoint(Some(&vec!["custom".to_string()])),
            None
        );
    }

    #[test]
    fn test_executor_type_get_entrypoint_bash() {
        let executor_type = ExecutorType::Bash;
        assert_eq!(
            executor_type.get_entrypoint(None),
            Some(vec!["bash".to_string()])
        );
        assert_eq!(
            executor_type.get_entrypoint(Some(&vec!["custom".to_string()])),
            Some(vec!["bash".to_string()])
        );
    }

    #[test]
    fn test_executor_type_get_entrypoint_python() {
        let executor_type = ExecutorType::Python;
        assert_eq!(
            executor_type.get_entrypoint(None),
            Some(vec!["python".to_string()])
        );
        assert_eq!(
            executor_type.get_entrypoint(Some(&vec!["custom".to_string()])),
            Some(vec!["python".to_string()])
        );
    }

    #[test]
    fn test_executor_type_get_entrypoint_custom() {
        let executor_type = ExecutorType::Custom;
        assert_eq!(executor_type.get_entrypoint(None), None);
        assert_eq!(
            executor_type.get_entrypoint(Some(&vec!["custom".to_string()])),
            Some(vec!["custom".to_string()])
        );
        assert_eq!(
            executor_type.get_entrypoint(Some(&vec!["sh".to_string(), "-c".to_string()])),
            Some(vec!["sh".to_string(), "-c".to_string()])
        );
    }

    #[test]
    fn test_executor_type_get_command_args_standard() {
        let executor_type = ExecutorType::Standard;
        let args = executor_type.get_command_args("task-001", "/workspace", None, None);
        assert!(args.is_empty());
    }

    #[test]
    fn test_executor_type_get_command_args_bash_default() {
        let executor_type = ExecutorType::Bash;
        let args = executor_type.get_command_args("task-001", "/workspace", None, None);
        assert_eq!(args, vec!["/workspace/task-001/run.sh"]);
    }

    #[test]
    fn test_executor_type_get_command_args_bash_custom() {
        let executor_type = ExecutorType::Bash;
        let args = executor_type.get_command_args(
            "task-001",
            "/workspace",
            Some(&"/scripts/run.sh".to_string()),
            None,
        );
        assert_eq!(args, vec!["/scripts/run.sh"]);
    }

    #[test]
    fn test_executor_type_get_command_args_python_default() {
        let executor_type = ExecutorType::Python;
        let args = executor_type.get_command_args("task-002", "/workspace", None, None);
        assert_eq!(args, vec!["/workspace/task-002/run.py"]);
    }

    #[test]
    fn test_executor_type_get_command_args_python_custom() {
        let executor_type = ExecutorType::Python;
        let args = executor_type.get_command_args(
            "task-002",
            "/workspace",
            Some(&"/scripts/main.py".to_string()),
            None,
        );
        assert_eq!(args, vec!["/scripts/main.py"]);
    }

    #[test]
    fn test_executor_type_get_command_args_custom() {
        let executor_type = ExecutorType::Custom;
        let args = executor_type.get_command_args("task-004", "/workspace", None, None);
        assert!(args.is_empty());

        let args = executor_type.get_command_args(
            "task-004",
            "/workspace",
            None,
            Some(&vec!["--flag".to_string(), "value".to_string()]),
        );
        assert_eq!(args, vec!["--flag", "value"]);
    }

    // ========== ExecutorConfig 方法测试 ==========

    #[test]
    fn test_executor_config_get_entrypoint() {
        let config = ExecutorConfig {
            executor_type: ExecutorType::Bash,
            image: "test:latest".to_string(),
            capacity: 2,
            timeout_minutes: 30,
            memory_limit: None,
            working_dir: "/workspace".to_string(),
            script_path: None,
            custom_entrypoint: None,
            custom_args: None,
            enable_host_access: false,
        };
        assert_eq!(config.get_entrypoint(), Some(vec!["bash".to_string()]));
    }

    #[test]
    fn test_executor_config_get_command_args() {
        let config = ExecutorConfig {
            executor_type: ExecutorType::Bash,
            image: "test:latest".to_string(),
            capacity: 2,
            timeout_minutes: 30,
            memory_limit: None,
            working_dir: "/workspace".to_string(),
            script_path: Some("/scripts/task.sh".to_string()),
            custom_entrypoint: None,
            custom_args: None,
            enable_host_access: false,
        };
        assert_eq!(
            config.get_command_args("task-123"),
            vec!["/scripts/task.sh"]
        );
    }

    #[test]
    fn test_executor_type_deserialization() {
        // 测试各种 executor_type 的字符串反序列化
        let toml_standard = r#"executor_type = "standard""#;
        let toml_bash = r#"executor_type = "bash""#;
        let toml_python = r#"executor_type = "python""#;
        let toml_custom = r#"executor_type = "custom""#;

        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct TestConfig {
            pub executor_type: ExecutorType,
        }

        let config_standard: TestConfig = toml::from_str(toml_standard).unwrap();
        assert_eq!(config_standard.executor_type, ExecutorType::Standard);

        let config_bash: TestConfig = toml::from_str(toml_bash).unwrap();
        assert_eq!(config_bash.executor_type, ExecutorType::Bash);

        let config_python: TestConfig = toml::from_str(toml_python).unwrap();
        assert_eq!(config_python.executor_type, ExecutorType::Python);

        let config_custom: TestConfig = toml::from_str(toml_custom).unwrap();
        assert_eq!(config_custom.executor_type, ExecutorType::Custom);
    }

    #[test]
    fn test_executor_full_config_with_executor_type() {
        let toml_str = r#"
[executor]
executor_type = "python"
image = "python-executor:3.11"
capacity = 4
timeout_minutes = 60
memory_limit = "2g"
working_dir = "/app"
script_path = "/app/main.py"
"#;

        let config: Config = toml::from_str(toml_str).unwrap();

        assert_eq!(config.executor.executor_type, ExecutorType::Python);
        assert_eq!(config.executor.image, "python-executor:3.11");
        assert_eq!(config.executor.capacity, 4);
        assert_eq!(config.executor.timeout_minutes, 60);
        assert_eq!(config.executor.memory_limit, Some("2g".to_string()));
        assert_eq!(config.executor.working_dir, "/app");
        assert_eq!(
            config.executor.script_path,
            Some("/app/main.py".to_string())
        );
    }

    #[test]
    fn test_executor_custom_config() {
        let toml_str = r#"
[executor]
executor_type = "custom"
image = "custom-executor:latest"
custom_entrypoint = ["/bin/sh", "-c"]
custom_args = ["echo", "hello", "world"]
"#;

        let config: Config = toml::from_str(toml_str).unwrap();

        assert_eq!(config.executor.executor_type, ExecutorType::Custom);
        assert_eq!(
            config.executor.custom_entrypoint,
            Some(vec!["/bin/sh".to_string(), "-c".to_string()])
        );
        assert_eq!(
            config.executor.custom_args,
            Some(vec![
                "echo".to_string(),
                "hello".to_string(),
                "world".to_string()
            ])
        );

        // 验证 get_entrypoint 和 get_command_args
        assert_eq!(
            config.executor.get_entrypoint(),
            Some(vec!["/bin/sh".to_string(), "-c".to_string()])
        );
        assert_eq!(
            config.executor.get_command_args("task-001"),
            vec!["echo", "hello", "world"]
        );
    }

    #[test]
    fn test_runtime_toml_escapes_special_chars() {
        let config = Config {
            server: ServerConfig {
                base_url: "http://example.com/test\\path\"quoted".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        let toml_str = config.to_runtime_toml();
        // 确保能正确反序列化回来
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.server.base_url, config.server.base_url);
    }

    #[test]
    fn test_runtime_toml_includes_enable_host_access_comment() {
        // Arrange
        let config = Config::default();

        // Act
        let toml_str = config.to_runtime_toml();

        // Assert
        assert!(
            toml_str.contains("# enable_host_access"),
            "运行时 TOML 应在 [executor] 段以注释形式包含 enable_host_access 字段说明，当前输出:\n{}",
            toml_str
        );
    }

    #[test]
    fn test_executor_config_default_enable_host_access_is_false() {
        // Arrange
        let executor = ExecutorConfig::default();

        // Assert
        assert!(!executor.enable_host_access);
    }

    #[test]
    fn test_executor_config_deserialize_without_enable_host_access_defaults_to_false() {
        // Arrange: 模拟旧配置文件，executor 节不含 enable_host_access
        let toml_str = r#"
[executor]
image = "custom-executor:v1"
capacity = 4
"#;

        // Act
        let config: Config = toml::from_str(toml_str).unwrap();

        // Assert
        assert!(!config.executor.enable_host_access);
    }

    #[test]
    fn test_executor_config_deserialize_with_enable_host_access_true() {
        // Arrange
        let toml_str = r#"
[executor]
enable_host_access = true
"#;

        // Act
        let config: Config = toml::from_str(toml_str).unwrap();

        // Assert
        assert!(config.executor.enable_host_access);
    }
}
