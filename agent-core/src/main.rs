//! OpenAaaS Agent Core - 主入口

use agent_core::config::Config;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod cmd;

#[derive(Parser)]
#[command(name = "agent-core")]
#[command(about = "OpenAaaS Agent Core - 调度与执行框架")]
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
    /// 运行调度器（前台）
    Run {
        /// 使用交互模式注册（如果未注册）
        #[arg(long)]
        interactive: bool,
    },
    /// 运行调度器（后台）
    #[command(name = "run-detached")]
    RunDetached,
    /// 停止后台调度器
    Stop,
    /// 查看状态
    Status,
    /// 初始化配置
    Init,
    /// 注册服务
    Register {
        /// 注册令牌
        #[arg(short, long)]
        token: Option<String>,
        /// Agent 名称
        #[arg(short, long)]
        name: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agent_core=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();
    let config_path = cli.config.unwrap_or_else(Config::config_path);

    match cli.command {
        Commands::Run { interactive } => cmd::run::run_foreground(config_path, interactive).await,
        Commands::RunDetached => cmd::detached::run_detached(config_path).await,
        Commands::Stop => cmd::stop::stop(config_path).await,
        Commands::Status => cmd::status::status(config_path).await,
        Commands::Init => cmd::init::init(config_path).await,
        Commands::Register { token, name } => cmd::register::register(config_path, token, name).await,
    }
}
