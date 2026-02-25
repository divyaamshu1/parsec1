//! Local LLM provider (Ollama, llama.cpp)

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

pub struct LocalProvider {
    config: ProviderConfig,
    client: Client,
    models: Vec<ModelInfo>,
    rate_limiter: Arc<Mutex<RateLimiter>>,
}

impl LocalProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let models = vec![
            ModelInfo {
                id: "llama2".to_string(),
                name: "Llama 2".to_string(),
                description: Some("Meta's Llama 2 model".to_string()),
                context_length: 4096,
                max_output_tokens: Some(2048),
                streaming: true,
                functions: false,
                vision: false,
            },
            ModelInfo {
                id: "codellama".to_string(),
                name: "Code Llama".to_string(),
                description: Some("Code-specialized Llama".to_string()),
                context_length: 16384,
                max_output_tokens: Some(4096),
                streaming: true,
                functions: false,
                vision: false,
            },
            ModelInfo {
                id: "mistral".to_string(),
                name: "Mistral".to_string(),
                description: Some("Mistral 7B model".to_string()),
                context_length: 8192,
                max_output_tokens: Some(2048),
                streaming: true,
                functions: false,
                vision: false,
            },
        ];

        Self {
            config,
            client: Client::new(),
            models,
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(120))), // Higher limit for local (no API cost)
        }
    }

    fn base_url(&self) -> String {
        self.config.api_url.clone().unwrap_or_else(|| "http://localhost:11434".to_string())
    }

    fn model(&self) -> String {
        self.config.default_model.clone().unwrap_or_else(|| "llama2".to_string())
    }
}

#[async_trait]
impl AIProvider for LocalProvider {
    fn name(&self) -> &str {
        "local"
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
            provider_id: "local".to_string(),
            models: self.models.clone(),
            streaming: true,
            functions: false,
            vision: false,
            embeddings: true,
            max_context_length: 16384,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
        }
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let url = format!("{}/api/generate", self.base_url());
        
        let body = json!({
            "model": request.model.as_deref().unwrap_or(&self.model()),
            "prompt": request.prompt,
            "stream": false,
            "options": {
                "temperature": request.temperature.unwrap_or(0.7),
            }
        });

        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Local API error: {}", response.status()));
        }

        #[derive(serde::Deserialize)]
        struct LocalResponse {
            response: String,
            prompt_eval_count: Option<usize>,
            eval_count: Option<usize>,
        }

        let data: LocalResponse = response.json().await?;

        Ok(CompletionResponse {
            text: data.response,
            model: request.model.unwrap_or_else(|| self.model()),
            usage: TokenUsage {
                prompt_tokens: data.prompt_eval_count.unwrap_or(0),
                completion_tokens: data.eval_count.unwrap_or(0),
                total_tokens: data.prompt_eval_count.unwrap_or(0) + data.eval_count.unwrap_or(0),
            },
            finish_reason: None,
        })
    }

    async fn complete_stream(&self, request: CompletionRequest) -> Result<CompletionStream> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let url = format!("{}/api/generate", self.base_url());
        
        let body = json!({
            "model": request.model.as_deref().unwrap_or(&self.model()),
            "prompt": request.prompt,
            "stream": true,
            "options": {
                "temperature": request.temperature.unwrap_or(0.7),
            }
        });

        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Local API error: {}", response.status()));
        }

        let stream = response.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk).to_string();
            
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(response) = json["response"].as_str() {
                    return Ok(CompletionChunk {
                        text: response.to_string(),
                        finish_reason: if json["done"].as_bool().unwrap_or(false) {
                            Some("stop".to_string())
                        } else {
                            None
                        },
                    });
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
        
        let url = format!("{}/api/chat", self.base_url());
        
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
            "stream": false,
            "options": {
                "temperature": request.temperature.unwrap_or(0.7),
            }
        });

        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Local API error: {}", response.status()));
        }

        #[derive(serde::Deserialize)]
        struct LocalChatResponse {
            message: LocalMessage,
            prompt_eval_count: Option<usize>,
            eval_count: Option<usize>,
        }

        #[derive(serde::Deserialize)]
        struct LocalMessage {
            role: String,
            content: String,
        }

        let data: LocalChatResponse = response.json().await?;

        Ok(ChatResponse {
            message: crate::ChatMessage {
                role: ChatRole::Assistant,
                content: data.message.content,
                name: None,
            },
            model: request.model.unwrap_or_else(|| self.model()),
            usage: TokenUsage {
                prompt_tokens: data.prompt_eval_count.unwrap_or(0),
                completion_tokens: data.eval_count.unwrap_or(0),
                total_tokens: data.prompt_eval_count.unwrap_or(0) + data.eval_count.unwrap_or(0),
            },
            finish_reason: None,
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<ChatStream> {
        // Apply rate limiting
        self.rate_limiter.lock().await.acquire().await?;
        
        let url = format!("{}/api/chat", self.base_url());
        
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
            "stream": true,
            "options": {
                "temperature": request.temperature.unwrap_or(0.7),
            }
        });

        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Local API error: {}", response.status()));
        }

        let stream = response.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk).to_string();
            
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(message) = json.get("message") {
                    if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                        return Ok(ChatChunk {
                            delta: ChatDelta {
                                role: Some(ChatRole::Assistant),
                                content: Some(content.to_string()),
                            },
                            finish_reason: if json["done"].as_bool().unwrap_or(false) {
                                Some("stop".to_string())
                            } else {
                                None
                            },
                        });
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
        
        let url = format!("{}/api/embeddings", self.base_url());
        
        let mut embeddings = Vec::new();

        for text in request.input {
            let body = json!({
                "model": request.model.as_deref().unwrap_or(&self.model()),
                "prompt": text,
            });

            let response = self.client
                .post(&url)
                .json(&body)
                .send()
                .await?;

            if !response.status().is_success() {
                return Err(anyhow!("Local embedding error: {}", response.status()));
            }

            #[derive(serde::Deserialize)]
            struct LocalEmbedding {
                embedding: Vec<f32>,
            }

            let data: LocalEmbedding = response.json().await?;
            embeddings.push(data.embedding);
        }

        Ok(EmbeddingResponse {
            embeddings,
            model: request.model.unwrap_or_else(|| self.model()),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
        })
    }

    async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url());
        self.client.get(&url).send().await.is_ok()
    }
}