use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("HTTP error {status}: {message}")]
    ApiError { status: u16, message: String },

    #[error("Network error: {0}. Please check your --server-url or config.")]
    Network(#[from] reqwest::Error),

    #[error("Config file not found. Run `openaaas-admin config init` first.")]
    ConfigNotFound,

    #[error("Config error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("Toml error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Toml serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("User cancelled operation")]
    Cancelled,

    #[allow(dead_code)]
    #[error("Not found")]
    NotFound,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    pub fn extract_api_error(status: u16, body: &str) -> Self {
        let message = if let Ok(val) = serde_json::from_str::<serde_json::Value>(body) {
            val.get("message")
                .or_else(|| val.get("detail"))
                .or_else(|| val.get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or(body)
                .to_string()
        } else {
            body.to_string()
        };
        AppError::ApiError { status, message }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_display() {
        let err = AppError::ApiError {
            status: 404,
            message: "Not found".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("404"));
        assert!(display.contains("Not found"));
    }

    #[test]
    fn test_network_error_display() {
        // Construct a reqwest::Error without creating a Client or performing I/O
        let err = reqwest::Proxy::all("://invalid").unwrap_err();
        let app_err = AppError::Network(err);
        let display = format!("{}", app_err);
        assert!(display.contains("Network error"));
    }
}
