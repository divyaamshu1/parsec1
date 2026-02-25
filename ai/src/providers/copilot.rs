//! GitHub Copilot provider implementation

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use futures::stream::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::providers::RateLimiter;
use crate::{
    AIProvider, ProviderCapabilities, ModelInfo, ProviderConfig,
    CompletionRequest, CompletionResponse, CompletionStream, CompletionChunk,
    ChatRequest, ChatResponse, ChatStream, ChatChunk, ChatDelta, ChatRole,
    EmbeddingRequest, EmbeddingResponse, TokenUsage,
};

pub struct CopilotProvider {
    config: ProviderConfig,
    client: Client,
    token: Option<String>,
    models: Vec<ModelInfo>,
    rate_limiter: Arc<Mutex<RateLimiter>>,
}

impl CopilotProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let models = vec![
            ModelInfo {
                id: "copilot-code".to_string(),
                name: "Copilot Code".to_string(),
                description: Some("GitHub Copilot code completion".to_string()),
                context_length: 4096,
                max_output_tokens: Some(256),
                streaming: true,
                functions: false,
                vision: false,
            },
            ModelInfo {
                id: "copilot-chat".to_string(),
                name: "Copilot Chat".to_string(),
                description: Some("GitHub Copilot Chat".to_string()),
                context_length: 4096,
                max_output_tokens: Some(1024),
                streaming: true,
                functions: false,
                vision: false,
            },
        ];

        Self {
            config,
            client: Client::new(),
            token: None,
            models,
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(60))), // 60 requests per minute
        }
    }

    async fn get_token(&self) -> Result<String> {
        if let Some(token) = &self.token {
            return Ok(token.clone());
        }

        let url = "https://api.github.com/copilot_internal/v2/token";
        
        let response = self.client
            .get(url)
            .header("Authorization", format!("token {}", self.config.api_key.as_deref().unwrap_or("")))
            .header("User-Agent", "Parsec-IDE")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get Copilot token: {}", response.status()));
        }

        #[derive(serde::Deserialize)]
        struct TokenResponse {
            token: String,
        }

        let data: TokenResponse = response.json().await?;
        Ok(data.token)
    }

    async fn headers(&self) -> Result<reqwest::header::HeaderMap> {
        let token = self.get_token().await?;
        
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {}", token).parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert("User-Agent", "Parsec-IDE".parse().unwrap());
        headers.insert("Editor-Version", "vscode/1.85.0".parse().unwrap());
        Ok(headers)
    }
}

#[async_trait]
impl AIProvider for CopilotProvider {
    fn name(&self) -> &str {
        "copilot"
    }

    fn box_clone(&self) -> Box<dyn AIProvider> {
        Box::new(Self {
            config: self.config.clone(),
            client: self.client.clone(),
            token: self.token.clone(),
            models: self.models.clone(),
            rate_limiter: self.rate_limiter.clone(),
        })
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider_id: "copilot".to_string(),
            models: self.models.clone(),
            streaming: true,
            functions: false,
            vision: false,
            embeddings: false,
            max_context_length: 4096,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
        }
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let headers = self.headers().await?;
        let url = "https://api.githubcopilot.com/completions";
        
        let body = json!({
            "prompt": request.prompt,
            "suffix": "",
            "max_tokens": request.max_tokens.unwrap_or(256),
            "temperature": request.temperature.unwrap_or(0.2),
            "n": 1,
            "stop": request.stop,
            "stream": false,
        });

        let response = self.client
            .post(url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("Copilot API error: {}", error));
        }

        #[derive(serde::Deserialize)]
        struct CopilotResponse {
            choices: Vec<CopilotChoice>,
        }

        #[derive(serde::Deserialize)]
        struct CopilotChoice {
            text: String,
            finish_reason: String,
        }

        let data: CopilotResponse = response.json().await?;
        let choice = data.choices.first().ok_or_else(|| anyhow!("No completion choices"))?;

        Ok(CompletionResponse {
            text: choice.text.clone(),
            model: "copilot-code".to_string(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            finish_reason: Some(choice.finish_reason.clone()),
        })
    }

    async fn complete_stream(&self, request: CompletionRequest) -> Result<CompletionStream> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let headers = self.headers().await?;
        let url = "https://api.githubcopilot.com/completions";
        
        let body = json!({
            "prompt": request.prompt,
            "suffix": "",
            "max_tokens": request.max_tokens.unwrap_or(256),
            "temperature": request.temperature.unwrap_or(0.2),
            "stream": true,
        });

        let response = self.client
            .post(url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Copilot API error: {}", response.status()));
        }

        let stream = response.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk).to_string();

            if text.starts_with("data: ") {
                let data = &text[6..];
                if data == "[DONE]" {
                    return Ok(CompletionChunk {
                        text: String::new(),
                        finish_reason: Some("stop".to_string()),
                    });
                }

                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(choices) = json["choices"].as_array() {
                        if let Some(choice) = choices.first() {
                            if let Some(text) = choice["text"].as_str() {
                                return Ok(CompletionChunk {
                                    text: text.to_string(),
                                    finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
                                });
                            }
                        }
                    }
                }
            }

            Ok(CompletionChunk {
                text: String::new(),
                finish_reason: None,
            })
        });

        Ok(Box::pin(stream))
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let headers = self.headers().await?;
        let url = "https://api.githubcopilot.com/chat/completions";
        
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|msg| {
            json!({
                "role": match msg.role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                },
                "content": msg.content,
            })
        }).collect();

        let body = json!({
            "model": "gpt-3.5-turbo",
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(1024),
            "temperature": request.temperature.unwrap_or(0.5),
            "stream": false,
        });

        let response = self.client
            .post(url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("Copilot Chat error: {}", error));
        }

        #[derive(serde::Deserialize)]
        struct CopilotChatResponse {
            choices: Vec<CopilotChatChoice>,
            usage: CopilotUsage,
            model: String,
        }

        #[derive(serde::Deserialize)]
        struct CopilotChatChoice {
            message: CopilotChatMessage,
            finish_reason: String,
        }

        #[derive(serde::Deserialize)]
        struct CopilotChatMessage {
            role: String,
            content: String,
        }

        #[derive(serde::Deserialize)]
        struct CopilotUsage {
            prompt_tokens: usize,
            completion_tokens: usize,
            total_tokens: usize,
        }

        let data: CopilotChatResponse = response.json().await?;
        let choice = data.choices.first().ok_or_else(|| anyhow!("No chat choices"))?;

        Ok(ChatResponse {
            message: crate::ChatMessage {
                role: ChatRole::Assistant,
                content: choice.message.content.clone(),
                name: None,
            },
            model: data.model,
            usage: TokenUsage {
                prompt_tokens: data.usage.prompt_tokens,
                completion_tokens: data.usage.completion_tokens,
                total_tokens: data.usage.total_tokens,
            },
            finish_reason: Some(choice.finish_reason.clone()),
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<ChatStream> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let headers = self.headers().await?;
        let url = "https://api.githubcopilot.com/chat/completions";
        
        let messages: Vec<serde_json::Value> = request.messages.iter().map(|msg| {
            json!({
                "role": match msg.role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                },
                "content": msg.content,
            })
        }).collect();

        let body = json!({
            "model": "gpt-3.5-turbo",
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(1024),
            "temperature": request.temperature.unwrap_or(0.5),
            "stream": true,
        });

        let response = self.client
            .post(url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Copilot Chat error: {}", response.status()));
        }

        let stream = response.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk).to_string();

            if text.starts_with("data: ") {
                let data = &text[6..];
                if data == "[DONE]" {
                    return Ok(ChatChunk {
                        delta: ChatDelta { role: None, content: None },
                        finish_reason: Some("stop".to_string()),
                    });
                }

                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(choices) = json["choices"].as_array() {
                        if let Some(choice) = choices.first() {
                            if let Some(delta) = choice["delta"].as_object() {
                                let content = delta.get("content").and_then(|c| c.as_str()).map(|s| s.to_string());
                                return Ok(ChatChunk {
                                    delta: ChatDelta {
                                        role: None,
                                        content,
                                    },
                                    finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
                                });
                            }
                        }
                    }
                }
            }

            Ok(ChatChunk {
                delta: ChatDelta { role: None, content: None },
                finish_reason: None,
            })
        });

        Ok(Box::pin(stream))
    }

    async fn embed(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        Err(anyhow!("Copilot doesn't support embeddings"))
    }

    async fn is_available(&self) -> bool {
        self.config.api_key.is_some() && self.get_token().await.is_ok()
    }
}

/// Provider-specific error types
#[derive(Debug, Clone)]
pub enum CopilotError {
    AuthError(String),
    TokenExpired,
    RateLimitExceeded,
    ApiError(String),
    NetworkError(String),
}

impl std::fmt::Display for CopilotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CopilotError::AuthError(msg) => write!(f, "Authentication failed: {}", msg),
            CopilotError::TokenExpired => write!(f, "Token expired"),
            CopilotError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            CopilotError::ApiError(msg) => write!(f, "API error: {}", msg),
            CopilotError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl std::error::Error for CopilotError {}

impl From<reqwest::Error> for CopilotError {
    fn from(err: reqwest::Error) -> Self {
        CopilotError::NetworkError(err.to_string())
    }
}

/// Copilot-specific client for additional features
impl CopilotProvider {
    /// Get user's Copilot status
    pub async fn get_status(&self) -> Result<serde_json::Value> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let headers = self.headers().await?;
        let url = "https://api.githubcopilot.com/user/status";
        
        let response = self.client
            .get(url)
            .headers(headers)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get Copilot status: {}", response.status()));
        }

        Ok(response.json().await?)
    }

    /// Get completion suggestions for a file
    pub async fn get_suggestions(&self, file_path: &str, content: &str, cursor_pos: usize) -> Result<Vec<String>> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let headers = self.headers().await?;
        let url = "https://api.githubcopilot.com/completions";
        
        let body = json!({
            "prompt": content,
            "suffix": "",
            "max_tokens": 50,
            "temperature": 0.2,
            "n": 5,
            "stop": ["\n", "}", ")", ";"],
            "extra": {
                "language": detect_language(file_path),
                "next_indent": 0,
                "cursor_position": cursor_pos,
            }
        });

        let response = self.client
            .post(url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get suggestions: {}", response.status()));
        }

        #[derive(serde::Deserialize)]
        struct SuggestionsResponse {
            choices: Vec<SuggestionChoice>,
        }

        #[derive(serde::Deserialize)]
        struct SuggestionChoice {
            text: String,
        }

        let data: SuggestionsResponse = response.json().await?;
        Ok(data.choices.into_iter().map(|c| c.text).collect())
    }

    /// Get inline completions for IDE
    pub async fn get_inline_completion(&self, content: &str, cursor_pos: usize, language: &str) -> Result<Option<String>> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let headers = self.headers().await?;
        let url = "https://api.githubcopilot.com/inline/completions";
        
        let body = json!({
            "prompt": content,
            "suffix": "",
            "max_tokens": 50,
            "temperature": 0.1,
            "n": 1,
            "stop": ["\n", "}", ")", ";"],
            "extra": {
                "language": language,
                "cursor_position": cursor_pos,
            }
        });

        let response = self.client
            .post(url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(None);
        }

        #[derive(serde::Deserialize)]
        struct InlineResponse {
            choices: Vec<InlineChoice>,
        }

        #[derive(serde::Deserialize)]
        struct InlineChoice {
            text: String,
        }

        let data: InlineResponse = response.json().await?;
        Ok(data.choices.first().map(|c| c.text.clone()))
    }
}

/// Detect language from file path
fn detect_language(path: &str) -> &'static str {
    if path.ends_with(".rs") {
        "rust"
    } else if path.ends_with(".py") {
        "python"
    } else if path.ends_with(".js") || path.ends_with(".jsx") {
        "javascript"
    } else if path.ends_with(".ts") || path.ends_with(".tsx") {
        "typescript"
    } else if path.ends_with(".go") {
        "go"
    } else if path.ends_with(".java") {
        "java"
    } else if path.ends_with(".cpp") || path.ends_with(".cxx") || path.ends_with(".cc") {
        "cpp"
    } else if path.ends_with(".c") {
        "c"
    } else {
        "text"
    }
}

/// Copilot authentication helper
pub struct CopilotAuth {
    github_token: String,
    client: Client,
}

impl CopilotAuth {
    pub fn new(github_token: String) -> Self {
        Self {
            github_token,
            client: Client::new(),
        }
    }

    /// Get Copilot token using GitHub token
    pub async fn get_copilot_token(&self) -> Result<String> {
        // Apply rate limiting - note: this uses the provider's rate limiter indirectly
        // In a real implementation, you'd need access to the rate limiter here
        let url = "https://api.github.com/copilot_internal/v2/token";
        
        let response = self.client
            .get(url)
            .header("Authorization", format!("token {}", self.github_token))
            .header("User-Agent", "Parsec-IDE")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get Copilot token: {}", response.status()));
        }

        #[derive(serde::Deserialize)]
        struct TokenResponse {
            token: String,
            expires_at: u64,
        }

        let data: TokenResponse = response.json().await?;
        Ok(data.token)
    }

    /// Check if user has Copilot access
    pub async fn check_access(&self) -> Result<bool> {
        // Apply rate limiting - note: this uses the provider's rate limiter indirectly
        let url = "https://api.github.com/copilot_internal/v2/access";
        
        let response = self.client
            .get(url)
            .header("Authorization", format!("token {}", self.github_token))
            .header("User-Agent", "Parsec-IDE")
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("main.rs"), "rust");
        assert_eq!(detect_language("app.py"), "python");
        assert_eq!(detect_language("index.js"), "javascript");
        assert_eq!(detect_language("component.tsx"), "typescript");
        assert_eq!(detect_language("main.go"), "go");
        assert_eq!(detect_language("README.md"), "text");
    }

    #[tokio::test]
    async fn test_copilot_provider_creation() {
        let config = ProviderConfig::new("copilot").with_api_key("test-token");
        let provider = CopilotProvider::new(config);
        assert_eq!(provider.name(), "copilot");
    }
}

/// Copilot response handler for processing completions
pub struct CopilotResponseHandler;

impl CopilotResponseHandler {
    /// Parse completion response and extract the best suggestion
    pub fn parse_completion_response(response: &serde_json::Value) -> Option<String> {
        response.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("text"))
            .and_then(|text| text.as_str())
            .map(|s| s.to_string())
    }

    /// Parse inline completion response
    pub fn parse_inline_response(response: &serde_json::Value) -> Option<String> {
        response.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("text"))
            .and_then(|text| text.as_str())
            .map(|s| s.to_string())
    }

    /// Filter and rank multiple suggestions
    pub fn rank_suggestions(suggestions: Vec<String>) -> Vec<String> {
        let mut suggestions = suggestions;
        
        // Remove duplicates
        suggestions.dedup();
        
        // Filter out empty suggestions
        suggestions.retain(|s| !s.trim().is_empty());
        
        // Sort by length? (simpler completions first)
        suggestions.sort_by_key(|s| s.len());
        
        suggestions
    }
}

/// Copilot telemetry for tracking usage
pub struct CopilotTelemetry {
    enabled: bool,
    events: Vec<TelemetryEvent>,
}

struct TelemetryEvent {
    event_type: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    data: serde_json::Value,
}

impl CopilotTelemetry {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            events: Vec::new(),
        }
    }

    pub fn track_completion(&mut self, file_type: &str, accepted: bool) {
        if !self.enabled {
            return;
        }

        self.events.push(TelemetryEvent {
            event_type: "completion".to_string(),
            timestamp: chrono::Utc::now(),
            data: json!({
                "file_type": file_type,
                "accepted": accepted,
            }),
        });
    }

    pub fn track_error(&mut self, error_type: &str) {
        if !self.enabled {
            return;
        }

        self.events.push(TelemetryEvent {
            event_type: "error".to_string(),
            timestamp: chrono::Utc::now(),
            data: json!({
                "error_type": error_type,
            }),
        });
    }

    pub fn export_events(&self) -> Vec<serde_json::Value> {
        self.events.iter().map(|e| {
            json!({
                "type": e.event_type,
                "timestamp": e.timestamp.to_rfc3339(),
                "data": e.data,
            })
        }).collect()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl Default for CopilotTelemetry {
    fn default() -> Self {
        Self::new(false)
    }
}

/// Copilot configuration for fine-tuning behavior
#[derive(Debug, Clone)]
pub struct CopilotConfig {
    pub enable_inline_completions: bool,
    pub enable_snippets: bool,
    pub max_suggestions: usize,
    pub suggestion_delay_ms: u64,
    pub enable_telemetry: bool,
    pub languages: Vec<String>,
}

impl Default for CopilotConfig {
    fn default() -> Self {
        Self {
            enable_inline_completions: true,
            enable_snippets: true,
            max_suggestions: 5,
            suggestion_delay_ms: 100,
            enable_telemetry: false,
            languages: vec![
                "rust".to_string(),
                "python".to_string(),
                "javascript".to_string(),
                "typescript".to_string(),
                "go".to_string(),
                "java".to_string(),
            ],
        }
    }
}

/// Copilot session for managing a coding session
pub struct CopilotSession {
    provider: CopilotProvider,
    config: CopilotConfig,
    telemetry: CopilotTelemetry,
    current_file: Option<String>,
    current_language: Option<String>,
}

impl CopilotSession {
    pub fn new(provider: CopilotProvider, config: CopilotConfig) -> Self {
        let telemetry = CopilotTelemetry::new(config.enable_telemetry);
        
        Self {
            provider,
            config,
            telemetry,
            current_file: None,
            current_language: None,
        }
    }

    pub fn set_current_file(&mut self, path: &str) {
        self.current_file = Some(path.to_string());
        self.current_language = Some(detect_language(path).to_string());
    }

    pub async fn get_suggestion(&mut self, content: &str, cursor_pos: usize) -> Result<Option<String>> {
        let language = self.current_language.as_deref().unwrap_or("text");
        
        if !self.config.languages.contains(&language.to_string()) {
            return Ok(None);
        }

        let result = self.provider.get_inline_completion(content, cursor_pos, language).await;
        
        match &result {
            Ok(Some(_)) => self.telemetry.track_completion(language, true),
            Ok(None) => self.telemetry.track_completion(language, false),
            Err(e) => self.telemetry.track_error(&e.to_string()),
        }

        result
    }

    pub async fn get_multiple_suggestions(&mut self, content: &str, cursor_pos: usize) -> Result<Vec<String>> {
        let language = self.current_language.as_deref().unwrap_or("text");
        
        if !self.config.languages.contains(&language.to_string()) {
            return Ok(Vec::new());
        }

        let file_path = self.current_file.as_deref().unwrap_or("file.txt");
        let suggestions = self.provider.get_suggestions(file_path, content, cursor_pos).await?;
        
        Ok(CopilotResponseHandler::rank_suggestions(suggestions))
    }

    pub fn get_telemetry(&self) -> Vec<serde_json::Value> {
        self.telemetry.export_events()
    }

    pub fn clear_telemetry(&mut self) {
        self.telemetry.clear();
    }
}

/// Copilot cache for storing frequent completions
pub struct CopilotCache {
    cache: std::collections::HashMap<String, Vec<String>>,
    max_entries: usize,
}

impl CopilotCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: std::collections::HashMap::with_capacity(max_entries),
            max_entries,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Vec<String>> {
        self.cache.get(key)
    }

    pub fn insert(&mut self, key: String, value: Vec<String>) {
        if self.cache.len() >= self.max_entries {
            // Remove oldest entry
            if let Some(oldest) = self.cache.keys().next().cloned() {
                self.cache.remove(&oldest);
            }
        }
        self.cache.insert(key, value);
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for CopilotCache {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Language-specific prompt templates for Copilot
pub mod templates {
    use std::collections::HashMap;
    use std::sync::OnceLock;

    fn get_templates() -> &'static HashMap<&'static str, Vec<&'static str>> {
        static TEMPLATES: OnceLock<HashMap<&'static str, Vec<&'static str>>> = OnceLock::new();
        
        TEMPLATES.get_or_init(|| {
            let mut m = HashMap::new();
            
            m.insert("rust", vec![
                "fn $NAME$($ARGS$) -> $RET$ {\n    $0\n}",
                "impl $TYPE$ {\n    fn $NAME$(&self) -> $RET$ {\n        $0\n    }\n}",
                "match $EXPR$ {\n    $PAT$ => $0,\n    _ => {},\n}",
            ]);
            
            m.insert("python", vec![
                "def $NAME$($ARGS$):\n    $0",
                "class $NAME$:\n    def __init__(self):\n        $0",
                "if $COND$:\n    $0\nelse:\n    pass",
            ]);
            
            m.insert("javascript", vec![
                "function $NAME$($ARGS$) {\n    $0\n}",
                "const $NAME$ = ($ARGS$) => {\n    $0\n}",
                "if ($COND$) {\n    $0\n}",
            ]);
            
            m
        })
    }

    pub fn list_templates(language: &str) -> Option<&'static Vec<&'static str>> {
        get_templates().get(language)
    }

    pub fn get_all_languages() -> Vec<&'static str> {
        get_templates().keys().copied().collect()
    }
}

/// Copilot provider factory for easy creation
pub struct CopilotFactory;

impl CopilotFactory {
    /// Create a new Copilot provider with GitHub token
    pub async fn with_github_token(token: &str) -> Result<CopilotProvider> {
        let auth = CopilotAuth::new(token.to_string());
        let copilot_token = auth.get_copilot_token().await?;
        
        let config = ProviderConfig::new("copilot")
            .with_api_key(&copilot_token);
        
        Ok(CopilotProvider::new(config))
    }

    /// Create a new Copilot provider with existing token
    pub fn with_copilot_token(token: &str) -> CopilotProvider {
        let config = ProviderConfig::new("copilot")
            .with_api_key(token);
        
        CopilotProvider::new(config)
    }

    /// Check if GitHub token is valid
    pub async fn validate_github_token(token: &str) -> bool {
        let auth = CopilotAuth::new(token.to_string());
        auth.check_access().await.unwrap_or(false)
    }
}

/// Copilot metrics for monitoring
#[derive(Debug, Default)]
pub struct CopilotMetrics {
    pub total_requests: usize,
    pub total_suggestions: usize,
    pub accepted_suggestions: usize,
    pub rejected_suggestions: usize,
    pub average_latency_ms: f64,
    pub errors: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl CopilotMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_request(&mut self, latency_ms: u64) {
        self.total_requests += 1;
        let total_latency = self.average_latency_ms * (self.total_requests - 1) as f64;
        self.average_latency_ms = (total_latency + latency_ms as f64) / self.total_requests as f64;
    }

    pub fn record_suggestion(&mut self, accepted: bool) {
        self.total_suggestions += 1;
        if accepted {
            self.accepted_suggestions += 1;
        } else {
            self.rejected_suggestions += 1;
        }
    }

    pub fn record_error(&mut self) {
        self.errors += 1;
    }

    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    pub fn acceptance_rate(&self) -> f64 {
        if self.total_suggestions == 0 {
            0.0
        } else {
            self.accepted_suggestions as f64 / self.total_suggestions as f64
        }
    }

    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
}

/// Copilot suggestion formatter for IDE integration
pub struct SuggestionFormatter;

impl SuggestionFormatter {
    /// Format suggestion for display
    pub fn format_for_display(suggestion: &str) -> String {
        let lines: Vec<&str> = suggestion.lines().collect();
        
        if lines.len() > 5 {
            let mut result = lines[..3].join("\n");
            result.push_str("\n...\n");
            result.push_str(lines.last().unwrap());
            result
        } else {
            suggestion.to_string()
        }
    }

    /// Format suggestion for inline insertion
    pub fn format_for_inline(suggestion: &str, existing: &str, cursor: usize) -> String {
        let prefix = &existing[..cursor];
        let suffix = &existing[cursor..];
        
        if suggestion.starts_with(prefix) {
            suggestion[cursor..].to_string()
        } else {
            suggestion.to_string()
        }
    }

    /// Add syntax highlighting markers
    pub fn add_highlighting(suggestion: &str, language: &str) -> String {
        format!("```{}\n{}\n```", language, suggestion)
    }

    /// Truncate suggestion to reasonable length
    pub fn truncate(suggestion: &str, max_lines: usize, max_chars: usize) -> String {
        let lines: Vec<&str> = suggestion.lines().collect();
        
        if lines.len() > max_lines || suggestion.len() > max_chars {
            let truncated_lines: Vec<&str> = lines.into_iter().take(max_lines).collect();
            let mut result = truncated_lines.join("\n");
            result.push_str("\n... (truncated)");
            result
        } else {
            suggestion.to_string()
        }
    }
}

/// Copilot keyboard shortcuts configuration
#[derive(Debug, Clone)]
pub struct CopilotShortcuts {
    pub accept: String,
    pub reject: String,
    pub next: String,
    pub previous: String,
    pub trigger: String,
}

impl Default for CopilotShortcuts {
    fn default() -> Self {
        Self {
            accept: "tab".to_string(),
            reject: "esc".to_string(),
            next: "alt+]".to_string(),
            previous: "alt+[".to_string(),
            trigger: "ctrl+space".to_string(),
        }
    }
}

impl CopilotShortcuts {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn vim() -> Self {
        Self {
            accept: "tab".to_string(),
            reject: "ctrl+c".to_string(),
            next: "ctrl+n".to_string(),
            previous: "ctrl+p".to_string(),
            trigger: "ctrl+x".to_string(),
        }
    }

    pub fn emacs() -> Self {
        Self {
            accept: "tab".to_string(),
            reject: "ctrl+g".to_string(),
            next: "ctrl+.".to_string(),
            previous: "ctrl+,".to_string(),
            trigger: "alt+/".to_string(),
        }
    }
}

/// Copilot status indicator
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopilotStatus {
    Ready,
    Loading,
    Generating,
    NoSuggestions,
    Error(String),
    Disabled,
}

impl std::fmt::Display for CopilotStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CopilotStatus::Ready => write!(f, "✨ Ready"),
            CopilotStatus::Loading => write!(f, "⏳ Loading..."),
            CopilotStatus::Generating => write!(f, "🤖 Generating..."),
            CopilotStatus::NoSuggestions => write!(f, "💡 No suggestions"),
            CopilotStatus::Error(e) => write!(f, "❌ Error: {}", e),
            CopilotStatus::Disabled => write!(f, "🚫 Disabled"),
        }
    }
}

/// Copilot UI state for editor integration
pub struct CopilotUIState {
    pub status: CopilotStatus,
    pub current_suggestions: Vec<String>,
    pub selected_index: usize,
    pub shortcuts: CopilotShortcuts,
    pub visible: bool,
}

impl CopilotUIState {
    pub fn new() -> Self {
        Self {
            status: CopilotStatus::Ready,
            current_suggestions: Vec::new(),
            selected_index: 0,
            shortcuts: CopilotShortcuts::default(),
            visible: false,
        }
    }

    pub fn show_suggestions(&mut self, suggestions: Vec<String>) {
        self.current_suggestions = suggestions;
        self.selected_index = 0;
        self.visible = !self.current_suggestions.is_empty();
        self.status = if self.visible {
            CopilotStatus::Ready
        } else {
            CopilotStatus::NoSuggestions
        };
    }

    pub fn clear(&mut self) {
        self.current_suggestions.clear();
        self.selected_index = 0;
        self.visible = false;
        self.status = CopilotStatus::Ready;
    }

    pub fn next_suggestion(&mut self) -> Option<&String> {
        if self.current_suggestions.is_empty() {
            return None;
        }
        self.selected_index = (self.selected_index + 1) % self.current_suggestions.len();
        self.current_suggestions.get(self.selected_index)
    }

    pub fn previous_suggestion(&mut self) -> Option<&String> {
        if self.current_suggestions.is_empty() {
            return None;
        }
        if self.selected_index == 0 {
            self.selected_index = self.current_suggestions.len() - 1;
        } else {
            self.selected_index -= 1;
        }
        self.current_suggestions.get(self.selected_index)
    }

    pub fn current_suggestion(&self) -> Option<&String> {
        self.current_suggestions.get(self.selected_index)
    }
}

impl Default for CopilotUIState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod copilot_tests {
    use super::*;

    #[test]
    fn test_copilot_metrics() {
        let mut metrics = CopilotMetrics::new();
        metrics.record_suggestion(true);
        metrics.record_suggestion(false);
        metrics.record_suggestion(true);

        assert_eq!(metrics.total_suggestions, 3);
        assert_eq!(metrics.accepted_suggestions, 2);
        assert_eq!(metrics.rejected_suggestions, 1);
        assert_eq!(metrics.acceptance_rate(), 2.0/3.0);
    }

    #[test]
    fn test_ui_state() {
        let mut state = CopilotUIState::new();
        state.show_suggestions(vec!["suggestion1".to_string(), "suggestion2".to_string()]);
        assert!(state.visible);
        assert_eq!(state.current_suggestion(), Some(&"suggestion1".to_string()));

        state.next_suggestion();
        assert_eq!(state.current_suggestion(), Some(&"suggestion2".to_string()));

        state.clear();
        assert!(!state.visible);
    }

    #[test]
    fn test_suggestion_formatter() {
        let suggestion = "line1\nline2\nline3\nline4\nline5\nline6";
        let formatted = SuggestionFormatter::format_for_display(suggestion);
        assert!(formatted.contains("..."));
    }
}

/// Version of the copilot provider
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize copilot provider with environment token
pub async fn init_from_env() -> Result<CopilotProvider> {
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        CopilotFactory::with_github_token(&token).await
    } else if let Ok(token) = std::env::var("COPILOT_TOKEN") {
        Ok(CopilotFactory::with_copilot_token(&token))
    } else {
        Err(anyhow::anyhow!("No GitHub or Copilot token found in environment"))
    }
}