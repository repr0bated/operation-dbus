#!/usr/bin/env python3
"""
Batch embedding job for Paperspace free GPU notebook.

Usage:
  1. Upload openclaw_chunks.jsonl to your Paperspace notebook
  2. Run this script on the GPU notebook
  3. Download the output .npy file
  4. Run: openclaw-index import-embeddings --chunks openclaw_chunks.jsonl --embeddings openclaw_embeddings.npy

This script runs on the Paperspace GPU and generates embeddings much faster
than the HuggingFace Inference API.
"""

import sys
import time

import numpy as np
import orjson


def main():
    input_path = sys.argv[1] if len(sys.argv) > 1 else "openclaw_chunks.jsonl"
    output_path = sys.argv[2] if len(sys.argv) > 2 else "openclaw_embeddings.npy"
    model_name = sys.argv[3] if len(sys.argv) > 3 else "BAAI/bge-base-en-v1.5"

    print(f"Loading model: {model_name}")
    from sentence_transformers import SentenceTransformer
    model = SentenceTransformer(model_name)

    # Check GPU
    import torch
    device = "cuda" if torch.cuda.is_available() else "cpu"
    print(f"Device: {device}")
    if device == "cuda":
        print(f"GPU: {torch.cuda.get_device_name(0)}")

    # Load chunks
    texts = []
    with open(input_path, "rb") as f:
        for line in f:
            record = orjson.loads(line)
            texts.append(record["text"])

    print(f"Loaded {len(texts)} chunks")

    # Embed
    t0 = time.time()
    embeddings = model.encode(
        texts,
        batch_size=64,
        show_progress_bar=True,
        normalize_embeddings=True,
        device=device,
    )
    elapsed = time.time() - t0
    print(f"Embedded {len(texts)} chunks in {elapsed:.1f}s ({len(texts)/elapsed:.0f} chunks/s)")

    # Save
    np.save(output_path, embeddings)
    print(f"Saved embeddings to {output_path} (shape: {embeddings.shape})")


if __name__ == "__main__":
    main()
