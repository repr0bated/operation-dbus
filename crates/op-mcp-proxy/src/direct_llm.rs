//! Direct-mode handler for LLM MCP methods.
//! Uses Gemini CLI OAuth token (~/.gemini/oauth_creds.json) for auth.

use crate::cloudaicompanion::{self, CloudAICompanion};
use simd_json::prelude::*;
use simd_json::OwnedValue as Value;
use tokio::sync::Mutex;
use tracing::{info, warn};

pub struct DirectLLM {
    companion: CloudAICompanion,
    cached_token: Mutex<Option<String>>,
}

impl DirectLLM {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            companion: CloudAICompanion::new(),
            cached_token: Mutex::new(None),
        })
    }

    /// Get a valid Gemini CLI token, refreshing if needed.
    async fn get_token(&self) -> anyhow::Result<String> {
        // Try cached token from the gemini CLI creds file
        match cloudaicompanion::read_gemini_cli_token() {
            Ok((token, expiry_ms)) => {
                let now_ms = chrono::Utc::now().timestamp_millis();
                // If token expires in more than 5 minutes, use it
                if expiry_ms > now_ms + 300_000 {
                    *self.cached_token.lock().await = Some(token.clone());
                    return Ok(token);
                }
                info!("Gemini CLI token expired or expiring soon, refreshing...");
            }
            Err(e) => {
                warn!("Cannot read gemini CLI token: {}", e);
            }
        }

        // Refresh
        let token = cloudaicompanion::refresh_gemini_cli_token().await?;
        *self.cached_token.lock().await = Some(token.clone());
        Ok(token)
    }

    /// Handle any MCP LLM-style request and return a JSON-RPC result.
    pub async fn handle(&self, req: &Value) -> Value {
        let id = req.get("id").cloned().unwrap_or_else(Value::null);
        let params = req.get("params").cloned().unwrap_or_else(Value::null);
        let prompt = match Self::extract_prompt(&params) {
            Ok(p) => p,
            Err(e) => return error(&id, -32700, e.to_string()),
        };
        let model = params
            .get("model")
            .and_then(|m| m.as_str())
            .filter(|m| !m.trim().is_empty());

        let token = match self.get_token().await {
            Ok(t) => t,
            Err(e) => return error(&id, -32603, format!("token: {e}")),
        };

        match self.companion.generate(&prompt, &token, model).await {
            Ok(text) => simd_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "completion": text,
                    "model": model.unwrap_or("gemini-2.5-flash"),
                    "stopReason": "stop"
                }
            }),
            Err(e) => error(&id, -32603, format!("generate: {e}")),
        }
    }

    fn extract_prompt(params: &Value) -> anyhow::Result<String> {
        if let Some(msg_array) = params.get("messages").and_then(|v| v.as_array()) {
            return Ok(msg_array
                .iter()
                .filter_map(|m| {
                    let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("user");
                    let content = m.get("content")?;
                    let txt = content
                        .get("text")
                        .and_then(|v| v.as_str())
                        .or_else(|| content.as_str())?;
                    Some(format!("{role}: {txt}"))
                })
                .collect::<Vec<_>>()
                .join("\n"));
        }

        if let Some(txt) = params.get("prompt").and_then(|v| v.as_str()) {
            return Ok(txt.to_string());
        }

        if let Some(txt) = params
            .get("ref")
            .and_then(|r| r.get("text"))
            .and_then(|v| v.as_str())
        {
            return Ok(txt.to_string());
        }

        anyhow::bail!("no prompt found")
    }
}

fn error(id: &Value, code: i32, msg: impl Into<String>) -> Value {
    simd_json::json!({
        "jsonrpc": "2.0",
        "id": id.clone(),
        "error": {
            "code": code,
            "message": msg.into()
        }
    })
}
