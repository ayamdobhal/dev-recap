use crate::error::{DevRecapError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default author email for filtering commits
    pub default_author_email: Option<String>,

    /// Claude API key (can be overridden by ANTHROPIC_AUTH_TOKEN env var)
    #[serde(default)]
    pub claude_api_key: Option<String>,

    /// Claude API base URL (can be overridden by ANTHROPIC_BASE_URL env var)
    /// Should be the base URL without /v1/messages (e.g., "https://api.anthropic.com" or "http://localhost:4000")
    /// The /v1/messages endpoint will be appended automatically
    pub claude_api_base_url: Option<String>,

    /// Claude model to use (optional, defaults to claude-sonnet-4-5-20250929)
    pub claude_model: Option<String>,

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
        let mut config: Config = toml::from_str(&contents)?;

        // Apply environment variable overrides (priority: env > config file)
        config.apply_env_overrides();

        config.validate()?;
        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        use std::env;

        // ANTHROPIC_AUTH_TOKEN takes precedence over config file
        if let Ok(api_key) = env::var("ANTHROPIC_AUTH_TOKEN") {
            self.claude_api_key = Some(api_key);
        }

        // ANTHROPIC_BASE_URL takes precedence over config file
        if let Ok(base_url) = env::var("ANTHROPIC_BASE_URL") {
            self.claude_api_base_url = Some(base_url);
        }
    }

    /// Get the effective API key (from env or config)
    pub fn get_api_key(&self) -> Result<String> {
        self.claude_api_key
            .clone()
            .ok_or_else(|| DevRecapError::MissingConfig(
                "claude_api_key is required (set ANTHROPIC_AUTH_TOKEN env var or add to config file)".to_string()
            ))
    }

    /// Get the effective base URL (from env, config, or default)
    pub fn get_base_url(&self) -> Option<String> {
        self.claude_api_base_url.clone()
    }

    /// Get the effective model (from config or default)
    pub fn get_model(&self) -> Option<String> {
        self.claude_model.clone()
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
        // Validate API key if present (it's now optional in config, can come from env)
        if let Some(ref api_key) = self.claude_api_key {
            if api_key.is_empty() {
                return Err(DevRecapError::MissingConfig(
                    "claude_api_key cannot be empty".to_string(),
                ));
            }
            // No longer validate key format since custom base URLs may use different auth schemes
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
    /// Always applies environment variable overrides
    pub fn load_or_create_default() -> Result<Self> {
        let mut config = match Self::load() {
            Ok(config) => config,
            Err(DevRecapError::Config(_)) => {
                eprintln!("Config file not found. Creating default config...");
                let mut cfg = Self::create_default()?;
                // Apply env overrides even for newly created config
                cfg.apply_env_overrides();
                return Ok(cfg);
            }
            Err(e) => return Err(e),
        };

        // Env vars are already applied in load_from, but this ensures
        // they're applied even if config was loaded differently
        config.apply_env_overrides();
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_author_email: None,
            claude_api_key: None, // Will be read from env or config file
            claude_api_base_url: None,
            claude_model: None,
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
    fn test_config_validation_empty_api_key() {
        let mut config = Config::default();
        config.claude_api_key = Some(String::new());
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_any_key_format() {
        // Any non-empty key format is valid (for custom base URLs)
        let mut config = Config::default();
        config.claude_api_key = Some(String::from("custom-auth-token-123"));
        assert!(config.validate().is_ok());

        config.claude_api_key = Some(String::from("sk-ant-valid-key-123"));
        assert!(config.validate().is_ok());

        config.claude_api_key = Some(String::from("bearer-token"));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_no_api_key() {
        let mut config = Config::default();
        config.claude_api_key = None;
        // Should be valid - API key is optional in config (can come from env)
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_get_api_key_from_config() {
        let mut config = Config::default();
        config.claude_api_key = Some("sk-ant-test-key".to_string());
        assert!(config.get_api_key().is_ok());
        assert_eq!(config.get_api_key().unwrap(), "sk-ant-test-key");
    }

    #[test]
    fn test_get_api_key_missing() {
        let config = Config::default();
        assert!(config.get_api_key().is_err());
    }

    #[test]
    fn test_config_serialization() {
        let mut config = Config::default();
        config.claude_api_key = Some("sk-ant-test".to_string());
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("claude_api_key"));
        assert!(toml_str.contains("default_timespan_days"));
    }

    #[test]
    fn test_config_serialization_no_api_key() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        // When claude_api_key is None, it won't appear in serialized output
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
        assert_eq!(config.claude_api_key, Some("sk-ant-test-key".to_string()));
        assert_eq!(config.default_timespan_days, 30);
        assert!(!config.cache_enabled);
    }
}
