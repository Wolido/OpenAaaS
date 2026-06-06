use crate::client::ApiClient;
use crate::config::Config;
use crate::error::Result;
use colored::Colorize;

pub async fn check(cli_server_url: Option<&str>, cli_api_key: Option<&str>) -> Result<()> {
    let config = Config::effective(cli_server_url, cli_api_key)?;
    let server_url = config.require_server_url()?;

    // 1. Check public health endpoint
    println!("Checking server health at {} ...", server_url);
    let client = ApiClient::new_without_auth(&config)?;
    match client.health_check().await {
        Ok(health) => {
            println!("  {} Health: {}", "●".green(), health.status.green().bold());
            println!("  {} Version: {}", "●".green(), health.version);
            println!("  {} Timestamp: {}", "●".green(), health.timestamp);
        }
        Err(e) => {
            println!(
                "  {} Health check failed: {}",
                "●".red(),
                e.to_string().red()
            );
            return Err(e);
        }
    }

    // 2. Check admin privileges
    println!("\nChecking admin API key ...");
    let auth_client = ApiClient::new(&config)?;
    match auth_client.check_admin().await {
        Ok(()) => {
            println!("  {} Admin API key is valid", "●".green());
        }
        Err(e) => {
            println!(
                "  {} Admin check failed: {}",
                "●".red(),
                e.to_string().red()
            );
            return Err(e);
        }
    }

    println!("\n{}", "✓ All checks passed".green().bold());
    Ok(())
}
