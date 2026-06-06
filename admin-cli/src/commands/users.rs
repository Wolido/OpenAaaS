use std::io::Write;

use tabled::Tabled;

use crate::client::ApiClient;
use crate::config::Config;
use crate::error::Result;
use crate::table::{build_table, print_success, print_warning};

#[derive(Tabled)]
struct UserListRow {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Role")]
    role: String,
    #[tabled(rename = "API Key")]
    api_key: String,
    #[tabled(rename = "Created At")]
    created_at: String,
}

pub async fn list(cli_server_url: Option<&str>, cli_api_key: Option<&str>) -> Result<()> {
    let config = Config::effective(cli_server_url, cli_api_key)?;
    let client = ApiClient::new(&config)?;
    let users = client.list_users().await?;

    let rows: Vec<UserListRow> = users
        .into_iter()
        .map(|u| UserListRow {
            id: u.id,
            name: u.name,
            role: u.role,
            api_key: mask_api_key(&u.api_key),
            created_at: u.created_at,
        })
        .collect();

    println!("{}", build_table(&rows));
    Ok(())
}

pub async fn delete(
    id: &str,
    cli_server_url: Option<&str>,
    cli_api_key: Option<&str>,
) -> Result<()> {
    let config = Config::effective(cli_server_url, cli_api_key)?;
    let client = ApiClient::new(&config)?;

    print!("Are you sure you want to delete user {}? [y/N]: ", id);
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let trimmed = input.trim().to_lowercase();
    if trimmed != "y" && trimmed != "yes" {
        print_warning("Cancelled.");
        return Ok(());
    }

    client.delete_user(id).await?;
    print_success(&format!("User {} deleted successfully", id));
    Ok(())
}

fn mask_api_key(key: &str) -> String {
    if key.len() > 8 {
        format!("{}***", &key[..8])
    } else {
        key.to_string()
    }
}
