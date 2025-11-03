use crate::error::{DevRecapError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default author email for filtering commits
    pub default_author_email: Option<String>,

    /// Claude API key
    pub claude_api_key: String,

    /// Default timespan in days (default: 14 days / 2 weeks)
    #[serde(default = "default_timespan")]
    pub default_timespan_days: u32,

    /// Directories/patterns to exclude from scanning
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,

    /// Maximum directory depth for scanning (None = unlimited)
    pub max_scan_depth: Option<u32>,

    /// Enable caching of AI summaries
    #[serde(default = "default_true")]
    pub cache_enabled: bool,

    /// Cache TTL in hours (default: 168 hours / 7 days)
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_hours: u32,

    /// GitHub token for API access (optional, increases rate limits)
    pub github_token: Option<String>,
}

impl Config {
    /// Load configuration from the default location (~/.config/dev-recap/config.toml)
    pub fn load() -> Result<Self> {
        let config_path = Self::default_config_path()?;
        Self::load_from(&config_path)
    }

    /// Load configuration from a specific path
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(DevRecapError::config(format!(
                "Config file not found at: {}",
                path.display()
            )));
        }

        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    /// Get the default config file path
    pub fn default_config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| DevRecapError::config("Could not determine home directory"))?;
        Ok(home.join(".config").join("dev-recap").join("config.toml"))
    }

    /// Get the default cache directory path
    pub fn default_cache_dir() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| DevRecapError::config("Could not determine home directory"))?;
        Ok(home.join(".cache").join("dev-recap"))
    }

    /// Create a default configuration file at the default location
    pub fn create_default() -> Result<Self> {
        let config_path = Self::default_config_path()?;

        // Create parent directories if they don't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let config = Self::default();
        let toml_string = toml::to_string_pretty(&config)?;
        fs::write(&config_path, toml_string)?;

        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.claude_api_key.is_empty() {
            return Err(DevRecapError::MissingConfig(
                "claude_api_key is required".to_string(),
            ));
        }

        if !self.claude_api_key.starts_with("sk-ant-") {
            return Err(DevRecapError::config(
                "Invalid Claude API key format (should start with 'sk-ant-')",
            ));
        }

        if self.default_timespan_days == 0 {
            return Err(DevRecapError::config("default_timespan_days must be > 0"));
        }

        if self.cache_ttl_hours == 0 {
            return Err(DevRecapError::config("cache_ttl_hours must be > 0"));
        }

        Ok(())
    }

    /// Load config from file, or create default if it doesn't exist
    pub fn load_or_create_default() -> Result<Self> {
        match Self::load() {
            Ok(config) => Ok(config),
            Err(DevRecapError::Config(_)) => {
                eprintln!("Config file not found. Creating default config...");
                Self::create_default()
            }
            Err(e) => Err(e),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_author_email: None,
            claude_api_key: String::from("sk-ant-YOUR_API_KEY_HERE"),
            default_timespan_days: default_timespan(),
            exclude_patterns: default_exclude_patterns(),
            max_scan_depth: None,
            cache_enabled: default_true(),
            cache_ttl_hours: default_cache_ttl(),
            github_token: None,
        }
    }
}

// Serde default functions
fn default_timespan() -> u32 {
    14 // 2 weeks
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        "node_modules".to_string(),
        "target".to_string(),
        ".git".to_string(),
        "dist".to_string(),
        "build".to_string(),
        "__pycache__".to_string(),
        ".venv".to_string(),
        "vendor".to_string(),
        ".next".to_string(),
        "out".to_string(),
    ]
}

fn default_cache_ttl() -> u32 {
    168 // 7 days in hours
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.default_timespan_days, 14);
        assert!(config.cache_enabled);
        assert_eq!(config.cache_ttl_hours, 168);
        assert!(!config.exclude_patterns.is_empty());
    }

    #[test]
    fn test_config_validation_missing_api_key() {
        let mut config = Config::default();
        config.claude_api_key = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_api_key() {
        let mut config = Config::default();
        config.claude_api_key = String::from("invalid-key");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let mut config = Config::default();
        config.claude_api_key = String::from("sk-ant-valid-key-123");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("claude_api_key"));
        assert!(toml_str.contains("default_timespan_days"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
            claude_api_key = "sk-ant-test-key"
            default_timespan_days = 30
            cache_enabled = false
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.claude_api_key, "sk-ant-test-key");
        assert_eq!(config.default_timespan_days, 30);
        assert!(!config.cache_enabled);
    }
}
