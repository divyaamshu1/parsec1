//! OpenAI provider implementation

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

/// OpenAI token usage response
#[derive(serde::Deserialize, Clone)]
struct OpenAIUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

pub struct OpenAIProvider {
    config: ProviderConfig,
    client: Client,
    models: Vec<ModelInfo>,
    rate_limiter: Arc<Mutex<RateLimiter>>,
}

impl OpenAIProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let models = vec![
            ModelInfo {
                id: "gpt-4".to_string(),
                name: "GPT-4".to_string(),
                description: Some("Most capable GPT-4 model".to_string()),
                context_length: 8192,
                max_output_tokens: Some(4096),
                streaming: true,
                functions: true,
                vision: false,
            },
            ModelInfo {
                id: "gpt-4-turbo".to_string(),
                name: "GPT-4 Turbo".to_string(),
                description: Some("Latest GPT-4 Turbo model".to_string()),
                context_length: 128000,
                max_output_tokens: Some(4096),
                streaming: true,
                functions: true,
                vision: true,
            },
            ModelInfo {
                id: "gpt-3.5-turbo".to_string(),
                name: "GPT-3.5 Turbo".to_string(),
                description: Some("Fast and efficient model".to_string()),
                context_length: 16385,
                max_output_tokens: Some(4096),
                streaming: true,
                functions: true,
                vision: false,
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
            headers.insert(
                "Authorization",
                format!("Bearer {}", key).parse().unwrap()
            );
        }
        
        if let Some(org) = &self.config.organization {
            headers.insert("OpenAI-Organization", org.parse().unwrap());
        }
        
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers
    }

    fn model(&self) -> String {
        self.config.default_model.clone().unwrap_or_else(|| "gpt-4".to_string())
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
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
            provider_id: "openai".to_string(),
            models: self.models.clone(),
            streaming: true,
            functions: true,
            vision: true,
            embeddings: true,
            max_context_length: 128000,
            cost_per_1k_input: 0.01,
            cost_per_1k_output: 0.03,
        }
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let url = "https://api.openai.com/v1/completions";
        
        let body = json!({
            "model": request.model.as_deref().unwrap_or(&self.model()),
            "prompt": request.prompt,
            "max_tokens": request.max_tokens.unwrap_or(256),
            "temperature": request.temperature.unwrap_or(0.7),
            "stop": request.stop,
            "stream": false,
        });

        let response = self.client
            .post(url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("OpenAI API error: {}", error));
        }
        
        #[derive(serde::Deserialize)]
        struct OpenAIResponse {
            choices: Vec<OpenAIChoice>,
            usage: OpenAIUsage,
            model: String,
        }

        #[derive(serde::Deserialize)]
        struct OpenAIChoice {
            text: String,
            finish_reason: String,
        }

        let data: OpenAIResponse = response.json().await?;
        let choice = data.choices.first().ok_or_else(|| anyhow!("No completion choices"))?;

        Ok(CompletionResponse {
            text: choice.text.clone(),
            model: data.model,
            usage: TokenUsage {
                prompt_tokens: data.usage.prompt_tokens,
                completion_tokens: data.usage.completion_tokens,
                total_tokens: data.usage.total_tokens,
            },
            finish_reason: Some(choice.finish_reason.clone()),
        })
    }

    async fn complete_stream(&self, request: CompletionRequest) -> Result<CompletionStream> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let url = "https://api.openai.com/v1/completions";
        
        let body = json!({
            "model": request.model.as_deref().unwrap_or(&self.model()),
            "prompt": request.prompt,
            "max_tokens": request.max_tokens.unwrap_or(256),
            "temperature": request.temperature.unwrap_or(0.7),
            "stop": request.stop,
            "stream": true,
        });

        let response = self.client
            .post(url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("OpenAI API error: {}", response.status()));
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
        
        let url = "https://api.openai.com/v1/chat/completions";
        
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
            "model": request.model.as_deref().unwrap_or(&self.model()),
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(256),
            "temperature": request.temperature.unwrap_or(0.7),
            "stream": false,
        });

        let response = self.client
            .post(url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("OpenAI API error: {}", error));
        }

        #[derive(serde::Deserialize)]
        struct OpenAIChatResponse {
            choices: Vec<OpenAIChatChoice>,
            usage: OpenAIUsage,
            model: String,
        }

        #[derive(serde::Deserialize)]
        struct OpenAIChatChoice {
            message: OpenAIChatMessage,
            finish_reason: String,
        }

        #[derive(serde::Deserialize)]
        struct OpenAIChatMessage {
            role: String,
            content: String,
        }

        let data: OpenAIChatResponse = response.json().await?;
        let choice = data.choices.first().ok_or_else(|| anyhow!("No chat choices"))?;

        Ok(ChatResponse {
            message: crate::ChatMessage {
                role: match choice.message.role.as_str() {
                    "assistant" => ChatRole::Assistant,
                    _ => ChatRole::Assistant,
                },
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
        
        let url = "https://api.openai.com/v1/chat/completions";
        
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
            "model": request.model.as_deref().unwrap_or(&self.model()),
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(256),
            "temperature": request.temperature.unwrap_or(0.7),
            "stream": true,
        });

        let response = self.client
            .post(url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("OpenAI API error: {}", response.status()));
        }

        let stream = response.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk).to_string();
            
            if text.starts_with("data: ") {
                let data = &text[6..];
                if data == "[DONE]" {
                    return Ok(ChatChunk {
                        delta: ChatDelta {
                            role: None,
                            content: None,
                        },
                        finish_reason: Some("stop".to_string()),
                    });
                }
                
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(choices) = json["choices"].as_array() {
                        if let Some(choice) = choices.first() {
                            if let Some(delta) = choice["delta"].as_object() {
                                let role = delta.get("role").and_then(|r| r.as_str()).map(|s| match s {
                                    "assistant" => ChatRole::Assistant,
                                    "user" => ChatRole::User,
                                    "system" => ChatRole::System,
                                    _ => ChatRole::Assistant,
                                });
                                let content = delta.get("content").and_then(|c| c.as_str()).map(|s| s.to_string());
                                
                                return Ok(ChatChunk {
                                    delta: ChatDelta { role, content },
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

    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let url = "https://api.openai.com/v1/embeddings";
        
        let body = json!({
            "model": request.model.as_deref().unwrap_or("text-embedding-ada-002"),
            "input": request.input,
        });

        let response = self.client
            .post(url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("OpenAI API error: {}", error));
        }

        #[derive(serde::Deserialize)]
        struct OpenAIEmbeddingResponse {
            data: Vec<OpenAIEmbedding>,
            usage: OpenAIUsage,
            model: String,
        }

        #[derive(serde::Deserialize)]
        struct OpenAIEmbedding {
            embedding: Vec<f32>,
            index: usize,
        }

        let data: OpenAIEmbeddingResponse = response.json().await?;
        
        let mut embeddings = vec![vec![]; request.input.len()];
        for item in data.data {
            embeddings[item.index] = item.embedding;
        }

        Ok(EmbeddingResponse {
            embeddings,
            model: data.model,
            usage: TokenUsage {
                prompt_tokens: data.usage.prompt_tokens,
                completion_tokens: 0,
                total_tokens: data.usage.total_tokens,
            },
        })
    }

    async fn is_available(&self) -> bool {
        self.config.api_key.is_some()
    }
}