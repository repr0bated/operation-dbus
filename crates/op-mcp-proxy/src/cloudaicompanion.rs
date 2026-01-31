//! Direct Cloud-AI-Companion client (subscription endpoint).

use identity::CachedToken;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

const BASE: &str = "https://cloudaicompanion.googleapis.com/v1/projects/geminidev-479406/locations/global/publishers/google/models";
const MODEL: &str = "gemini-2.0-flash";

pub struct CloudAICompanion {
    cli: Client,
}

impl CloudAICompanion {
    pub fn new() -> Self {
        Self {
            cli: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("http client"),
        }
    }

    /// Generate text using the subscription endpoint.
    pub async fn generate(&self, prompt: &str, token: &CachedToken) -> anyhow::Result<String> {
        let url = format!("{BASE}/{MODEL}:generateContent");
        let body = simd_json::json!({
            "contents": [{ "role": "user", "parts": [{ "text": prompt }] }],
            "generationConfig": {
                "temperature": 0.7,
                "maxOutputTokens": 8192,
                "topP": 0.95,
                "topK": 40
            }
        });
        debug!("POST {} …", url);
        let resp = self.cli
            .post(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", token.access_token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("cloudaicompanion error {}: {}", resp.status(), resp.text().await?);
        }
        let json: simd_json::OwnedValue = resp.json().await?;
        let text = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        Ok(text)
    }
}
