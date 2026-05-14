use std::path::PathBuf;
use agent_core::main_support::*;
use tracing::info;
use std::process::Command as StdCommand;

pub async fn run_detached(config_path: PathBuf) -> anyhow::Result<()> {
    let config = ensure_agent_runtime_config(&config_path).await?;

    // 检查是否已有实例在运行
    if let Some(pid) = check_running(&config)? {
        println!("Agent 已在后台运行 (PID: {})", pid);
        return Ok(());
    }

    info!("启动后台模式...");

    // 获取当前可执行文件路径
    let exe_path = std::env::current_exe()?;

    // 创建日志文件路径
    let log_path = config.data_dir().join("agent.log");
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    {
        use std::fs::File;
        use std::process::Stdio;

        let log_file = File::create(&log_path)?;
        let child = StdCommand::new("nohup")
            .arg(&exe_path)
            .arg("--config")
            .arg(&config_path)
            .arg("run")
            .stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file))
            .stdin(Stdio::null())
            .spawn()?;

        println!("Agent 已在后台启动 (PID: {})", child.id());
        println!("查看日志: tail -f {}", log_path.display());
    }

    #[cfg(not(unix))]
    {
        use std::fs::File;
        use std::process::Stdio;

        let log_file = File::create(&log_path)?;
        let _ = StdCommand::new("cmd")
            .args(["/C", "start", "/B", ""])
            .arg(&exe_path)
            .arg("--config")
            .arg(&config_path)
            .arg("run")
            .stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file))
            .spawn()?;

        println!("Agent 已在后台启动");
        println!("查看日志: {}", log_path.display());
    }

    Ok(())
}
