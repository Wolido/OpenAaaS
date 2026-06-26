use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "openaaas-admin")]
#[command(about = "OpenAaaS Server Admin CLI")]
#[command(version)]
#[command(
    after_help = "First time? Run 'openaaas-admin config init' to set up your server URL and API key."
)]
pub struct Cli {
    /// Override server URL
    #[arg(long, global = true)]
    pub server_url: Option<String>,

    /// Override admin API key
    #[arg(long, global = true)]
    pub api_key: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage configuration (run 'config init' to initialize)
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Check server health and admin privileges
    Health,
    /// Manage services
    Services {
        #[command(subcommand)]
        command: ServicesCommands,
    },
    /// Manage users
    Users {
        #[command(subcommand)]
        command: UsersCommands,
    },
    /// Manage permissions
    Permissions {
        #[command(subcommand)]
        command: PermissionsCommands,
    },
    /// Show task statistics
    Stats,
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Initialize configuration (set server URL and API key interactively or via flags)
    Init {
        /// Server URL
        #[arg(long)]
        server_url: Option<String>,
        /// Admin API key
        #[arg(long)]
        api_key: Option<String>,
    },
    /// Show current configuration
    Show,
}

#[derive(Subcommand)]
pub enum ServicesCommands {
    /// List all services
    List,
    /// Show service details
    Show {
        /// Service ID
        id: String,
    },
    /// Create a new service
    Create {
        /// Service name
        #[arg(long)]
        name: String,
        /// Service description
        #[arg(long)]
        description: String,
        /// Service usage instructions
        #[arg(long)]
        usage: String,
        /// Make service public (default is restricted)
        #[arg(long)]
        public: bool,
    },
    /// Update a service
    Update {
        /// Service ID
        id: String,
        /// New name
        #[arg(long)]
        name: Option<String>,
        /// New description
        #[arg(long)]
        description: Option<String>,
        /// New usage instructions
        #[arg(long)]
        usage: Option<String>,
        /// Make service public
        #[arg(long)]
        public: bool,
        /// Make service restricted
        #[arg(long)]
        restricted: bool,
    },
    /// Delete a service
    Delete {
        /// Service ID
        id: String,
        /// Force delete even if tasks exist
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum UsersCommands {
    /// List all users
    List,
    /// Delete a user
    Delete {
        /// User ID
        id: String,
    },
}

#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct ListPermissionsArgs {
    /// User ID
    #[arg(long)]
    pub user: Option<String>,
    /// Service ID
    #[arg(long)]
    pub service: Option<String>,
}

#[derive(Subcommand)]
pub enum PermissionsCommands {
    /// List user's service permissions or service's authorized users
    List(ListPermissionsArgs),
    /// Grant permission to a restricted service
    Grant {
        /// User ID
        #[arg(long)]
        user: String,
        /// Service ID
        #[arg(long)]
        service: String,
    },
    /// Revoke permission from a restricted service
    Revoke {
        /// User ID
        #[arg(long)]
        user: String,
        /// Service ID
        #[arg(long)]
        service: String,
    },
}
