use tabled::Tabled;

use crate::client::ApiClient;
use crate::config::Config;
use crate::error::{AppError, Result};
use crate::table::{build_table, print_success, print_warning};

#[derive(Tabled)]
struct PermissionRow {
    #[tabled(rename = "Service ID")]
    service_id: String,
    #[tabled(rename = "Service Name")]
    service_name: String,
    #[tabled(rename = "Granted At")]
    granted_at: String,
}

#[derive(Tabled)]
struct ServiceUserRow {
    #[tabled(rename = "User ID")]
    user_id: String,
    #[tabled(rename = "User Name")]
    user_name: String,
    #[tabled(rename = "Role")]
    role: String,
    #[tabled(rename = "Granted At")]
    granted_at: String,
}

pub async fn list(
    user_id: Option<&str>,
    service_id: Option<&str>,
    cli_server_url: Option<&str>,
    cli_api_key: Option<&str>,
) -> Result<()> {
    let config = Config::effective(cli_server_url, cli_api_key)?;
    let client = ApiClient::new(&config)?;

    match (user_id, service_id) {
        (Some(user_id), _) => {
            let permissions = client.list_user_permissions(user_id).await?;

            let rows: Vec<PermissionRow> = permissions
                .into_iter()
                .map(|p| PermissionRow {
                    service_id: p.service_id,
                    service_name: p.service_name,
                    granted_at: p.granted_at.to_rfc3339(),
                })
                .collect();

            println!("{}", build_table(&rows));
        }
        (None, Some(service_id)) => {
            let result = client.list_service_users(service_id).await?;

            if result.is_public {
                print_warning(&format!(
                    "Service '{}' is public. All users have access by default.",
                    service_id
                ));
                if result.users.is_empty() {
                    println!("No explicit permission records (all users have access by default).");
                }
            } else if result.users.is_empty() {
                println!("No explicit permissions granted.");
            }

            if !result.users.is_empty() {
                let rows: Vec<ServiceUserRow> = result
                    .users
                    .into_iter()
                    .map(|u| ServiceUserRow {
                        user_id: u.user_id,
                        user_name: u.user_name.as_deref().unwrap_or("-").to_string(),
                        role: u.role,
                        granted_at: u.granted_at.to_rfc3339(),
                    })
                    .collect();

                println!("{}", build_table(&rows));
            }
        }
        (None, None) => {
            return Err(AppError::Other(
                "Must specify either --user or --service".to_string(),
            ));
        }
    }

    Ok(())
}

pub async fn grant(
    user_id: &str,
    service_id: &str,
    cli_server_url: Option<&str>,
    cli_api_key: Option<&str>,
) -> Result<()> {
    let config = Config::effective(cli_server_url, cli_api_key)?;
    let client = ApiClient::new(&config)?;
    client.grant_permission(service_id, user_id).await?;
    print_success(&format!(
        "Granted user {} permission to service {}",
        user_id, service_id
    ));
    Ok(())
}

pub async fn revoke(
    user_id: &str,
    service_id: &str,
    cli_server_url: Option<&str>,
    cli_api_key: Option<&str>,
) -> Result<()> {
    let config = Config::effective(cli_server_url, cli_api_key)?;
    let client = ApiClient::new(&config)?;
    client.revoke_permission(service_id, user_id).await?;
    print_success(&format!(
        "Revoked user {} permission to service {}",
        user_id, service_id
    ));
    Ok(())
}
