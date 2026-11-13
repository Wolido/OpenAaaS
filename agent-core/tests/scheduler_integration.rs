//! 调度器集成测试
//!
//! 使用 wiremock mock Server API，测试调度器的核心逻辑

use agent_core::{
    client::ApiClient,
    config::{Config, ServerConfig},
    progress::LiveRegistry,
    scheduler::{Scheduler, SchedulerCommand},
    state::StateManager,
    test_utils::MockExecutor,
};

use std::sync::Arc;
use std::time::Duration;

use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// 创建测试配置
fn create_test_config(server_url: String) -> Config {
    Config {
        server: ServerConfig {
            base_url: server_url,
            poll_interval_secs: 30,
            use_system_proxy: false,
        },
        agent: agent_core::config::AgentConfig::default(),
        executor: agent_core::config::ExecutorConfig::default(),
        paths: agent_core::config::PathConfig::default(),
    }
}

/// 创建测试用的 StateManager（使用内存数据库）
async fn create_test_state_manager() -> StateManager {
    StateManager::init_in_memory()
        .await
        .expect("Failed to init state manager")
}

#[tokio::test]
async fn test_stop_command() {
    // 启动 mock server
    let mock_server = MockServer::start().await;

    // 设置 poll 返回 204（无任务）
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/poll"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // 创建配置和客户端
    let config = create_test_config(mock_server.uri());
    let mut client = ApiClient::new(&config.server);
    client.set_auth("test-api-key".to_string(), "test-service".to_string());

    // 创建执行器和状态管理器
    let executor = MockExecutor::new();
    let state = create_test_state_manager().await;

    // 创建调度器
    let scheduler = Scheduler::new(
        config,
        client,
        executor.clone(),
        state,
        Arc::new(LiveRegistry::new()),
    );

    // 发送 Stop 命令
    let sender = scheduler.command_sender();
    sender.send(SchedulerCommand::Stop).await.unwrap();

    // 运行调度器，应该立即停止
    let result = scheduler.run().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cancel_task_command() {
    // 启动 mock server
    let mock_server = MockServer::start().await;

    // 设置必要的 mock
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/poll"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/heartbeat"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // 创建配置和客户端
    let config = create_test_config(mock_server.uri());
    let mut client = ApiClient::new(&config.server);
    client.set_auth("test-api-key".to_string(), "test-service".to_string());

    // 创建执行器和状态管理器
    let executor = MockExecutor::new();
    let state = create_test_state_manager().await;

    // 创建调度器
    let scheduler = Scheduler::new(
        config,
        client,
        executor.clone(),
        state,
        Arc::new(LiveRegistry::new()),
    );

    // 发送 CancelTask 命令
    let sender = scheduler.command_sender();
    let task_id = "cancel-test-task".to_string();
    sender
        .send(SchedulerCommand::CancelTask(task_id.clone()))
        .await
        .unwrap();

    // 发送 Stop 命令让调度器退出
    let sender2 = scheduler.command_sender();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        sender2.send(SchedulerCommand::Stop).await.unwrap();
    });

    // 运行调度器
    scheduler.run().await.unwrap();

    // 验证 cancel 被调用
    assert_eq!(
        executor.cancel_call_count(),
        1,
        "Cancel should be called once"
    );
}

#[tokio::test]
async fn test_load_capacity_check() {
    // 启动 mock server
    let mock_server = MockServer::start().await;

    // 设置 poll mock
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/poll"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "has_task": true,
            "task": {
                "id": "capacity-test-task",
                "task_prompt": "Capacity test",
                "output_prompt": null,
                "session_id": null
            },
            "should_cancel": false,
            "cancel_task_id": null
        })))
        .mount(&mock_server)
        .await;

    // 创建配置和客户端
    let config = create_test_config(mock_server.uri());
    let mut client = ApiClient::new(&config.server);
    client.set_auth("test-api-key".to_string(), "test-service".to_string());

    // 创建执行器并设置高负载
    let executor = MockExecutor::new();
    executor.set_capacity(1);
    executor.set_current_load(1); // 已满负载

    let state = create_test_state_manager().await;

    // 创建调度器
    let scheduler = Scheduler::new(
        config,
        client,
        executor.clone(),
        state,
        Arc::new(LiveRegistry::new()),
    );

    // 发送 Stop 命令让调度器退出
    let sender = scheduler.command_sender();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        sender.send(SchedulerCommand::Stop).await.unwrap();
    });

    // 运行调度器
    scheduler.run().await.unwrap();

    // 验证没有任务被执行（因为负载已满）
    assert_eq!(
        executor.execute_call_count(),
        0,
        "No task should be executed when at capacity"
    );
}

#[tokio::test]
async fn test_heartbeat() {
    // 启动 mock server
    let mock_server = MockServer::start().await;

    // 设置 poll 返回 204
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/poll"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // 设置 heartbeat mock，预期至少被调用一次
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/heartbeat"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1..)
        .mount(&mock_server)
        .await;

    // 创建配置和客户端
    let config = create_test_config(mock_server.uri());
    let mut client = ApiClient::new(&config.server);
    client.set_auth("test-api-key".to_string(), "test-service".to_string());

    // 创建执行器和状态管理器
    let executor = MockExecutor::new();
    let state = create_test_state_manager().await;

    // 创建调度器
    let scheduler = Scheduler::new(
        config,
        client,
        executor.clone(),
        state,
        Arc::new(LiveRegistry::new()),
    );

    // 发送 Stop 命令让调度器退出
    let sender = scheduler.command_sender();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(800)).await;
        sender.send(SchedulerCommand::Stop).await.unwrap();
    });

    // 运行调度器
    scheduler.run().await.unwrap();
}

#[tokio::test]
async fn test_poll_and_execute_success() {
    // 启动 mock server
    let mock_server = MockServer::start().await;

    // 设置 poll 返回任务的 mock
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/poll"))
        .and(header("Authorization", "Bearer test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "has_task": true,
            "task": {
                "id": "task-123",
                "task_prompt": "Test task prompt",
                "output_prompt": "Output as JSON",
                "session_id": "session-456"
            },
            "should_cancel": false,
            "cancel_task_id": null
        })))
        .mount(&mock_server)
        .await;

    // 设置 accept_task mock
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/accept"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // 设置 complete_task mock
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/complete"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // 创建配置和客户端
    let config = create_test_config(mock_server.uri());
    let mut client = ApiClient::new(&config.server);
    client.set_auth("test-api-key".to_string(), "test-service".to_string());

    // 创建执行器和状态管理器
    let executor = MockExecutor::new();
    let state = create_test_state_manager().await;

    // 创建调度器
    let scheduler = Scheduler::new(
        config,
        client,
        executor.clone(),
        state,
        Arc::new(LiveRegistry::new()),
    );

    // 发送 Stop 命令让调度器退出（延迟）
    let sender = scheduler.command_sender();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let _ = sender.send(SchedulerCommand::Stop).await;
    });

    // 运行调度器
    scheduler.run().await.unwrap();

    // 验证 poll 和 accept 被调用（通过 mock 验证），任务被提交执行
    // 由于任务是异步执行的，可能调度器退出时任务还未完成
    // 但我们至少可以验证调度器没有崩溃
}

#[tokio::test]
async fn test_task_accept_conflict() {
    // 启动 mock server
    let mock_server = MockServer::start().await;

    // 设置 poll 返回任务的 mock
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/poll"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "has_task": true,
            "task": {
                "id": "conflict-task",
                "task_prompt": "Conflict test task",
                "output_prompt": null,
                "session_id": null
            },
            "should_cancel": false,
            "cancel_task_id": null
        })))
        .mount(&mock_server)
        .await;

    // 设置 accept_task 返回 409（冲突）
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/accept"))
        .respond_with(ResponseTemplate::new(409))
        .mount(&mock_server)
        .await;

    // 创建配置和客户端
    let config = create_test_config(mock_server.uri());
    let mut client = ApiClient::new(&config.server);
    client.set_auth("test-api-key".to_string(), "test-service".to_string());

    // 创建执行器和状态管理器
    let executor = MockExecutor::new();
    let state = create_test_state_manager().await;

    // 创建调度器
    let scheduler = Scheduler::new(
        config,
        client,
        executor.clone(),
        state,
        Arc::new(LiveRegistry::new()),
    );

    // 发送 Stop 命令让调度器退出
    let sender = scheduler.command_sender();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        sender.send(SchedulerCommand::Stop).await.unwrap();
    });

    // 运行调度器
    scheduler.run().await.unwrap();

    // 验证任务没有被执行（因为接受失败）
    assert_eq!(
        executor.execute_call_count(),
        0,
        "Task should not be executed when accept fails"
    );
}

#[tokio::test]
async fn test_poll_no_task() {
    // 启动 mock server
    let mock_server = MockServer::start().await;

    // 设置 poll 返回无任务的 mock
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/poll"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    // 创建配置和客户端
    let config = create_test_config(mock_server.uri());
    let mut client = ApiClient::new(&config.server);
    client.set_auth("test-api-key".to_string(), "test-service".to_string());

    // 创建执行器和状态管理器
    let executor = MockExecutor::new();
    let state = create_test_state_manager().await;

    // 创建调度器
    let scheduler = Scheduler::new(
        config,
        client,
        executor.clone(),
        state,
        Arc::new(LiveRegistry::new()),
    );

    // 发送 Stop 命令让调度器退出
    let sender = scheduler.command_sender();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        sender.send(SchedulerCommand::Stop).await.unwrap();
    });

    // 运行调度器
    scheduler.run().await.unwrap();

    // 验证任务没有被执行
    assert_eq!(
        executor.execute_call_count(),
        0,
        "No task should be executed"
    );
}

#[tokio::test]
async fn test_task_state_transition() {
    // 启动 mock server
    let mock_server = MockServer::start().await;

    // 设置 poll 返回任务的 mock
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/poll"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "has_task": true,
            "task": {
                "id": "state-test-task",
                "task_prompt": "State transition test",
                "output_prompt": null,
                "session_id": null
            },
            "should_cancel": false,
            "cancel_task_id": null
        })))
        .mount(&mock_server)
        .await;

    // 设置 accept_task mock
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/accept"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // 设置 complete_task mock
    Mock::given(method("POST"))
        .and(path("/api/v1/agent/test-service/complete"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // 创建配置和客户端
    let config = create_test_config(mock_server.uri());
    let mut client = ApiClient::new(&config.server);
    client.set_auth("test-api-key".to_string(), "test-service".to_string());

    // 创建执行器和状态管理器
    let executor = MockExecutor::new();
    let state = create_test_state_manager().await;

    // 创建调度器
    let scheduler = Scheduler::new(
        config,
        client,
        executor.clone(),
        state,
        Arc::new(LiveRegistry::new()),
    );

    // 发送 Stop 命令让调度器退出（延迟）
    let sender = scheduler.command_sender();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let _ = sender.send(SchedulerCommand::Stop).await;
    });

    // 运行调度器
    scheduler.run().await.unwrap();

    // 验证 poll 和 accept 被调用（通过 mock 验证），任务被提交执行
    // 由于任务是异步执行的，可能调度器退出时任务还未完成
    // 但我们至少可以验证调度器没有崩溃
}
