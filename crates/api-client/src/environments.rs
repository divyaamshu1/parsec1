//! Environment variables management for API testing

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};
use tokio::fs;
use tracing::{info, warn, debug};

use crate::rest::{RESTRequest, RequestBody, Auth};
use crate::graphql::GraphQLQuery;
use crate::grpc::GRPCRequest;

/// Environment
#[derive(Debug, Clone)]
pub struct Environment {
    pub name: String,
    pub variables: HashMap<String, String>,
    pub base_url: Option<String>,
    pub headers: HashMap<String, String>,
    pub auth: Option<Auth>,
    pub is_active: bool,
}

/// Environment manager
pub struct EnvironmentManager {
    environments: Arc<tokio::sync::RwLock<HashMap<String, Environment>>>,
    active: Arc<tokio::sync::RwLock<Option<String>>>,
}

impl EnvironmentManager {
    /// Create new environment manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            environments: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            active: Arc::new(tokio::sync::RwLock::new(None)),
        })
    }

    /// Add environment
    pub async fn add(&self, name: &str, environment: Environment) {
        self.environments.write().await.insert(name.to_string(), environment);
    }

    /// Remove environment
    pub async fn remove(&self, name: &str) {
        self.environments.write().await.remove(name);
    }

    /// Set active environment
    pub async fn set_active(&self, name: Option<&str>) -> Result<()> {
        if let Some(name) = name {
            if !self.environments.read().await.contains_key(name) {
                return Err(anyhow!("Environment not found: {}", name));
            }
            *self.active.write().await = Some(name.to_string());
        } else {
            *self.active.write().await = None;
        }
        Ok(())
    }

    /// Get active environment
    pub async fn active(&self) -> Option<Environment> {
        let active = self.active.read().await.clone();
        if let Some(name) = active {
            self.environments.read().await.get(&name).cloned()
        } else {
            None
        }
    }

    /// List environments
    pub async fn list(&self) -> Vec<String> {
        self.environments.read().await.keys().cloned().collect()
    }

    /// Resolve variables in string
    pub async fn resolve_variables(&self, text: &str) -> Result<String> {
        let env = self.active().await;
        let mut result = text.to_string();

        if let Some(env) = env {
            for (key, value) in &env.variables {
                let placeholder = format!("{{{{{}}}}}", key);
                result = result.replace(&placeholder, value);
            }

            if let Some(base_url) = &env.base_url {
                result = result.replace("{{base_url}}", base_url);
            }
        }

        Ok(result)
    }

    /// Apply environment to REST request
    pub async fn apply_to_request(&self, mut request: RESTRequest) -> Result<RESTRequest> {
        request.url = self.resolve_variables(&request.url).await?;

        for (key, value) in &mut request.headers {
            *value = self.resolve_variables(value).await?;
        }

        for (key, value) in &mut request.params {
            *value = self.resolve_variables(value).await?;
        }

        if let Some(body) = &mut request.body {
            match body {
                RequestBody::Text(text) => {
                    *text = self.resolve_variables(text).await?;
                }
                RequestBody::JSON(json) => {
                    // Would need to recursively resolve variables in JSON
                }
                _ => {}
            }
        }

        // Apply environment auth if request doesn't have its own
        if request.auth.is_none() {
            if let Some(env) = self.active().await {
                request.auth = env.auth;
            }
        }

        Ok(request)
    }

    /// Apply environment to GraphQL query
    pub async fn apply_to_graphql(&self, mut query: GraphQLQuery) -> Result<GraphQLQuery> {
        query.url = self.resolve_variables(&query.url).await?;

        for (key, value) in &mut query.headers {
            *value = self.resolve_variables(value).await?;
        }

        if let Some(vars) = &mut query.variables {
            // Would need to recursively resolve variables
        }

        Ok(query)
    }

    /// Apply environment to gRPC request
    pub async fn apply_to_grpc(&self, mut request: GRPCRequest) -> Result<GRPCRequest> {
        for (key, value) in &mut request.metadata {
            *value = self.resolve_variables(value).await?;
        }
        Ok(request)
    }

    /// Import from file
    pub async fn import(&self, path: &Path, format: EnvironmentFormat) -> Result<()> {
        let content = fs::read_to_string(path).await?;
        
        match format {
            EnvironmentFormat::JSON => self.import_json(&content).await,
            EnvironmentFormat::YAML => self.import_yaml(&content).await,
            EnvironmentFormat::ENV => self.import_env(&content).await,
            EnvironmentFormat::Postman => self.import_postman(&content).await,
        }
    }

    async fn import_json(&self, content: &str) -> Result<()> {
        let json: Value = serde_json::from_str(content)?;
        
        if let Some(envs) = json.as_array() {
            for env_val in envs {
                if let (Some(name), Some(vars)) = (
                    env_val["name"].as_str(),
                    env_val["values"].as_object()
                ) {
                    let mut variables = HashMap::new();
                    for (k, v) in vars {
                        variables.insert(k.clone(), v.as_str().unwrap_or("").to_string());
                    }

                    self.add(name, Environment {
                        name: name.to_string(),
                        variables,
                        base_url: env_val["baseUrl"].as_str().map(|s| s.to_string()),
                        headers: HashMap::new(),
                        auth: None,
                        is_active: false,
                    }).await;
                }
            }
        }

        Ok(())
    }

    async fn import_yaml(&self, content: &str) -> Result<()> {
        // Parse YAML
        Err(anyhow!("YAML import not yet implemented"))
    }

    async fn import_env(&self, content: &str) -> Result<()> {
        let mut variables = HashMap::new();
        
        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                variables.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        self.add("imported", Environment {
            name: "imported".to_string(),
            variables,
            base_url: None,
            headers: HashMap::new(),
            auth: None,
            is_active: false,
        }).await;

        Ok(())
    }

    async fn import_postman(&self, content: &str) -> Result<()> {
        // Parse Postman environment format
        Err(anyhow!("Postman environment import not yet implemented"))
    }

    /// Export to file
    pub async fn export(&self, name: &str, format: EnvironmentFormat) -> Result<String> {
        let env = self.environments.read().await.get(name).cloned()
            .ok_or_else(|| anyhow!("Environment not found: {}", name))?;

        match format {
            EnvironmentFormat::JSON => self.export_json(&env),
            EnvironmentFormat::YAML => self.export_yaml(&env),
            EnvironmentFormat::ENV => self.export_env(&env),
            EnvironmentFormat::Postman => self.export_postman(&env),
        }
    }

    fn export_json(&self, env: &Environment) -> Result<String> {
        let mut vars = serde_json::Map::new();
        for (k, v) in &env.variables {
            vars.insert(k.clone(), Value::String(v.clone()));
        }

        let json = json!({
            "name": env.name,
            "values": vars,
            "baseUrl": env.base_url,
        });

        Ok(serde_json::to_string_pretty(&json)?)
    }

    fn export_yaml(&self, env: &Environment) -> Result<String> {
        // Convert to YAML
        Err(anyhow!("YAML export not yet implemented"))
    }

    fn export_env(&self, env: &Environment) -> Result<String> {
        let mut output = String::new();
        for (k, v) in &env.variables {
            output.push_str(&format!("{}={}\n", k, v));
        }
        Ok(output)
    }

    fn export_postman(&self, env: &Environment) -> Result<String> {
        // Convert to Postman format
        Err(anyhow!("Postman export not yet implemented"))
    }
}

/// Environment format
#[derive(Debug, Clone, Copy)]
pub enum EnvironmentFormat {
    JSON,
    YAML,
    ENV,
    Postman,
}