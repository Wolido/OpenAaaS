//! OpenAaaS Server
//!
//! 异步Agent即服务 (Agent-as-a-Service) 服务端 - 一对一服务模型

use clap::{Parser, Subcommand};
use open_aaas_server::config::AppConfig;
use std::path::PathBuf;

mod bg_tasks;
mod cli;
mod cmd;

const SERVER_LONG_ABOUT: &str = "OpenAaaS Server - 异步 Agent 即服务的调度中心\n\n负责接收 Client 任务、管理 Agent 注册与心跳、分发任务并中转结果文件。";
const SERVER_AFTER_HELP: &str = "常用流程:\n  open-aaas-server run\n  open-aaas-server status\n  open-aaas-server stop\n\n首次运行:\n  如果当前目录没有 config.toml，run 会自动生成配置、SQLite 数据库和数据目录。\n  默认数据目录: ./data\n  默认监听地址: 0.0.0.0:8080";
const SERVER_RUN_AFTER_HELP: &str = "默认行为:\n  配置文件: ./config.toml\n  数据目录: ./data\n  数据库: ./data/app.db\n  文件目录: ./data/files\n  监听地址: 0.0.0.0:8080";
const SERVER_DETACHED_AFTER_HELP: &str =
    "输出位置:\n  pidfile: 数据目录/server.pid\n  日志: 数据目录/server.log";
const SERVER_RUN_LONG_ABOUT: &str = "前台启动 OpenAaaS Server；首次运行会自动创建默认配置、数据目录、SQLite 数据库、文件目录、secret_key 和 admin_api_key。";
const SERVER_DETACHED_LONG_ABOUT: &str =
    "后台启动 OpenAaaS Server；适合长期运行，日志和 pidfile 默认写入数据目录。";
const SERVER_STATUS_LONG_ABOUT: &str = "查看 OpenAaaS Server 运行状态；会读取配置文件和 pidfile，显示配置路径、数据目录、监听地址和运行状态。";
const SERVER_STOP_LONG_ABOUT: &str =
    "停止后台运行的 OpenAaaS Server；根据 pidfile 查找进程并发送停止信号。";

#[derive(Parser)]
#[command(name = "server")]
#[command(about = "OpenAaaS Server - 异步 Agent 即服务中心")]
#[command(long_about = SERVER_LONG_ABOUT)]
#[command(after_help = SERVER_AFTER_HELP)]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// 配置文件路径，默认使用当前目录的 config.toml
    #[arg(long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 前台启动 Server
    #[command(long_about = SERVER_RUN_LONG_ABOUT)]
    #[command(after_help = SERVER_RUN_AFTER_HELP)]
    Run,
    /// 后台启动 Server
    #[command(name = "run-detached")]
    #[command(long_about = SERVER_DETACHED_LONG_ABOUT)]
    #[command(after_help = SERVER_DETACHED_AFTER_HELP)]
    RunDetached,
    /// 查看 Server 运行状态
    #[command(long_about = SERVER_STATUS_LONG_ABOUT)]
    Status,
    /// 停止后台 Server
    #[command(long_about = SERVER_STOP_LONG_ABOUT)]
    Stop,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config_path = cli.config.unwrap_or_else(AppConfig::config_path);
    match cli.command {
        Commands::Run => cmd::run::run_foreground(config_path).await,
        Commands::RunDetached => cmd::detached::run_detached(config_path).await,
        Commands::Stop => cmd::stop::stop(config_path).await,
        Commands::Status => cmd::status::status(config_path).await,
    }
}
