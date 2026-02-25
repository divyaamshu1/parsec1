//! Anthropic Claude provider implementation

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

pub struct AnthropicProvider {
    config: ProviderConfig,
    client: Client,
    models: Vec<ModelInfo>,
    rate_limiter: Arc<Mutex<RateLimiter>>,
}

impl AnthropicProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let models = vec![
            ModelInfo {
                id: "claude-3-opus-20240229".to_string(),
                name: "Claude 3 Opus".to_string(),
                description: Some("Most powerful Claude 3 model".to_string()),
                context_length: 200000,
                max_output_tokens: Some(4096),
                streaming: true,
                functions: false,
                vision: true,
            },
            ModelInfo {
                id: "claude-3-sonnet-20240229".to_string(),
                name: "Claude 3 Sonnet".to_string(),
                description: Some("Balanced Claude 3 model".to_string()),
                context_length: 200000,
                max_output_tokens: Some(4096),
                streaming: true,
                functions: false,
                vision: true,
            },
            ModelInfo {
                id: "claude-3-haiku-20240307".to_string(),
                name: "Claude 3 Haiku".to_string(),
                description: Some("Fastest Claude 3 model".to_string()),
                context_length: 200000,
                max_output_tokens: Some(4096),
                streaming: true,
                functions: false,
                vision: true,
            },
        ];

        Self {
            config,
            client: Client::new(),
            models,
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(60))), // 60 requests per minute
        }
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        
        if let Some(key) = &self.config.api_key {
            headers.insert("x-api-key", key.parse().unwrap());
        }
        
        headers.insert("anthropic-version", "2023-06-01".parse().unwrap());
        headers.insert("content-type", "application/json".parse().unwrap());
        headers
    }

    fn model(&self) -> String {
        self.config.default_model.clone().unwrap_or_else(|| "claude-3-opus-20240229".to_string())
    }
}

#[async_trait]
impl AIProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn box_clone(&self) -> Box<dyn AIProvider> {
        Box::new(Self {
            config: self.config.clone(),
            client: self.client.clone(),
            models: self.models.clone(),
            rate_limiter: self.rate_limiter.clone(),
        })
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider_id: "anthropic".to_string(),
            models: self.models.clone(),
            streaming: true,
            functions: false,
            vision: true,
            embeddings: false,
            max_context_length: 200000,
            cost_per_1k_input: 0.015,
            cost_per_1k_output: 0.075,
        }
    }

    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
        Err(anyhow!("Anthropic doesn't support pure completion"))
    }

    async fn complete_stream(&self, _request: CompletionRequest) -> Result<CompletionStream> {
        Err(anyhow!("Anthropic doesn't support pure completion"))
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let url = "https://api.anthropic.com/v1/messages";
        
        let mut messages = Vec::new();
        let mut system = None;

        for msg in &request.messages {
            match msg.role {
                ChatRole::System => system = Some(msg.content.clone()),
                _ => {
                    messages.push(json!({
                        "role": match msg.role {
                            ChatRole::User => "user",
                            ChatRole::Assistant => "assistant",
                            _ => "user",
                        },
                        "content": msg.content,
                    }));
                }
            }
        }

        let mut body = json!({
            "model": request.model.as_deref().unwrap_or(&self.model()),
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(256),
            "temperature": request.temperature.unwrap_or(0.7),
            "stream": false,
        });

        if let Some(s) = system {
            body["system"] = json!(s);
        }

        let response = self.client
            .post(url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("Anthropic API error: {}", error));
        }

        #[derive(serde::Deserialize)]
        struct AnthropicResponse {
            id: String,
            model: String,
            content: Vec<AnthropicContent>,
            stop_reason: String,
            usage: AnthropicUsage,
        }

        #[derive(serde::Deserialize)]
        struct AnthropicContent {
            text: String,
        }

        #[derive(serde::Deserialize)]
        struct AnthropicUsage {
            input_tokens: usize,
            output_tokens: usize,
        }

        let data: AnthropicResponse = response.json().await?;
        let text = data.content.iter().map(|c| c.text.clone()).collect::<Vec<_>>().join("\n");

        Ok(ChatResponse {
            message: crate::ChatMessage {
                role: ChatRole::Assistant,
                content: text,
                name: None,
            },
            model: data.model,
            usage: TokenUsage {
                prompt_tokens: data.usage.input_tokens,
                completion_tokens: data.usage.output_tokens,
                total_tokens: data.usage.input_tokens + data.usage.output_tokens,
            },
            finish_reason: Some(data.stop_reason),
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<ChatStream> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let url = "https://api.anthropic.com/v1/messages";
        
        let messages: Vec<serde_json::Value> = request.messages.iter()
            .filter(|msg| !matches!(msg.role, ChatRole::System))
            .map(|msg| {
                json!({
                    "role": match msg.role {
                        ChatRole::User => "user",
                        ChatRole::Assistant => "assistant",
                        _ => "user",
                    },
                    "content": msg.content,
                })
            })
            .collect();

        let body = json!({
            "model": request.model.as_deref().unwrap_or(&self.model()),
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(256),
            "stream": true,
        });

        let response = self.client
            .post(url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Anthropic API error: {}", response.status()));
        }

        let stream = response.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk).to_string();

            for line in text.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        return Ok(ChatChunk {
                            delta: ChatDelta { role: None, content: None },
                            finish_reason: Some("stop".to_string()),
                        });
                    }

                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(delta) = json.get("delta") {
                            if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                return Ok(ChatChunk {
                                    delta: ChatDelta {
                                        role: Some(ChatRole::Assistant),
                                        content: Some(text.to_string()),
                                    },
                                    finish_reason: None,
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
        Err(anyhow!("Anthropic doesn't support embeddings"))
    }

    async fn is_available(&self) -> bool {
        self.config.api_key.is_some()
    }
}