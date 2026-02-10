//! Direct-mode handler for LLM MCP methods.

use crate::cloudaicompanion::CloudAICompanion;
use identity::{CachedToken, SessionManager};
use simd_json::OwnedValue;

pub struct DirectLLM {
    session: SessionManager,
    companion: CloudAICompanion,
}

impl DirectLLM {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            session: SessionManager::new()?,
            companion: CloudAICompanion::new(),
        })
    }

    /// Handle any MCP LLM-style request and return a JSON-RPC result.
    pub async fn handle(&self, req: &Value) -> Value {
        let id = &req["id"];
        let method = req["method"].as_str().unwrap_or("");
        let params = &req["params"];
        let prompt = match Self::extract_prompt(params) {
            Ok(p) => p,
            Err(e) => return error(id, -32700, e.to_string()),
        };
        let token = match self.session.get_valid_token().await {
            Ok(t) => t,
            Err(e) => return error(id, -32603, format!("token: {e}")),
        };
        match self.companion.generate(&prompt, &token).await {
            Ok(text) => Value::Object(simd_json::value::owned::Object::from_iter([
                ("jsonrpc".into(), Value::String("2.0".into())),
                ("id".into(), id.clone()),
                (
                    "result".into(),
                    Value::Object(simd_json::value::owned::Object::from_iter([
                        ("completion".into(), Value::String(text)),
                        ("model".into(), Value::String("gemini-2.0-flash".into())),
                        ("stopReason".into(), Value::String("stop".into())),
                    ])),
                ),
            ])),
            Err(e) => error(id, -32603, format!("generate: {e}")),
        }
    }

    fn extract_prompt(p: &Value) -> anyhow::Result<String> {
        if let Some(msg_array) = p["messages"].as_array() {
            return Ok(msg_array
                .iter()
                .filter_map(|m| {
                    let role = m["role"].as_str().unwrap_or("user");
                    let txt = m["content"]["text"]
                        .as_str()
                        .or_else(|| m["content"].as_str())?;
                    Some(format!("{role}: {txt}"))
                })
                .collect::<Vec<_>>()
                .join("\n"));
        }
        if let Some(txt) = p["prompt"].as_str() {
            return Ok(txt.to_string());
        }
        anyhow::bail!("no prompt found")
    }
}

fn error(id: &Value, code: i32, msg: impl Into<String>) -> Value {
    Value::Object(simd_json::value::owned::Object::from_iter([
        ("jsonrpc".into(), Value::String("2.0".into())),
        ("id".into(), id.clone()),
        (
            "error".into(),
            Value::Object(simd_json::value::owned::Object::from_iter([
                ("code".into(), Value::Number(code.into())),
                ("message".into(), Value::String(msg.into())),
            ])),
        ),
    ]))
}
