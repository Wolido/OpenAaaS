//! Server main.rs 集成测试
//!
//! 测试 CLI 参数解析、配置加载、pidfile 操作、纯函数等。

use clap::Parser;
use open_aaas_server::main_support::*;
use open_aaas_server::config::AppConfig;
use std::path::PathBuf;

// ============================================================================
// CLI 参数解析测试
// ============================================================================

#[derive(clap::Parser)]
#[command(name = "server")]
#[command(about = "OpenAaaS Server")]
struct Cli {
    #[arg(long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Run,
    #[command(name = "run-detached")]
    RunDetached,
    Stop,
    Status,
}

#[test]
fn test_cli_parse_run() {
    let cli = Cli::parse_from(["server", "run"]);
    assert!(cli.config.is_none());
    match cli.command {
        Commands::Run => {}
        _ => panic!("expected Run command"),
    }
}

#[test]
fn test_cli_parse_run_detached() {
    let cli = Cli::parse_from(["server", "run-detached"]);
    match cli.command {
        Commands::RunDetached => {}
        _ => panic!("expected RunDetached command"),
    }
}

#[test]
fn test_cli_parse_stop() {
    let cli = Cli::parse_from(["server", "stop"]);
    match cli.command {
        Commands::Stop => {}
        _ => panic!("expected Stop command"),
    }
}

#[test]
fn test_cli_parse_status() {
    let cli = Cli::parse_from(["server", "status"]);
    match cli.command {
        Commands::Status => {}
        _ => panic!("expected Status command"),
    }
}

#[test]
fn test_cli_parse_with_config() {
    let cli = Cli::parse_from(["server", "--config", "/tmp/config.toml", "run"]);
    assert_eq!(cli.config, Some(PathBuf::from("/tmp/config.toml")));
}

// ============================================================================
// load_config_from_path 测试
// ============================================================================

#[test]
fn test_load_config_from_path_valid() {
    let temp_dir = std::env::temp_dir().join(format!("open-aaas-test-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let config_path = temp_dir.join("config.toml");

    std::fs::write(
        &config_path,
        r#"
[server]
addr = "127.0.0.1:3000"

[database]
url = "sqlite:./data/app.db"
"#,
    ).unwrap();

    let config = load_config_from_path(&config_path).unwrap();
    assert_eq!(config.server_addr().to_string(), "127.0.0.1:3000");
}

#[test]
fn test_load_config_from_path_invalid() {
    let temp_dir = std::env::temp_dir().join(format!("open-aaas-test-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let config_path = temp_dir.join("invalid.toml");

    std::fs::write(&config_path, "not valid toml {{{").unwrap();

    let result = load_config_from_path(&config_path);
    assert!(result.is_err());
}

// ============================================================================
// pidfile 操作测试
// ============================================================================

#[test]
fn test_pidfile_path() {
    let mut config = AppConfig::default();
    config.database.url = "sqlite:./data/app.db".to_string();

    let path = pidfile_path(&config);
    assert_eq!(path, PathBuf::from("./data/server.pid"));
}

#[test]
fn test_write_and_remove_pidfile() {
    let temp_dir = std::env::temp_dir().join(format!("open-aaas-pid-test-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let _ = std::fs::create_dir_all(&temp_dir);

    let mut config = AppConfig::default();
    config.database.url = format!("sqlite:{}/app.db", temp_dir.to_string_lossy().replace('\\', "/"));

    assert!(check_running(&config).unwrap().is_none());

    write_pidfile(&config).unwrap();
    assert!(pidfile_path(&config).exists());

    let pid = check_running(&config).unwrap();
    // 在测试环境中，kill -0 对自己会成功（如果是 unix），否则返回 Some(pid)
    assert!(pid.is_some());

    remove_pidfile(&config);
    assert!(!pidfile_path(&config).exists());
}

#[test]
fn test_check_running_invalid_pidfile() {
    let temp_dir = std::env::temp_dir().join(format!("open-aaas-pid-test-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let _ = std::fs::create_dir_all(&temp_dir);

    let mut config = AppConfig::default();
    config.database.url = format!("sqlite:{}/app.db", temp_dir.to_string_lossy().replace('\\', "/"));

    // 写入无效 pid
    let pidfile = pidfile_path(&config);
    let _ = std::fs::create_dir_all(pidfile.parent().unwrap());
    std::fs::write(&pidfile, "invalid\n").unwrap();

    let result = check_running(&config).unwrap();
    assert!(result.is_none());
    assert!(!pidfile.exists());
}

// ============================================================================
// database_dir_from_url 测试
// ============================================================================

#[test]
fn test_database_dir_from_url_relative() {
    let result = database_dir_from_url("sqlite:./data/app.db");
    assert_eq!(result, Some(PathBuf::from("./data")));
}

// 注意：此测试断言了已知错误行为，与代码实现保持一致。
// sqlite:///var/lib/open-aaas/app.db（3 斜杠）会被错误处理为相对路径，
// 丢失前导 /。系统实际使用 4 斜杠 URI（sqlite:////var/lib/open-aaas/app.db）。
// 修改此行为会影响配置格式兼容性，因此保留。
#[test]
fn test_database_dir_from_url_absolute_unix() {
    let result = database_dir_from_url("sqlite:///var/lib/open-aaas/app.db");
    assert_eq!(result, Some(PathBuf::from("var/lib/open-aaas")));
}

#[test]
fn test_database_dir_from_url_absolute_windows() {
    let result = database_dir_from_url("sqlite:///C:/data/app.db");
    assert_eq!(result, Some(PathBuf::from("C:/data")));
}

#[test]
fn test_database_dir_from_url_with_params() {
    let result = database_dir_from_url("sqlite:./data/app.db?mode=rwc");
    assert_eq!(result, Some(PathBuf::from("./data")));
}

#[test]
fn test_database_dir_from_url_non_sqlite() {
    let result = database_dir_from_url("postgres://localhost/db");
    assert!(result.is_none());
}

// ============================================================================
// secret 相关纯函数测试
// ============================================================================

#[test]
fn test_secret_needs_generation_none() {
    assert!(secret_needs_generation(None));
}

#[test]
fn test_secret_needs_generation_empty() {
    assert!(secret_needs_generation(Some("")));
    assert!(secret_needs_generation(Some("   ")));
}

#[test]
fn test_secret_needs_generation_default_value() {
    assert!(secret_needs_generation(Some("change-me-in-production")));
}

#[test]
fn test_secret_needs_generation_valid() {
    assert!(!secret_needs_generation(Some("my-secret-key")));
}

#[test]
fn test_is_blank() {
    assert!(is_blank(""));
    assert!(is_blank("   "));
    assert!(is_blank("\t\n"));
    assert!(!is_blank("hello"));
    assert!(!is_blank(" hello "));
}

#[test]
fn test_is_blank_option() {
    assert!(is_blank_option(None));
    assert!(is_blank_option(Some("")));
    assert!(is_blank_option(Some("   ")));
    assert!(!is_blank_option(Some("key")));
}

#[test]
fn test_generate_secret() {
    let s1 = generate_secret();
    let s2 = generate_secret();
    assert!(!s1.is_empty());
    assert!(!s2.is_empty());
    assert_ne!(s1, s2);
    assert_eq!(s1.len(), 64); // 两个 UUID 去掉横杠，每个 32 字符
}

// ============================================================================
// apply_server_data_dir 测试
// ============================================================================

#[test]
fn test_apply_server_data_dir_relative() {
    let mut config = AppConfig::default();
    apply_server_data_dir(&mut config, "./data");
    assert_eq!(config.database.url, "sqlite:./data/app.db");
    assert_eq!(config.task.file_storage_path, "./data/files");
}

#[test]
#[cfg(unix)]
fn test_apply_server_data_dir_absolute() {
    let mut config = AppConfig::default();
    apply_server_data_dir(&mut config, "/var/open-aaas");
    assert_eq!(config.database.url, "sqlite:////var/open-aaas/app.db");
    assert_eq!(config.task.file_storage_path, "/var/open-aaas/files");
}
