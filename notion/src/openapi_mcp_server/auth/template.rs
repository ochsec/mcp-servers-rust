use super::types::AuthConfig;
use std::collections::HashMap;

pub struct AuthTemplate {
    template: String,
    variables: HashMap<String, String>,
}

impl AuthTemplate {
    pub fn new(template: String) -> Self {
        Self {
            template,
            variables: HashMap::new(),
        }
    }

    pub fn set_variable<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    pub fn render(&self) -> String {
        let mut result = self.template.clone();
        
        for (key, value) in &self.variables {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        
        result
    }

    pub fn render_auth_config(&self) -> AuthConfig {
        let rendered = self.render();
        
        // Try to parse as JSON first
        if let Ok(config) = serde_json::from_str::<AuthConfig>(&rendered) {
            return config;
        }

        // Fallback to simple bearer token
        AuthConfig::bearer(rendered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_rendering() {
        let template = AuthTemplate::new("Bearer {{token}}".to_string())
            .set_variable("token", "my-secret-token");
        
        assert_eq!(template.render(), "Bearer my-secret-token");
    }

    #[test]
    fn test_json_template_rendering() {
        let template = AuthTemplate::new(r#"{"auth_type": "bearer", "token": "{{token}}"}"#.to_string())
            .set_variable("token", "my-secret-token");
        
        let config = template.render_auth_config();
        match config.auth_type {
            super::types::AuthType::Bearer => {},
            _ => panic!("Expected bearer auth type"),
        }
        assert_eq!(config.token, Some("my-secret-token".to_string()));
    }
}