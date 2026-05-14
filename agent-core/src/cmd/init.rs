use std::path::PathBuf;
use agent_core::config::Config;
use anyhow::Context;
use tracing::info;

pub async fn init(config_path: PathBuf) -> anyhow::Result<()> {
    info!("初始化配置...");
    let config = Config::default();
    config.save_to_path(&config_path).await?;
    info!("配置已保存到: {:?}", config_path);
    println!("配置文件已创建: {:?}", config_path);
    println!("可直接运行 agent-core run，缺少的启动信息会在终端提示录入");
    Ok(())
}

/// 检查 Server 是否可达
pub async fn check_server_available(url: &str) -> anyhow::Result<bool> {
    // 解析 URL 获取 host 和 port
    let parsed = reqwest::Url::parse(url).context("解析 Server URL 失败")?;

    let host = parsed.host_str().context("URL 中缺少 host")?;
    let port = parsed.port_or_known_default().unwrap_or(8080);

    // 尝试 TCP 连接
    match tokio::net::TcpStream::connect((host, port)).await {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => Ok(false),
        Err(e) => Err(anyhow::anyhow!("连接检查失败: {}", e)),
    }
}
