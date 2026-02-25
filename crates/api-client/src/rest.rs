//! REST API client with full HTTP support

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Result, anyhow};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};
use tracing::{info, warn, debug};

use crate::APIClientConfig;

/// REST request
#[derive(Debug, Clone)]
pub struct RESTRequest {
    pub method: HTTPMethod,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub params: HashMap<String, String>,
    pub body: Option<RequestBody>,
    pub auth: Option<Auth>,
    pub timeout: Option<Duration>,
    pub follow_redirects: bool,
}

/// HTTP method
#[derive(Debug, Clone, Copy)]
pub enum HTTPMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

impl HTTPMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HTTPMethod::GET => "GET",
            HTTPMethod::POST => "POST",
            HTTPMethod::PUT => "PUT",
            HTTPMethod::DELETE => "DELETE",
            HTTPMethod::PATCH => "PATCH",
            HTTPMethod::HEAD => "HEAD",
            HTTPMethod::OPTIONS => "OPTIONS",
        }
    }
}

impl From<&str> for HTTPMethod {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "GET" => HTTPMethod::GET,
            "POST" => HTTPMethod::POST,
            "PUT" => HTTPMethod::PUT,
            "DELETE" => HTTPMethod::DELETE,
            "PATCH" => HTTPMethod::PATCH,
            "HEAD" => HTTPMethod::HEAD,
            "OPTIONS" => HTTPMethod::OPTIONS,
            _ => HTTPMethod::GET,
        }
    }
}

/// Request body
#[derive(Debug, Clone)]
pub enum RequestBody {
    JSON(Value),
    Text(String),
    Form(HashMap<String, String>),
    Multipart(Vec<MultipartField>),
    Binary(Vec<u8>),
    GraphQL(GraphQLBody),
}

/// GraphQL body
#[derive(Debug, Clone)]
pub struct GraphQLBody {
    pub query: String,
    pub variables: Option<Value>,
    pub operation_name: Option<String>,
}

/// Multipart field
#[derive(Debug, Clone)]
pub struct MultipartField {
    pub name: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub data: Vec<u8>,
}

/// Authentication
#[derive(Debug, Clone)]
pub enum Auth {
    None,
    Bearer(String),
    Basic { username: String, password: String },
    APIKey { key: String, value: String, in_header: bool },
    OAuth2 { token: String, token_type: String },
}

/// REST response
#[derive(Debug, Clone)]
pub struct RESTResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Value>,
    pub raw_body: Option<Vec<u8>>,
    pub size: usize,
    pub duration: Duration,
    pub url: String,
}

/// REST client
pub struct RESTClient {
    client: Client,
    config: APIClientConfig,
}

impl RESTClient {
    /// Create new REST client
    pub fn new(config: APIClientConfig) -> Result<Self> {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent);

        if !config.follow_redirects {
            builder = builder.redirect(reqwest::redirect::Policy::none());
        }

        if let Some(proxy_url) = &config.proxy_url {
            if let Ok(proxy) = reqwest::Proxy::all(proxy_url) {
                builder = builder.proxy(proxy);
            }
        }

        let client = builder.build()?;

        Ok(Self { client, config })
    }

    /// Send REST request
    pub async fn send(&self, request: RESTRequest) -> Result<RESTResponse> {
        let start = std::time::Instant::now();

        // Build URL with params
        let mut url = reqwest::Url::parse(&request.url)?;
        if !request.params.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (k, v) in &request.params {
                pairs.append_pair(k, v);
            }
            drop(pairs);
        }

        // Create request builder
        let method = match request.method {
            HTTPMethod::GET => Method::GET,
            HTTPMethod::POST => Method::POST,
            HTTPMethod::PUT => Method::PUT,
            HTTPMethod::DELETE => Method::DELETE,
            HTTPMethod::PATCH => Method::PATCH,
            HTTPMethod::HEAD => Method::HEAD,
            HTTPMethod::OPTIONS => Method::OPTIONS,
        };

        let mut builder = self.client.request(method, url);

        // Add headers
        for (k, v) in &request.headers {
            builder = builder.header(k, v);
        }

        // Add auth
        if let Some(auth) = &request.auth {
            builder = self.apply_auth(builder, auth);
        }

        // Add body
        if let Some(body) = &request.body {
            builder = self.apply_body(builder, body)?;
        }

        // Set timeout
        if let Some(timeout) = request.timeout {
            builder = builder.timeout(timeout);
        }

        // Send request
        let response = builder.send().await?;

        // Parse response
        let status = response.status();
        let status_text = status.canonical_reason().unwrap_or("Unknown").to_string();
        let headers = response.headers().iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let body_bytes = response.bytes().await?;
        let size = body_bytes.len();

        // Try to parse as JSON
        let body = serde_json::from_slice(&body_bytes).ok();

        Ok(RESTResponse {
            status: status.as_u16(),
            status_text,
            headers,
            body,
            raw_body: Some(body_bytes.to_vec()),
            size,
            duration: start.elapsed(),
            url: request.url,
        })
    }

    /// Apply authentication to request
    fn apply_auth(&self, builder: RequestBuilder, auth: &Auth) -> RequestBuilder {
        match auth {
            Auth::None => builder,
            Auth::Bearer(token) => builder.bearer_auth(token),
            Auth::Basic { username, password } => builder.basic_auth(username, Some(password)),
            Auth::APIKey { key, value, in_header: true } => builder.header(key, value),
            Auth::APIKey { key, value, in_header: false } => {
                // Add as query parameter
                builder.query(&[(key.as_str(), value.as_str())])
            }
            Auth::OAuth2 { token, token_type } => builder.header("Authorization", format!("{} {}", token_type, token)),
        }
    }

    /// Apply body to request
    fn apply_body(&self, builder: RequestBuilder, body: &RequestBody) -> Result<RequestBuilder> {
        Ok(match body {
            RequestBody::JSON(value) => builder.json(value),
            RequestBody::Text(text) => builder.body(text.clone()),
            RequestBody::Form(form) => builder.form(form),
            RequestBody::Multipart(fields) => {
                let mut form = reqwest::multipart::Form::new();
                for field in fields {
                    let part = reqwest::multipart::Part::bytes(field.data.clone())
                        .file_name(field.filename.clone().unwrap_or_else(|| field.name.clone()));
                    
                    let part = if let Some(ct) = &field.content_type {
                        part.mime_str(ct)?
                    } else {
                        part
                    };
                    
                    form = form.part(field.name.clone(), part);
                }
                builder.multipart(form)
            }
            RequestBody::Binary(data) => builder.body(data.clone()),
            RequestBody::GraphQL(gql) => {
                let mut body = HashMap::new();
                body.insert("query", gql.query.clone());
                if let Some(vars) = &gql.variables {
                    body.insert("variables", serde_json::to_string(vars)?);
                }
                if let Some(op) = &gql.operation_name {
                    body.insert("operationName", op.clone());
                }
                builder.json(&body)
            }
        })
    }

    /// Send GraphQL request (convenience)
    pub async fn send_graphql(&self, url: &str, query: &str, variables: Option<Value>) -> Result<RESTResponse> {
        let gql_body = GraphQLBody {
            query: query.to_string(),
            variables,
            operation_name: None,
        };

        let request = RESTRequest {
            method: HTTPMethod::POST,
            url: url.to_string(),
            headers: HashMap::new(),
            params: HashMap::new(),
            body: Some(RequestBody::GraphQL(gql_body)),
            auth: None,
            timeout: None,
            follow_redirects: self.config.follow_redirects,
        };

        self.send(request).await
    }
}

impl From<&RESTResponse> for crate::RequestHistory {
    fn from(resp: &RESTResponse) -> Self {
        crate::RequestHistory {
            id: uuid::Uuid::new_v4().to_string(),
            method: "REST".to_string(),
            url: resp.url.clone(),
            status: Some(resp.status),
            timestamp: chrono::Utc::now(),
            duration: resp.duration,
            request_size: 0,
            response_size: resp.size,
            collection: None,
            environment: None,
        }
    }
}