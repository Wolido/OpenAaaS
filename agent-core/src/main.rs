//! OpenAaaS Agent Core - 主入口

use agent_core::config::Config;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod cmd;

const AGENT_LONG_ABOUT: &str = "OpenAaaS Agent Core - Agent 执行节点\n\n连接 OpenAaaS Server，注册本机服务能力，轮询任务，并通过 Docker 执行任务。";
const AGENT_AFTER_HELP: &str = "首次使用:\n  agent-core init\n  编辑 config.toml 中的 server.base_url\n  agent-core register --token <TOKEN> --name my-agent\n  agent-core run\n\n默认值:\n  配置文件: ./config.toml\n  数据目录: ./data\n  Server 地址: http://127.0.0.1:8080\n  Executor 镜像: open-aaas-executor:latest";
const AGENT_INIT_AFTER_HELP: &str = "生成后通常需要:\n  1. 编辑 server.base_url\n  2. 运行 agent-core register --token <TOKEN>\n  3. 运行 agent-core run";
const AGENT_RUN_AFTER_HELP: &str = "启动前请确认:\n  Server 已启动\n  config.toml 中的 server.base_url 正确\n  Agent 已注册，或使用 --interactive 交互注册";
const AGENT_DETACHED_AFTER_HELP: &str =
    "输出位置:\n  pidfile: 数据目录/agent.pid\n  日志: 数据目录/agent.log";
const AGENT_INIT_LONG_ABOUT: &str =
    "生成 Agent Core 默认配置文件；只创建 config.toml，不注册 Agent，也不启动调度器。";
const AGENT_REGISTER_LONG_ABOUT: &str = "向 OpenAaaS Server 注册 Agent；成功后会把 service_id 和 api_key 写入 config.toml，token 需要从 Server 管理端获取。";
const AGENT_RUN_LONG_ABOUT: &str =
    "前台启动 Agent 调度器；启动后连接 Server、发送心跳、轮询任务，并通过 Docker 执行任务。";
const AGENT_DETACHED_LONG_ABOUT: &str =
    "后台启动 Agent 调度器；适合长期运行，日志和 pidfile 默认写入数据目录。";
const AGENT_STATUS_LONG_ABOUT: &str =
    "查看 Agent Core 状态；显示配置文件、数据目录、Server 地址、注册状态和执行器配置。";
const AGENT_STOP_LONG_ABOUT: &str =
    "停止后台运行的 Agent Core；根据 pidfile 查找进程并发送停止信号。";

#[derive(Parser)]
#[command(name = "agent-core")]
#[command(about = "OpenAaaS Agent Core - Agent 调度与执行")]
#[command(long_about = AGENT_LONG_ABOUT)]
#[command(after_help = AGENT_AFTER_HELP)]
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
    /// 前台启动 Agent 调度器
    #[command(long_about = AGENT_RUN_LONG_ABOUT)]
    #[command(after_help = AGENT_RUN_AFTER_HELP)]
    Run {
        /// 未注册时交互输入 token 并完成注册
        #[arg(long)]
        interactive: bool,
    },
    /// 后台启动 Agent 调度器
    #[command(name = "run-detached")]
    #[command(long_about = AGENT_DETACHED_LONG_ABOUT)]
    #[command(after_help = AGENT_DETACHED_AFTER_HELP)]
    RunDetached,
    /// 停止后台 Agent
    #[command(long_about = AGENT_STOP_LONG_ABOUT)]
    Stop,
    /// 查看 Agent 状态
    #[command(long_about = AGENT_STATUS_LONG_ABOUT)]
    Status,
    /// 生成默认配置文件
    #[command(long_about = AGENT_INIT_LONG_ABOUT)]
    #[command(after_help = AGENT_INIT_AFTER_HELP)]
    Init,
    /// 向 Server 注册 Agent
    #[command(long_about = AGENT_REGISTER_LONG_ABOUT)]
    Register {
        /// 注册令牌，例如 rt_xxx
        #[arg(short, long)]
        token: Option<String>,
        /// Agent 名称，默认 agent-core
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
        Commands::Register { token, name } => {
            cmd::register::register(config_path, token, name).await
        }
    }
}
