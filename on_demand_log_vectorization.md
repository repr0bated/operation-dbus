# On-Demand Log Vectorization in Operation-DBus

## Configurable Vectorization Levels

Operation-DBus introduces runtime-configurable vectorization with four semantic depth levels:

### 1. None

- **Behavior:** No vectorization or embedding.
- **Implementation:** `vector_features` left empty or zeroed.
- **Overhead:** None.

### 2. Low

- **Purpose:** Basic keyword and embedding extraction.
- **Model:** `MiniLM-L3-v2` (\~17M parameters, 384-dim, \~61MB).
- **Performance:** \~19k sentences/sec on CPU.
- **Use Case:** Fast, coarse semantic representation capturing simple log semantics.

### 3. Medium

- **Purpose:** Sentence-level transformer encoding.
- **Model:** `MiniLM-L6-v2` (\~22.7M params, 384-dim, \~80MB).
- **Performance:** \~14k sentences/sec on CPU.
- **Alternative:** `all-distilroberta-v1` (768-dim, slower but deeper).
- **Use Case:** Captures contextual meaning of full log messages.

### 4. High

- **Purpose:** Full document-level embedding including metadata.
- **Model:** `all-MPNet-base-v2` (\~110M params, 768-dim, \~420MB).
- **Performance:** \~2.8k sentences/sec on CPU.
- **Use Case:** High-fidelity semantic search, clustering, anomaly detection.

### Notes

- Low/Medium models output 384-d vectors; High uses 768-d.
- Mixed dimensionality requires separate or reindexed vector databases.

---

## Implementation Plan

### 1. Configuration of Vectorization Level

Define `OP_DBUS_VECTOR_LEVEL={none|low|medium|high}` via environment or CLI (`--vectorize-level`).

```rust
enum VectorizationLevel { None, Low, Medium, High }
```

Default: `None` for safety.

### 2. Model Initialization

Load transformer at startup based on configured level:

- **Low:** MiniLM-L3
- **Medium:** MiniLM-L6
- **High:** MPNet or RoBERTa-base

Embedding handled through a persistent `TextEmbedder` struct using ONNX Runtime or Rust-BERT (`tch`).

### 3. PluginFootprint Integration

In `FootprintGenerator::create_footprint()`:

```rust
match GLOBAL_LEVEL {
    None => Vec::new(),
    _ => {
        let text = self.prepare_text_for_embedding(...);
        EMBEDDER.embed(&text).unwrap_or_default()
    }
}
```

- Use flattened log metadata for embedding input.
- L2-normalize output for cosine similarity.
- Store 384-d or 768-d vector as `vector_features`.

### 4. Asynchronous Processing

For performance:

- Offload embedding to background `StreamingBlockchain` thread.
- Optionally batch multiple logs per inference pass.

### 5. Optimization

- Use quantized ONNX or GGML models (int8/FP16) to reduce memory.
- Enable multi-threaded inference for CPU efficiency.
- Expect memory: Low/Med <100MB; High \~420MB (210MB FP16).

### 6. Fallback and Robustness

- **Model Load Failure:** Fallback to lower level.
- **Timeouts/Errors:** Skip vector or mark incomplete.
- **Dynamic Downgrade:** (future) Adaptive based on system load.

### 7. Storage and Usage

- Write vectors to `.vec` JSON files within the `vectors` subvolume.
- Include metadata: `{ "vector_level": "medium" }`.
- Use Qdrant for vector search (384-d or 768-d index).
- Support offline backfill by upgrading vectorization level later.

---

## Summary Table

| Level  | Model         | Dim | Params | Speed (CPU) | Memory  | Typical Use    |
| ------ | ------------- | --- | ------ | ----------- | ------- | -------------- |
| None   | N/A           | 0   | 0      | N/A         | 0       | No embedding   |
| Low    | MiniLM-L3-v2  | 384 | 17M    | \~19k/s     | \~61MB  | Keyword-level  |
| Medium | MiniLM-L6-v2  | 384 | 22.7M  | \~14k/s     | \~80MB  | Sentence-level |
| High   | MPNet-base-v2 | 768 | 110M   | \~2.8k/s    | \~420MB | Full semantics |

---

## References

- [`plugin_footprint.rs`](https://github.com/repr0bated/operation-dbus/blob/main/src/blockchain/plugin_footprint.rs)
- [`streaming_blockchain.rs`](https://github.com/repr0bated/operation-dbus/blob/main/src/blockchain/streaming_blockchain.rs)
- [MiniLM-L3-v2](https://huggingface.co/sentence-transformers/paraphrase-MiniLM-L3-v2)
- [MiniLM-L6-v2](https://huggingface.co/sentence-transformers/paraphrase-MiniLM-L6-v2)
- [MPNet-base-v2](https://huggingface.co/sentence-transformers/all-mpnet-base-v2)
- [ONNX Quantized Models](https://huggingface.co/onnx-models)

