use std::path::PathBuf;
use open_aaas_server::main_support::*;
use crate::cli;

pub async fn run_detached(config_path: PathBuf) -> anyhow::Result<()> {
    let config = cli::prepare_server_config(&config_path)?;

    if let Some(pid) = check_running(&config)? {
        println!("Server 已在后台运行 (PID: {})", pid);
        return Ok(());
    }

    let exe_path = std::env::current_exe()?;

    // 创建日志文件路径（从 database_url 推断 data_dir）
    let data_dir = database_dir_from_url(&config.database.url)
        .unwrap_or_else(|| PathBuf::from("./data"));
    let log_path = data_dir.join("server.log");
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    {
        use std::fs::File;
        use std::process::Stdio;

        let log_file = File::create(&log_path)?;
        let child = std::process::Command::new("nohup")
            .arg(&exe_path)
            .arg("--config")
            .arg(&config_path)
            .arg("run")
            .stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file))
            .stdin(Stdio::null())
            .spawn()?;

        println!("Server 已在后台启动 (PID: {})", child.id());
        println!("查看日志: tail -f {}", log_path.display());
    }

    #[cfg(not(unix))]
    {
        use std::fs::File;
        use std::process::Stdio;

        let log_file = File::create(&log_path)?;
        let _ = std::process::Command::new("cmd")
            .args(&["/C", "start", "/B", ""])
            .arg(&exe_path)
            .arg("--config")
            .arg(&config_path)
            .arg("run")
            .stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file))
            .spawn()?;

        println!("Server 已在后台启动");
        println!("查看日志: {}", log_path.display());
    }

    Ok(())
}
