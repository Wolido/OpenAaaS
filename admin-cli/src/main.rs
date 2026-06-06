use clap::Parser;

mod cli;
mod client;
mod commands;
mod config;
mod error;
mod models;
mod table;

use cli::{Cli, Commands, ConfigCommands, PermissionsCommands, ServicesCommands, UsersCommands};
use error::{AppError, Result};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        table::print_error(&e.to_string());
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    let server_url = cli.server_url.as_deref();
    let api_key = cli.api_key.as_deref();

    match cli.command {
        Commands::Config { command } => match command {
            ConfigCommands::Init {
                server_url,
                api_key,
            } => commands::config::init(server_url, api_key).await,
            ConfigCommands::Show => commands::config::show(),
        },
        Commands::Health => commands::health::check(server_url, api_key).await,
        Commands::Services { command } => match command {
            ServicesCommands::List => commands::services::list(server_url, api_key).await,
            ServicesCommands::Show { id } => {
                commands::services::show(&id, server_url, api_key).await
            }
            ServicesCommands::Create {
                name,
                description,
                usage,
                public,
            } => {
                commands::services::create(name, description, usage, public, server_url, api_key)
                    .await
            }
            ServicesCommands::Update {
                id,
                name,
                description,
                usage,
                public,
                restricted,
            } => {
                if public && restricted {
                    return Err(AppError::Other(
                        "Cannot specify both --public and --restricted".to_string(),
                    ));
                }
                let access = if public {
                    Some(true)
                } else if restricted {
                    Some(false)
                } else {
                    None
                };
                commands::services::update(
                    &id,
                    name,
                    description,
                    usage,
                    access,
                    server_url,
                    api_key,
                )
                .await
            }
            ServicesCommands::Delete { id, force } => {
                commands::services::delete(&id, force, server_url, api_key).await
            }
        },
        Commands::Users { command } => match command {
            UsersCommands::List => commands::users::list(server_url, api_key).await,
            UsersCommands::Delete { id } => commands::users::delete(&id, server_url, api_key).await,
        },
        Commands::Permissions { command } => match command {
            PermissionsCommands::List(args) => {
                commands::permissions::list(
                    args.user.as_deref(),
                    args.service.as_deref(),
                    server_url,
                    api_key,
                )
                .await
            }
            PermissionsCommands::Grant { user, service } => {
                commands::permissions::grant(&user, &service, server_url, api_key).await
            }
            PermissionsCommands::Revoke { user, service } => {
                commands::permissions::revoke(&user, &service, server_url, api_key).await
            }
        },
        Commands::Stats => commands::stats::show(server_url, api_key).await,
    }
}
