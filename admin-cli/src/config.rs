use std::fs::Permissions;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Config {
    pub server_url: Option<String>,
    pub api_key: Option<String>,
}

impl Config {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("openaaas-admin")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Err(AppError::ConfigNotFound);
        }
        let content = std::fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir();
        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }
        let path = dir.join("config.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        #[cfg(unix)]
        {
            std::fs::set_permissions(&path, Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    /// Build effective config with priority:
    /// CLI args > Environment variables > Config file
    pub fn effective(cli_server_url: Option<&str>, cli_api_key: Option<&str>) -> Result<Self> {
        // Start with config file (or empty)
        let mut config = Self::load().unwrap_or_default();

        // Override with environment variables
        if let Ok(url) = std::env::var("OPENAAAS_SERVER_URL") {
            config.server_url = Some(url);
        }
        if let Ok(key) = std::env::var("OPENAAAS_API_KEY") {
            config.api_key = Some(key);
        }

        // Override with CLI args
        if let Some(url) = cli_server_url {
            config.server_url = Some(url.to_string());
        }
        if let Some(key) = cli_api_key {
            config.api_key = Some(key.to_string());
        }

        Ok(config)
    }

    pub fn require_server_url(&self) -> Result<String> {
        self.server_url.clone().ok_or_else(|| {
            AppError::Config(
                "Server URL not configured. Use --server-url or run `config init`.".to_string(),
            )
        })
    }

    pub fn require_api_key(&self) -> Result<String> {
        self.api_key.clone().ok_or_else(|| {
            AppError::Config(
                "API key not configured. Use --api-key or run `config init`.".to_string(),
            )
        })
    }

    pub fn masked_api_key(&self) -> String {
        match &self.api_key {
            Some(key) if key.len() > 8 => format!("{}***", &key[..8]),
            Some(key) => key.clone(),
            None => "(not set)".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parse() {
        let toml = r#"
server_url = "http://localhost:8080"
api_key = "ak_admin_12345678"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.server_url, Some("http://localhost:8080".to_string()));
        assert_eq!(config.api_key, Some("ak_admin_12345678".to_string()));
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.server_url.is_none());
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_masked_api_key() {
        let config = Config {
            server_url: None,
            api_key: Some("ak_admin_1234567890".to_string()),
        };
        assert_eq!(config.masked_api_key(), "ak_admin***");

        let config_short = Config {
            server_url: None,
            api_key: Some("short".to_string()),
        };
        assert_eq!(config_short.masked_api_key(), "short");

        let config_none = Config::default();
        assert_eq!(config_none.masked_api_key(), "(not set)");
    }

    #[test]
    fn test_effective_priority() {
        // Set env var temporarily
        unsafe {
            std::env::set_var("OPENAAAS_SERVER_URL", "http://env:8080");
            std::env::set_var("OPENAAAS_API_KEY", "env_key");
        }

        let config = Config::effective(Some("http://cli:8080"), Some("cli_key")).unwrap();
        assert_eq!(config.server_url, Some("http://cli:8080".to_string()));
        assert_eq!(config.api_key, Some("cli_key".to_string()));

        let config2 = Config::effective(None, None).unwrap();
        assert_eq!(config2.server_url, Some("http://env:8080".to_string()));
        assert_eq!(config2.api_key, Some("env_key".to_string()));

        unsafe {
            std::env::remove_var("OPENAAAS_SERVER_URL");
            std::env::remove_var("OPENAAAS_API_KEY");
        }
    }

    #[test]
    fn test_config_serialize_roundtrip() {
        let config = Config {
            server_url: Some("http://test:8080".to_string()),
            api_key: Some("ak_test".to_string()),
        };
        let s = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&s).unwrap();
        assert_eq!(config, parsed);
    }
}
