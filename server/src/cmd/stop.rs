use std::path::PathBuf;
use std::time::Duration;
use open_aaas_server::main_support::*;

pub async fn stop(config_path: PathBuf) -> anyhow::Result<()> {
    let pidfile = match load_config_from_path(&config_path) {
        Ok(config) => pidfile_path(&config),
        Err(_) => PathBuf::from("./data/server.pid"),
    };

    if !pidfile.exists() {
        println!("Server 未在运行");
        return Ok(());
    }

    let pid_str = std::fs::read_to_string(&pidfile)?;
    let pid = match pid_str.trim().parse::<u32>() {
        Ok(p) if p > 0 => p,
        _ => {
            let _ = std::fs::remove_file(&pidfile);
            println!("Server 未在运行 (pidfile 已清理)");
            return Ok(());
        }
    };

    #[cfg(unix)]
    {
        use std::process::Stdio;

        // 检查进程是否存在
        let output = std::process::Command::new("kill")
            .args(&["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()?;

        if !output.status.success() {
            println!("Server 未在运行 (PID: {} 已不存在)", pid);
            let _ = std::fs::remove_file(&pidfile);
            return Ok(());
        }

        // 发送优雅关闭信号
        let output = std::process::Command::new("kill")
            .args(&["-TERM", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()?;

        if !output.status.success() {
            // 竞态：进程可能在 kill -0 和 kill -TERM 之间退出
            let check = std::process::Command::new("kill")
                .args(&["-0", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output()?;
            if !check.status.success() {
                println!("Server 已停止 (PID: {} 已退出)", pid);
                let _ = std::fs::remove_file(&pidfile);
                return Ok(());
            }
            anyhow::bail!("停止 Server 失败");
        }

        println!("正在停止 Server (PID: {})...", pid);

        // 等待进程退出（轮询最多 5 秒）
        let mut stopped = false;
        for _ in 0..50 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let output = std::process::Command::new("kill")
                .args(&["-0", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output()?;
            if !output.status.success() {
                stopped = true;
                break;
            }
        }

        if !stopped {
            println!("优雅关闭超时，强制终止 (PID: {})...", pid);
            let output = std::process::Command::new("kill")
                .args(&["-KILL", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output()?;
            if !output.status.success() {
                // 确认进程是否真的不存在了
                let check = std::process::Command::new("kill")
                    .args(&["-0", &pid.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .output()?;
                if check.status.success() {
                    anyhow::bail!("强制终止 Server 失败");
                }
            }
        }

        // 最后再确认进程不存在，才删除 pidfile
        let check = std::process::Command::new("kill")
            .args(&["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()?;
        if !check.status.success() {
            let _ = std::fs::remove_file(&pidfile);
            println!("Server 已停止");
        } else {
            anyhow::bail!("Server 停止后进程仍存在");
        }
    }

    #[cfg(not(unix))]
    {
        let _ = std::process::Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/F"])
            .output()?;
        let _ = std::fs::remove_file(&pidfile);
        println!("Server 已停止");
    }

    Ok(())
}
