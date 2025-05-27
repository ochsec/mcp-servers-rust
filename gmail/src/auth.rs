use crate::error::{GmailError, Result};
use dirs::home_dir;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl, reqwest::async_http_client,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{error, info, warn};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub redirect_uris: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCredentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub token_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OAuthKeysFile {
    installed: Option<OAuthConfig>,
    web: Option<OAuthConfig>,
}

#[allow(dead_code)]
pub struct GoogleAuth {
    client: BasicClient,
    config: OAuthConfig,
    credentials: Option<OAuthCredentials>,
    http_client: Client,
}

impl GoogleAuth {
    pub async fn new() -> Result<Self> {
        let config = Self::load_oauth_config().await?;
        let client = Self::create_oauth_client(&config)?;
        let http_client = Client::new();
        
        let credentials = Self::load_credentials().await.ok();

        Ok(Self {
            client,
            config,
            credentials,
            http_client,
        })
    }

    async fn load_oauth_config() -> Result<OAuthConfig> {
        let config_dir = Self::get_config_dir()?;
        let oauth_path = std::env::var("GMAIL_OAUTH_PATH")
            .unwrap_or_else(|_| config_dir.join("gcp-oauth.keys.json").to_string_lossy().to_string());

        // Check for OAuth keys in current directory first
        let local_oauth_path = std::env::current_dir()
            .map_err(|e| GmailError::IoError(e))?
            .join("gcp-oauth.keys.json");

        let oauth_path = if local_oauth_path.exists() {
            // Copy to config directory if found locally
            if let Some(config_path) = PathBuf::from(&oauth_path).parent() {
                if !config_path.exists() {
                    fs::create_dir_all(&config_path)?;
                }
            }
            fs::copy(&local_oauth_path, &oauth_path)?;
            info!("OAuth keys found in current directory, copied to global config.");
            oauth_path
        } else {
            oauth_path
        };

        let content = fs::read_to_string(&oauth_path)
            .map_err(|_| GmailError::AuthError(format!("OAuth keys file not found. Please place gcp-oauth.keys.json in current directory or {}", oauth_path)))?;

        let keys_file: OAuthKeysFile = serde_json::from_str(&content)?;
        
        let config = keys_file.installed.or(keys_file.web)
            .ok_or_else(|| GmailError::AuthError("Invalid OAuth keys file format. File should contain either 'installed' or 'web' credentials.".to_string()))?;

        Ok(config)
    }

    fn create_oauth_client(config: &OAuthConfig) -> Result<BasicClient> {
        let client_id = ClientId::new(config.client_id.clone());
        let client_secret = ClientSecret::new(config.client_secret.clone());
        let auth_url = AuthUrl::new(config.auth_uri.clone())
            .map_err(|_| GmailError::AuthError("Invalid auth URL".to_string()))?;
        let token_url = TokenUrl::new(config.token_uri.clone())
            .map_err(|_| GmailError::AuthError("Invalid token URL".to_string()))?;

        Ok(BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url)))
    }

    fn get_config_dir() -> Result<PathBuf> {
        let home = home_dir().ok_or_else(|| GmailError::AuthError("Unable to find home directory".to_string()))?;
        let config_dir = home.join(".gmail-mcp");
        
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }
        
        Ok(config_dir)
    }

    async fn load_credentials() -> Result<OAuthCredentials> {
        let config_dir = Self::get_config_dir()?;
        let credentials_path = std::env::var("GMAIL_CREDENTIALS_PATH")
            .unwrap_or_else(|_| config_dir.join("credentials.json").to_string_lossy().to_string());

        let content = fs::read_to_string(&credentials_path)?;
        let credentials: OAuthCredentials = serde_json::from_str(&content)?;
        Ok(credentials)
    }

    pub async fn authenticate(&mut self, callback_url: &str) -> Result<()> {
        let redirect_url = RedirectUrl::new(callback_url.to_string())
            .map_err(|_| GmailError::AuthError("Invalid callback URL".to_string()))?;

        let client = self.client.clone().set_redirect_uri(redirect_url);

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("https://www.googleapis.com/auth/gmail.modify".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        info!("Please visit this URL to authenticate: {}", auth_url);

        // Open browser if possible
        if let Err(e) = open::that(auth_url.as_str()) {
            warn!("Could not open browser automatically: {}", e);
        }

        // Start local server to receive callback
        let listener = TcpListener::bind("127.0.0.1:3000").await
            .map_err(|e| GmailError::AuthError(format!("Failed to start local server: {}", e)))?;

        info!("Waiting for OAuth callback...");

        loop {
            let (mut stream, _) = listener.accept().await
                .map_err(|e| GmailError::AuthError(format!("Failed to accept connection: {}", e)))?;

            let mut reader = BufReader::new(&mut stream);
            let mut request_line = String::new();
            reader.read_line(&mut request_line).await?;

            let request_parts: Vec<&str> = request_line.split_whitespace().collect();
            if request_parts.len() >= 2 && request_parts[0] == "GET" {
                let path = request_parts[1];
                
                if path.starts_with("/oauth2callback") {
                    let url = format!("http://localhost:3000{}", path);
                    let parsed_url = Url::parse(&url)
                        .map_err(|_| GmailError::AuthError("Invalid callback URL".to_string()))?;

                    let mut code = None;
                    let mut state = None;

                    for (key, value) in parsed_url.query_pairs() {
                        match key.as_ref() {
                            "code" => code = Some(value.to_string()),
                            "state" => state = Some(value.to_string()),
                            _ => {}
                        }
                    }

                    if let Some(auth_code) = code {
                        // Verify CSRF token
                        if let Some(returned_state) = state {
                            if returned_state != *csrf_token.secret() {
                                let response = "HTTP/1.1 400 Bad Request\r\n\r\nCSRF token mismatch";
                                stream.write_all(response.as_bytes()).await?;
                                continue;
                            }
                        }

                        // Exchange code for token
                        match self.exchange_code_for_token(auth_code, pkce_verifier).await {
                            Ok(credentials) => {
                                self.credentials = Some(credentials.clone());
                                self.save_credentials(&credentials).await?;
                                
                                let response = "HTTP/1.1 200 OK\r\n\r\nAuthentication successful! You can close this window.";
                                stream.write_all(response.as_bytes()).await?;
                                return Ok(());
                            }
                            Err(e) => {
                                error!("Token exchange failed: {}", e);
                                let response = "HTTP/1.1 500 Internal Server Error\r\n\r\nAuthentication failed";
                                stream.write_all(response.as_bytes()).await?;
                                return Err(e);
                            }
                        }
                    } else {
                        let response = "HTTP/1.1 400 Bad Request\r\n\r\nNo authorization code provided";
                        stream.write_all(response.as_bytes()).await?;
                    }
                }
            }
        }
    }

    async fn exchange_code_for_token(
        &self,
        code: String,
        pkce_verifier: oauth2::PkceCodeVerifier,
    ) -> Result<OAuthCredentials> {
        let token_result = self
            .client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(|e| GmailError::OAuthError(format!("Token exchange failed: {}", e)))?;

        let credentials = OAuthCredentials {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
            expires_in: token_result.expires_in().map(|d| d.as_secs()),
            token_type: "Bearer".to_string(),
        };

        Ok(credentials)
    }

    async fn save_credentials(&self, credentials: &OAuthCredentials) -> Result<()> {
        let config_dir = Self::get_config_dir()?;
        let credentials_path = config_dir.join("credentials.json");
        
        let content = serde_json::to_string_pretty(credentials)?;
        fs::write(&credentials_path, content)?;
        
        Ok(())
    }

    pub fn get_access_token(&self) -> Result<&str> {
        self.credentials
            .as_ref()
            .map(|c| c.access_token.as_str())
            .ok_or_else(|| GmailError::AuthError("No access token available. Please authenticate first.".to_string()))
    }

    pub async fn refresh_token_if_needed(&mut self) -> Result<()> {
        // TODO: Implement token refresh logic
        // For now, just check if we have credentials
        if self.credentials.is_none() {
            return Err(GmailError::AuthError("No credentials available. Please authenticate first.".to_string()));
        }
        Ok(())
    }
}