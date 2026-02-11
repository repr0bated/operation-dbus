//! Google Cloud authentication for cloudaicompanion.googleapis.com.
//!
//! Supports multiple token sources:
//! 1. Cached token file (WG/MCP-proxy session context)
//! 2. gcloud CLI
//! 3. Application Default Credentials

use std::path::PathBuf;
use std::process::Command;

use chrono::{DateTime, Duration, Utc};
use tracing::{debug, info, warn};

const OAUTH_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/cloud-platform",
    "https://www.googleapis.com/auth/generative-language",
];

#[derive(Clone)]
pub struct GCloudAuth {
    /// Path to cached token file from local session context
    token_file_path: Option<PathBuf>,
}

impl GCloudAuth {
    pub fn new() -> Self {
        // 1) Explicit file path override
        let explicit = std::env::var("MCP_PROXY_TOKEN_FILE")
            .ok()
            .map(PathBuf::from)
            .filter(|p| p.exists());

        // 2) Preferred local token locations
        let discovered = dirs::home_dir().and_then(|home| {
            let candidates = [
                home.join(".config").join("op-mcp-proxy"),
                home.join(".op-mcp-proxy"),
                home.join(".antigravity-server"), // backward-compat
            ];
            candidates.into_iter().find_map(find_token_file_in_dir)
        });

        let token_file_path = explicit.or(discovered);

        if let Some(ref path) = token_file_path {
            debug!("Found cached token file at: {:?}", path);
        }

        Self {
            token_file_path,
        }
    }

    /// Get a valid OAuth token and its expiration time
    pub async fn get_token(&self) -> anyhow::Result<(String, DateTime<Utc>)> {
        // Try sources in order of preference

        // 1. Environment variable (for testing)
        if let Ok(token) = std::env::var("GCLOUD_TOKEN") {
            info!("Using token from GCLOUD_TOKEN env var");
            // Assume 1 hour validity
            return Ok((token, Utc::now() + Duration::hours(1)));
        }

        // 2. Cached token file
        if let Some(token) = self.try_cached_token_file().await {
            info!("Using token from cached token file");
            // These tokens are typically valid for 1 hour
            return Ok((token, Utc::now() + Duration::minutes(55)));
        }

        // 3. gcloud CLI
        if let Some((token, expires)) = self.try_gcloud_cli().await {
            info!("Using token from gcloud CLI");
            return Ok((token, expires));
        }

        // 4. Application Default Credentials via gcloud
        if let Some((token, expires)) = self.try_adc().await {
            info!("Using Application Default Credentials");
            return Ok((token, expires));
        }

        anyhow::bail!("Could not obtain OAuth token. Please run: gcloud auth login")
    }

    async fn try_cached_token_file(&self) -> Option<String> {
        let path = self.token_file_path.as_ref()?;

        let content = std::fs::read_to_string(path).ok()?;
        let token = content.trim().to_string();

        if token.is_empty() {
            return None;
        }

        // Basic validation - OAuth tokens start with "ya29."
        if token.starts_with("ya29.") {
            Some(token)
        } else {
            warn!("Cached token does not look like an OAuth token");
            None
        }
    }

    async fn try_gcloud_cli(&self) -> Option<(String, DateTime<Utc>)> {
        let scopes = OAUTH_SCOPES.join(",");

        let output = Command::new("gcloud")
            .args([
                "auth",
                "print-access-token",
                &format!("--scopes={}", scopes),
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!("gcloud auth print-access-token failed: {}", stderr);
            return None;
        }

        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if token.is_empty() {
            return None;
        }

        // gcloud tokens are valid for 1 hour
        Some((token, Utc::now() + Duration::minutes(55)))
    }

    async fn try_adc(&self) -> Option<(String, DateTime<Utc>)> {
        // Try application-default credentials
        let output = Command::new("gcloud")
            .args(["auth", "application-default", "print-access-token"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if token.is_empty() {
            return None;
        }

        Some((token, Utc::now() + Duration::minutes(55)))
    }

    /// Force a token refresh via gcloud
    #[allow(dead_code)]
    pub async fn refresh_token(&self) -> anyhow::Result<(String, DateTime<Utc>)> {
        let scopes = OAUTH_SCOPES.join(",");

        let output = Command::new("gcloud")
            .args([
                "auth",
                "print-access-token",
                &format!("--scopes={}", scopes),
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("gcloud auth failed: {}", stderr);
        }

        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok((token, Utc::now() + Duration::minutes(55)))
    }
}

impl Default for GCloudAuth {
    fn default() -> Self {
        Self::new()
    }
}

fn find_token_file_in_dir(dir: PathBuf) -> Option<PathBuf> {
    std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().map(|ext| ext == "token").unwrap_or(false))
}
