use std::path::PathBuf;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::watch;
use axum::{Router, extract::DefaultBodyLimit};
use tower_http::{compression::CompressionLayer, timeout::TimeoutLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;
use open_aaas_server::{handlers, main_support::*, state::AppState};
use crate::cli;

pub async fn run_foreground(config_path: PathBuf) -> anyhow::Result<()> {
    // 加载 .env 文件（如果存在）
    dotenvy::dotenv().ok();

    // 1. 准备配置
    let config = cli::prepare_server_config(&config_path)?;

    // 检查是否已有实例在运行
    if let Some(pid) = check_running(&config)? {
        anyhow::bail!("Server 已在运行 (PID: {})，请先停止或运行 `server stop`", pid);
    }

    // 写入 pidfile
    write_pidfile(&config)?;

    // 2. 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("open_aaas_server={},tower_http=warn", config.log_level).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!(
        "Starting OpenAaaS Server v{} (一对一服务模型)",
        env!("CARGO_PKG_VERSION")
    );

    // 2. 加载配置
    tracing::info!(
        "Server config: addr={}, timeout_secs={}, max_body_size={}",
        config.server.addr,
        config.server.timeout_secs,
        config.server.max_body_size
    );
    tracing::info!("Database config: {:?}", config.database);


    // 3. 创建数据库连接
    let state = AppState::new(config.clone()).await?;

    // 4. 初始化数据库表
    state.db.init_tables().await?;
    tracing::info!("Database tables initialized");

    // 确保 admin 用户存在
    let secret_key = config.secret_key.as_deref()
        .ok_or_else(|| anyhow::anyhow!("Secret key not configured"))?;
    let admin_api_key = cli::runtime_admin_api_key(&config)
        .unwrap_or_else(|| format!("ak_admin_{}", Uuid::new_v4().simple()));
    let admin_hash = open_aaas_server::auth::hash_api_key(secret_key, &admin_api_key);
    let result = sqlx::query(
        "INSERT OR IGNORE INTO users (id, api_key, name, role) VALUES (?, ?, ?, ?)"
    )
    .bind("admin")
    .bind(&admin_hash)
    .bind("Administrator")
    .bind("admin")
    .execute(state.db.pool())
    .await?;

    if result.rows_affected() > 0 {
        tracing::warn!("Admin 用户已创建，API Key: {}（请保存，只展示一次）", admin_api_key);
    }

    // 更新管理员API Key（优先环境变量，其次 config.toml）
    if let Some(admin_key) = cli::runtime_admin_api_key(&config) {
        let hashed_admin_key = open_aaas_server::auth::hash_api_key(secret_key, &admin_key);
        sqlx::query("UPDATE users SET api_key = ? WHERE id = 'admin'")
            .bind(&hashed_admin_key)
            .execute(state.db.pool())
            .await?;
        tracing::info!("Admin API Key updated from environment variable");
    }

    // 查询管理员API Key状态（仅输出提示信息，不泄露key值）
    let admin_key_exists: Option<String> =
        sqlx::query_scalar("SELECT api_key FROM users WHERE id = 'admin'")
            .fetch_optional(state.db.pool())
            .await?;

    tracing::info!("========================================");
    if let Some(key) = admin_key_exists {
        if key.is_empty() || key == "not-set" {
            tracing::warn!("Admin API Key 未设置，请设置 ADMIN_API_KEY 环境变量");
        } else {
            tracing::info!("Admin API Key 已配置 (长度: {} 字符)", key.len());
        }
    } else {
        tracing::warn!("Admin 用户不存在，请检查数据库初始化");
    }
    tracing::info!("========================================");

    // 5. 构建路由
    let app = Router::new()
        .merge(handlers::routes(state.clone()))
        // Multipart 上传默认会受到 Axum 默认 body limit（约 2MB）影响，
        // 这里关闭默认限制，具体文件大小由业务层 max_file_size_mb 逐文件控制。
        .layer(DefaultBodyLimit::disable())
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(config.server.timeout_secs),
        ))
        .with_state(state.clone());

    // 7. 启动后台心跳检查任务
    let (shutdown_tx, _shutdown_rx) = watch::channel(());
    let _heartbeat_handle = crate::bg_tasks::spawn_heartbeat_task(state.clone(), shutdown_tx.clone());

    // 8. 启动后台任务清理任务
    let retention_days = config.task.result_retention_days;
    let (cleanup_shutdown_tx, _cleanup_shutdown_rx) = watch::channel(false);
    let cleanup_handle = crate::bg_tasks::spawn_cleanup_task(state.clone(), cleanup_shutdown_tx.clone(), retention_days);

    // 9. 启动服务器
    let listener = TcpListener::bind(&config.server_addr()).await?;
    tracing::info!("Server listening on {}", config.server_addr());

    // 优雅关闭
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            let _ = shutdown_tx.send(());
            let _ = cleanup_shutdown_tx.send(true);
        })
        .await?;

    // 等待后台任务完成
    cleanup_handle.await.ok();
    tracing::info!("Cleanup task stopped");

    // 删除 pidfile
    remove_pidfile(&config);

    // 关闭数据库连接
    state.db.close().await;
    tracing::info!("Server shutdown complete");

    Ok(())
}

/// 优雅关闭信号处理
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, shutting down gracefully...");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, shutting down gracefully...");
        }
    }
}
