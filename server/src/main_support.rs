//! Server 主入口支持函数
//!
//! 从 main.rs 提取的可测试纯函数和工具函数。

use crate::config::AppConfig;
use std::path::{Path, PathBuf};

pub fn load_config_from_path(path: &Path) -> anyhow::Result<AppConfig> {
    let config = config::Config::builder()
        .add_source(config::File::from(path).required(false))
        .add_source(config::Environment::with_prefix("APP").separator("__"))
        .build()?;
    Ok(config.try_deserialize()?)
}

pub fn pidfile_path(config: &AppConfig) -> PathBuf {
    database_dir_from_url(&config.database.url)
        .unwrap_or_else(|| PathBuf::from("./data"))
        .join("server.pid")
}

pub fn check_running(config: &AppConfig) -> anyhow::Result<Option<u32>> {
    let pidfile = pidfile_path(config);
    if !pidfile.exists() {
        return Ok(None);
    }

    let pid_str = std::fs::read_to_string(&pidfile)?;
    let pid = match pid_str.trim().parse::<u32>() {
        Ok(p) if p > 0 => p,
        _ => {
            let _ = std::fs::remove_file(&pidfile);
            return Ok(None);
        }
    };

    #[cfg(unix)]
    {
        use std::process::Stdio;
        let output = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()?;

        if output.status.success() {
            Ok(Some(pid))
        } else {
            let _ = std::fs::remove_file(&pidfile);
            Ok(None)
        }
    }

    #[cfg(not(unix))]
    {
        match std::process::Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid), "/NH", "/FO", "CSV"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .output()
        {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains(&pid.to_string()) {
                    return Ok(Some(pid));
                }
            }
            _ => {
                // tasklist 不可用，无法验证进程；保守策略：删除 pidfile 并允许启动
            }
        }
        let _ = std::fs::remove_file(&pidfile);
        Ok(None)
    }
}

pub fn write_pidfile(config: &AppConfig) -> anyhow::Result<()> {
    let pidfile = pidfile_path(config);
    if let Some(parent) = pidfile.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let pid = std::process::id();
    std::fs::write(&pidfile, pid.to_string())?;
    Ok(())
}

pub fn remove_pidfile(config: &AppConfig) {
    let _ = std::fs::remove_file(pidfile_path(config));
}

/// 从 SQLite URL 中提取数据库文件所在目录。
///
/// # 注意
/// 标准 3 斜杠绝对路径（如 `sqlite:///var/lib/app.db`）会被错误处理，
/// 丢失前导 `/`，返回 `var/lib`。系统通过生成 4 斜杠 URI
///（`sqlite:////var/lib/app.db`）作为 workaround。
/// 修改此行为会影响配置格式兼容性，因此保留当前实现。
pub fn database_dir_from_url(url: &str) -> Option<PathBuf> {
    let path = url.strip_prefix("sqlite:")?;
    // 处理三斜杠绝对路径格式: sqlite:///C:/path → C:/path
    let path = path.strip_prefix("///").unwrap_or(path);
    let path = path.split(&['?', '#']).next().unwrap_or(path);
    let db_path = PathBuf::from(path);
    db_path.parent().map(PathBuf::from)
}

pub fn secret_needs_generation(value: Option<&str>) -> bool {
    match value {
        Some(secret) => {
            let trimmed = secret.trim();
            trimmed.is_empty() || trimmed == "change-me-in-production"
        }
        None => true,
    }
}

pub fn is_blank(value: &str) -> bool {
    value.trim().is_empty()
}

pub fn is_blank_option(value: Option<&str>) -> bool {
    value.is_none_or(is_blank)
}

pub fn generate_secret() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

pub fn apply_server_data_dir(config: &mut AppConfig, data_dir: &str) {
    let data_path = std::path::PathBuf::from(data_dir);
    let db_path = data_path.join("app.db");
    let path_str = {
        let path = db_path
            .to_str()
            .expect("data dir path should be valid UTF-8");
        #[cfg(windows)]
        {
            path.replace('\\', "/")
        }
        #[cfg(not(windows))]
        {
            path.to_string()
        }
    };
    config.database.url = if db_path.is_absolute() {
        format!("sqlite:///{}", path_str)
    } else {
        format!("sqlite:{}", path_str)
    };
    let file_storage_path = {
        let path = data_path
            .join("files")
            .to_str()
            .expect("data dir path should be valid UTF-8")
            .to_string();
        #[cfg(windows)]
        {
            path.replace('\\', "/")
        }
        #[cfg(not(windows))]
        {
            path
        }
    };
    config.task.file_storage_path = file_storage_path;
}
