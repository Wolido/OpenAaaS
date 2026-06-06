use tabled::Tabled;

use crate::client::ApiClient;
use crate::config::Config;
use crate::error::Result;
use crate::models::TaskStatus;
use crate::table::build_table;

#[derive(Tabled)]
struct StatsRow {
    #[tabled(rename = "Metric")]
    metric: String,
    #[tabled(rename = "Count")]
    count: usize,
}

pub async fn show(cli_server_url: Option<&str>, cli_api_key: Option<&str>) -> Result<()> {
    let config = Config::effective(cli_server_url, cli_api_key)?;
    let client = ApiClient::new(&config)?;

    let mut all_tasks = Vec::new();
    let mut offset = 0i64;
    loop {
        let tasks = client.list_admin_tasks(100, offset).await?;
        if tasks.is_empty() {
            break;
        }
        all_tasks.extend(tasks);
        offset += 100;
    }

    let total = all_tasks.len();
    let pending = all_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Pending)
        .count();
    let running = all_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Running)
        .count();
    let completed = all_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Completed)
        .count();
    let failed = all_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Failed)
        .count();
    let cancelled = all_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Cancelled)
        .count();
    let cancelling = all_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Cancelling)
        .count();

    let rows = vec![
        StatsRow {
            metric: "Total".to_string(),
            count: total,
        },
        StatsRow {
            metric: "Pending".to_string(),
            count: pending,
        },
        StatsRow {
            metric: "Running".to_string(),
            count: running,
        },
        StatsRow {
            metric: "Completed".to_string(),
            count: completed,
        },
        StatsRow {
            metric: "Failed".to_string(),
            count: failed,
        },
        StatsRow {
            metric: "Cancelled".to_string(),
            count: cancelled,
        },
        StatsRow {
            metric: "Cancelling".to_string(),
            count: cancelling,
        },
    ];

    println!("{}", build_table(&rows));
    Ok(())
}
