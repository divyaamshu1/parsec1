//! AI integration for Parsec IDE
//!
//! Provides a unified interface for various AI providers (OpenAI, Anthropic, etc.)
//! with support for completions, chat, and code analysis.

#![allow(dead_code, unused_imports, unused_variables, unused_mut, ambiguous_glob_reexports, mismatched_lifetime_syntaxes)]

pub mod providers;
pub mod context;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio::sync::{RwLock, Mutex};
use serde::{Serialize, Deserialize};

pub use providers::*;
pub use context::*;

/// Main AI engine for Parsec
pub struct AIEngine {
    /// Available providers
    providers: Arc<RwLock<HashMap<String, Box<dyn AIProvider>>>>,
    /// Active provider
    active_provider: Arc<RwLock<Option<String>>>,
    /// Configuration
    config: AIConfig,
    /// Provider manager for caching and advanced features (wrapped in Mutex for interior mutability)
    provider_manager: Arc<Mutex<providers::ProviderManager>>,
}

/// AI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub default_provider: String,
    pub default_model: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub enable_caching: bool,
    pub cache_size: usize,
    pub timeout_seconds: u64,
    pub retry_count: usize,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            default_provider: "openai".to_string(),
            default_model: "gpt-4".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            enable_caching: true,
            cache_size: 100,
            timeout_seconds: 30,
            retry_count: 3,
        }
    }
}

/// AI provider trait - all providers must implement this
#[async_trait]
pub trait AIProvider: Send + Sync {
    fn name(&self) -> &str;
    fn capabilities(&self) -> ProviderCapabilities;
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    async fn complete_stream(&self, request: CompletionRequest) -> Result<CompletionStream>;
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
    async fn chat_stream(&self, request: ChatRequest) -> Result<ChatStream>;
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse>;
    async fn is_available(&self) -> bool;
    
    /// Clone boxed provider (for internal use)
    fn box_clone(&self) -> Box<dyn AIProvider>;
}

/// Provider capabilities
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub provider_id: String,
    pub models: Vec<ModelInfo>,
    pub streaming: bool,
    pub functions: bool,
    pub vision: bool,
    pub embeddings: bool,
    pub max_context_length: usize,
    pub cost_per_1k_input: f64,
    pub cost_per_1k_output: f64,
}

/// Model information
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub context_length: usize,
    pub max_output_tokens: Option<usize>,
    pub streaming: bool,
    pub functions: bool,
    pub vision: bool,
}

/// Completion request
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub stream: bool,
}

/// Completion response
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub text: String,
    pub model: String,
    pub usage: TokenUsage,
    pub finish_reason: Option<String>,
}

/// Chat request
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model: Option<String>,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

/// Chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub name: Option<String>,
}

/// Chat role
#[derive(Debug, Clone)]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// Chat response
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub model: String,
    pub usage: TokenUsage,
    pub finish_reason: Option<String>,
}

/// Embedding request
#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    pub input: Vec<String>,
    pub model: Option<String>,
}

/// Embedding response
#[derive(Debug, Clone)]
pub struct EmbeddingResponse {
    pub embeddings: Vec<Vec<f32>>,
    pub model: String,
    pub usage: TokenUsage,
}

/// Token usage
#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// Completion stream chunk
#[derive(Debug, Clone)]
pub struct CompletionChunk {
    pub text: String,
    pub finish_reason: Option<String>,
}

/// Chat stream chunk
#[derive(Debug, Clone)]
pub struct ChatChunk {
    pub delta: ChatDelta,
    pub finish_reason: Option<String>,
}

/// Chat delta for streaming
#[derive(Debug, Clone)]
pub struct ChatDelta {
    pub role: Option<ChatRole>,
    pub content: Option<String>,
}

/// Stream types
pub type CompletionStream = futures::stream::BoxStream<'static, Result<CompletionChunk>>;
pub type ChatStream = futures::stream::BoxStream<'static, Result<ChatChunk>>;

impl AIEngine {
    /// Create a new AI engine
    pub fn new(config: AIConfig) -> Self {
        let provider_manager = Arc::new(Mutex::new(providers::ProviderManager::new()));
        
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            active_provider: Arc::new(RwLock::new(Some(config.default_provider.clone()))),
            config,
            provider_manager,
        }
    }

    /// Register an AI provider
    pub async fn register_provider(&self, provider: Box<dyn AIProvider>) {
        let name = provider.name().to_string();
        self.providers.write().await.insert(name.clone(), provider);
        
        // Create a new provider instance for the manager
        let config = ProviderConfig::new(&name);
        if let Ok(provider_instance) = ProviderFactory::create(config) {
            // Lock the mutex to get mutable access
            let mut manager = self.provider_manager.lock().await;
            manager.add(&name, provider_instance);
        }
    }

    /// Set active provider
    pub async fn set_active_provider(&self, name: &str) -> Result<()> {
        let providers = self.providers.read().await;
        if providers.contains_key(name) {
            *self.active_provider.write().await = Some(name.to_string());
            
            // Lock the mutex to get mutable access
            let mut manager = self.provider_manager.lock().await;
            manager.set_active(name)?;
            
            Ok(())
        } else {
            Err(anyhow!("Provider not found: {}", name))
        }
    }

    /// Get active provider
    pub async fn active_provider(&self) -> Result<Box<dyn AIProvider>> {
        let active = self.active_provider.read().await.clone();
        let name = active.ok_or_else(|| anyhow!("No active provider"))?;
        
        let providers = self.providers.read().await;
        let provider = providers.get(&name)
            .ok_or_else(|| anyhow!("Active provider not found: {}", name))?;
        
        // Use box_clone to get a new boxed instance
        Ok(provider.box_clone())
    }

    /// List available providers
    pub async fn list_providers(&self) -> Vec<String> {
        self.providers.read().await.keys().cloned().collect()
    }

    /// Complete text (with optional caching)
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let active = self.active_provider.read().await.clone();
        let name = active.ok_or_else(|| anyhow!("No active provider"))?;
        
        if self.config.enable_caching {
            let manager = self.provider_manager.lock().await;
            manager.complete_with_cache(&name, request).await
        } else {
            let provider = self.active_provider().await?;
            provider.complete(request).await
        }
    }

    /// Complete text with streaming
    pub async fn complete_stream(&self, request: CompletionRequest) -> Result<CompletionStream> {
        let provider = self.active_provider().await?;
        provider.complete_stream(request).await
    }

    /// Chat completion (with optional caching)
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let active = self.active_provider.read().await.clone();
        let name = active.ok_or_else(|| anyhow!("No active provider"))?;
        
        if self.config.enable_caching {
            let manager = self.provider_manager.lock().await;
            manager.chat_with_cache(&name, request).await
        } else {
            let provider = self.active_provider().await?;
            provider.chat(request).await
        }
    }

    /// Chat completion with streaming
    pub async fn chat_stream(&self, request: ChatRequest) -> Result<ChatStream> {
        let provider = self.active_provider().await?;
        provider.chat_stream(request).await
    }

    /// Generate embeddings
    pub async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        let provider = self.active_provider().await?;
        provider.embed(request).await
    }

    /// Clear response cache
    pub async fn clear_cache(&self) {
        let manager = self.provider_manager.lock().await;
        manager.clear_cache().await;
    }

    /// Get cache size
    pub async fn cache_size(&self) -> usize {
        let manager = self.provider_manager.lock().await;
        manager.cache_size().await
    }

    /// Get provider manager reference
    pub fn provider_manager(&self) -> Arc<Mutex<providers::ProviderManager>> {
        self.provider_manager.clone()
    }
}

/// Implement box_clone for each provider type
/// Add this to each provider's implementation file

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = AIConfig::default();
        assert_eq!(config.default_provider, "openai");
        assert_eq!(config.default_model, "gpt-4");
        assert_eq!(config.enable_caching, true);
        assert_eq!(config.cache_size, 100);
    }

    #[tokio::test]
    async fn test_ai_engine_creation() {
        let engine = AIEngine::new(AIConfig::default());
        assert_eq!(engine.list_providers().await.len(), 0);
        assert_eq!(engine.cache_size().await, 0);
    }
}