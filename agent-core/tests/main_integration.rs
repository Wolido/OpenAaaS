//! Agent Core main.rs 集成测试
//!
//! 测试 CLI 参数解析、pidfile 操作、URL 处理、配置初始化等。

use agent_core::config::Config;
use agent_core::main_support::*;
use clap::Parser;
use std::path::PathBuf;

// ============================================================================
// CLI 参数解析测试
// ============================================================================

#[derive(clap::Parser)]
#[command(name = "agent-core")]
#[command(about = "OpenAaaS Agent Core")]
struct Cli {
    #[arg(long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Run {
        #[arg(long)]
        interactive: bool,
    },
    #[command(name = "run-detached")]
    RunDetached,
    Stop,
    Status,
    Init,
    Register {
        #[arg(short, long)]
        token: Option<String>,
        #[arg(short, long)]
        name: Option<String>,
    },
}

#[test]
fn test_cli_parse_run() {
    let cli = Cli::parse_from(["agent-core", "run"]);
    assert!(cli.config.is_none());
    match cli.command {
        Commands::Run { interactive } => assert!(!interactive),
        _ => panic!("expected Run command"),
    }
}

#[test]
fn test_cli_parse_run_interactive() {
    let cli = Cli::parse_from(["agent-core", "run", "--interactive"]);
    match cli.command {
        Commands::Run { interactive } => assert!(interactive),
        _ => panic!("expected Run command with interactive"),
    }
}

#[test]
fn test_cli_parse_run_detached() {
    let cli = Cli::parse_from(["agent-core", "run-detached"]);
    match cli.command {
        Commands::RunDetached => {}
        _ => panic!("expected RunDetached command"),
    }
}

#[test]
fn test_cli_parse_stop() {
    let cli = Cli::parse_from(["agent-core", "stop"]);
    match cli.command {
        Commands::Stop => {}
        _ => panic!("expected Stop command"),
    }
}

#[test]
fn test_cli_parse_status() {
    let cli = Cli::parse_from(["agent-core", "status"]);
    match cli.command {
        Commands::Status => {}
        _ => panic!("expected Status command"),
    }
}

#[test]
fn test_cli_parse_init() {
    let cli = Cli::parse_from(["agent-core", "init"]);
    match cli.command {
        Commands::Init => {}
        _ => panic!("expected Init command"),
    }
}

#[test]
fn test_cli_parse_register() {
    let cli = Cli::parse_from([
        "agent-core",
        "register",
        "--token",
        "rt_abc123",
        "--name",
        "my-agent",
    ]);
    match cli.command {
        Commands::Register { token, name } => {
            assert_eq!(token, Some("rt_abc123".to_string()));
            assert_eq!(name, Some("my-agent".to_string()));
        }
        _ => panic!("expected Register command"),
    }
}

#[test]
fn test_cli_parse_with_config() {
    let cli = Cli::parse_from(["agent-core", "--config", "/tmp/config.toml", "run"]);
    assert_eq!(cli.config, Some(PathBuf::from("/tmp/config.toml")));
}

// ============================================================================
// pidfile 操作测试
// ============================================================================

#[test]
fn test_pidfile_path() {
    let config = Config::default();
    let path = pidfile_path(&config);
    assert!(path.to_string_lossy().ends_with("agent.pid"));
}

#[test]
fn test_write_and_remove_pidfile() {
    let temp_dir = std::env::temp_dir().join(format!(
        "agent-core-pid-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::create_dir_all(&temp_dir);

    let mut config = Config::default();
    config.paths.data_dir = Some(temp_dir.clone());

    assert!(check_running(&config).unwrap().is_none());

    write_pidfile(&config).unwrap();
    assert!(pidfile_path(&config).exists());

    let pid = check_running(&config).unwrap();
    assert!(pid.is_some());

    remove_pidfile(&config);
    assert!(!pidfile_path(&config).exists());
}

#[test]
fn test_check_running_invalid_pidfile() {
    let temp_dir = std::env::temp_dir().join(format!(
        "agent-core-pid-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::create_dir_all(&temp_dir);

    let mut config = Config::default();
    config.paths.data_dir = Some(temp_dir.clone());

    let pidfile = pidfile_path(&config);
    std::fs::write(&pidfile, "invalid\n").unwrap();

    // check_running 对无效 pid 会删除 pidfile 并返回 Ok(None)
    let result = check_running(&config);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
    assert!(!pidfile.exists());
}

// ============================================================================
// is_interactive_terminal 测试
// ============================================================================

#[test]
fn test_is_interactive_terminal_returns_bool() {
    let result = is_interactive_terminal();
    // 测试结果取决于运行环境是否有 TTY，但应始终返回 bool
    let _: bool = result;
}

// ============================================================================
// normalize_server_url / validate_server_url 测试
// ============================================================================

#[test]
fn test_normalize_server_url_with_http() {
    assert_eq!(
        normalize_server_url("http://example.com"),
        "http://example.com"
    );
}

#[test]
fn test_normalize_server_url_with_https() {
    assert_eq!(
        normalize_server_url("https://example.com"),
        "https://example.com"
    );
}

#[test]
fn test_normalize_server_url_without_scheme() {
    assert_eq!(normalize_server_url("example.com"), "https://example.com");
}

#[test]
fn test_normalize_server_url_trims_trailing_slash() {
    assert_eq!(
        normalize_server_url("https://example.com/"),
        "https://example.com"
    );
    assert_eq!(
        normalize_server_url("https://example.com///"),
        "https://example.com"
    );
}

#[test]
fn test_normalize_server_url_trims_whitespace() {
    assert_eq!(
        normalize_server_url("  https://example.com  "),
        "https://example.com"
    );
}

#[test]
fn test_validate_server_url_valid_http() {
    assert!(validate_server_url("http://127.0.0.1:8080"));
}

#[test]
fn test_validate_server_url_valid_https() {
    assert!(validate_server_url("https://www.open-aaas.com"));
}

#[test]
fn test_validate_server_url_invalid() {
    assert!(!validate_server_url("not-a-url"));
    assert!(!validate_server_url(""));
}

#[test]
fn test_validate_server_url_ftp() {
    // validate_server_url 只检查是否有 host，不限制 scheme
    assert!(validate_server_url("ftp://example.com"));
}

// ============================================================================
// ensure_agent_runtime_config 测试
// ============================================================================

#[tokio::test]
async fn test_ensure_agent_runtime_config_existing_valid() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let original = Config::default();
    original.save_to_path(&config_path).await.unwrap();

    let config = ensure_agent_runtime_config(&config_path).await.unwrap();
    assert!(!config.server.base_url.is_empty());
    assert!(config.paths.data_dir.is_some());
    assert_eq!(config.agent.name, Some("agent-core".to_string()));
}

#[tokio::test]
async fn test_ensure_agent_runtime_config_normalizes_url() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let mut original = Config::default();
    original.server.base_url = "example.com/".to_string();
    original.save_to_path(&config_path).await.unwrap();

    let config = ensure_agent_runtime_config(&config_path).await.unwrap();
    assert_eq!(config.server.base_url, "https://example.com");
}

#[tokio::test]
async fn test_ensure_agent_runtime_config_missing_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // 确保文件不存在
    assert!(!config_path.exists());

    let config = ensure_agent_runtime_config(&config_path).await.unwrap();
    assert_eq!(config.server.base_url, "http://127.0.0.1:8080");
    assert!(config.paths.data_dir.is_some());
    assert_eq!(config.agent.name, Some("agent-core".to_string()));
    // 缺失文件时，Config::load_from_path 会创建默认配置并保存
    assert!(config_path.exists());
}
