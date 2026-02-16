//! Code Search Tools - Semantic search across indexed repositories
//!
//! Queries local Qdrant vector DB populated by openclaw-indexer.
//! Embeds queries via HuggingFace Inference API, searches Qdrant for
//! semantically similar code chunks.

use anyhow::{Context, Result};
use async_trait::async_trait;
use simd_json::{json, OwnedValue as Value};
use simd_json::prelude::*;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::registry::ToolDefinition;
use crate::Tool;

const QDRANT_URL: &str = "http://127.0.0.1:6333";
const COLLECTION: &str = "code_chunks";
const HF_EMBED_MODEL: &str = "BAAI/bge-base-en-v1.5";

fn qdrant_url() -> String {
    std::env::var("QDRANT_URL").unwrap_or_else(|_| QDRANT_URL.to_string())
}

fn hf_token() -> Option<String> {
    std::env::var("HF_TOKEN").ok()
}

/// Embed a query string via HuggingFace Inference API
async fn embed_query(text: &str) -> Result<Vec<f64>> {
    let token = hf_token().context("HF_TOKEN not set - needed for code search embeddings")?;
    let client = reqwest::Client::new();
    let url = format!(
        "https://router.huggingface.co/hf-inference/models/{}",
        HF_EMBED_MODEL
    );

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&simd_json::json!({"inputs": text}))
        .send()
        .await
        .context("Failed to call HF embedding API")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("HF API error ({}): {}", status, body));
    }

    let body = response.text().await?;
    let mut body_mut = body.clone();
    let parsed: Value = unsafe { simd_json::from_str(&mut body_mut) }
        .context("Failed to parse HF embedding response")?;

    // Response is a flat array of floats
    match parsed.as_array() {
        Some(arr) => {
            let vec: Vec<f64> = arr
                .iter()
                .filter_map(|v| v.as_f64())
                .collect();
            if vec.is_empty() {
                Err(anyhow::anyhow!("Empty embedding returned"))
            } else {
                Ok(vec)
            }
        }
        None => Err(anyhow::anyhow!("Unexpected embedding format: {}", &body[..body.len().min(200)])),
    }
}

/// Search Qdrant with a vector and optional filters
async fn search_qdrant(
    vector: Vec<f64>,
    repo: Option<&str>,
    language: Option<&str>,
    limit: usize,
) -> Result<Value> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/collections/{}/points/query",
        qdrant_url(),
        COLLECTION
    );

    let mut must_conditions: Vec<Value> = Vec::new();
    if let Some(r) = repo {
        must_conditions.push(json!({
            "key": "repo",
            "match": {"value": r}
        }));
    }
    if let Some(l) = language {
        must_conditions.push(json!({
            "key": "language",
            "match": {"value": l}
        }));
    }

    let mut body = json!({
        "query": vector,
        "limit": limit,
        "with_payload": true
    });

    if !must_conditions.is_empty() {
        body["filter"] = json!({"must": must_conditions});
    }

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .context("Failed to query Qdrant")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Qdrant error ({}): {}", status, body));
    }

    let text = response.text().await?;
    let mut text_mut = text.clone();
    let parsed: Value = unsafe { simd_json::from_str(&mut text_mut) }
        .context("Failed to parse Qdrant response")?;

    Ok(parsed)
}

// ============================================================================
// CODE SEARCH TOOL
// ============================================================================

pub struct CodeSearchTool;

#[async_trait]
impl Tool for CodeSearchTool {
    fn name(&self) -> &str {
        "code_search"
    }

    fn description(&self) -> &str {
        "Semantic code search across all indexed repositories. \
         Finds functions, structs, types, and code blocks matching a natural language query. \
         Powered by vector embeddings in local Qdrant."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural language search query (e.g. 'D-Bus service discovery', 'async network bridge creation')"
                },
                "repo": {
                    "type": "string",
                    "description": "Optional: filter by repo name (e.g. 'operation-dbus', 'rtnetlink')"
                },
                "language": {
                    "type": "string",
                    "description": "Optional: filter by language (e.g. 'rust', 'python', 'typescript')"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max results to return (default: 5, max: 20)",
                    "default": 5
                }
            },
            "required": ["query"]
        })
    }

    fn category(&self) -> &str {
        "code_intelligence"
    }

    fn tags(&self) -> Vec<String> {
        vec!["search".into(), "code".into(), "rag".into(), "qdrant".into()]
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let query = input.get("query")
            .and_then(|v| v.as_str())
            .context("Missing required 'query' parameter")?;

        let repo = input.get("repo").and_then(|v| v.as_str());
        let language = input.get("language").and_then(|v| v.as_str());
        let limit = input.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(5)
            .min(20) as usize;

        info!("Code search: '{}' (repo={:?}, lang={:?}, limit={})", query, repo, language, limit);

        // Embed the query
        let vector = embed_query(query).await?;
        debug!("Embedded query into {}d vector", vector.len());

        // Search Qdrant
        let result = search_qdrant(vector, repo, language, limit).await?;

        // Format results
        let empty_vec = vec![];
        let points = result.get("result")
            .and_then(|r| r.get("points"))
            .and_then(|p| p.as_array())
            .unwrap_or(&empty_vec);

        let empty_obj = json!({});
        let mut results: Vec<Value> = Vec::new();
        for point in points {
            let payload = point.get("payload").unwrap_or(&empty_obj);
            let score = point.get("score").and_then(|s| s.as_f64()).unwrap_or(0.0);

            results.push(json!({
                "score": score,
                "repo": payload.get("repo").and_then(|v| v.as_str()).unwrap_or(""),
                "file": payload.get("file_path").and_then(|v| v.as_str()).unwrap_or(""),
                "language": payload.get("language").and_then(|v| v.as_str()).unwrap_or(""),
                "type": payload.get("chunk_type").and_then(|v| v.as_str()).unwrap_or(""),
                "name": payload.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                "lines": format!(
                    "{}-{}",
                    payload.get("line_start").and_then(|v| v.as_u64()).unwrap_or(0),
                    payload.get("line_end").and_then(|v| v.as_u64()).unwrap_or(0)
                ),
                "content": payload.get("content").and_then(|v| v.as_str()).unwrap_or("")
            }));
        }

        Ok(json!({
            "results": results,
            "count": results.len(),
            "query": query
        }))
    }
}

// ============================================================================
// CODE INDEX STATUS TOOL
// ============================================================================

pub struct CodeIndexStatusTool;

#[async_trait]
impl Tool for CodeIndexStatusTool {
    fn name(&self) -> &str {
        "code_index_status"
    }

    fn description(&self) -> &str {
        "Get the status of the code search index: collection stats, repos indexed, chunk counts."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn category(&self) -> &str {
        "code_intelligence"
    }

    fn tags(&self) -> Vec<String> {
        vec!["search".into(), "status".into()]
    }

    async fn execute(&self, _input: Value) -> Result<Value> {
        let client = reqwest::Client::new();

        // Get collection info
        let url = format!("{}/collections/{}", qdrant_url(), COLLECTION);
        let resp = client.get(&url).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let text = r.text().await?;
                let mut text_mut = text.clone();
                let parsed: Value = unsafe { simd_json::from_str(&mut text_mut) }
                    .unwrap_or(json!({"error": "parse failed"}));

                let empty = json!({});
                let result = parsed.get("result").unwrap_or(&empty);
                let points = result.get("points_count").and_then(|v| v.as_u64()).unwrap_or(0);
                let vectors = result.get("vectors_count").and_then(|v| v.as_u64()).unwrap_or(0);
                let status = result.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");

                Ok(json!({
                    "status": "ok",
                    "collection": COLLECTION,
                    "points_count": points,
                    "vectors_count": vectors,
                    "index_status": status,
                    "qdrant_url": qdrant_url()
                }))
            }
            Ok(r) => {
                Ok(json!({
                    "status": "error",
                    "message": format!("Qdrant returned {}", r.status()),
                    "qdrant_url": qdrant_url()
                }))
            }
            Err(e) => {
                Ok(json!({
                    "status": "unavailable",
                    "message": format!("Cannot reach Qdrant: {}", e),
                    "qdrant_url": qdrant_url()
                }))
            }
        }
    }
}

// ============================================================================
// LIST INDEXED REPOS TOOL
// ============================================================================

pub struct ListIndexedReposTool;

#[async_trait]
impl Tool for ListIndexedReposTool {
    fn name(&self) -> &str {
        "list_indexed_repos"
    }

    fn description(&self) -> &str {
        "List all repositories indexed in the code search system with chunk counts."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn category(&self) -> &str {
        "code_intelligence"
    }

    fn tags(&self) -> Vec<String> {
        vec!["search".into(), "repos".into()]
    }

    async fn execute(&self, _input: Value) -> Result<Value> {
        let client = reqwest::Client::new();

        // Scroll through all points to get unique repos
        let url = format!(
            "{}/collections/{}/points/scroll",
            qdrant_url(),
            COLLECTION
        );

        let body = json!({
            "limit": 10000,
            "with_payload": ["repo"]
        });

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to scroll Qdrant")?;

        if !resp.status().is_success() {
            return Ok(json!({"status": "error", "message": "Qdrant query failed"}));
        }

        let text = resp.text().await?;
        let mut text_mut = text.clone();
        let parsed: Value = unsafe { simd_json::from_str(&mut text_mut) }
            .context("Failed to parse Qdrant response")?;

        let empty_vec = vec![];
        let points = parsed.get("result")
            .and_then(|r| r.get("points"))
            .and_then(|p| p.as_array())
            .unwrap_or(&empty_vec);

        // Count per repo
        let mut repo_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        for point in points {
            if let Some(repo) = point.get("payload")
                .and_then(|p| p.get("repo"))
                .and_then(|r| r.as_str())
            {
                *repo_counts.entry(repo.to_string()).or_default() += 1;
            }
        }

        let mut repos: Vec<Value> = repo_counts
            .into_iter()
            .map(|(name, count)| json!({"repo": name, "chunks": count}))
            .collect();
        repos.sort_by(|a, b| {
            let a_name = a.get("repo").and_then(|v| v.as_str()).unwrap_or("");
            let b_name = b.get("repo").and_then(|v| v.as_str()).unwrap_or("");
            a_name.cmp(b_name)
        });

        Ok(json!({
            "repos": repos,
            "total_repos": repos.len()
        }))
    }
}

// ============================================================================
// REGISTRATION
// ============================================================================

pub async fn register_code_search_tools(registry: &crate::ToolRegistry) -> Result<()> {
    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(CodeSearchTool),
        Arc::new(CodeIndexStatusTool),
        Arc::new(ListIndexedReposTool),
    ];

    for tool in tools {
        let name = tool.name().to_string();
        let definition = ToolDefinition {
            name: name.clone(),
            description: tool.description().to_string(),
            input_schema: tool.input_schema(),
            schema_version: "https://json-schema.org/draft/next/schema".to_string(),
            category: tool.category().to_string(),
            namespace: "code_intelligence.v1".to_string(),
            tags: tool.tags(),
        };
        registry.register(name.into(), tool, definition).await?;
    }

    Ok(())
}
