use anyhow::Context;
use std::io::{self, IsTerminal, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use open_aaas_server::{config::AppConfig, main_support::*};

pub fn prepare_server_config(config_path: &PathBuf) -> anyhow::Result<AppConfig> {
    let interactive = io::stdin().is_terminal() && io::stdout().is_terminal();
    let is_first_startup = !config_path.exists();
    let mut changed = false;

    let mut config = if config_path.exists() {
        load_config_from_path(config_path).context("加载 config.toml 失败")?
    } else {
        let config = AppConfig::startup_default();
        eprintln!("config.toml 不存在，将在当前目录生成默认配置。");
        changed = true;
        config
    };

    if database_dir_from_url(&config.database.url).is_none()
        || is_blank(&config.task.file_storage_path)
    {
        let data_dir = choose_server_data_dir(interactive, "./data")?;
        apply_server_data_dir(&mut config, &data_dir);
        changed = true;
    }

    if interactive && is_first_startup {
        let default_addr = config.server.addr.to_string();
        let addr = loop {
            let input = prompt_with_default(
                "Server listen address",
                &default_addr,
                "The address and port the server will bind to.",
            )?;
            match input.parse::<SocketAddr>() {
                Ok(addr) => break addr,
                Err(e) => {
                    eprintln!("Invalid address '{}': {}. Please try again.", input, e);
                    continue;
                }
            }
        };
        if addr != config.server.addr {
            config.server.addr = addr;
            changed = true;
        }
    }

    if secret_needs_generation(config.secret_key.as_deref()) {
        config.secret_key = Some(generate_secret());
        changed = true;
    }

    if is_blank_option(config.admin_api_key.as_deref()) {
        let generated = format!("ak_admin_{}", &generate_secret()[..16]);
        let value = if interactive {
            prompt_with_default(
                "Admin API key",
                &generated,
                "用于管理员接口和生成 agent-core 注册 token。直接回车使用随机值。",
            )?
        } else {
            generated
        };
        config.admin_api_key = Some(value);
        changed = true;
    }

    ensure_server_runtime_dirs(&config)?;

    if changed {
        config.save_to_path(config_path)?;
        eprintln!("已更新配置文件: {}", config_path.display());
    }

    Ok(config)
}

pub fn runtime_admin_api_key(config: &AppConfig) -> Option<String> {
    std::env::var("ADMIN_API_KEY")
        .ok()
        .filter(|value| !is_blank(value))
        .or_else(|| {
            config
                .admin_api_key
                .clone()
                .filter(|value| !is_blank(value))
        })
}

fn choose_server_data_dir(interactive: bool, default_dir: &str) -> anyhow::Result<String> {
    if !interactive {
        return Ok(default_dir.to_string());
    }

    eprintln!();
    eprintln!("--- Data Directory ---");
    eprintln!("Purpose: stores the server SQLite database and runtime files.");
    eprintln!("Default: {default_dir}");
    eprintln!();

    if prompt_yes_no("Use the default data directory?", true)? {
        return Ok(default_dir.to_string());
    }

    loop {
        let value = prompt_raw("Enter custom data directory", None)?;
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
        eprintln!("Data directory cannot be empty.");
    }
}

fn ensure_server_runtime_dirs(config: &AppConfig) -> anyhow::Result<()> {
    if let Some(data_dir) = database_dir_from_url(&config.database.url) {
        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("创建数据目录失败: {}", data_dir.display()))?;
    }

    if !is_blank(&config.task.file_storage_path) {
        std::fs::create_dir_all(&config.task.file_storage_path)
            .with_context(|| format!("创建文件目录失败: {}", config.task.file_storage_path))?;
    }

    Ok(())
}

pub(crate) fn prompt_with_default(label: &str, default: &str, help: &str) -> anyhow::Result<String> {
    eprintln!();
    eprintln!("{label}: {help}");
    let value = prompt_raw(label, Some(default))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

pub(crate) fn prompt_yes_no(label: &str, default_yes: bool) -> anyhow::Result<bool> {
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

fn prompt_raw(label: &str, default: Option<&str>) -> anyhow::Result<String> {
    let mut stdout = io::stdout();
    match default {
        Some(default) => write!(stdout, "{label} [{default}]: ")?,
        None => write!(stdout, "{label}: ")?,
    }
    stdout.flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim_end_matches(['\n', '\r']).to_string())
}
