use anyhow::Result;
use openapiv3::{OpenAPI, Operation, Parameter, ParameterData, ReferenceOr};
use reqwest::{Client, Method};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;
use tracing::{error, info, warn};

use crate::openapi_mcp_server::openapi::file_upload::is_file_upload_parameter;

#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    pub base_url: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug)]
pub struct HttpClientResponse<T = Value> {
    pub data: T,
    pub status: u16,
    pub headers: reqwest::header::HeaderMap,
}

#[derive(Error, Debug)]
pub enum HttpClientError {
    #[error("HTTP request failed with status {status}: {message}")]
    RequestFailed {
        status: u16,
        message: String,
        data: Option<Value>,
        headers: Option<reqwest::header::HeaderMap>,
    },
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("File error: {0}")]
    FileError(String),
    #[error("Operation error: {0}")]
    OperationError(String),
}

pub struct HttpClient {
    client: Client,
    config: HttpClientConfig,
    openapi_spec: OpenAPI,
}

impl HttpClient {
    pub fn new(config: HttpClientConfig, openapi_spec: OpenAPI) -> Result<Self> {
        let mut default_headers = reqwest::header::HeaderMap::new();
        
        // Add default headers
        default_headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        default_headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("notion-mcp-server-rust"),
        );

        // Add custom headers from config
        for (key, value) in &config.headers {
            if let (Ok(header_name), Ok(header_value)) = (
                reqwest::header::HeaderName::try_from(key),
                reqwest::header::HeaderValue::try_from(value),
            ) {
                default_headers.insert(header_name, header_value);
            } else {
                warn!("Invalid header: {} = {}", key, value);
            }
        }

        let client = Client::builder()
            .default_headers(default_headers)
            .build()?;

        Ok(Self {
            client,
            config,
            openapi_spec,
        })
    }

    pub async fn execute_operation(
        &self,
        operation_info: &crate::openapi_mcp_server::openapi::parser::OperationInfo,
        params: HashMap<String, Value>,
    ) -> Result<HttpClientResponse<Value>, HttpClientError> {
        let operation = &operation_info.operation;
        let method = &operation_info.method;
        let path = &operation_info.path;

        info!("Executing {} {} with params: {:?}", method, path, params);

        // Check for file uploads
        let file_params = is_file_upload_parameter(operation);
        let has_file_upload = !file_params.is_empty();

        // Build the URL
        let mut url = format!("{}{}", self.config.base_url, path);
        let mut body_params = params.clone();
        let mut query_params = Vec::new();

        // Process parameters  
        for param_ref in &operation.parameters {
            if let Some(param) = self.resolve_parameter(param_ref) {
                if let Some(param_value) = params.get(&param.name) {
                    // Determine parameter location from the parameter definition
                    let location = match param_ref {
                        ReferenceOr::Item(p) => match p {
                            Parameter::Query { .. } => "query",
                            Parameter::Header { .. } => "header", 
                            Parameter::Path { .. } => "path",
                            Parameter::Cookie { .. } => "cookie",
                        },
                        ReferenceOr::Reference { .. } => "query", // Default fallback
                    };
                    match location {
                        "path" => {
                            // Replace path parameters in URL
                            let placeholder = format!("{{{}}}", param.name);
                            if let Some(value_str) = param_value.as_str() {
                                url = url.replace(&placeholder, value_str);
                            } else {
                                url = url.replace(&placeholder, &param_value.to_string());
                            }
                            body_params.remove(&param.name);
                        }
                        "query" => {
                            // Add to query parameters
                            if let Some(value_str) = param_value.as_str() {
                                query_params.push((param.name.clone(), value_str.to_string()));
                            } else {
                                query_params.push((param.name.clone(), param_value.to_string()));
                            }
                            body_params.remove(&param.name);
                        }
                        "header" => {
                            // Headers are handled separately
                            body_params.remove(&param.name);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Add query parameters to URL
        if !query_params.is_empty() {
            url.push('?');
            for (i, (key, value)) in query_params.iter().enumerate() {
                if i > 0 {
                    url.push('&');
                }
                url.push_str(&format!("{}={}", 
                    urlencoding::encode(key), 
                    urlencoding::encode(value)
                ));
            }
        }

        // Parse HTTP method
        let http_method = match method.to_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            "PATCH" => Method::PATCH,
            _ => return Err(HttpClientError::OperationError(format!("Unsupported HTTP method: {}", method))),
        };

        // Build request
        let mut request_builder = self.client.request(http_method.clone(), &url);

        // Handle request body
        if has_file_upload {
            // Handle multipart form data for file uploads
            let form = self.prepare_multipart_form(operation, &body_params).await?;
            request_builder = request_builder.multipart(form);
        } else if !body_params.is_empty() && matches!(method.to_uppercase().as_str(), "POST" | "PUT" | "PATCH") {
            // Handle JSON body for non-GET requests
            if operation.request_body.is_some() {
                // If there's a request body definition, send all remaining params as JSON
                request_builder = request_builder.json(&body_params);
            } else {
                // If no request body, move remaining params to query string
                let mut url_with_params = url.clone();
                if url_with_params.contains('?') {
                    url_with_params.push('&');
                } else {
                    url_with_params.push('?');
                }
                
                let additional_params: Vec<String> = body_params.iter()
                    .map(|(k, v)| format!("{}={}", 
                        urlencoding::encode(k), 
                        urlencoding::encode(&v.to_string())
                    ))
                    .collect();
                url_with_params.push_str(&additional_params.join("&"));
                
                request_builder = self.client.request(http_method, &url_with_params);
            }
        }

        // Execute request
        let response = request_builder.send().await?;
        let status = response.status().as_u16();
        let headers = response.headers().clone();

        if response.status().is_success() {
            let data: Value = if headers.get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.contains("application/json"))
                .unwrap_or(false)
            {
                response.json().await?
            } else {
                let text = response.text().await?;
                Value::String(text)
            };

            Ok(HttpClientResponse { data, status, headers })
        } else {
            let error_data: Option<Value> = if headers.get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.contains("application/json"))
                .unwrap_or(false)
            {
                response.json().await.ok()
            } else {
                response.text().await.ok().map(Value::String)
            };

            Err(HttpClientError::RequestFailed {
                status,
                message: format!("Request failed with status {}", status),
                data: error_data,
                headers: Some(headers),
            })
        }
    }

    async fn prepare_multipart_form(
        &self,
        operation: &Operation,
        params: &HashMap<String, Value>,
    ) -> Result<reqwest::multipart::Form, HttpClientError> {
        let mut form = reqwest::multipart::Form::new();
        let file_params = is_file_upload_parameter(operation);

        for (key, value) in params {
            if file_params.contains(key) {
                // Handle file upload
                match value {
                    Value::String(file_path) => {
                        self.add_file_to_form(&mut form, key, file_path).await?;
                    }
                    Value::Array(file_paths) => {
                        for file_path_value in file_paths {
                            if let Value::String(file_path) = file_path_value {
                                self.add_file_to_form(&mut form, key, file_path).await?;
                            }
                        }
                    }
                    _ => {
                        return Err(HttpClientError::FileError(format!(
                            "File parameter {} must be a string path or array of string paths", 
                            key
                        )));
                    }
                }
            } else {
                // Handle regular form field
                let value_str = match value {
                    Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };
                form = form.text(key.clone(), value_str);
            }
        }

        Ok(form)
    }

    async fn add_file_to_form(
        &self,
        form: &mut reqwest::multipart::Form,
        field_name: &str,
        file_path: &str,
    ) -> Result<(), HttpClientError> {
        let path = Path::new(file_path);
        
        if !path.exists() {
            return Err(HttpClientError::FileError(format!(
                "File not found: {}", 
                file_path
            )));
        }

        let file_contents = tokio::fs::read(path).await
            .map_err(|e| HttpClientError::FileError(format!(
                "Failed to read file {}: {}", 
                file_path, 
                e
            )))?;

        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();

        let mime_type = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        let part = reqwest::multipart::Part::bytes(file_contents)
            .file_name(file_name)
            .mime_str(&mime_type)
            .map_err(|e| HttpClientError::FileError(format!("Invalid MIME type: {}", e)))?;

        *form = std::mem::take(form).part(field_name.to_string(), part);
        
        Ok(())
    }

    fn resolve_parameter(&self, param_ref: &ReferenceOr<Parameter>) -> Option<ParameterData> {
        match param_ref {
            ReferenceOr::Item(param) => {
                // Extract the parameter data from the specific parameter type
                match param {
                    Parameter::Query { parameter_data, .. } => Some(parameter_data.clone()),
                    Parameter::Header { parameter_data, .. } => Some(parameter_data.clone()),
                    Parameter::Path { parameter_data, .. } => Some(parameter_data.clone()),
                    Parameter::Cookie { parameter_data, .. } => Some(parameter_data.clone()),
                }
            }
            ReferenceOr::Reference { reference } => {
                // Try to resolve parameter reference
                if let Some(components) = &self.openapi_spec.components {
                    if let Some(ReferenceOr::Item(param)) = components.parameters.get(reference) {
                        return self.resolve_parameter(&ReferenceOr::Item(param.clone()));
                    }
                }
                error!("Failed to resolve parameter reference: {}", reference);
                None
            }
        }
    }
}