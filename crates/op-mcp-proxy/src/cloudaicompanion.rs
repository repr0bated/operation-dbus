//! Code Assist client – uses the cloudcode-pa.googleapis.com endpoint
//! with Gemini CLI OAuth credentials (~/.gemini/oauth_creds.json).

use anyhow::Context;
use reqwest::{header, Client};
use simd_json::prelude::*;
use simd_json::OwnedValue;
use std::path::PathBuf;
use tracing::{debug, info, warn};

const CODE_ASSIST_ENDPOINT: &str = "https://cloudcode-pa.googleapis.com";
const CODE_ASSIST_API_VERSION: &str = "v1internal";
const DEFAULT_MODEL: &str = "gemini-2.5-flash";
const DEFAULT_USER_AGENT: &str =
    "google-cloud-code-vscode/1.22.0 (GPN:Cloud Code for VS Code) vscode/1.85.0 (linux; x64)";
const DEFAULT_X_GOOG_API_CLIENT: &str = "gl-rust/1.76.0 gax/2.12.0 gapic/1.0.0";
const DEFAULT_ORIGIN: &str = "vscode://googlecloudtools.cloudcode";
const DEFAULT_REFERER: &str = "vscode://googlecloudtools.cloudcode";
const DEFAULT_X_CLIENT_DATA: &str =
    "eyJpc0lkZSI6dHJ1ZSwiaWRlVHlwZSI6InZzY29kZSIsImlkZVZlcnNpb24iOiIxLjg1LjAiLCJwbHVnaW5WZXJzaW9uIjoiMS4yMi4wIn0=";

#[derive(Debug, Clone)]
struct IdeEmulationHeaders {
    user_agent: String,
    x_goog_api_client: String,
    origin: String,
    referer: String,
    x_client_data: String,
}

pub struct CloudAICompanion {
    cli: Client,
    project: String,
    headers: IdeEmulationHeaders,
    send_user_project_header: bool,
}

impl CloudAICompanion {
    pub fn new() -> Self {
        let project = std::env::var("MCP_PROXY_GCLOUD_PROJECT")
            .or_else(|_| std::env::var("GCLOUD_PROJECT"))
            .or_else(|_| std::env::var("GOOGLE_CLOUD_PROJECT"))
            .unwrap_or_else(|_| {
                // Read quota_project_id from ADC credentials file
                read_adc_project().unwrap_or_else(|| "operation-dbus".to_string())
            });

        let headers = IdeEmulationHeaders {
            user_agent: std::env::var("MCP_PROXY_USER_AGENT")
                .or_else(|_| std::env::var("USER_AGENT"))
                .unwrap_or_else(|_| DEFAULT_USER_AGENT.to_string()),
            x_goog_api_client: std::env::var("MCP_PROXY_X_GOOG_API_CLIENT")
                .or_else(|_| std::env::var("X_GOOG_API_CLIENT"))
                .unwrap_or_else(|_| DEFAULT_X_GOOG_API_CLIENT.to_string()),
            origin: std::env::var("MCP_PROXY_ORIGIN")
                .unwrap_or_else(|_| DEFAULT_ORIGIN.to_string()),
            referer: std::env::var("MCP_PROXY_REFERER")
                .unwrap_or_else(|_| DEFAULT_REFERER.to_string()),
            x_client_data: std::env::var("MCP_PROXY_X_CLIENT_DATA")
                .unwrap_or_else(|_| DEFAULT_X_CLIENT_DATA.to_string()),
        };
        let send_user_project_header = env_flag("MCP_PROXY_SEND_X_GOOG_USER_PROJECT", true);

        info!("Code Assist project: {}", project);
        info!(
            "MCP bridge IDE emulation enabled (user-agent: {})",
            headers.user_agent
        );

        Self {
            cli: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("http client"),
            project,
            headers,
            send_user_project_header,
        }
    }

    /// Generate text using the Code Assist endpoint (cloudcode-pa.googleapis.com).
    pub async fn generate(
        &self,
        prompt: &str,
        token: &str,
        model: Option<&str>,
    ) -> anyhow::Result<String> {
        let env_model = std::env::var("MODEL_ID").ok();
        let model = model
            .or(env_model.as_deref())
            .unwrap_or(DEFAULT_MODEL);
        let endpoint =
            std::env::var("CODE_ASSIST_ENDPOINT").unwrap_or_else(|_| CODE_ASSIST_ENDPOINT.into());
        let url = format!("{}/{}:generateContent", endpoint, CODE_ASSIST_API_VERSION);

        let body = serde_json::json!({
            "model": model,
            "project": self.project,
            "user_prompt_id": uuid::Uuid::new_v4().to_string(),
            "request": {
                "contents": [{ "role": "user", "parts": [{ "text": prompt }] }],
                "generationConfig": {
                    "temperature": 0.7,
                    "maxOutputTokens": 8192,
                    "topP": 0.95,
                    "topK": 40
                },
                "session_id": ""
            }
        });

        debug!("POST {} model={}", url, model);

        let mut request = self
            .cli
            .post(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::USER_AGENT, &self.headers.user_agent)
            .header("x-goog-api-client", &self.headers.x_goog_api_client)
            .header("x-client-data", &self.headers.x_client_data)
            .header(header::ORIGIN, &self.headers.origin)
            .header(header::REFERER, &self.headers.referer)
            .body(body.to_string());

        if self.send_user_project_header && !self.project.is_empty() {
            request = request.header("x-goog-user-project", &self.project);
        }

        let resp = request.send().await?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "code-assist error {}: {}",
                resp.status(),
                resp.text().await?
            );
        }

        let mut resp_bytes = resp.bytes().await?.to_vec();
        let json: OwnedValue = simd_json::from_slice(&mut resp_bytes)
            .context("failed to parse code-assist response")?;

        // Code Assist wraps the response in { "response": { ... }, "traceId": "..." }
        let inner = json.get("response").context("missing 'response' in code-assist reply")?;

        if let Some(reason) = inner
            .get("promptFeedback")
            .and_then(|pf| pf.get("blockReason"))
            .and_then(|r| r.as_str())
        {
            anyhow::bail!("content blocked: {}", reason);
        }

        let text = inner
            .get("candidates")
            .and_then(|c| c.get_idx(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get_idx(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        if text.is_empty() {
            anyhow::bail!("empty response text from code-assist");
        }

        Ok(text)
    }
}

/// Read the Gemini CLI access token from ~/.gemini/oauth_creds.json.
/// Returns (access_token, expiry_epoch_ms).
pub fn read_gemini_cli_token() -> anyhow::Result<(String, i64)> {
    let path = gemini_creds_path()
        .context("cannot locate ~/.gemini/oauth_creds.json")?;
    let mut text = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    let creds: OwnedValue = unsafe { simd_json::from_str(&mut text) }
        .with_context(|| format!("cannot parse {}", path.display()))?;

    let token = creds
        .get("access_token")
        .and_then(|v| v.as_str())
        .context("missing access_token in gemini oauth_creds")?
        .to_string();
    let expiry = creds
        .get("expiry_date")
        .and_then(|v| v.as_f64())
        .map(|v| v as i64)
        .unwrap_or(0);

    Ok((token, expiry))
}

/// Refresh the Gemini CLI token using its refresh_token and client credentials.
pub async fn refresh_gemini_cli_token() -> anyhow::Result<String> {
    let path = gemini_creds_path()
        .context("cannot locate ~/.gemini/oauth_creds.json")?;
    let mut text = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    let creds: OwnedValue = unsafe { simd_json::from_str(&mut text) }
        .with_context(|| format!("cannot parse {}", path.display()))?;

    let refresh_token = creds
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .context("missing refresh_token")?;

    let (client_id, client_secret) = read_env_oauth_client()
        .or_else(read_adc_oauth_client)
        .context(
            "missing OAuth client credentials; set GEMINI_OAUTH_CLIENT_ID and \
             GEMINI_OAUTH_CLIENT_SECRET or run gcloud application-default auth login",
        )?;

    let cli = Client::new();
    let resp = cli
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("token refresh failed {}: {}", resp.status(), resp.text().await?);
    }

    let mut resp_bytes = resp.bytes().await?.to_vec();
    let body: OwnedValue = simd_json::from_slice(&mut resp_bytes)
        .context("cannot parse token refresh response")?;

    let new_token = body
        .get("access_token")
        .and_then(|v| v.as_str())
        .context("missing access_token in refresh response")?
        .to_string();
    let expires_in = body
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(3600);

    // Update the cached credentials file
    let new_expiry = chrono::Utc::now().timestamp_millis() + (expires_in as i64 * 1000);
    let updated = serde_json::json!({
        "access_token": new_token,
        "scope": creds.get("scope").and_then(|v| v.as_str()).unwrap_or(""),
        "token_type": "Bearer",
        "expiry_date": new_expiry,
        "refresh_token": refresh_token,
    });
    if let Err(e) = std::fs::write(&path, serde_json::to_string_pretty(&updated)?) {
        warn!("Could not update gemini oauth_creds.json: {}", e);
    } else {
        info!("Refreshed gemini CLI token, expires in {}s", expires_in);
    }

    Ok(new_token)
}

fn gemini_creds_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".gemini").join("oauth_creds.json"))
        .filter(|p| p.exists())
}

fn read_adc_project() -> Option<String> {
    let path = dirs::config_dir()?
        .join("gcloud")
        .join("application_default_credentials.json");
    let mut text = std::fs::read_to_string(path).ok()?;
    let val: OwnedValue = unsafe { simd_json::from_str(&mut text) }.ok()?;
    val.get("quota_project_id")
        .and_then(|v| v.as_str())
        .map(String::from)
}

fn read_adc_oauth_client() -> Option<(String, String)> {
    let path = dirs::config_dir()?
        .join("gcloud")
        .join("application_default_credentials.json");
    let mut text = std::fs::read_to_string(path).ok()?;
    let val: OwnedValue = unsafe { simd_json::from_str(&mut text) }.ok()?;
    let client_id = val.get("client_id").and_then(|v| v.as_str())?.to_string();
    let client_secret = val.get("client_secret").and_then(|v| v.as_str())?.to_string();
    Some((client_id, client_secret))
}

fn read_env_oauth_client() -> Option<(String, String)> {
    let client_id = std::env::var("GEMINI_OAUTH_CLIENT_ID").ok()?;
    let client_secret = std::env::var("GEMINI_OAUTH_CLIENT_SECRET").ok()?;
    Some((client_id, client_secret))
}

fn env_flag(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(default)
}
