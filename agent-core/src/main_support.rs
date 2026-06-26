//! Agent Core 主入口支持函数
//!
//! 从 main.rs 提取的可测试纯函数和工具函数。

use crate::config::Config;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

pub fn pidfile_path(config: &Config) -> PathBuf {
    config.data_dir().join("agent.pid")
}

pub fn check_running(config: &Config) -> anyhow::Result<Option<u32>> {
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
            return Ok(Some(pid));
        }
        let _ = std::fs::remove_file(&pidfile);
        Ok(None)
    }

    #[cfg(not(unix))]
    {
        match std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH", "/FO", "CSV"])
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

pub fn write_pidfile(config: &Config) -> anyhow::Result<()> {
    let pidfile = pidfile_path(config);
    if let Some(parent) = pidfile.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let pid = std::process::id();
    std::fs::write(&pidfile, pid.to_string())?;
    Ok(())
}

pub fn remove_pidfile(config: &Config) {
    let _ = std::fs::remove_file(pidfile_path(config));
}

pub fn is_interactive_terminal() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

pub fn normalize_server_url(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}

pub fn validate_server_url(value: &str) -> bool {
    match reqwest::Url::parse(value) {
        Ok(url) => url.host_str().is_some(),
        Err(_) => false,
    }
}

pub async fn ensure_agent_runtime_config(config_path: &Path) -> anyhow::Result<Config> {
    let config_exists = config_path.exists();
    let mut config = Config::load_from_path(config_path).await?;
    let interactive = is_interactive_terminal();
    let mut changed = false;
    let should_confirm_server_url = interactive && !config_exists;

    if config.server.base_url.trim().is_empty() {
        config.server.base_url = "http://127.0.0.1:8080".to_string();
        changed = true;
    } else {
        let normalized = normalize_server_url(&config.server.base_url);
        if normalized != config.server.base_url {
            config.server.base_url = normalized;
            changed = true;
        }
    }

    if should_confirm_server_url || config.server.base_url.trim().is_empty() {
        let selected = prompt_server_url(&config.server.base_url)?;
        if selected != config.server.base_url {
            config.server.base_url = selected;
            changed = true;
        }
    }

    if config.paths.data_dir.is_none() {
        config.paths.data_dir = Some(if interactive {
            choose_agent_data_dir("./data")?
        } else {
            PathBuf::from("./data")
        });
        changed = true;
    }

    if config
        .agent
        .name
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        config.agent.name = Some("agent-core".to_string());
        changed = true;
    }

    let is_first_startup = interactive && !config_exists;

    if is_first_startup {
        let image = prompt_executor_image(&config.executor.image)?;
        if image != config.executor.image {
            config.executor.image = image;
            changed = true;
        }

        let capacity = prompt_executor_capacity(config.executor.capacity)?;
        if capacity != config.executor.capacity {
            config.executor.capacity = capacity;
            changed = true;
        }
    }

    if changed {
        config.save_to_path(config_path).await?;
    }

    Ok(config)
}

fn prompt_server_url(default: &str) -> anyhow::Result<String> {
    println!();
    println!("--- Server URL ---");
    println!(
        "Purpose: the HTTP address of the OpenAaaS server that this agent-core will connect to."
    );
    println!("Ask the server administrator for this value.");
    println!("Default: {default}");
    println!();

    loop {
        let value = prompt_raw("Server URL", Some(default))?;
        let raw = if value.trim().is_empty() {
            default
        } else {
            value.trim()
        };
        let normalized = normalize_server_url(raw);
        if validate_server_url(&normalized) {
            return Ok(normalized);
        }
        eprintln!(
            "Invalid Server URL. Use a full URL such as https://www.open-aaas.com or http://127.0.0.1:8080."
        );
    }
}

fn choose_agent_data_dir(default_dir: &str) -> anyhow::Result<PathBuf> {
    println!();
    println!("--- Data Directory ---");
    println!("Purpose: stores agent-core local state, pid/log runtime files, and task workspaces.");
    println!("Default: {default_dir}");
    println!();

    if prompt_yes_no("Use the default data directory?", true)? {
        return Ok(PathBuf::from(default_dir));
    }

    loop {
        let value = prompt_raw("Enter custom data directory", None)?;
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
        eprintln!("Data directory cannot be empty.");
    }
}

fn prompt_yes_no(label: &str, default_yes: bool) -> anyhow::Result<bool> {
    let default = if default_yes { "Y" } else { "N" };
    loop {
        let value = prompt_raw(label, Some(default))?;
        let normalized = if value.trim().is_empty() {
            default.to_string()
        } else {
            value.trim().to_string()
        };

        match normalized.as_str() {
            "Y" | "y" | "yes" | "YES" => return Ok(true),
            "N" | "n" | "no" | "NO" => return Ok(false),
            _ => eprintln!("Please enter Y or N."),
        }
    }
}

pub fn prompt_raw(label: &str, default: Option<&str>) -> anyhow::Result<String> {
    let mut stdout = std::io::stdout();
    match default {
        Some(default) => write!(stdout, "{label} [{default}]: ")?,
        None => write!(stdout, "{label}: ")?,
    }
    stdout.flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim_end_matches(['\n', '\r']).to_string())
}

fn prompt_executor_image(default: &str) -> anyhow::Result<String> {
    println!();
    println!("--- Executor Image ---");
    println!("Purpose: the Docker image used by the executor to run tasks.");
    println!("Default: {default}");
    println!();

    let value = prompt_raw("Executor image", Some(default))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn prompt_executor_capacity(default: usize) -> anyhow::Result<usize> {
    println!();
    println!("--- Executor Capacity ---");
    println!("Purpose: the maximum number of concurrent tasks the executor can run.");
    println!("Default: {default}");
    println!();

    loop {
        let input = prompt_raw("Executor capacity", Some(&default.to_string()))?;
        if input.trim().is_empty() {
            return Ok(default);
        }
        match input.trim().parse::<usize>() {
            Ok(0) => {
                eprintln!("Capacity must be greater than 0. Please try again.");
                continue;
            }
            Ok(n) => return Ok(n),
            Err(e) => {
                eprintln!("Invalid number '{}': {}. Please try again.", input, e);
                continue;
            }
        }
    }
}
