use std::path::PathBuf;
use open_aaas_server::main_support::*;

pub async fn status(config_path: PathBuf) -> anyhow::Result<()> {
    let (pidfile, data_dir, addr) = match load_config_from_path(&config_path) {
        Ok(config) => {
            let pidfile = pidfile_path(&config);
            let data_dir = database_dir_from_url(&config.database.url)
                .unwrap_or_else(|| PathBuf::from("./data"));
            let addr = config.server_addr().to_string();
            (pidfile, data_dir, addr)
        }
        Err(_) => {
            let default_pidfile = PathBuf::from("./data/server.pid");
            (default_pidfile, PathBuf::from("./data"), "unknown".to_string())
        }
    };

    let running = if pidfile.exists() {
        let pid_str = std::fs::read_to_string(&pidfile).unwrap_or_default();
        let pid = match pid_str.trim().parse::<u32>() {
            Ok(p) if p > 0 => p,
            _ => {
                let _ = std::fs::remove_file(&pidfile);
                println!("运行状态: 未运行");
                return Ok(());
            }
        };

        #[cfg(unix)]
        {
            use std::process::Stdio;
            let output = std::process::Command::new("kill")
                .args(&["-0", &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output()?;
            if output.status.success() {
                Some(pid)
            } else {
                None
            }
        }

        #[cfg(not(unix))]
        {
            Some(pid)
        }
    } else {
        None
    };

    println!("OpenAaaS Server 状态");
    println!("====================");
    println!("配置文件: {}", config_path.display());
    println!("数据目录: {}", data_dir.display());
    println!("监听地址: {}", addr);
    println!();
    if let Some(pid) = running {
        println!("运行状态: 运行中 (PID: {})", pid);
    } else {
        println!("运行状态: 未运行");
    }

    Ok(())
}
