use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    #[serde(rename = "bearer")]
    Bearer,
    #[serde(rename = "basic")]
    Basic,
    #[serde(rename = "api_key")]
    ApiKey,
    #[serde(rename = "oauth2")]
    OAuth2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub auth_type: AuthType,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub api_key: Option<String>,
    pub headers: HashMap<String, String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            auth_type: AuthType::Bearer,
            token: None,
            username: None,
            password: None,
            api_key: None,
            headers: HashMap::new(),
        }
    }
}

impl AuthConfig {
    pub fn bearer(token: String) -> Self {
        Self {
            auth_type: AuthType::Bearer,
            token: Some(token),
            ..Default::default()
        }
    }

    pub fn basic(username: String, password: String) -> Self {
        Self {
            auth_type: AuthType::Basic,
            username: Some(username),
            password: Some(password),
            ..Default::default()
        }
    }

    pub fn api_key(key: String) -> Self {
        Self {
            auth_type: AuthType::ApiKey,
            api_key: Some(key),
            ..Default::default()
        }
    }

    pub fn add_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    pub fn to_headers(&self) -> HashMap<String, String> {
        let mut headers = self.headers.clone();

        match self.auth_type {
            AuthType::Bearer => {
                if let Some(token) = &self.token {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", token));
                }
            }
            AuthType::Basic => {
                if let (Some(username), Some(password)) = (&self.username, &self.password) {
                    use base64::Engine;
                    let credentials = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", username, password));
                    headers.insert("Authorization".to_string(), format!("Basic {}", credentials));
                }
            }
            AuthType::ApiKey => {
                if let Some(api_key) = &self.api_key {
                    headers.insert("Authorization".to_string(), api_key.clone());
                }
            }
            AuthType::OAuth2 => {
                if let Some(token) = &self.token {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", token));
                }
            }
        }

        headers
    }
}