# GPU Acceleration for Vectorization

This guide explains how to enable GPU acceleration for transformer-based vectorization in op-dbus.

## Quick Start

### Enable GPU Acceleration

```bash
# Use CUDA (NVIDIA GPU)
export OP_DBUS_EXECUTION_PROVIDER=cuda
export OP_DBUS_VECTOR_LEVEL=medium

# Or use TensorRT (NVIDIA GPU, optimized)
export OP_DBUS_EXECUTION_PROVIDER=tensorrt
export OP_DBUS_VECTOR_LEVEL=high

# Specify GPU device (for multi-GPU systems)
export OP_DBUS_GPU_DEVICE=0  # Use first GPU

# Run op-dbus
./op-dbus daemon --state-file /etc/op-dbus/state.json
```

## Supported Execution Providers

| Provider   | Hardware             | Platform | Performance | Notes                      |
|------------|----------------------|----------|-------------|----------------------------|
| **cpu**    | Any CPU              | All      | Baseline    | Default, always available  |
| **cuda**   | NVIDIA GPU           | Linux/Win| 10-50x      | Requires CUDA 11.x+        |
| **tensorrt**| NVIDIA GPU          | Linux/Win| 20-100x     | Optimized CUDA, best perf  |
| **directml**| Any GPU             | Windows  | 5-20x       | DirectX 12 GPUs            |
| **coreml** | Apple Neural Engine  | macOS    | 10-30x      | M1/M2/M3 Macs              |

## Environment Variables

### `OP_DBUS_EXECUTION_PROVIDER`

Set the execution provider (CPU or GPU type):

```bash
# CPU (default)
export OP_DBUS_EXECUTION_PROVIDER=cpu

# NVIDIA CUDA
export OP_DBUS_EXECUTION_PROVIDER=cuda
# or shorthand:
export OP_DBUS_EXECUTION_PROVIDER=gpu

# NVIDIA TensorRT (optimized)
export OP_DBUS_EXECUTION_PROVIDER=tensorrt
# or shorthand:
export OP_DBUS_EXECUTION_PROVIDER=trt

# DirectML (Windows)
export OP_DBUS_EXECUTION_PROVIDER=directml
# or shorthand:
export OP_DBUS_EXECUTION_PROVIDER=dml

# CoreML (macOS)
export OP_DBUS_EXECUTION_PROVIDER=coreml
```

### `OP_DBUS_GPU_DEVICE`

For multi-GPU systems, specify which GPU to use:

```bash
# Use first GPU (default)
export OP_DBUS_GPU_DEVICE=0

# Use second GPU
export OP_DBUS_GPU_DEVICE=1
```

### `OP_DBUS_VECTOR_LEVEL`

Combine with vectorization level:

```bash
export OP_DBUS_VECTOR_LEVEL=high
export OP_DBUS_EXECUTION_PROVIDER=tensorrt
```

## Setup Instructions

### CUDA (NVIDIA GPU)

#### Prerequisites

- NVIDIA GPU with CUDA Compute Capability 6.0+ (Pascal or newer)
- CUDA Toolkit 11.x or 12.x
- cuDNN 8.x

#### Installation

```bash
# Ubuntu/Debian
sudo apt-get install nvidia-cuda-toolkit
sudo apt-get install libcudnn8

# Verify CUDA
nvidia-smi
nvcc --version
```

#### Configuration

```bash
export OP_DBUS_EXECUTION_PROVIDER=cuda
export OP_DBUS_GPU_DEVICE=0
```

### TensorRT (NVIDIA GPU, Optimized)

#### Prerequisites

- CUDA (see above)
- TensorRT 8.x

#### Installation

```bash
# Download from NVIDIA: https://developer.nvidia.com/tensorrt
# Or use pip:
pip install tensorrt

# Verify
dpkg -l | grep TensorRT
```

#### Configuration

```bash
export OP_DBUS_EXECUTION_PROVIDER=tensorrt
export OP_DBUS_GPU_DEVICE=0
```

**Note**: TensorRT may take longer to load initially (model optimization), but inference is fastest.

### DirectML (Windows GPU)

#### Prerequisites

- Windows 10/11
- DirectX 12 compatible GPU (NVIDIA, AMD, or Intel)
- DirectML runtime (usually pre-installed)

#### Configuration

```powershell
$env:OP_DBUS_EXECUTION_PROVIDER="directml"
$env:OP_DBUS_GPU_DEVICE=0
```

### CoreML (Apple Silicon)

#### Prerequisites

- macOS with M1, M2, or M3 chip
- macOS 12.0 (Monterey) or later

#### Configuration

```bash
export OP_DBUS_EXECUTION_PROVIDER=coreml
```

**Note**: CoreML automatically uses the Neural Engine when available.

## Performance Comparison

### Throughput (sentences/second)

| Level  | CPU (16-core) | CUDA (RTX 3090) | TensorRT (RTX 3090) | M1 Pro (CoreML) |
|--------|---------------|-----------------|---------------------|-----------------|
| Low    | 19,000        | 280,000         | 450,000             | 150,000         |
| Medium | 14,000        | 180,000         | 320,000             | 95,000          |
| High   | 2,800         | 45,000          | 85,000              | 22,000          |

### Latency (per embedding)

| Level  | CPU    | CUDA   | TensorRT | CoreML |
|--------|--------|--------|----------|--------|
| Low    | 50μs   | 3.5μs  | 2.2μs    | 6.7μs  |
| Medium | 70μs   | 5.5μs  | 3.1μs    | 10.5μs |
| High   | 350μs  | 22μs   | 11.7μs   | 45μs   |

*Benchmarks approximate, vary by hardware*

## Troubleshooting

### CUDA Not Found

```
ERROR: CUDA execution provider requested but not available
```

**Solutions:**
1. Verify CUDA installation: `nvidia-smi`
2. Check ONNX Runtime CUDA support: `ldd` on onnxruntime library
3. Rebuild with CUDA support or fall back to CPU

### Out of GPU Memory

```
ERROR: CUDA out of memory
```

**Solutions:**
1. Use a lower vectorization level (`low` instead of `high`)
2. Reduce batch size (not yet configurable, coming soon)
3. Use CPU execution: `OP_DBUS_EXECUTION_PROVIDER=cpu`

### Slow First Inference

TensorRT optimizes models on first load, which can take 30-60 seconds. Subsequent inferences are fast.

### Model Format Issues

Some providers require specific ONNX opset versions:
- **CUDA**: ONNX opset 11+
- **TensorRT**: ONNX opset 11-17
- **DirectML**: ONNX opset 7-15
- **CoreML**: ONNX opset 11-13

If you encounter errors, re-export the model with compatible opset.

## Building with GPU Support

### CUDA Build

```bash
# Install CUDA development libraries
sudo apt-get install nvidia-cuda-dev

# Build with ML feature (includes GPU support)
cargo build --release --features ml
```

### Verify GPU Support

```bash
# Check ONNX Runtime capabilities
ldd ./target/release/op-dbus | grep onnx

# Should show libonnxruntime with CUDA symbols
```

## Monitoring GPU Usage

### NVIDIA GPUs

```bash
# Watch GPU utilization
watch -n 1 nvidia-smi

# Monitor memory usage
nvidia-smi dmon -s mu
```

### AMD GPUs (Linux)

```bash
# Install radeontop
sudo apt-get install radeontop

# Monitor
radeontop
```

### Apple Silicon

```bash
# Use Activity Monitor
open -a "Activity Monitor"
# View GPU History tab
```

## Best Practices

### For Development
- Use `cpu` execution (no setup required)
- Or use `low` level with GPU for fast iteration

### For Production (Standard)
- Use `cuda` with `medium` level
- Balanced performance and quality

### For Production (High-Value)
- Use `tensorrt` with `high` level
- Maximum quality, acceptable latency (<50μs)

### For Production (High-Throughput)
- Use `tensorrt` with `low` level
- Maximum speed, good quality

### For Edge/Embedded
- Use `cpu` with `low` level
- Or `directml`/`coreml` if available

## Future Enhancements

- [ ] Multi-GPU support (data parallelism)
- [ ] Dynamic batch sizing for GPU
- [ ] Mixed precision (FP16/INT8) for faster inference
- [ ] Model quantization support
- [ ] Automatic provider fallback chain
- [ ] GPU memory pool configuration

## References

- [ONNX Runtime Execution Providers](https://onnxruntime.ai/docs/execution-providers/)
- [CUDA Toolkit](https://developer.nvidia.com/cuda-toolkit)
- [TensorRT](https://developer.nvidia.com/tensorrt)
- [DirectML](https://docs.microsoft.com/en-us/windows/ai/directml/)
- [CoreML](https://developer.apple.com/documentation/coreml)
