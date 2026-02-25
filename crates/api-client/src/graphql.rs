//! GraphQL client with schema introspection and query builder

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};
use tracing::{info, warn, debug};

use crate::rest::{RESTClient, RESTRequest, HTTPMethod, RequestBody, RESTResponse};
use crate::APIClientConfig;

/// GraphQL query
#[derive(Debug, Clone)]
pub struct GraphQLQuery {
    pub query: String,
    pub variables: Option<Value>,
    pub operation_name: Option<String>,
    pub url: String,
    pub headers: HashMap<String, String>,
}

/// GraphQL response
#[derive(Debug, Clone)]
pub struct GraphQLResponse {
    pub data: Option<Value>,
    pub errors: Vec<GraphQLError>,
    pub extensions: Option<Value>,
    pub status: u16,
    pub duration: std::time::Duration,
}

/// GraphQL error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    pub locations: Option<Vec<GraphQLErrorLocation>>,
    pub path: Option<Vec<String>>,
    pub extensions: Option<Value>,
}

/// GraphQL error location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLErrorLocation {
    pub line: usize,
    pub column: usize,
}

/// GraphQL schema
#[derive(Debug, Clone)]
pub struct GraphQLSchema {
    pub types: HashMap<String, GraphQLType>,
    pub query_type: String,
    pub mutation_type: Option<String>,
    pub subscription_type: Option<String>,
}

/// GraphQL type
#[derive(Debug, Clone)]
pub struct GraphQLType {
    pub name: String,
    pub kind: GraphQLTypeKind,
    pub description: Option<String>,
    pub fields: Vec<GraphQLField>,
    pub input_fields: Vec<GraphQLInputField>,
    pub interfaces: Vec<String>,
    pub possible_types: Vec<String>,
    pub enum_values: Vec<GraphQLEnumValue>,
}

/// GraphQL type kind
#[derive(Debug, Clone)]
pub enum GraphQLTypeKind {
    Scalar,
    Object,
    Interface,
    Union,
    Enum,
    InputObject,
    List,
    NonNull,
}

/// GraphQL field
#[derive(Debug, Clone)]
pub struct GraphQLField {
    pub name: String,
    pub description: Option<String>,
    pub args: Vec<GraphQLInputValue>,
    pub type_ref: GraphQLTypeRef,
    pub deprecated: Option<String>,
}

/// GraphQL input field
#[derive(Debug, Clone)]
pub struct GraphQLInputField {
    pub name: String,
    pub description: Option<String>,
    pub type_ref: GraphQLTypeRef,
    pub default_value: Option<Value>,
}

/// GraphQL input value
#[derive(Debug, Clone)]
pub struct GraphQLInputValue {
    pub name: String,
    pub description: Option<String>,
    pub type_ref: GraphQLTypeRef,
    pub default_value: Option<Value>,
}

/// GraphQL type reference
#[derive(Debug, Clone)]
pub struct GraphQLTypeRef {
    pub name: String,
    pub kind: GraphQLTypeKind,
    pub of_type: Option<Box<GraphQLTypeRef>>,
}

/// GraphQL enum value
#[derive(Debug, Clone)]
pub struct GraphQLEnumValue {
    pub name: String,
    pub description: Option<String>,
    pub deprecation_reason: Option<String>,
}

/// GraphQL client
pub struct GraphQLClient {
    rest_client: RESTClient,
    schema_cache: Arc<tokio::sync::RwLock<HashMap<String, GraphQLSchema>>>,
}

impl GraphQLClient {
    /// Create new GraphQL client
    pub fn new(config: APIClientConfig) -> Result<Self> {
        Ok(Self {
            rest_client: RESTClient::new(config)?,
            schema_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }

    /// Execute GraphQL query
    pub async fn query(&self, query: GraphQLQuery) -> Result<GraphQLResponse> {
        let start = std::time::Instant::now();

        let request = RESTRequest {
            method: HTTPMethod::POST,
            url: query.url.clone(),
            headers: query.headers,
            params: HashMap::new(),
            body: Some(RequestBody::GraphQL(crate::rest::GraphQLBody {
                query: query.query,
                variables: query.variables,
                operation_name: query.operation_name,
            })),
            auth: None,
            timeout: None,
            follow_redirects: true,
        };

        let response = self.rest_client.send(request).await?;

        let graphql_response = if let Some(body) = response.body {
            serde_json::from_value::<GraphQLResponseData>(body)?
        } else {
            GraphQLResponseData::default()
        };

        Ok(GraphQLResponse {
            data: graphql_response.data,
            errors: graphql_response.errors.unwrap_or_default(),
            extensions: graphql_response.extensions,
            status: response.status,
            duration: start.elapsed(),
        })
    }

    /// Introspect schema
    pub async fn introspect(&self, url: &str, headers: HashMap<String, String>) -> Result<GraphQLSchema> {
        // Check cache
        {
            let cache = self.schema_cache.read().await;
            if let Some(schema) = cache.get(url) {
                return Ok(schema.clone());
            }
        }

        // Introspection query
        let introspection_query = r#"
            query IntrospectionQuery {
                __schema {
                    queryType { name }
                    mutationType { name }
                    subscriptionType { name }
                    types {
                        ...FullType
                    }
                }
            }
            fragment FullType on __Type {
                kind
                name
                description
                fields(includeDeprecated: true) {
                    name
                    description
                    args {
                        ...InputValue
                    }
                    type {
                        ...TypeRef
                    }
                    isDeprecated
                    deprecationReason
                }
                inputFields {
                    ...InputValue
                }
                interfaces {
                    ...TypeRef
                }
                enumValues(includeDeprecated: true) {
                    name
                    description
                    isDeprecated
                    deprecationReason
                }
                possibleTypes {
                    ...TypeRef
                }
            }
            fragment InputValue on __InputValue {
                name
                description
                type { ...TypeRef }
                defaultValue
            }
            fragment TypeRef on __Type {
                kind
                name
                ofType {
                    kind
                    name
                    ofType {
                        kind
                        name
                        ofType {
                            kind
                            name
                            ofType {
                                kind
                                name
                                ofType {
                                    kind
                                    name
                                    ofType {
                                        kind
                                        name
                                        ofType {
                                            kind
                                            name
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        "#;

        let query = GraphQLQuery {
            query: introspection_query.to_string(),
            variables: None,
            operation_name: None,
            url: url.to_string(),
            headers,
        };

        let response = self.query(query).await?;

        if let Some(data) = response.data {
            if let Some(schema_data) = data.get("__schema") {
                let schema = self.parse_schema(schema_data)?;
                
                // Cache schema
                self.schema_cache.write().await.insert(url.to_string(), schema.clone());
                
                return Ok(schema);
            }
        }

        Err(anyhow!("Failed to introspect schema"))
    }

    /// Parse schema from introspection data
    fn parse_schema(&self, schema_data: &Value) -> Result<GraphQLSchema> {
        // This is a simplified parser - a real implementation would parse all types
        let query_type = schema_data["queryType"]["name"]
            .as_str()
            .unwrap_or("Query")
            .to_string();

        let mutation_type = schema_data["mutationType"]["name"]
            .as_str()
            .map(|s| s.to_string());

        let subscription_type = schema_data["subscriptionType"]["name"]
            .as_str()
            .map(|s| s.to_string());

        Ok(GraphQLSchema {
            types: HashMap::new(), // Would parse all types here
            query_type,
            mutation_type,
            subscription_type,
        })
    }

    /// Generate query from schema
    pub fn generate_query(&self, schema: &GraphQLSchema, type_name: &str, fields: Vec<String>) -> String {
        let fields_str = fields.join("\n    ");
        format!("query {{\n  {}{{\n    {}\n  }}\n}}", type_name, fields_str)
    }

    /// Format query with variables
    pub fn format_query(&self, query: &str, variables: &Value) -> Result<String> {
        let mut result = query.to_string();
        
        if let Some(obj) = variables.as_object() {
            for (key, value) in obj {
                let placeholder = format!("${}", key);
                let value_str = serde_json::to_string(value)?;
                result = result.replace(&placeholder, &value_str);
            }
        }

        Ok(result)
    }
}

#[derive(Debug, Default, Deserialize)]
struct GraphQLResponseData {
    data: Option<Value>,
    errors: Option<Vec<GraphQLError>>,
    extensions: Option<Value>,
}