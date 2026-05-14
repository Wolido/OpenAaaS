use std::path::PathBuf;
use std::time::Duration;
use agent_core::{config::Config, main_support::*};
use std::process::Command as StdCommand;
use tracing::info;

pub async fn stop(config_path: PathBuf) -> anyhow::Result<()> {
    let config = Config::load_from_path(&config_path).await?;

    match check_running(&config)? {
        Some(pid) => {
            info!("停止 Agent (PID: {})...", pid);

            #[cfg(unix)]
            {
                use std::process::Stdio;
                let output = StdCommand::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .output()?;

                if output.status.success() {
                    println!("Agent 已停止 (PID: {})", pid);
                    // 等待进程退出
                    for _ in 0..10 {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        if check_running(&config)?.is_none() {
                            break;
                        }
                    }
                    // 强制清理 pidfile
                    remove_pidfile(&config);
                } else {
                    println!("停止 Agent 失败");
                }
            }

            #[cfg(not(unix))]
            {
                // Windows: 使用 taskkill
                let _ = StdCommand::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/F"])
                    .output()?;
                println!("Agent 已停止");
            }
        }
        None => {
            println!("Agent 未在运行");
        }
    }

    Ok(())
}
