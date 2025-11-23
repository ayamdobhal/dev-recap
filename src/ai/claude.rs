use crate::error::{DevRecapError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const CLAUDE_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-sonnet-4-5-20250929";
const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Claude API client
pub struct ClaudeClient {
    api_key: String,
    api_url: String,
    client: Client,
    model: String,
    max_tokens: u32,
}

impl ClaudeClient {
    /// Create a new Claude API client
    #[allow(dead_code)]
    pub fn new(api_key: String) -> Result<Self> {
        Self::with_base_url(api_key, None, None)
    }

    /// Create a new Claude API client with custom base URL and model
    /// The base_url should be the API base (e.g., "https://api.anthropic.com" or "http://localhost:4000")
    /// The "/v1/messages" endpoint will be appended automatically
    pub fn with_base_url(
        api_key: String,
        base_url: Option<String>,
        model: Option<String>,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()?;

        // Construct the full messages endpoint URL
        let base = base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let api_url = format!("{}/v1/messages", base.trim_end_matches('/'));

        Ok(Self {
            api_key,
            api_url,
            client,
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            max_tokens: DEFAULT_MAX_TOKENS,
        })
    }

    /// Set the model to use
    #[allow(dead_code)]
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// Set max tokens
    #[allow(dead_code)]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Generate a summary from a prompt
    pub async fn generate_summary(&self, prompt: String) -> Result<String> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt,
            }],
        };

        let response = self
            .client
            .post(&self.api_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", CLAUDE_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DevRecapError::claude_api(format!(
                "API request failed with status {}: {}",
                status, error_text
            )));
        }

        let claude_response: ClaudeResponse = response.json().await?;

        // Extract text from first content block
        if let Some(content) = claude_response.content.first() {
            Ok(content.text.clone())
        } else {
            Err(DevRecapError::claude_api(
                "No content in Claude response".to_string(),
            ))
        }
    }
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    block_type: String,
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ClaudeClient::new("sk-ant-test-key".to_string()).unwrap();
        assert_eq!(client.model, DEFAULT_MODEL);
        assert_eq!(client.max_tokens, DEFAULT_MAX_TOKENS);
    }

    #[test]
    fn test_client_builder() {
        let client = ClaudeClient::new("sk-ant-test-key".to_string())
            .unwrap()
            .with_model("claude-3-opus-20240229".to_string())
            .with_max_tokens(8192);

        assert_eq!(client.model, "claude-3-opus-20240229");
        assert_eq!(client.max_tokens, 8192);
    }

    #[test]
    fn test_base_url_construction() {
        // Test default URL
        let client = ClaudeClient::new("test-key".to_string()).unwrap();
        assert_eq!(client.api_url, "https://api.anthropic.com/v1/messages");

        // Test custom base URL without trailing slash
        let client = ClaudeClient::with_base_url(
            "test-key".to_string(),
            Some("http://localhost:4000".to_string()),
            None,
        )
        .unwrap();
        assert_eq!(client.api_url, "http://localhost:4000/v1/messages");

        // Test custom base URL with trailing slash
        let client = ClaudeClient::with_base_url(
            "test-key".to_string(),
            Some("http://localhost:4000/".to_string()),
            None,
        )
        .unwrap();
        assert_eq!(client.api_url, "http://localhost:4000/v1/messages");

        // Test full Anthropic URL
        let client = ClaudeClient::with_base_url(
            "test-key".to_string(),
            Some("https://api.anthropic.com".to_string()),
            None,
        )
        .unwrap();
        assert_eq!(client.api_url, "https://api.anthropic.com/v1/messages");
    }
}
