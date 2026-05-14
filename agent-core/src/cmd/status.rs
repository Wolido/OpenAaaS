use std::path::PathBuf;
use agent_core::config::Config;

pub async fn status(config_path: PathBuf) -> anyhow::Result<()> {
    // 加载配置
    let config = Config::load_from_path(&config_path).await?;

    println!("OpenAaaS Agent 状态");
    println!("====================");
    println!("配置文件: {:?}", config_path);
    println!("数据目录: {:?}", config.data_dir());
    println!();
    println!("Server URL: {}", config.server.base_url);
    println!("轮询间隔: {} 秒", config.server.poll_interval_secs);
    println!();

    if let Some(ref service_id) = config.agent.service_id {
        println!("注册状态: 已注册");
        println!("Service ID: {}", service_id);
    } else {
        println!("注册状态: 未注册");
        println!("运行: agent-core register --token <TOKEN>");
    }

    if let Some(ref name) = config.agent.name {
        println!("Agent 名称: {}", name);
    }

    println!();
    println!("执行器配置:");
    println!("  镜像: {}", config.executor.image);
    println!("  容量: {}", config.executor.capacity);
    println!("  超时: {} 分钟", config.executor.timeout_minutes);

    Ok(())
}
