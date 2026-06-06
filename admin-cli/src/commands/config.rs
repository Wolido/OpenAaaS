use crate::config::Config;
use crate::error::Result;
use crate::table::{build_table_from_rows, print_error, print_success};

pub async fn init(server_url: Option<String>, api_key: Option<String>) -> Result<()> {
    let mut config = Config::load().unwrap_or_default();

    // Interactive or param-based init
    let url = if let Some(u) = server_url {
        u
    } else {
        loop {
            println!("Enter server URL [http://localhost:8080]: ");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let trimmed = input.trim();
            let url = if trimmed.is_empty() {
                "http://localhost:8080".to_string()
            } else {
                trimmed.to_string()
            };
            if url.starts_with("http://") || url.starts_with("https://") {
                break url;
            }
            print_error("Invalid URL. Must start with http:// or https://");
        }
    };

    let key = if let Some(k) = api_key {
        k
    } else {
        rpassword::prompt_password("Enter admin API key: ")?
    };

    if key.is_empty() {
        return Err(crate::error::AppError::Config(
            "API key cannot be empty".to_string(),
        ));
    }

    config.server_url = Some(url);
    config.api_key = Some(key);
    config.save()?;
    print_success(&format!(
        "Configuration saved to {}",
        Config::config_path().display()
    ));
    Ok(())
}

pub fn show() -> Result<()> {
    let config = Config::load()?;
    let rows = vec![
        vec![
            "server_url".to_string(),
            config.server_url.clone().unwrap_or_default(),
        ],
        vec!["api_key".to_string(), config.masked_api_key()],
    ];
    println!(
        "{}",
        build_table_from_rows(vec!["Key".to_string(), "Value".to_string()], rows)
    );
    Ok(())
}
