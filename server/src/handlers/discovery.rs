//! API 发现端点
//!
//! 提供给客户端自动发现 API 文档的端点，无需认证

use axum::{Json, extract::State};
use serde_json::{Value, json};

use crate::state::AppState;

/// GET /api/v1/discovery - 返回 API 文档
pub async fn discovery(State(_state): State<AppState>) -> Json<Value> {
    Json(json!({
        "api": {
            "name": "OpenAaaS",
            "version": env!("CARGO_PKG_VERSION"),
            "base_url": "/api/v1"
        },
        "instructions": "你是一个 OpenAaaS 客户端代理。你的工作流程是：\n\n1. **认证**：如果用户没有 api_key，调用 register 获取并保存\n2. **浏览服务**：调用 list_services 获取轻量服务列表（id, name, description, agent_status）\n3. **了解服务**：对候选服务调用 get_service_usage 获取详细用法说明\n4. **提交任务**：根据 usage 说明构造 task_prompt 和 output_prompt，调用 submit_task\n5. **跟踪结果**：保存返回的 task_id。需要结果时调用 get_task 查询状态，或用 list_files + download_file 获取结果\n\n重要原则：\n- 遵循渐进式披露：先 list_services 筛选，再 get_service_usage 获取详情，避免一次性加载大量长文本\n- public 服务自动可用；restricted 服务需要 has_permission=true\n- 任务异步执行，提交后立即返回，不要立即阻塞轮询，待用户需要时再调用 get_task 查询状态\n- submit_task 使用 multipart/form-data，可附带文件",
        "design_principles": {
            "progressive_disclosure": "信息渐进式披露：API 设计遵循'先摘要筛选，再按需详情'的原则。list_services 返回轻量列表（name + description + status），不占用上下文；get_service_usage 按需获取单个服务的详细用法说明。调用方应先浏览列表筛选候选，再对目标服务获取 usage，避免一次性加载大量长文本。"
        },
        "auth": {
            "type": "bearer",
            "header": "Authorization: Bearer {api_key}"
        },
        "workflows": {
            "client_register": {
                "steps": [
                    "1. POST /client/auth/register - 注册获取 API Key",
                    "2. 保存返回的 api_key，用于后续请求认证",
                    "3. 建议在调用方项目目录写入 .env，例如 OPEN_AAAS_API_KEY={api_key}",
                    "4. GET /client/services - 获取轻量服务列表浏览可用服务",
                    "5. GET /client/services/{id}/usage - 按需获取目标服务的详细 usage",
                    "6. 如需使用受限服务，联系管理员授权"
                ],
                "env_example": "OPEN_AAAS_API_KEY={api_key}",
                "note": "这里的 .env 是调用 OpenAaaS API 的 client 项目配置，和 server、agent-core 的部署配置无关。"
            },
            "submit_task": {
                "steps": [
                    "1. GET /client/services - 获取轻量服务列表（name/description/agent_status），浏览筛选候选",
                    "2. GET /client/services/{id}/usage - 对筛选出的候选服务，按需获取详细 usage（能力范围、调用规范）",
                    "3. 根据 usage 说明，构造正确的 task_prompt 和 output_prompt",
                    "4. POST /client/tasks - 创建任务（支持 application/json 和 multipart/form-data。JSON 方式不支持附件，如需上传文件请使用 multipart）",
                    "5. 保存返回的 task_id，任务在后台异步执行",
                    "6. 使用 Dashboard 查看进度，或稍后主动查询任务状态",
                    "7. GET /client/files/list/{task_id} - 获取结果文件列表",
                    "8. GET /client/files/{id}/download - 下载结果"
                ],
                "note": "任务执行时间从几分钟到几小时不等，提交后立即返回。**请勿轮询**。推荐使用 Dashboard 实时监控，或任务完成后再查询结果。"
            },
        },
        "endpoints": [
            {
                "name": "create_task",
                "method": "POST",
                "path": "/client/tasks",
                "content_type": "application/json 或 multipart/form-data",
                "body": {
                    "service_id": "string (必需) - 服务ID",
                    "task_prompt": "string (必需) - 任务描述",
                    "output_prompt": "string (必需) - 输出格式要求",
                    "session_id": "string (可选) - 会话ID",
                    "files": "binary (仅 multipart 支持，可选，可多个) - 输入文件"
                },
                "response": {
                    "id": "string",
                    "status": "pending",
                    "input": {
                        "task_prompt": "string",
                        "output_prompt": "string",
                        "input_files": ["string"]
                    },
                    "created_at": "ISO8601"
                }
            },
            {
                "name": "list_tasks",
                "method": "GET",
                "path": "/client/tasks?status=&service_id=&limit=20&offset=0"
            },
            {
                "name": "get_task",
                "method": "GET",
                "path": "/client/tasks/{id}"
            },
            {
                "name": "cancel_task",
                "method": "POST",
                "path": "/client/tasks/{id}/cancel",
                "limitation": "只能取消pending或running状态"
            },
            {
                "name": "register",
                "method": "POST",
                "path": "/client/auth/register",
                "body": {
                    "name": "string (必需) - Client 名称，1-64字符，trim后不能为空"
                },
                "response": {
                    "api_key": "string - 保存好，后续请求需要使用",
                    "id": "string - Client ID"
                },
                "note": "无需认证，首次调用生成 API Key"
            },
            {
                "name": "list_services",
                "method": "GET",
                "path": "/client/services",
                "response": {
                    "services": [
                        {
                            "id": "string",
                            "name": "string",
                            "description": "string|null",
                            "agent_status": "online|offline|busy",
                            "registration_status": "string - 注册状态",
                            "agent_last_heartbeat": "string|null - 最后心跳时间",
                            "access_type": "public|restricted",
                            "has_permission": "true|false"
                        }
                    ]
                },
                "note": "返回轻量列表（不含 usage 长文本），用于快速浏览和筛选服务。如需了解服务详细能力，请使用 get_service_usage 按需获取。public 服务自动可用；restricted 服务需要 has_permission=true 才能使用。"
            },
            {
                "name": "get_service_usage",
                "method": "GET",
                "path": "/client/services/{id}/usage",
                "response": {
                    "id": "string",
                    "name": "string",
                    "usage": "string - 服务使用说明/用法"
                },
                "note": "渐进式披露的关键步骤：在 list_services 筛选出候选服务后，调用此接口获取目标服务的详细 usage（能力范围、调用规范、返回格式、限制条件）。usage 内容通常较长，只应在确定使用该服务时获取，避免占用上下文。"
            },
            {
                "name": "download_file",
                "method": "GET",
                "path": "/client/files/{id}/download"
            },
            {
                "name": "list_files",
                "method": "GET",
                "path": "/client/files/list/{task_id}"
            },
            {
                "name": "update_profile",
                "method": "PUT",
                "path": "/client/profile",
                "body": {
                    "name": "string (必需) - 新用户名"
                },
                "response": {
                    "id": "string",
                    "name": "string",
                    "api_key": "string (空 - 请从注册响应中保存)",
                    "role": "string",
                    "created_at": "ISO8601"
                }
            },
        ],
        "types": {
            "ServiceListItem": {
                "id": "string",
                "name": "string",
                "description": "string",
                "agent_status": "online|offline|busy",
                "registration_status": "string - 注册状态",
                "agent_last_heartbeat": "string|null - 最后心跳时间",
                "access_type": "public|restricted",
                "has_permission": "true|false"
            },
            "ServiceUsage": {
                "id": "string",
                "name": "string",
                "usage": "string - 服务使用说明/用法"
            },
            "TaskInput": {
                "task_prompt": "string",
                "output_prompt": "string",
                "input_files": ["string"]
            }
        },
        "errors": {
            "400": "请求参数错误",
            "401": "认证失败",
            "404": "资源不存在",
            "409": "状态冲突"
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    async fn setup_test_state() -> (AppState, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let mut config = AppConfig::default();
        config.database.url = "sqlite::memory:".to_string();
        config.task.file_storage_path = temp_dir.path().to_string_lossy().to_string();
        let _ = tokio::fs::create_dir_all(&config.task.file_storage_path).await;
        let state = AppState::new(config)
            .await
            .expect("Failed to create test state");
        (state, temp_dir)
    }

    #[tokio::test]
    async fn test_discovery_returns_expected_api_info() {
        let (state, _temp_dir) = setup_test_state().await;
        let Json(value) = discovery(State(state)).await;

        let api = value.get("api").expect("api field missing");
        assert_eq!(api.get("name").and_then(|v| v.as_str()), Some("OpenAaaS"));
        assert_eq!(
            api.get("version").and_then(|v| v.as_str()),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(
            api.get("base_url").and_then(|v| v.as_str()),
            Some("/api/v1")
        );
    }

    #[tokio::test]
    async fn test_discovery_endpoints_not_empty() {
        let (state, _temp_dir) = setup_test_state().await;
        let Json(value) = discovery(State(state)).await;

        let endpoints = value.get("endpoints").expect("endpoints field missing");
        let endpoints_arr = endpoints.as_array().expect("endpoints should be an array");
        assert!(
            !endpoints_arr.is_empty(),
            "endpoints list should not be empty"
        );

        let names: Vec<&str> = endpoints_arr
            .iter()
            .filter_map(|e| e.get("name").and_then(|n| n.as_str()))
            .collect();

        assert!(
            names.contains(&"create_task"),
            "should contain create_task endpoint"
        );
        assert!(
            names.contains(&"list_services"),
            "should contain list_services endpoint"
        );
        assert!(
            names.contains(&"register"),
            "should contain register endpoint"
        );
    }
}
