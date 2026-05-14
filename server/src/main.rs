//! OpenAaaS Server
//!
//! 异步Agent即服务 (Agent-as-a-Service) 服务端 - 一对一服务模型

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use open_aaas_server::config::AppConfig;

mod cli;
mod cmd;
mod bg_tasks;

#[derive(Parser)]
#[command(name = "server")]
#[command(about = "OpenAaaS Server - 异步Agent即服务")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// 配置文件路径；不传时读取当前工作目录下的 config.toml
    #[arg(long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 运行服务器（前台）
    Run,
    /// 运行服务器（后台）
    #[command(name = "run-detached")]
    RunDetached,
    /// 停止后台服务器
    Stop,
    /// 查看服务器状态
    Status,
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
