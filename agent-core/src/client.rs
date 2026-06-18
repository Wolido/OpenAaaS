//! OpenAaaS Server HTTP Client

use crate::config::ServerConfig;
use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// 对注册 token 进行脱敏，仅保留前 4 个 Unicode 字符，其余用 `***` 替代。
fn mask_token(token: &str) -> String {
    if token.chars().count() > 4 {
        format!("{}***", token.chars().take(4).collect::<String>())
    } else {
        "***".to_string()
    }
}

/// API 客户端
pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    service_id: Option<String>,
}

/// 注册请求
#[derive(Debug, Serialize)]
pub struct RegisterRequest {
    pub registration_token: String,
    pub capacity: usize,
}

/// 注册响应
#[derive(Debug, Deserialize)]
pub struct RegisterResponse {
    pub api_key: String,
    pub service_id: String,
}

/// 轮询请求
#[derive(Debug, Serialize)]
pub struct PollRequest {
    pub current_load: i64,
    pub available_capacity: i64,
}

/// 心跳请求
#[derive(Debug, Serialize)]
pub struct HeartbeatRequest {
    pub status: String,          // "online" | "busy" | "offline"
    pub current_load: i64,       // 当前负载
    pub capacity: i64,           // 总容量
    pub available_capacity: i64, // 可用容量
}

/// 轮询响应
#[derive(Debug, Deserialize)]
pub struct PollResponse {
    pub has_task: bool,
    pub task: Option<ServerTask>,
    pub should_cancel: bool,
    pub cancel_task_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PollOutcome {
    pub task: Option<ServerTask>,
    pub cancel_task_id: Option<String>,
}

/// Server 任务
#[derive(Debug, Clone, Deserialize)]
pub struct ServerInputFile {
    pub id: String,
    pub filename: String,
    pub mime_type: Option<String>,
    pub size_bytes: i64,
}

/// Server 任务
#[derive(Debug, Clone, Deserialize)]
pub struct ServerTask {
    pub id: String,
    pub task_prompt: String,
    pub output_prompt: Option<String>,
    pub session_id: String,
    #[serde(default)]
    pub input_files: Vec<ServerInputFile>,
}

/// 任务完成状态
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskCompleteStatus {
    Completed,
    Failed,
    Cancelled,
}

impl fmt::Display for TaskCompleteStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskCompleteStatus::Completed => write!(f, "completed"),
            TaskCompleteStatus::Failed => write!(f, "failed"),
            TaskCompleteStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// 完成任务请求
#[derive(Debug, Serialize)]
pub struct CompleteTaskRequest {
    pub task_id: String,
    pub status: TaskCompleteStatus,
    pub output: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub file_ids: Vec<String>,
}

/// 上传文件响应
#[derive(Debug, Deserialize)]
pub struct UploadFileResponse {
    pub file_id: String,
}

impl ApiClient {
    /// 创建新的 API 客户端
    pub fn new(server_config: &ServerConfig) -> Self {
        let client = if server_config.use_system_proxy {
            // 使用系统代理（适合连接远程 Server）
            Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("创建 HTTP 客户端失败")
        } else {
            // 禁用代理（适合本地开发，避免 Shadowrocket 等干扰）
            Client::builder()
                .timeout(Duration::from_secs(30))
                .no_proxy()
                .build()
                .expect("创建 HTTP 客户端失败")
        };

        Self {
            client,
            base_url: server_config.base_url.trim_end_matches('/').to_string(),
            api_key: None,
            service_id: None,
        }
    }

    /// 设置认证信息
    pub fn set_auth(&mut self, api_key: String, service_id: String) {
        self.api_key = Some(api_key);
        self.service_id = Some(service_id);
    }

    /// 获取请求头
    fn headers(&self) -> anyhow::Result<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();

        if let Some(ref api_key) = self.api_key {
            let auth_value = format!("Bearer {}", api_key)
                .parse::<reqwest::header::HeaderValue>()
                .context("API Key 格式无效，包含非法字符")?;
            headers.insert("Authorization", auth_value);

            let api_key_value = api_key
                .parse::<reqwest::header::HeaderValue>()
                .context("X-API-Key 格式无效，包含非法字符")?;
            headers.insert("X-API-Key", api_key_value);
        }

        if let Some(ref service_id) = self.service_id {
            let service_value = service_id
                .parse::<reqwest::header::HeaderValue>()
                .context("Service ID 格式无效")?;
            headers.insert("X-Service-ID", service_value);
        }

        // 常量的 parse 不会失败，但为一致性也处理错误
        let content_type = "application/json"
            .parse::<reqwest::header::HeaderValue>()
            .context("Content-Type 格式错误")?;
        headers.insert("Content-Type", content_type);

        Ok(headers)
    }

    /// 注册服务
    pub async fn register(&mut self, token: &str, capacity: usize) -> Result<RegisterResponse> {
        let url = format!("{}/api/v1/agent/register", self.base_url);

        let request = RegisterRequest {
            registration_token: token.to_string(),
            capacity,
        };

        // 序列化为 JSON 字符串，用于调试
        let request_json =
            serde_json::to_string(&request).unwrap_or_else(|_| "序列化失败".to_string());

        // 在 info 日志中隐藏完整 token，仅显示前 4 个 Unicode 字符前缀
        let masked_token = mask_token(token);
        let display_request = RegisterRequest {
            registration_token: masked_token,
            capacity,
        };
        let display_json =
            serde_json::to_string(&display_request).unwrap_or_else(|_| "序列化失败".to_string());

        info!("========================================");
        info!("注册请求详情:");
        info!("  URL: {}", url);
        info!("  Method: POST");
        info!("  Headers: Content-Type: application/json");
        info!("  Body: {}", display_json);
        info!("========================================");

        debug!("完整注册请求体（含真实 token）: {}", request_json);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(request_json) // 使用序列化后的字符串
            .send()
            .await
            .context("注册请求发送失败")?;

        let status = response.status();
        info!("收到响应: HTTP {}", status);

        match response.status() {
            StatusCode::OK | StatusCode::CREATED => {
                let result: RegisterResponse = response.json().await.context("解析注册响应失败")?;
                info!("注册成功: service_id={}", result.service_id);

                // 保存认证信息
                self.set_auth(result.api_key.clone(), result.service_id.clone());

                Ok(result)
            }
            StatusCode::CONFLICT => {
                anyhow::bail!("服务已注册，无法重复注册")
            }
            StatusCode::UNAUTHORIZED => {
                anyhow::bail!("注册令牌无效或已过期")
            }
            status => {
                let text = response.text().await.unwrap_or_default();
                let error_msg = match status {
                    StatusCode::NOT_FOUND => {
                        "注册令牌无效或已过期。请检查:\n\
                         1. 令牌是否正确复制\n\
                         2. 该服务是否已被其他 Agent 注册\n\
                         3. 是否需要管理员创建新的服务"
                    }
                    StatusCode::SERVICE_UNAVAILABLE => {
                        "Server 服务暂时不可用。请检查:\n\
                         1. Server 是否正常运行\n\
                         2. 网络连接是否正常\n\
                         3. Server 地址配置是否正确"
                    }
                    StatusCode::INTERNAL_SERVER_ERROR => {
                        "Server 内部错误。请联系管理员查看 Server 日志"
                    }
                    _ => &format!("HTTP {} - {}", status, text),
                };
                anyhow::bail!("{}", error_msg)
            }
        }
    }

    /// 轮询任务
    pub async fn poll(&self, current_load: usize, capacity: usize) -> Result<PollOutcome> {
        let service_id = self.service_id.as_ref().context("未设置 service_id")?;

        let url = format!("{}/api/v1/agent/{}/poll", self.base_url, service_id);

        let request = PollRequest {
            current_load: current_load as i64,
            available_capacity: capacity.saturating_sub(current_load) as i64,
        };

        debug!(
            "轮询任务: current_load={}, capacity={}",
            current_load, capacity
        );

        let response = self
            .client
            .post(&url)
            .headers(self.headers().context("构建请求头失败")?)
            .json(&request)
            .send()
            .await
            .context("轮询请求失败")?;

        match response.status() {
            StatusCode::OK => {
                let result: PollResponse = response.json().await.context("解析轮询响应失败")?;

                let cancel_task_id = if result.should_cancel {
                    if let Some(task_id) = result.cancel_task_id.clone() {
                        warn!("Server 请求取消任务: {}", task_id);
                    }
                    result.cancel_task_id.clone()
                } else {
                    None
                };

                let task = if result.has_task { result.task } else { None };

                Ok(PollOutcome {
                    task,
                    cancel_task_id,
                })
            }
            StatusCode::NO_CONTENT => {
                // 没有任务
                Ok(PollOutcome {
                    task: None,
                    cancel_task_id: None,
                })
            }
            StatusCode::UNAUTHORIZED => {
                error!("认证失败，请检查 API Key");
                anyhow::bail!("认证失败")
            }
            status => {
                let text = response.text().await.unwrap_or_default();
                anyhow::bail!("轮询失败: HTTP {} - {}", status, text)
            }
        }
    }

    /// 接受任务
    pub async fn accept_task(&self, task_id: &str) -> Result<bool> {
        let service_id = self.service_id.as_ref().context("未设置 service_id")?;

        let url = format!("{}/api/v1/agent/{}/accept", self.base_url, service_id);

        let response = self
            .client
            .post(&url)
            .headers(self.headers().context("构建请求头失败")?)
            .json(&serde_json::json!({ "task_id": task_id }))
            .send()
            .await
            .context("接受任务请求失败")?;

        match response.status() {
            StatusCode::OK => {
                info!("成功接受任务: {}", task_id);
                Ok(true)
            }
            StatusCode::CONFLICT => {
                warn!("任务 {} 已被其他 Agent 接受", task_id);
                Ok(false)
            }
            status => {
                let text = response.text().await.unwrap_or_default();
                anyhow::bail!("接受任务失败: HTTP {} - {}", status, text)
            }
        }
    }

    /// 下载任务输入文件
    pub async fn download_input_file(&self, file_id: &str) -> Result<Vec<u8>> {
        let service_id = self.service_id.as_ref().context("未设置 service_id")?;

        let url = format!(
            "{}/api/v1/files/agent/{}/files/{}/download",
            self.base_url, service_id, file_id
        );

        let response = self
            .client
            .get(&url)
            .headers(self.headers().context("构建请求头失败")?)
            .send()
            .await
            .context("下载输入文件请求失败")?;

        match response.status() {
            StatusCode::OK => {
                let bytes = response.bytes().await.context("读取输入文件响应失败")?;
                Ok(bytes.to_vec())
            }
            status => {
                let text = response.text().await.unwrap_or_default();
                anyhow::bail!("下载输入文件失败: HTTP {} - {}", status, text)
            }
        }
    }

    /// 完成任务
    pub async fn complete_task(
        &self,
        task_id: &str,
        status: TaskCompleteStatus,
        output: Option<serde_json::Value>,
        error_message: Option<String>,
        file_ids: Vec<String>,
    ) -> Result<()> {
        let service_id = self.service_id.as_ref().context("未设置 service_id")?;

        let url = format!("{}/api/v1/agent/{}/complete", self.base_url, service_id);

        let request = CompleteTaskRequest {
            task_id: task_id.to_string(),
            status,
            output,
            error_message,
            file_ids,
        };

        info!("完成任务上报: {} -> {}", task_id, status);

        let response = self
            .client
            .post(&url)
            .headers(self.headers().context("构建请求头失败")?)
            .json(&request)
            .send()
            .await
            .context("完成任务请求失败")?;

        match response.status() {
            StatusCode::OK => {
                debug!("任务 {} 完成上报成功", task_id);
                Ok(())
            }
            status => {
                let text = response.text().await.unwrap_or_default();
                anyhow::bail!("完成任务失败: HTTP {} - {}", status, text)
            }
        }
    }

    /// 发送心跳
    pub async fn heartbeat(&self, current_load: usize, capacity: usize) -> Result<()> {
        let service_id = self.service_id.as_ref().context("未设置 service_id")?;

        let url = format!("{}/api/v1/agent/{}/heartbeat", self.base_url, service_id);

        let status = if current_load >= capacity {
            "busy"
        } else {
            "online"
        };
        let request = HeartbeatRequest {
            status: status.to_string(),
            current_load: current_load as i64,
            capacity: capacity as i64,
            available_capacity: (capacity.saturating_sub(current_load)) as i64,
        };

        let response = self
            .client
            .post(&url)
            .headers(self.headers().context("构建请求头失败")?)
            .json(&request)
            .send()
            .await
            .context("心跳请求失败")?;

        match response.status() {
            StatusCode::OK => {
                debug!("心跳发送成功");
                Ok(())
            }
            StatusCode::UNAUTHORIZED => {
                error!("心跳认证失败");
                anyhow::bail!("认证失败")
            }
            status => {
                let text = response.text().await.unwrap_or_default();
                anyhow::bail!("心跳失败: HTTP {} - {}", status, text)
            }
        }
    }

    /// 上传文件
    ///
    /// Args:
    ///   task_id: 任务 ID
    ///   file_path: 本地文件路径
    ///
    /// Returns:
    ///   成功返回 file_id，失败返回错误
    pub async fn upload_file(&self, task_id: &str, file_path: &std::path::Path) -> Result<String> {
        self.upload_file_as(task_id, file_path, None).await
    }

    /// 使用指定展示文件名上传文件。
    ///
    /// Agent workspace 可能包含 output/response.md 这类相对路径；保留相对路径
    /// 可以避免 Dashboard 上多个同名文件混在一起。
    pub async fn upload_file_as(
        &self,
        task_id: &str,
        file_path: &std::path::Path,
        upload_name: Option<&str>,
    ) -> Result<String> {
        let service_id = self.service_id.as_ref().context("未设置 service_id")?;

        let url = format!(
            "{}/api/v1/files/agent/{}/files/upload",
            self.base_url, service_id
        );

        // 读取文件内容
        let file_content = tokio::fs::read(file_path)
            .await
            .context(format!("读取文件失败: {}", file_path.display()))?;

        let file_name = upload_name.unwrap_or_else(|| {
            file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unnamed")
        });

        // 构建 multipart 表单
        let form = reqwest::multipart::Form::new()
            .text("task_id", task_id.to_string())
            .part(
                "file",
                reqwest::multipart::Part::bytes(file_content)
                    .file_name(file_name.to_string())
                    .mime_str("application/octet-stream")
                    .context("设置 MIME 类型失败")?,
            );

        let mut headers = self.headers().context("构建请求头失败")?;
        headers.remove(reqwest::header::CONTENT_TYPE);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .multipart(form)
            .send()
            .await
            .context("上传文件请求失败")?;

        match response.status() {
            StatusCode::OK | StatusCode::CREATED => {
                let result: UploadFileResponse =
                    response.json().await.context("解析上传响应失败")?;
                info!("文件上传成功: file_id={}", result.file_id);
                Ok(result.file_id)
            }
            status => {
                let text = response.text().await.unwrap_or_default();
                anyhow::bail!("上传文件失败: HTTP {} - {}", status, text)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde_json::json;

    fn create_test_config(use_system_proxy: bool) -> ServerConfig {
        ServerConfig {
            base_url: "http://127.0.0.1:8080".to_string(),
            poll_interval_secs: 10,
            use_system_proxy,
        }
    }

    // ======== ApiClient::new() 测试 ========

    #[test]
    fn test_api_client_new_with_system_proxy() {
        let config = create_test_config(true);
        let client = ApiClient::new(&config);

        assert_eq!(client.base_url, "http://127.0.0.1:8080");
        assert!(client.api_key.is_none());
        assert!(client.service_id.is_none());
    }

    #[test]
    fn test_api_client_new_without_system_proxy() {
        let config = create_test_config(false);
        let client = ApiClient::new(&config);

        assert_eq!(client.base_url, "http://127.0.0.1:8080");
        assert!(client.api_key.is_none());
        assert!(client.service_id.is_none());
    }

    // ======== ApiClient::set_auth() 测试 ========

    #[test]
    fn test_api_client_set_auth() {
        let config = create_test_config(false);
        let mut client = ApiClient::new(&config);

        client.set_auth("test-api-key".to_string(), "test-service-id".to_string());

        assert_eq!(client.api_key, Some("test-api-key".to_string()));
        assert_eq!(client.service_id, Some("test-service-id".to_string()));
    }

    // ======== 请求/响应结构序列化测试 ========

    #[test]
    fn test_register_request_serialization() {
        let request = RegisterRequest {
            registration_token: "test-token-123".to_string(),
            capacity: 5,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test-token-123"));
        assert!(json.contains("5"));

        // 验证 JSON 格式
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["registration_token"], "test-token-123");
        assert_eq!(value["capacity"], 5);
    }

    #[test]
    fn test_register_response_deserialization() {
        let json = r#"{
            "api_key": "api-key-abc",
            "service_id": "service-xyz",
            "name": "Test Agent"
        }"#;

        let response: RegisterResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.api_key, "api-key-abc");
        assert_eq!(response.service_id, "service-xyz");
    }

    #[test]
    fn test_poll_request_serialization() {
        let request = PollRequest {
            current_load: 2,
            available_capacity: 3,
        };

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["current_load"], 2);
        assert_eq!(value["available_capacity"], 3);
    }

    #[test]
    fn test_poll_response_deserialization_has_task_true() {
        let json = r#"{
            "has_task": true,
            "task": {
                "id": "task-123",
                "task_prompt": "Do something",
                "output_prompt": null,
                "session_id": "session-456"
            },
            "should_cancel": false,
            "cancel_task_id": null
        }"#;

        let response: PollResponse = serde_json::from_str(json).unwrap();
        assert!(response.has_task);
        assert!(response.task.is_some());
        let task = response.task.unwrap();
        assert_eq!(task.id, "task-123");
        assert_eq!(task.task_prompt, "Do something");
        assert_eq!(task.session_id, "session-456".to_string());
    }

    #[test]
    fn test_poll_response_deserialization_has_task_false() {
        let json = r#"{
            "has_task": false,
            "task": null,
            "should_cancel": false,
            "cancel_task_id": null
        }"#;

        let response: PollResponse = serde_json::from_str(json).unwrap();
        assert!(!response.has_task);
        assert!(response.task.is_none());
    }

    #[test]
    fn test_server_task_deserialization() {
        let json = r#"{
            "id": "task-789",
            "task_prompt": "Process data",
            "output_prompt": "Output format: JSON",
            "session_id": "session-999"
        }"#;

        let task: ServerTask = serde_json::from_str(json).unwrap();
        assert_eq!(task.id, "task-789");
        assert_eq!(task.task_prompt, "Process data");
        assert_eq!(task.output_prompt, Some("Output format: JSON".to_string()));
        assert_eq!(task.session_id, "session-999".to_string());
    }

    #[test]
    fn test_complete_task_request_serialization() {
        let request = CompleteTaskRequest {
            task_id: "task-123".to_string(),
            status: TaskCompleteStatus::Completed,
            output: Some(serde_json::json!({"result": "success"})),
            error_message: None,
            file_ids: vec!["file-1".to_string(), "file-2".to_string()],
        };

        let json = serde_json::to_string(&request).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["task_id"], "task-123");
        assert_eq!(value["status"], "completed");
        assert_eq!(value["output"]["result"], "success");
        assert!(value["error_message"].is_null());
        assert_eq!(value["file_ids"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_task_complete_status_serialization() {
        // 测试 Completed 序列化
        let status = TaskCompleteStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"completed\"");

        // 测试 Failed 序列化
        let status = TaskCompleteStatus::Failed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"failed\"");

        // 测试 Cancelled 序列化
        let status = TaskCompleteStatus::Cancelled;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"cancelled\"");
    }

    #[test]
    fn test_task_complete_status_display() {
        assert_eq!(format!("{}", TaskCompleteStatus::Completed), "completed");
        assert_eq!(format!("{}", TaskCompleteStatus::Failed), "failed");
        assert_eq!(format!("{}", TaskCompleteStatus::Cancelled), "cancelled");
    }

    #[test]
    fn test_upload_file_response_deserialization() {
        let json = r#"{
            "file_id": "file-abc-123"
        }"#;

        let response: UploadFileResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.file_id, "file-abc-123");
    }

    // ======== headers() 方法测试 ========

    #[test]
    fn test_headers_without_auth() {
        let config = create_test_config(false);
        let client = ApiClient::new(&config);

        let headers = client.headers().unwrap();

        // 未设置 auth 时，不应包含 Authorization / X-API-Key / X-Service-ID
        assert!(!headers.contains_key("Authorization"));
        assert!(!headers.contains_key("X-API-Key"));
        assert!(!headers.contains_key("X-Service-ID"));

        // 应包含 Content-Type
        assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
    }

    #[test]
    fn test_headers_with_auth() {
        let config = create_test_config(false);
        let mut client = ApiClient::new(&config);
        client.set_auth(
            "my-secret-api-key".to_string(),
            "my-service-123".to_string(),
        );

        let headers = client.headers().unwrap();

        // 设置 auth 后，应包含正确的 Authorization / X-API-Key / X-Service-ID
        let auth_header = headers.get("Authorization").unwrap();
        assert_eq!(auth_header, "Bearer my-secret-api-key");

        let api_key_header = headers.get("X-API-Key").unwrap();
        assert_eq!(api_key_header, "my-secret-api-key");

        let service_header = headers.get("X-Service-ID").unwrap();
        assert_eq!(service_header, "my-service-123");

        // 应包含 Content-Type
        assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
    }

    // ======== HTTP Mock 测试 ========

    #[tokio::test]
    async fn test_register_success() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/agent/register")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "api_key": "test-api-key-123",
                "service_id": "test-service-456",
                "name": "Test Agent"
            }"#,
            )
            .create_async()
            .await;

        let config = ServerConfig {
            base_url: server.url(),
            poll_interval_secs: 10,
            use_system_proxy: false,
        };

        let mut client = ApiClient::new(&config);
        let result = client.register("test-token", 5).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.api_key, "test-api-key-123");
        assert_eq!(response.service_id, "test-service-456");

        // 验证 client 已设置 auth
        assert_eq!(client.api_key, Some("test-api-key-123".to_string()));
        assert_eq!(client.service_id, Some("test-service-456".to_string()));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_register_unauthorized() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/agent/register")
            .with_status(401)
            .create_async()
            .await;

        let config = ServerConfig {
            base_url: server.url(),
            poll_interval_secs: 10,
            use_system_proxy: false,
        };

        let mut client = ApiClient::new(&config);
        let result = client.register("invalid-token", 5).await;

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("注册令牌无效") || error_msg.contains("Unauthorized"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_poll_success() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/agent/test-service/poll")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "has_task": true,
                "task": {
                    "id": "task-123",
                    "task_prompt": "Do something important",
                    "output_prompt": "Output as JSON",
                    "session_id": "session-456"
                },
                "should_cancel": false,
                "cancel_task_id": null
            }"#,
            )
            .create_async()
            .await;

        let config = ServerConfig {
            base_url: server.url(),
            poll_interval_secs: 10,
            use_system_proxy: false,
        };

        let mut client = ApiClient::new(&config);
        client.set_auth("test-api-key".to_string(), "test-service".to_string());

        let result = client.poll(0, 5).await;

        assert!(result.is_ok());
        let outcome = result.unwrap();
        assert!(outcome.cancel_task_id.is_none());
        assert!(outcome.task.is_some());
        let task = outcome.task.unwrap();
        assert_eq!(task.id, "task-123");
        assert_eq!(task.task_prompt, "Do something important");
        assert_eq!(task.output_prompt, Some("Output as JSON".to_string()));
        assert_eq!(task.session_id, "session-456".to_string());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_poll_empty() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/agent/test-service/poll")
            .with_status(204)
            .create_async()
            .await;

        let config = ServerConfig {
            base_url: server.url(),
            poll_interval_secs: 10,
            use_system_proxy: false,
        };

        let mut client = ApiClient::new(&config);
        client.set_auth("test-api-key".to_string(), "test-service".to_string());

        let result = client.poll(0, 5).await;

        assert!(result.is_ok());
        let outcome = result.unwrap();
        assert!(outcome.task.is_none());
        assert!(outcome.cancel_task_id.is_none());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_accept_task_success() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/agent/test-service/accept")
            .with_status(200)
            .create_async()
            .await;

        let config = ServerConfig {
            base_url: server.url(),
            poll_interval_secs: 10,
            use_system_proxy: false,
        };

        let mut client = ApiClient::new(&config);
        client.set_auth("test-api-key".to_string(), "test-service".to_string());

        let result = client.accept_task("task-123").await;

        assert!(result.is_ok());
        assert!(result.unwrap());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_complete_task_success() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/agent/test-service/complete")
            .with_status(200)
            .create_async()
            .await;

        let config = ServerConfig {
            base_url: server.url(),
            poll_interval_secs: 10,
            use_system_proxy: false,
        };

        let mut client = ApiClient::new(&config);
        client.set_auth("test-api-key".to_string(), "test-service".to_string());

        let output = Some(serde_json::json!({
            "result": "success",
            "files": ["output.txt"]
        }));

        let result = client
            .complete_task(
                "task-123",
                TaskCompleteStatus::Completed,
                output,
                None,
                vec![],
            )
            .await;

        assert!(result.is_ok());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_heartbeat_success() {
        let mut server = Server::new_async().await;

        // 创建 mock - 添加请求体验证
        let mock = server
            .mock("POST", "/api/v1/agent/test-service/heartbeat")
            .match_header("Authorization", "Bearer test-api-key")
            .match_body(mockito::Matcher::Json(json!({
                "status": "online",
                "current_load": 1,
                "capacity": 5,
                "available_capacity": 4
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"success": true}"#)
            .create_async()
            .await;

        let config = ServerConfig {
            base_url: server.url(),
            poll_interval_secs: 10,
            use_system_proxy: false,
        };

        let mut client = ApiClient::new(&config);
        client.set_auth("test-api-key".to_string(), "test-service".to_string());

        // 测试心跳 - 传入负载参数
        let result = client.heartbeat(1, 5).await;
        assert!(result.is_ok());

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_upload_file_success() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/files/agent/test-service/files/upload")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "file_id": "file-abc-123",
                "filename": "test.txt",
                "size_bytes": 1024
            }"#,
            )
            .create_async()
            .await;

        let config = ServerConfig {
            base_url: server.url(),
            poll_interval_secs: 10,
            use_system_proxy: false,
        };

        let mut client = ApiClient::new(&config);
        client.set_auth("test-api-key".to_string(), "test-service".to_string());

        // 创建临时文件
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_upload.txt");
        tokio::fs::write(&file_path, "test content").await.unwrap();

        let result = client.upload_file("task-123", &file_path).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "file-abc-123");

        mock.assert_async().await;
    }

    // ==================== HeartbeatRequest 测试 ====================

    #[test]
    fn test_heartbeat_request_serialization_full() {
        let req = HeartbeatRequest {
            status: "busy".to_string(),
            current_load: 3,
            capacity: 5,
            available_capacity: 2,
        };

        let json = serde_json::to_string(&req).unwrap();

        // 验证 JSON 包含所有字段
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["status"], "busy");
        assert_eq!(value["current_load"], 3);
        assert_eq!(value["capacity"], 5);
        assert_eq!(value["available_capacity"], 2);
    }

    #[test]
    fn test_heartbeat_request_serialization_online() {
        let req = HeartbeatRequest {
            status: "online".to_string(),
            current_load: 0,
            capacity: 5,
            available_capacity: 5,
        };

        let json = serde_json::to_string(&req).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["status"], "online");
        assert_eq!(value["current_load"], 0);
        assert_eq!(value["capacity"], 5);
        assert_eq!(value["available_capacity"], 5);
    }

    #[test]
    fn test_heartbeat_request_serialization_offline() {
        let req = HeartbeatRequest {
            status: "offline".to_string(),
            current_load: 0,
            capacity: 5,
            available_capacity: 5,
        };

        let json = serde_json::to_string(&req).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["status"], "offline");
    }

    // ==================== 状态计算逻辑测试 ====================

    #[test]
    fn test_status_calculation_online() {
        // current_load < capacity -> "online"
        let current_load = 2usize;
        let capacity = 5usize;
        let status = if current_load >= capacity {
            "busy"
        } else {
            "online"
        };
        assert_eq!(status, "online");
    }

    #[test]
    fn test_status_calculation_busy_at_capacity() {
        // current_load == capacity -> "busy"
        let current_load = 5usize;
        let capacity = 5usize;
        let status = if current_load >= capacity {
            "busy"
        } else {
            "online"
        };
        assert_eq!(status, "busy");
    }

    #[test]
    fn test_status_calculation_busy_exceeds() {
        // current_load > capacity -> "busy"
        let current_load = 7usize;
        let capacity = 5usize;
        let status = if current_load >= capacity {
            "busy"
        } else {
            "online"
        };
        assert_eq!(status, "busy");
    }

    #[test]
    fn test_status_calculation_zero_load() {
        // current_load = 0 -> "online"
        let current_load = 0usize;
        let capacity = 5usize;
        let status = if current_load >= capacity {
            "busy"
        } else {
            "online"
        };
        assert_eq!(status, "online");
    }

    #[test]
    fn test_available_capacity_calculation() {
        // available_capacity = capacity - current_load
        let current_load = 3usize;
        let capacity = 5usize;
        let available_capacity = capacity.saturating_sub(current_load);
        assert_eq!(available_capacity, 2);
    }

    #[test]
    fn test_available_capacity_saturating_sub() {
        // 测试 saturating_sub 不会溢出
        let current_load = 10usize;
        let capacity = 5usize;
        let available_capacity = capacity.saturating_sub(current_load);
        assert_eq!(available_capacity, 0);
    }

    // ==================== Heartbeat 方法测试（使用 mock server）====================

    #[tokio::test]
    async fn test_heartbeat_with_load_reporting() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/agent/test-service/heartbeat")
            .match_body(mockito::Matcher::JsonString(
                r#"{
                "status": "online",
                "current_load": 2,
                "capacity": 5,
                "available_capacity": 3
            }"#
                .to_string(),
            ))
            .with_status(200)
            .create_async()
            .await;

        let config = ServerConfig {
            base_url: server.url(),
            poll_interval_secs: 10,
            use_system_proxy: false,
        };

        let mut client = ApiClient::new(&config);
        client.set_auth("test-api-key".to_string(), "test-service".to_string());

        // 发送心跳，上报负载
        let result = client.heartbeat(2, 5).await;

        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_heartbeat_busy_status() {
        let mut server = Server::new_async().await;

        // 当 current_load >= capacity 时，状态应为 "busy"
        let mock = server
            .mock("POST", "/api/v1/agent/test-service/heartbeat")
            .match_body(mockito::Matcher::JsonString(
                r#"{
                "status": "busy",
                "current_load": 5,
                "capacity": 5,
                "available_capacity": 0
            }"#
                .to_string(),
            ))
            .with_status(200)
            .create_async()
            .await;

        let config = ServerConfig {
            base_url: server.url(),
            poll_interval_secs: 10,
            use_system_proxy: false,
        };

        let mut client = ApiClient::new(&config);
        client.set_auth("test-api-key".to_string(), "test-service".to_string());

        // 满载时发送心跳
        let result = client.heartbeat(5, 5).await;

        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_heartbeat_zero_load() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/api/v1/agent/test-service/heartbeat")
            .match_body(mockito::Matcher::JsonString(
                r#"{
                "status": "online",
                "current_load": 0,
                "capacity": 5,
                "available_capacity": 5
            }"#
                .to_string(),
            ))
            .with_status(200)
            .create_async()
            .await;

        let config = ServerConfig {
            base_url: server.url(),
            poll_interval_secs: 10,
            use_system_proxy: false,
        };

        let mut client = ApiClient::new(&config);
        client.set_auth("test-api-key".to_string(), "test-service".to_string());

        // 零负载时发送心跳
        let result = client.heartbeat(0, 5).await;

        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[test]
    fn test_mask_token_long_ascii() {
        assert_eq!(mask_token("abcdefgh"), "abcd***");
    }

    #[test]
    fn test_mask_token_short() {
        assert_eq!(mask_token("abc"), "***");
        assert_eq!(mask_token("abcd"), "***");
    }

    #[test]
    fn test_mask_token_empty() {
        assert_eq!(mask_token(""), "***");
    }

    #[test]
    fn test_mask_token_unicode() {
        // 中文字符为 3 字节 UTF-8 编码，按字节切片会 panic；按字符切片安全
        assert_eq!(mask_token("中文测试令牌"), "中文测试***");
        // 混合 ASCII 与 Unicode 字符
        assert_eq!(mask_token("αβγδε"), "αβγδ***");
        // 4 个 Unicode 字符应全部隐藏
        assert_eq!(mask_token("中文测试"), "***");
    }
}
