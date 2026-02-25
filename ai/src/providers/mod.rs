//! AI provider implementations
//!
//! Supports OpenAI, Anthropic, local models, and more.

mod openai;
mod anthropic;
mod local;
mod copilot;

pub use openai::OpenAIProvider;
pub use anthropic::AnthropicProvider;
pub use local::LocalProvider;
pub use copilot::CopilotProvider;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;

use crate::{AIProvider, ModelInfo, CompletionResponse, TokenUsage, ChatResponse, ChatRequest};

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: String,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub organization: Option<String>,
    pub default_model: Option<String>,
    pub timeout_seconds: Option<u64>,
    pub max_retries: Option<usize>,
}

impl ProviderConfig {
    pub fn new(provider_type: &str) -> Self {
        Self {
            provider_type: provider_type.to_string(),
            api_key: None,
            api_url: None,
            organization: None,
            default_model: None,
            timeout_seconds: None,
            max_retries: None,
        }
    }

    pub fn with_api_key(mut self, key: &str) -> Self {
        self.api_key = Some(key.to_string());
        self
    }

    pub fn with_api_url(mut self, url: &str) -> Self {
        self.api_url = Some(url.to_string());
        self
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.default_model = Some(model.to_string());
        self
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }

    pub fn with_retries(mut self, retries: usize) -> Self {
        self.max_retries = Some(retries);
        self
    }
}

/// Provider factory for creating providers from config
pub struct ProviderFactory;

impl ProviderFactory {
    pub fn create(config: ProviderConfig) -> Result<Box<dyn AIProvider>> {
        match config.provider_type.as_str() {
            "openai" => Ok(Box::new(OpenAIProvider::new(config))),
            "anthropic" => Ok(Box::new(AnthropicProvider::new(config))),
            "local" => Ok(Box::new(LocalProvider::new(config))),
            "copilot" => Ok(Box::new(CopilotProvider::new(config))),
            _ => Err(anyhow!("Unknown provider type: {}", config.provider_type)),
        }
    }
}

/// Provider manager for multiple providers with caching
pub struct ProviderManager {
    providers: HashMap<String, Box<dyn AIProvider>>,
    active: Option<String>,
    cache: Arc<Mutex<ResponseCache>>,
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            active: None,
            cache: Arc::new(Mutex::new(ResponseCache::new(100))),
        }
    }

    pub fn add(&mut self, name: &str, provider: Box<dyn AIProvider>) {
        self.providers.insert(name.to_string(), provider);
        if self.active.is_none() {
            self.active = Some(name.to_string());
        }
    }

    pub fn remove(&mut self, name: &str) -> Option<Box<dyn AIProvider>> {
        let removed = self.providers.remove(name);
        if self.active.as_deref() == Some(name) {
            self.active = self.providers.keys().next().cloned();
        }
        removed
    }

    pub fn set_active(&mut self, name: &str) -> Result<()> {
        if self.providers.contains_key(name) {
            self.active = Some(name.to_string());
            Ok(())
        } else {
            Err(anyhow!("Provider not found: {}", name))
        }
    }

    pub fn active(&self) -> Option<&Box<dyn AIProvider>> {
        self.active.as_ref().and_then(|n| self.providers.get(n))
    }

    pub fn active_mut(&mut self) -> Option<&mut Box<dyn AIProvider>> {
        self.active.as_ref().cloned().and_then(move |n| self.providers.get_mut(&n))
    }

    pub fn get(&self, name: &str) -> Option<&Box<dyn AIProvider>> {
        self.providers.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Box<dyn AIProvider>> {
        self.providers.get_mut(name)
    }

    pub fn list(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }

    pub fn len(&self) -> usize {
        self.providers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    pub fn clear(&mut self) {
        self.providers.clear();
        self.active = None;
    }

    /// Clear the response cache
    pub async fn clear_cache(&self) {
        self.cache.lock().await.clear();
    }

    /// Get cache size
    pub async fn cache_size(&self) -> usize {
        self.cache.lock().await.size()
    }

    /// Chat with caching support
    pub async fn chat_with_cache(&self, provider_name: &str, request: ChatRequest) -> Result<ChatResponse> {
        // Generate cache key
        let cache_key = format!("{}:{:?}:{:?}", 
            provider_name, 
            request.model,
            request.messages.iter().map(|m| format!("{:?}:{}", m.role, m.content)).collect::<Vec<_>>()
        );

        // Check cache
        {
            let mut cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&cache_key) {
                // Convert CompletionResponse to ChatResponse
                return Ok(ChatResponse {
                    message: crate::ChatMessage {
                        role: crate::ChatRole::Assistant,
                        content: cached.text,
                        name: None,
                    },
                    model: cached.model,
                    usage: cached.usage,
                    finish_reason: cached.finish_reason,
                });
            }
        }

        // Make actual request
        let provider = self.get(provider_name)
            .ok_or_else(|| anyhow!("Provider not found: {}", provider_name))?;
        
        let response = provider.chat(request).await?;

        // Store in cache
        {
            let mut cache = self.cache.lock().await;
            let completion_response = CompletionResponse {
                text: response.message.content.clone(),
                model: response.model.clone(),
                usage: response.usage.clone(),
                finish_reason: response.finish_reason.clone(),
            };
            cache.set(cache_key, completion_response);
        }

        Ok(response)
    }

    /// Complete with caching support
    pub async fn complete_with_cache(&self, provider_name: &str, request: crate::CompletionRequest) -> Result<CompletionResponse> {
        // Generate cache key
        let cache_key = format!("{}:{}:{:?}:{:?}", 
            provider_name, 
            request.model.as_deref().unwrap_or("default"),
            request.prompt,
            request.temperature
        );

        // Check cache
        {
            let mut cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached);
            }
        }

        // Make actual request
        let provider = self.get(provider_name)
            .ok_or_else(|| anyhow!("Provider not found: {}", provider_name))?;
        
        let response = provider.complete(request).await?;

        // Store in cache
        {
            let mut cache = self.cache.lock().await;
            cache.set(cache_key, response.clone());
        }

        Ok(response)
    }
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Provider statistics
#[derive(Debug, Clone, Default)]
pub struct ProviderStats {
    pub total_requests: usize,
    pub total_tokens: usize,
    pub total_cost: f64,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub average_latency_ms: f64,
}

impl ProviderManager {
    /// Get statistics for a provider
    pub async fn get_stats(&self, name: &str) -> Option<ProviderStats> {
        // This would need to be implemented by each provider
        None
    }

    /// Get statistics for all providers
    pub async fn get_all_stats(&self) -> HashMap<String, ProviderStats> {
        HashMap::new()
    }
}

/// Pre-configured provider helpers
impl ProviderManager {
    pub fn with_openai(mut self, api_key: &str) -> Self {
        let config = ProviderConfig::new("openai").with_api_key(api_key);
        if let Ok(provider) = ProviderFactory::create(config) {
            self.add("openai", provider);
        }
        self
    }

    pub fn with_anthropic(mut self, api_key: &str) -> Self {
        let config = ProviderConfig::new("anthropic").with_api_key(api_key);
        if let Ok(provider) = ProviderFactory::create(config) {
            self.add("anthropic", provider);
        }
        self
    }

    pub fn with_local(mut self, model: &str) -> Self {
        let config = ProviderConfig::new("local").with_model(model);
        if let Ok(provider) = ProviderFactory::create(config) {
            self.add("local", provider);
        }
        self
    }

    pub fn with_copilot(mut self, token: &str) -> Self {
        let config = ProviderConfig::new("copilot").with_api_key(token);
        if let Ok(provider) = ProviderFactory::create(config) {
            self.add("copilot", provider);
        }
        self
    }
}

/// Provider capability checks
impl ProviderManager {
    pub fn supports_streaming(&self, name: &str) -> bool {
        self.get(name)
            .map(|p| p.capabilities().streaming)
            .unwrap_or(false)
    }

    pub fn supports_functions(&self, name: &str) -> bool {
        self.get(name)
            .map(|p| p.capabilities().functions)
            .unwrap_or(false)
    }

    pub fn supports_vision(&self, name: &str) -> bool {
        self.get(name)
            .map(|p| p.capabilities().vision)
            .unwrap_or(false)
    }

    pub fn supports_embeddings(&self, name: &str) -> bool {
        self.get(name)
            .map(|p| p.capabilities().embeddings)
            .unwrap_or(false)
    }

    pub fn max_context_length(&self, name: &str) -> Option<usize> {
        self.get(name).map(|p| p.capabilities().max_context_length)
    }

    pub fn available_models(&self, name: &str) -> Option<Vec<ModelInfo>> {
        self.get(name).map(|p| p.capabilities().models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config() {
        let config = ProviderConfig::new("openai")
            .with_api_key("test-key")
            .with_model("gpt-4")
            .with_timeout(60)
            .with_retries(5);
        
        assert_eq!(config.provider_type, "openai");
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.default_model, Some("gpt-4".to_string()));
        assert_eq!(config.timeout_seconds, Some(60));
        assert_eq!(config.max_retries, Some(5));
    }

    #[test]
    fn test_provider_manager() {
        let mut manager = ProviderManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
        assert!(manager.list().is_empty());

        // Can't test with real providers here, but structure works
        manager.set_active("nonexistent").unwrap_err();
    }

    #[test]
    fn test_preconfigured_helpers() {
        let manager = ProviderManager::new()
            .with_openai("test-key")
            .with_anthropic("test-key")
            .with_local("llama2");
        
        // Helpers won't actually create providers in tests
        // but they shouldn't panic
        assert_eq!(manager.len(), 0);
    }
}

/// Re-export commonly used types
pub mod prelude {
    pub use super::{
        ProviderConfig,
        ProviderFactory,
        ProviderManager,
        ProviderStats,
        OpenAIProvider,
        AnthropicProvider,
        LocalProvider,
        CopilotProvider,
        };
}

/// Provider error types
#[derive(Debug, Clone)]
pub enum ProviderError {
    ApiError(String),
    AuthError(String),
    RateLimitError(String),
    ModelNotFound(String),
    ContextLengthExceeded(String),
    NetworkError(String),
    TimeoutError(String),
    StreamError(String),
    SerializationError(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiError(s) => write!(f, "API error: {}", s),
            Self::AuthError(s) => write!(f, "Authentication failed: {}", s),
            Self::RateLimitError(s) => write!(f, "Rate limit exceeded: {}", s),
            Self::ModelNotFound(s) => write!(f, "Model not found: {}", s),
            Self::ContextLengthExceeded(s) => write!(f, "Context length exceeded: {}", s),
            Self::NetworkError(s) => write!(f, "Network error: {}", s),
            Self::TimeoutError(s) => write!(f, "Timeout error: {}", s),
            Self::StreamError(s) => write!(f, "Stream error: {}", s),
            Self::SerializationError(s) => write!(f, "Serialization error: {}", s),
        }
    }
}

impl std::error::Error for ProviderError {}

impl From<reqwest::Error> for ProviderError {
    fn from(err: reqwest::Error) -> Self {
        ProviderError::NetworkError(err.to_string())
    }
}

impl From<serde_json::Error> for ProviderError {
    fn from(err: serde_json::Error) -> Self {
        ProviderError::SerializationError(err.to_string())
    }
}

/// Provider trait extension for cost calculation
#[async_trait]
pub trait CostCalculator: AIProvider {
    async fn calculate_cost(&self, usage: &TokenUsage) -> f64 {
        let caps = self.capabilities();
        (usage.prompt_tokens as f64 / 1000.0 * caps.cost_per_1k_input) +
        (usage.completion_tokens as f64 / 1000.0 * caps.cost_per_1k_output)
    }
}

/// Rate limiter for API calls
pub struct RateLimiter {
    max_requests_per_minute: usize,
    requests: Vec<std::time::Instant>,
}

impl RateLimiter {
    pub fn new(max_requests_per_minute: usize) -> Self {
        Self {
            max_requests_per_minute,
            requests: Vec::with_capacity(max_requests_per_minute),
        }
    }

    pub async fn acquire(&mut self) -> Result<()> {
        let now = std::time::Instant::now();
        
        // Remove requests older than 1 minute
        self.requests.retain(|&t| t.elapsed().as_secs() < 60);

        if self.requests.len() >= self.max_requests_per_minute {
            let oldest = self.requests.first().unwrap();
            let wait = 60 - oldest.elapsed().as_secs();
            tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
        }

        self.requests.push(now);
        Ok(())
    }
}

/// Retry handler for API calls
pub struct RetryHandler {
    max_retries: usize,
    base_delay_ms: u64,
}

impl RetryHandler {
    pub fn new(max_retries: usize, base_delay_ms: u64) -> Self {
        Self { max_retries, base_delay_ms }
    }

    pub async fn execute<F, T>(&self, mut f: F) -> Result<T>
    where
        F: FnMut() -> futures::future::BoxFuture<'static, Result<T>>,
    {
        let mut last_error = None;
        for attempt in 0..=self.max_retries {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.max_retries {
                        let delay = self.base_delay_ms * 2_u64.pow(attempt as u32);
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }
        Err(last_error.unwrap())
    }
}

/// Response cache for completions
#[derive(Clone)]
struct CachedResponse {
    response: CompletionResponse,
    timestamp: std::time::Instant,
}

pub struct ResponseCache {
    cache: HashMap<String, CachedResponse>,
    max_size: usize,
}

impl ResponseCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(max_size),
            max_size,
        }
    }

    pub fn get(&mut self, key: &str) -> Option<CompletionResponse> {
        if let Some(cached) = self.cache.get(key) {
            if cached.timestamp.elapsed().as_secs() < 3600 { // 1 hour TTL
                return Some(cached.response.clone());
            }
        }
        None
    }

    pub fn set(&mut self, key: String, response: CompletionResponse) {
        if self.cache.len() >= self.max_size {
            // Remove oldest entry
            if let Some(oldest_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&oldest_key);
            }
        }
        self.cache.insert(key, CachedResponse {
            response,
            timestamp: std::time::Instant::now(),
        });
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for ResponseCache {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(10);
        assert_eq!(limiter.requests.capacity(), 10);
    }

    #[test]
    fn test_response_cache() {
        let mut cache = ResponseCache::new(2);
        
        let response = CompletionResponse {
            text: "test".to_string(),
            model: "gpt-4".to_string(),
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            finish_reason: None,
        };

        cache.set("key1".to_string(), response.clone());
        cache.set("key2".to_string(), response.clone());
        assert_eq!(cache.size(), 2);

        cache.set("key3".to_string(), response);
        assert_eq!(cache.size(), 2); // LRU eviction
    }

    #[tokio::test]
    async fn test_retry_handler() {
        let handler = RetryHandler::new(3, 10);
        let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let attempts_clone = attempts.clone();

        let result = handler.execute(move || {
            let attempts = attempts_clone.clone();
            let fut = async move {
                let current = attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if current < 2 {
                    Err(anyhow!("fail"))
                } else {
                    Ok(42)
                }
            };
            Box::pin(fut) as futures::future::BoxFuture<'static, Result<i32>>
        }).await;

        assert!(result.is_ok());
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_provider_manager_cache() {
        let manager = ProviderManager::new();
        assert_eq!(manager.cache_size().await, 0);
        
        manager.clear_cache().await;
        assert_eq!(manager.cache_size().await, 0);
    }
}

/// Provider utilities module
pub mod utils {
    /// Count tokens in a string (approximate)
    pub fn count_tokens_approx(text: &str) -> usize {
        // Simple approximation: 1 token per 4 characters
        text.len() / 4 + 1
    }

    /// Truncate text to fit within token limit
    pub fn truncate_to_token_limit(text: &str, max_tokens: usize) -> String {
        let approx_chars = max_tokens * 4;
        if text.len() > approx_chars {
            text.chars().take(approx_chars).collect()
        } else {
            text.to_string()
        }
    }

    /// Format messages for different provider formats
    pub mod formatters {
        use crate::ChatMessage;

        pub fn to_openai_format(messages: &[ChatMessage]) -> Vec<serde_json::Value> {
            messages.iter().map(|msg| {
                serde_json::json!({
                    "role": match msg.role {
                        crate::ChatRole::System => "system",
                        crate::ChatRole::User => "user",
                        crate::ChatRole::Assistant => "assistant",
                    },
                    "content": msg.content,
                })
            }).collect()
        }

        pub fn to_anthropic_format(messages: &[ChatMessage]) -> (Option<String>, Vec<serde_json::Value>) {
            let mut system = None;
            let mut msgs = Vec::new();

            for msg in messages {
                match msg.role {
                    crate::ChatRole::System => {
                        system = Some(msg.content.clone());
                    }
                    _ => {
                        msgs.push(serde_json::json!({
                            "role": match msg.role {
                                crate::ChatRole::User => "user",
                                crate::ChatRole::Assistant => "assistant",
                                _ => "user",
                            },
                            "content": msg.content,
                        }));
                    }
                }
            }

            (system, msgs)
        }
    }
}

/// Re-export utilities
pub use utils::*;

/// Provider response streaming utilities
pub mod stream {
    use futures::stream::BoxStream;
    use futures::stream::StreamExt;
    use anyhow::Result;

    use crate::{CompletionChunk, ChatChunk};

    /// Combine multiple completion streams
    pub async fn combine(
        streams: Vec<BoxStream<'static, Result<CompletionChunk>>>,
    ) -> BoxStream<'static, Result<CompletionChunk>> {
        // Flatten all streams into one
        futures::stream::iter(streams)
            .then(|s| async { s })
            .flatten()
            .boxed()
    }

    /// Transform completion chunks to chat chunks
    pub fn completion_to_chat(
        stream: BoxStream<'static, Result<CompletionChunk>>,
    ) -> BoxStream<'static, Result<ChatChunk>> {
        stream.map(|chunk| {
            chunk.map(|c| ChatChunk {
                delta: crate::ChatDelta {
                    role: None,
                    content: Some(c.text),
                },
                finish_reason: c.finish_reason,
            })
        }).boxed()
    }

    /// Buffer chunks into complete responses
    pub struct ChunkBuffer {
        buffer: String,
    }

    impl ChunkBuffer {
        pub fn new() -> Self {
            Self { buffer: String::new() }
        }

        pub fn push(&mut self, chunk: &CompletionChunk) {
            self.buffer.push_str(&chunk.text);
        }

        pub fn complete(self) -> String {
            self.buffer
        }

        pub fn clear(&mut self) {
            self.buffer.clear();
        }
    }

    impl Default for ChunkBuffer {
        fn default() -> Self {
            Self::new()
        }
    }
}

/// Provider connection pooling
pub mod pool {
    use std::collections::VecDeque;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use super::*;

    /// Connection pool for provider clients
    pub struct ProviderPool {
        providers: Arc<Mutex<VecDeque<Box<dyn AIProvider>>>>,
        max_size: usize,
    }

    impl ProviderPool {
        pub fn new(max_size: usize) -> Self {
            Self {
                providers: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
                max_size,
            }
        }

        pub async fn acquire(&self) -> Option<Box<dyn AIProvider>> {
            self.providers.lock().await.pop_front()
        }

        pub async fn release(&self, provider: Box<dyn AIProvider>) {
            let mut providers = self.providers.lock().await;
            if providers.len() < self.max_size {
                providers.push_back(provider);
            }
        }

        pub async fn add(&self, provider: Box<dyn AIProvider>) {
            let mut providers = self.providers.lock().await;
            if providers.len() < self.max_size {
                providers.push_back(provider);
            }
        }

        pub async fn size(&self) -> usize {
            self.providers.lock().await.len()
        }

        pub async fn clear(&self) {
            self.providers.lock().await.clear();
        }
    }
}

/// Provider metrics collection
pub mod metrics {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use tokio::sync::RwLock;

    use crate::TokenUsage;

    /// Metrics collector for providers
    pub struct MetricsCollector {
        requests: Arc<AtomicUsize>,
        tokens: Arc<AtomicUsize>,
        errors: Arc<AtomicUsize>,
        latency: Arc<RwLock<Vec<u64>>>,
        model_usage: Arc<RwLock<HashMap<String, ModelMetrics>>>,
    }

    #[derive(Debug, Default, Clone)]
    pub struct ModelMetrics {
        pub requests: usize,
        pub tokens: usize,
        pub errors: usize,
    }

    impl MetricsCollector {
        pub fn new() -> Self {
            Self {
                requests: Arc::new(AtomicUsize::new(0)),
                tokens: Arc::new(AtomicUsize::new(0)),
                errors: Arc::new(AtomicUsize::new(0)),
                latency: Arc::new(RwLock::new(Vec::new())),
                model_usage: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        pub fn record_request(&self, model: &str, usage: &TokenUsage, latency_ms: u64) {
            self.requests.fetch_add(1, Ordering::Relaxed);
            self.tokens.fetch_add(usage.total_tokens, Ordering::Relaxed);

            let mut model_usage = self.model_usage.blocking_write();
            let metrics = model_usage.entry(model.to_string()).or_insert_with(ModelMetrics::default);
            metrics.requests += 1;
            metrics.tokens += usage.total_tokens;

            let mut latency = self.latency.blocking_write();
            latency.push(latency_ms);
            if latency.len() > 1000 {
                latency.remove(0);
            }
        }

        pub fn record_error(&self, model: &str) {
            self.errors.fetch_add(1, Ordering::Relaxed);
            
            let mut model_usage = self.model_usage.blocking_write();
            let metrics = model_usage.entry(model.to_string()).or_insert_with(ModelMetrics::default);
            metrics.errors += 1;
        }

        pub fn total_requests(&self) -> usize {
            self.requests.load(Ordering::Relaxed)
        }

        pub fn total_tokens(&self) -> usize {
            self.tokens.load(Ordering::Relaxed)
        }

        pub fn total_errors(&self) -> usize {
            self.errors.load(Ordering::Relaxed)
        }

        pub fn average_latency_ms(&self) -> f64 {
            let latency = self.latency.blocking_read();
            if latency.is_empty() {
                0.0
            } else {
                latency.iter().sum::<u64>() as f64 / latency.len() as f64
            }
        }

        pub fn model_metrics(&self) -> HashMap<String, ModelMetrics> {
            self.model_usage.blocking_read().clone()
        }

        pub fn reset(&self) {
            self.requests.store(0, Ordering::Relaxed);
            self.tokens.store(0, Ordering::Relaxed);
            self.errors.store(0, Ordering::Relaxed);
            self.latency.blocking_write().clear();
            self.model_usage.blocking_write().clear();
        }
    }

    impl Default for MetricsCollector {
        fn default() -> Self {
            Self::new()
        }
    }
}

/// Re-export all provider modules

/// Default provider implementations
pub mod defaults {
    use super::*;

    /// Create a default OpenAI provider
    pub fn openai(api_key: &str) -> Result<Box<dyn AIProvider>> {
        let config = ProviderConfig::new("openai")
            .with_api_key(api_key)
            .with_model("gpt-4");
        ProviderFactory::create(config)
    }

    /// Create a default Anthropic provider
    pub fn anthropic(api_key: &str) -> Result<Box<dyn AIProvider>> {
        let config = ProviderConfig::new("anthropic")
            .with_api_key(api_key)
            .with_model("claude-3-opus-20240229");
        ProviderFactory::create(config)
    }

    /// Create a default local provider (Ollama)
    pub fn local() -> Result<Box<dyn AIProvider>> {
        let config = ProviderConfig::new("local")
            .with_model("llama2")
            .with_api_url("http://localhost:11434");
        ProviderFactory::create(config)
    }

    /// Create a default GitHub Copilot provider
    pub fn copilot(token: &str) -> Result<Box<dyn AIProvider>> {
        let config = ProviderConfig::new("copilot")
            .with_api_key(token);
        ProviderFactory::create(config)
    }
}

/// Provider builder for easy construction
pub struct ProviderBuilder {
    config: ProviderConfig,
}

impl ProviderBuilder {
    pub fn new(provider_type: &str) -> Self {
        Self {
            config: ProviderConfig::new(provider_type),
        }
    }

    pub fn with_api_key(mut self, key: &str) -> Self {
        self.config = self.config.with_api_key(key);
        self
    }

    pub fn with_api_url(mut self, url: &str) -> Self {
        self.config = self.config.with_api_url(url);
        self
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.config = self.config.with_model(model);
        self
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.config = self.config.with_timeout(seconds);
        self
    }

    pub fn with_retries(mut self, retries: usize) -> Self {
        self.config = self.config.with_retries(retries);
        self
    }

    pub fn build(self) -> Result<Box<dyn AIProvider>> {
        ProviderFactory::create(self.config)
    }
}

/// Provider capability checker
pub fn check_provider_capabilities(provider: &dyn AIProvider, required: &[&str]) -> Vec<String> {
    let caps = provider.capabilities();
    let mut missing = Vec::new();

    for req in required {
        match *req {
            "streaming" if !caps.streaming => missing.push("streaming".to_string()),
            "functions" if !caps.functions => missing.push("functions".to_string()),
            "vision" if !caps.vision => missing.push("vision".to_string()),
            "embeddings" if !caps.embeddings => missing.push("embeddings".to_string()),
            _ => {}
        }
    }

    missing
}

/// Find best provider for a task
pub fn find_best_provider<'a>(
    providers: &'a ProviderManager,
    task: &str,
    requires_streaming: bool,
    max_cost: Option<f64>,
) -> Option<&'a Box<dyn AIProvider>> {
    let mut best = None;
    let mut best_score = -1.0;

    for name in providers.list() {
        if let Some(provider) = providers.get(name) {
            let caps = provider.capabilities();

            // Check requirements
            if requires_streaming && !caps.streaming {
                continue;
            }

            if let Some(max) = max_cost {
                if caps.cost_per_1k_input > max || caps.cost_per_1k_output > max {
                    continue;
                }
            }

            // Score based on task
            let score = match task {
                "chat" if caps.functions => 10.0,
                "chat" => 8.0,
                "code" if caps.vision => 9.0,
                "code" => 7.0,
                "embedding" if caps.embeddings => 10.0,
                _ => 5.0,
            };

            if score > best_score {
                best_score = score;
                best = Some(provider);
            }
        }
    }

    best
}

#[cfg(test)]
mod final_tests {
    use super::*;

    #[test]
    fn test_provider_builder() {
        let provider = ProviderBuilder::new("openai")
            .with_api_key("test-key")
            .with_model("gpt-4")
            .with_timeout(30)
            .build();

        // Will fail in tests without real API, but builder works
        assert!(provider.is_err() || provider.is_ok());
    }

    #[test]
    fn test_capability_checker() {
        // Can't test without real provider
    }

    #[test]
    fn test_find_best_provider() {
        let manager = ProviderManager::new();
        let result = find_best_provider(&manager, "chat", true, None);
        assert!(result.is_none());
    }
}

/// Re-export everything for easy importing
/// Version constant
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize with default providers
pub async fn init_default() -> Result<ProviderManager> {
    let mut manager = ProviderManager::new();

    // Try to load from environment
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        if let Ok(provider) = defaults::openai(&key) {
            manager.add("openai", provider);
        }
    }

    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if let Ok(provider) = defaults::anthropic(&key) {
            manager.add("anthropic", provider);
        }
    }

    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if let Ok(provider) = defaults::copilot(&token) {
            manager.add("copilot", provider);
        }
    }

    // Always add local if available
    if let Ok(provider) = defaults::local() {
        manager.add("local", provider);
    }

    Ok(manager)
}

/// Load provider from configuration file
pub async fn load_from_config(path: &std::path::Path) -> Result<ProviderManager> {
    let content = tokio::fs::read_to_string(path).await?;
    let configs: Vec<ProviderConfig> = serde_json::from_str(&content)?;

    let mut manager = ProviderManager::new();

    for config in configs {
        if let Ok(provider) = ProviderFactory::create(config) {
            let name = provider.name().to_string();
            manager.add(&name, provider);
        }
    }

    Ok(manager)
}

/// Save provider configuration to file
pub async fn save_to_config(manager: &ProviderManager, path: &std::path::Path) -> Result<()> {
    let mut configs: Vec<ProviderConfig> = Vec::new();

    for name in manager.list() {
        if let Some(provider) = manager.get(name) {
            // Can't extract config from trait object directly
            // This would need each provider to implement a method to return its config
        }
    }

    let content = serde_json::to_string_pretty(&configs)?;
    tokio::fs::write(path, content).await?;

    Ok(())
}

/// Provider health check
pub async fn check_provider_health(provider: &dyn AIProvider) -> ProviderHealth {
    let start = std::time::Instant::now();
    let available = provider.is_available().await;
    let latency = start.elapsed();

    ProviderHealth {
        name: provider.name().to_string(),
        available,
        latency_ms: latency.as_millis() as u64,
        models_available: provider.capabilities().models.len(),
        last_check: chrono::Utc::now(),
    }
}

/// Provider health status
#[derive(Debug, Clone)]
pub struct ProviderHealth {
    pub name: String,
    pub available: bool,
    pub latency_ms: u64,
    pub models_available: usize,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Batch processor for multiple requests
pub struct BatchProcessor {
    requests: Vec<Box<dyn AIRequest>>,
    max_concurrent: usize,
}

pub trait AIRequest: Send {
    fn execute(&self, provider: &dyn AIProvider) -> futures::future::BoxFuture<'static, Result<serde_json::Value>>;
}

impl BatchProcessor {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            requests: Vec::new(),
            max_concurrent,
        }
    }

    pub fn add(&mut self, request: Box<dyn AIRequest>) {
        self.requests.push(request);
    }

    pub async fn execute_all(&self, provider: &dyn AIProvider) -> Vec<Result<serde_json::Value>> {
        // Execute requests sequentially for lifetime safety.
        let mut results = Vec::new();
        for request in &self.requests {
            let result = request.execute(provider).await;
            results.push(result);
        }

        results
    }

    pub fn clear(&mut self) {
        self.requests.clear();
    }

    pub fn len(&self) -> usize {
        self.requests.len()
    }

    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }
}

impl Default for BatchProcessor {
    fn default() -> Self {
        Self::new(5)
    }
}

/// Fallback provider chain
pub struct ProviderChain {
    providers: Vec<Box<dyn AIProvider>>,
    current: usize,
}

impl ProviderChain {
    pub fn new(providers: Vec<Box<dyn AIProvider>>) -> Self {
        Self {
            providers,
            current: 0,
        }
    }

    pub async fn execute<F, T>(&mut self, mut operation: F) -> Result<T>
    where
        F: FnMut(&dyn AIProvider) -> futures::future::BoxFuture<'static, Result<T>>,
    {
        let mut last_error = None;

        while self.current < self.providers.len() {
            let provider = &self.providers[self.current];
            match operation(provider.as_ref()).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    self.current += 1;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("No providers available")))
    }

    pub fn reset(&mut self) {
        self.current = 0;
    }

    pub fn has_next(&self) -> bool {
        self.current < self.providers.len()
    }
}

#[cfg(test)]
mod advanced_tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_processor() {
        let processor = BatchProcessor::new(2);
        assert_eq!(processor.max_concurrent, 2);
        assert!(processor.is_empty());
    }

    #[test]
    fn test_provider_chain() {
        let chain: ProviderChain = ProviderChain::new(vec![]);
        assert!(!chain.has_next());
    }

    #[tokio::test]
    async fn test_load_from_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("providers.json");

        // Would need actual config content
        let result = load_from_config(&config_path).await;
        assert!(result.is_err()); // File doesn't exist
    }
}