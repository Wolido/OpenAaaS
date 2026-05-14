use std::fs;
use std::path::PathBuf;
use agent_core::{
    client::ApiClient, config::Config, executor::docker::DockerExecutor,
    executor::Executor, main_support::*, scheduler::{Scheduler, SchedulerCommand},
    state::StateManager,
};
use tokio::signal;
use tracing::{info, warn};
use super::init::check_server_available;
use super::register::{prompt_registration_token, register};

pub async fn run_foreground(config_path: PathBuf, interactive: bool) -> anyhow::Result<()> {
    info!("启动前台模式");

    // 加载配置
    let mut config = ensure_agent_runtime_config(&config_path).await?;
    info!("配置加载完成: data_dir={:?}", config.data_dir());

    // 检查是否已有实例在运行
    if let Some(pid) = check_running(&config)? {
        anyhow::bail!(
            "Agent 已在运行 (PID: {})，请先停止或运行 `agent-core stop`",
            pid
        );
    }

    // 检查 Server 端口是否可达
    let server_url = &config.server.base_url;
    info!("检查 Server 连接: {}", server_url);
    match check_server_available(server_url).await {
        Ok(true) => info!("Server 连接正常"),
        Ok(false) => {
            warn!("无法连接到 Server: {}", server_url);
            warn!("请检查:");
            warn!("  1. Server 是否已启动");
            warn!("  2. 配置文件中的 server.base_url 是否正确");
            warn!("  3. 网络连接是否正常");
            // 不强制退出，允许离线启动（如果设计支持）
        }
        Err(e) => {
            warn!("检查 Server 连接时出错: {}", e);
        }
    }

    // 确保挂载目录存在
    config.ensure_mount_dirs().await?;

    // 获取 docker 挂载参数
    let docker_mounts = config.docker_mounts();
    info!("Docker 挂载: {:?}", docker_mounts);

    // 检查是否已注册
    if !config.agent.has_credentials() {
        if interactive || is_interactive_terminal() {
            let token = prompt_registration_token()?;
            let name = config.agent.name.clone();
            register(config_path.clone(), Some(token), name).await?;
            println!("继续启动 agent-core...");
            config = Config::load_from_path(&config_path).await?;
        } else {
            anyhow::bail!("Agent 未注册，请先运行: agent-core register --token <TOKEN>");
        }
    }

    let pidfile = pidfile_path(&config);

    // 写入 pidfile
    write_pidfile(&config)?;
    info!("pidfile 已写入: {:?}", pidfile);

    // 创建 API 客户端
    let mut client = ApiClient::new(&config.server);
    client.set_auth(
        config.agent.api_key.clone().unwrap(),
        config.agent.service_id.clone().unwrap(),
    );

    // 初始化状态管理
    let state = StateManager::init(config.database_path()).await?;
    info!("状态管理器初始化完成");

    // 创建执行器
    let executor = DockerExecutor::new(config.executor.clone(), config.data_dir());
    info!("Docker 执行器创建完成，容量: {}", executor.capacity());

    // 创建调度器
    let scheduler = Scheduler::new(config, client, executor, state);
    println!("agent-core 已启动，正在轮询 server 等待任务...");

    // 优雅退出处理
    let scheduler_clone = scheduler.command_sender();
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt()).ok();
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).ok();

            tokio::select! {
                _ = async { sigint.as_mut()?.recv().await }, if sigint.is_some() => {}
                _ = async { sigterm.as_mut()?.recv().await }, if sigterm.is_some() => {}
                _ = signal::ctrl_c() => {}
            }
        }
        #[cfg(not(unix))]
        {
            let _ = signal::ctrl_c().await;
        }

        info!("收到退出信号，正在优雅停止...");
        let _ = scheduler_clone.send(SchedulerCommand::Stop).await;
    });

    // 运行调度器
    scheduler.run().await?;

    // 删除 pidfile
    let _ = fs::remove_file(&pidfile);
    info!("pidfile 已删除");

    info!("Agent 已停止");
    Ok(())
}
