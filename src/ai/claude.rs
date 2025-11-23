use crate::error::{DevRecapError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const CLAUDE_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-3-5-sonnet-20241022";
const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Claude API client
pub struct ClaudeClient {
    api_key: String,
    client: Client,
    model: String,
    max_tokens: u32,
}

impl ClaudeClient {
    /// Create a new Claude API client
    pub fn new(api_key: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()?;

        Ok(Self {
            api_key,
            client,
            model: DEFAULT_MODEL.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
        })
    }

    /// Set the model to use
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// Set max tokens
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
            .post(CLAUDE_API_URL)
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
}
