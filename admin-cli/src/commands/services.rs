use std::io::Write;

use tabled::Tabled;

use crate::client::ApiClient;
use crate::config::Config;
use crate::error::Result;
use crate::models::{CreateServiceRequest, UpdateServiceRequest};
use crate::table::{build_table, print_error, print_highlight, print_success, print_warning};

#[derive(Tabled)]
struct ServiceListRow {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Agent Status")]
    agent_status: String,
    #[tabled(rename = "Registration")]
    registration_status: String,
    #[tabled(rename = "Access")]
    access_type: String,
}

#[derive(Tabled)]
struct ServiceDetailRow {
    #[tabled(rename = "Field")]
    field: String,
    #[tabled(rename = "Value")]
    value: String,
}

pub async fn list(cli_server_url: Option<&str>, cli_api_key: Option<&str>) -> Result<()> {
    let config = Config::effective(cli_server_url, cli_api_key)?;
    let client = ApiClient::new(&config)?;
    let services = client.list_services().await?;

    let rows: Vec<ServiceListRow> = services
        .into_iter()
        .map(|s| ServiceListRow {
            id: s.id,
            name: s.name,
            agent_status: s.agent_status,
            registration_status: s.registration_status,
            access_type: s.access_type,
        })
        .collect();

    println!("{}", build_table(&rows));
    Ok(())
}

pub async fn show(id: &str, cli_server_url: Option<&str>, cli_api_key: Option<&str>) -> Result<()> {
    let config = Config::effective(cli_server_url, cli_api_key)?;
    let client = ApiClient::new(&config)?;

    let service = client.get_service(id).await?;
    let usage = client.get_service_usage(id).await;

    let mut rows = vec![
        ServiceDetailRow {
            field: "ID".to_string(),
            value: service.id,
        },
        ServiceDetailRow {
            field: "Name".to_string(),
            value: service.name,
        },
        ServiceDetailRow {
            field: "Description".to_string(),
            value: service.description,
        },
        ServiceDetailRow {
            field: "Agent Status".to_string(),
            value: service.agent_status,
        },
        ServiceDetailRow {
            field: "Registration".to_string(),
            value: service.registration_status,
        },
        ServiceDetailRow {
            field: "Capacity".to_string(),
            value: format!("{}/{}", service.agent_current_load, service.agent_capacity),
        },
        ServiceDetailRow {
            field: "Public".to_string(),
            value: if service.is_public {
                "Yes".to_string()
            } else {
                "No".to_string()
            },
        },
        ServiceDetailRow {
            field: "Created At".to_string(),
            value: service.created_at.to_rfc3339(),
        },
    ];

    if let Ok(u) = usage {
        rows.push(ServiceDetailRow {
            field: "Usage".to_string(),
            value: u.usage,
        });
    }

    println!("{}", build_table(&rows));
    Ok(())
}

pub async fn create(
    name: String,
    description: String,
    usage: String,
    public: bool,
    cli_server_url: Option<&str>,
    cli_api_key: Option<&str>,
) -> Result<()> {
    let config = Config::effective(cli_server_url, cli_api_key)?;
    let client = ApiClient::new(&config)?;

    let req = CreateServiceRequest {
        name,
        description,
        usage,
        is_public: public,
    };

    let resp = client.create_service(&req).await?;

    let rows = vec![
        ServiceDetailRow {
            field: "ID".to_string(),
            value: resp.id.clone(),
        },
        ServiceDetailRow {
            field: "Name".to_string(),
            value: resp.name,
        },
        ServiceDetailRow {
            field: "Registration Status".to_string(),
            value: resp.registration_status,
        },
        ServiceDetailRow {
            field: "Created At".to_string(),
            value: resp.created_at.to_rfc3339(),
        },
    ];
    println!("{}", build_table(&rows));
    println!();
    print_highlight(
        "Registration Token (save this for agent-core)",
        &resp.registration_token,
    );

    Ok(())
}

pub async fn update(
    id: &str,
    name: Option<String>,
    description: Option<String>,
    usage: Option<String>,
    access: Option<bool>,
    cli_server_url: Option<&str>,
    cli_api_key: Option<&str>,
) -> Result<()> {
    let config = Config::effective(cli_server_url, cli_api_key)?;
    let client = ApiClient::new(&config)?;

    let req = UpdateServiceRequest {
        name,
        description,
        usage,
        is_public: access,
    };

    if req.name.is_none()
        && req.description.is_none()
        && req.usage.is_none()
        && req.is_public.is_none()
    {
        print_warning("No fields to update");
        return Ok(());
    }

    let _resp = client.update_service(id, &req).await?;
    print_success(&format!("Service {} updated successfully", id));
    Ok(())
}

pub async fn delete(
    id: &str,
    force: bool,
    cli_server_url: Option<&str>,
    cli_api_key: Option<&str>,
) -> Result<()> {
    let config = Config::effective(cli_server_url, cli_api_key)?;
    let client = ApiClient::new(&config)?;

    if force {
        // Secondary confirmation for force delete
        print_warning(&format!(
            "You are about to FORCE delete service {}. This will cancel all active tasks.",
            id
        ));
        print!("Type the service ID to confirm [{}]: ", id);
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed != id {
            print_error("Confirmation failed. Service ID mismatch. Aborting.");
            return Err(crate::error::AppError::Cancelled);
        }
    } else {
        // Prompt y/N for normal delete
        print!("Are you sure you want to delete service {}? [y/N]: ", id);
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim().to_lowercase();
        if trimmed != "y" && trimmed != "yes" {
            print_warning("Deletion cancelled");
            return Ok(());
        }
    }

    let resp = client.delete_service(id, force).await?;

    if resp.deleted {
        if force {
            print_success(&format!(
                "Service {} force deleted. {} tasks cancelled, {} tasks retained.",
                id, resp.tasks_cancelled, resp.tasks_retained
            ));
        } else {
            print_success(&format!("Service {} deleted successfully", id));
        }
    }

    Ok(())
}
