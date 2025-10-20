# Transformer-Based Vectorization Setup

This document describes how to set up and use transformer-based vectorization for blockchain footprints in op-dbus.

## Overview

op-dbus supports **on-demand transformer vectorization** with four configurable semantic depth levels:

- **None** (default): No vectorization, zero overhead
- **Low**: MiniLM-L3-v2 (384-dim, ~61MB, ~19k sentences/sec)
- **Medium**: MiniLM-L6-v2 (384-dim, ~80MB, ~14k sentences/sec)
- **High**: MPNet-base-v2 (768-dim, ~420MB, ~2.8k sentences/sec)

## Building with ML Support

### Compile with Transformer Support

```bash
cargo build --release --features ml
```

### Compile without ML (default)

```bash
cargo build --release
```

When built without the `ml` feature, the system automatically falls back to fast heuristic vectorization.

## Configuration

### Environment Variable

Set the vectorization level via environment variable:

```bash
# No vectorization (default)
export OP_DBUS_VECTOR_LEVEL=none

# Low level (fast, keyword-based)
export OP_DBUS_VECTOR_LEVEL=low

# Medium level (balanced, recommended)
export OP_DBUS_VECTOR_LEVEL=medium

# High level (slow, best quality)
export OP_DBUS_VECTOR_LEVEL=high
```

### Model Directory

By default, models are stored in `/var/lib/op-dbus/models/`. Override with:

```bash
export OP_DBUS_MODEL_DIR=/custom/path/to/models
```

## Model Setup

### Automatic Download (Recommended)

**Models are automatically downloaded on first use!**

When you enable vectorization, op-dbus will:
1. Check if the model exists locally
2. If missing, download from Hugging Face automatically
3. Cache in `/var/lib/op-dbus/models/` for future use

No manual setup required!

```bash
# Just set the level and run - model downloads automatically
export OP_DBUS_VECTOR_LEVEL=medium
./op-dbus daemon

# First run logs:
# INFO: Model not found locally, downloading from Hugging Face...
# INFO: Downloading model sentence-transformers/paraphrase-MiniLM-L6-v2...
# INFO:   ✓ Downloaded model.onnx
# INFO:   ✓ Downloaded tokenizer.json
# INFO: Model download complete
# INFO: Successfully loaded medium model (80MB) on cpu
```

### Directory Structure

Models are automatically organized as:

```
/var/lib/op-dbus/models/
├── paraphrase-MiniLM-L3-v2/
│   ├── model.onnx
│   └── tokenizer.json
├── paraphrase-MiniLM-L6-v2/
│   ├── model.onnx
│   └── tokenizer.json
└── all-mpnet-base-v2/
    ├── model.onnx
    └── tokenizer.json
```

### Manual Download (Optional)

If you prefer to download models manually or for offline use:

#### Option 1: Using Hugging Face Hub

```bash
# Install huggingface-hub
pip install huggingface-hub onnx

# Download and convert model
python3 << EOF
from huggingface_hub import snapshot_download
from pathlib import Path

model_name = "sentence-transformers/paraphrase-MiniLM-L6-v2"
model_dir = "/var/lib/op-dbus/models"

# Download model
snapshot_download(
    repo_id=model_name,
    local_dir=f"{model_dir}/{model_name.split('/')[-1]}",
    allow_patterns=["*.onnx", "tokenizer.json"]
)
EOF
```

#### Option 2: Manual Download

1. Visit https://huggingface.co/sentence-transformers/paraphrase-MiniLM-L6-v2
2. Download `model.onnx` and `tokenizer.json`
3. Place in `/var/lib/op-dbus/models/paraphrase-MiniLM-L6-v2/`

### Offline / Air-Gapped Environments

For systems without internet access:

1. Download models on a machine with internet
2. Copy the entire `/var/lib/op-dbus/models/` directory to the offline system
3. Models will be used without attempting download

```bash
# On internet-connected machine
scp -r /var/lib/op-dbus/models/ user@offline-host:/var/lib/op-dbus/
```

### Converting PyTorch Models to ONNX

If only PyTorch models are available:

```bash
# Install dependencies
pip install transformers onnx torch

# Convert model
python3 << EOF
from transformers import AutoTokenizer, AutoModel
import torch

model_name = "sentence-transformers/paraphrase-MiniLM-L6-v2"
output_dir = "/var/lib/op-dbus/models/paraphrase-MiniLM-L6-v2"

# Load model and tokenizer
tokenizer = AutoTokenizer.from_pretrained(model_name)
model = AutoModel.from_pretrained(model_name)

# Export to ONNX
dummy_input = tokenizer("example", return_tensors="pt")
torch.onnx.export(
    model,
    (dummy_input["input_ids"], dummy_input["attention_mask"]),
    f"{output_dir}/model.onnx",
    input_names=["input_ids", "attention_mask"],
    output_names=["sentence_embedding"],
    dynamic_axes={
        "input_ids": {0: "batch", 1: "sequence"},
        "attention_mask": {0: "batch", 1: "sequence"},
        "sentence_embedding": {0: "batch"}
    }
)

# Save tokenizer
tokenizer.save_pretrained(output_dir)
EOF
```

## Usage

### Starting with Vectorization

```bash
# Set level
export OP_DBUS_VECTOR_LEVEL=medium

# Run op-dbus
./target/release/op-dbus daemon --state-file /etc/op-dbus/state.json
```

### Lazy Loading

Models are loaded **on-demand** on the first footprint creation. This means:

- No startup delay
- Memory only used when vectorization is needed
- Automatic fallback to heuristic if model fails to load

### Fallback Behavior

If a model fails to load, the system automatically:

1. Tries to fall back to a lower level model
2. Falls back to fast heuristic vectorization
3. Logs warnings but continues operation

## Architecture

### Vectorization Flow

```
Plugin Operation
    ↓
FootprintGenerator::create_footprint()
    ↓
Check OP_DBUS_VECTOR_LEVEL
    ↓
├─ None → Skip vectorization (empty vector)
├─ Low/Medium/High → Transformer embedding
│   ├─ Prepare text from footprint data
│   ├─ Load model (lazy, once)
│   ├─ Generate embedding
│   └─ L2-normalize for cosine similarity
│
└─ Fallback → Heuristic vectorization
    ↓
Store in PluginFootprint.vector_features
    ↓
Write to blockchain
    ↓
Sync to Qdrant vector DB
```

### Text Preparation

Footprint data is converted to text for embedding:

```
plugin: network
operation: create
interface: eth0
ip: 192.168.1.100
status: active
meta.host: server01
meta.user: admin
```

## Performance

### Benchmarks (CPU)

| Level  | Model         | Speed     | Latency | Memory |
|--------|---------------|-----------|---------|--------|
| None   | -             | Instant   | <1ms    | 0MB    |
| Low    | MiniLM-L3-v2  | 19k/sec   | ~50μs   | 61MB   |
| Medium | MiniLM-L6-v2  | 14k/sec   | ~70μs   | 80MB   |
| High   | MPNet-base-v2 | 2.8k/sec  | ~350μs  | 420MB  |

### Recommendations

- **Development**: `none` (zero overhead)
- **Production (standard)**: `medium` (good balance)
- **Production (high-value)**: `high` (best quality for critical data)
- **High-throughput**: `low` or `none`

## Troubleshooting

### Model Not Found

```
ERROR: Model not found at /var/lib/op-dbus/models/paraphrase-MiniLM-L6-v2
```

**Solution**: Download the model (see Model Setup above)

### ONNX Runtime Error

```
ERROR: Failed to load ONNX model
```

**Solution**: Ensure ONNX Runtime is installed:
```bash
sudo apt-get install libonnxruntime
```

Or rebuild with `download-binaries` feature (already enabled in Cargo.toml).

### Out of Memory

```
ERROR: Cannot allocate memory for model
```

**Solution**: Use a lower vectorization level or disable (`none`)

### Slow Performance

If vectorization is blocking operations:

**Solution**: The current implementation is synchronous. Future enhancement will add async processing (see Roadmap).

## Roadmap

- [ ] Async/background vectorization to avoid blocking plugin ops
- [ ] Automatic model download from Hugging Face
- [ ] Batch processing for multiple footprints
- [ ] GPU acceleration support
- [ ] Custom fine-tuned models for domain-specific embeddings
- [ ] Hybrid mode: combine heuristic (32-dim) + transformer (32-dim)

## References

- [On-Demand Log Vectorization Plan](on_demand_log_vectorization.md)
- [Plugin Footprint Source](src/blockchain/plugin_footprint.rs)
- [Model Manager Source](src/ml/model_manager.rs)
- [ONNX Runtime](https://onnxruntime.ai/)
- [Sentence Transformers](https://www.sbert.net/)
