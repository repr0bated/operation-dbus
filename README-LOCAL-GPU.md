# 🚀 Local GPU AI Setup Guide
## Utilizing Free GPU Time on Lightning AI & Paperspace

This guide helps you set up local AI models using your free GPU credits and optimize your Intel GPU with MKL.

## 🎯 **Which Model Should You Use Locally?**

### **TOP RECOMMENDATION: `microsoft/phi-2`**
- **Size**: 2.7B parameters (fits in most GPU memory)
- **Quality**: Excellent code and reasoning capabilities
- **Speed**: Fast inference, good for real-time chat
- **Requirements**: 4GB+ GPU RAM

### **Other Great Options:**
1. **`microsoft/DialoGPT-medium`** - Conversational chatbot (345M params)
2. **`distilgpt2`** - Fast and lightweight (82M params)
3. **`microsoft/DialoGPT-small`** - Quick responses (117M params)

## 🖥️ **Setting Up Your Intel GPU with MKL**

### **Step 1: Run the Setup Script**
```bash
cd /git/operation-dbus
sudo ./setup-intel-gpu-ml.sh
```

This script will:
- ✅ Install Intel GPU drivers
- ✅ Set up Intel oneAPI toolkit (includes MKL)
- ✅ Install optimized Python libraries
- ✅ Configure environment variables

### **Step 2: Download a Model**
```bash
# Install Hugging Face CLI
pip install huggingface_hub

# Download Phi-2 (recommended)
huggingface-cli download microsoft/phi-2 --local-dir ./models/phi-2

# Or download DialoGPT for conversational use
huggingface-cli download microsoft/DialoGPT-medium --local-dir ./models/dialogpt
```

### **Step 3: Test Your Setup**
```bash
python -c "
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer

print('GPU available:', torch.cuda.is_available())
print('GPU device count:', torch.cuda.device_count())

# Load a small test model
model_name = 'distilgpt2'  # Very fast to test
tokenizer = AutoTokenizer.from_pretrained(model_name)
model = AutoModelForCausalLM.from_pretrained(model_name)

# Test inference
input_text = 'Hello, I am'
inputs = tokenizer(input_text, return_tensors='pt')
outputs = model.generate(**inputs, max_length=20, num_return_sequences=1)
response = tokenizer.decode(outputs[0], skip_special_tokens=True)
print('Test response:', response)
"
```

## ⚡ **Utilizing Your Free GPU Time**

### **Option 1: Lightning AI**
```bash
# 1. Go to https://lightning.ai
# 2. Sign up/Login with your free credits
# 3. Start a GPU instance (A100, V100, etc.)
# 4. Clone your repo:
git clone https://github.com/yourusername/operation-dbus.git
cd operation-dbus

# 5. Install dependencies
pip install transformers accelerate torch torchvision torchaudio

# 6. Run model training/inference
python -c "
from transformers import pipeline
import torch

# Use GPU if available
device = 0 if torch.cuda.is_available() else -1

# Load a model
generator = pipeline('text-generation', model='microsoft/phi-2', device=device)

# Generate text
result = generator('Write a Python function to calculate factorial:', max_length=200)
print(result[0]['generated_text'])
"
```

### **Option 2: Paperspace Gradient**
```bash
# 1. Go to https://gradient.paperspace.com
# 2. Sign up/Login with free credits
# 3. Create a new project
# 4. Use their ML templates or start from scratch

# 5. In your notebook/environment:
!pip install transformers accelerate

from transformers import AutoModelForCausalLM, AutoTokenizer
import torch

# Load model on GPU
model_name = 'microsoft/phi-2'
tokenizer = AutoTokenizer.from_pretrained(model_name)
model = AutoModelForCausalLM.from_pretrained(model_name, torch_dtype=torch.float16, device_map='auto')

# Generate
prompt = 'Explain quantum computing in simple terms:'
inputs = tokenizer(prompt, return_tensors='pt').to('cuda')
outputs = model.generate(**inputs, max_length=300, temperature=0.7)
response = tokenizer.decode(outputs[0], skip_special_tokens=True)
print(response)
```

## 🔧 **Integration with Your Chat System**

### **Using Local Models in Your Chat Interface**

1. **Add Local Model Support to Ollama:**
```bash
# Create a custom model file
cat > phi2-modelfile << EOF
FROM microsoft/phi-2
PARAMETER temperature 0.7
PARAMETER top_p 0.9
PARAMETER num_ctx 2048
SYSTEM "You are a helpful AI assistant."
EOF

# Create the model in Ollama
ollama create phi2-local -f phi2-modelfile
```

2. **Use the Model in Your Chat:**
```bash
# Switch to local Ollama model
cd /git/operation-dbus
./switch-model.sh ollama select phi2-local
```

### **GPU Memory Optimization**

For your Intel GPU with MKL optimization:

```bash
# Set environment variables for Intel GPU
export ONEAPI_DEVICE_SELECTOR=level_zero:0  # Use Intel GPU
export ZE_AFFINITY_MASK=0  # GPU 0
export TORCH_USE_XPU=1  # Enable Intel GPU in PyTorch

# Run with memory optimization
python -c "
import torch
import intel_extension_for_pytorch as ipex

# Enable optimizations
model = model.to('xpu')  # Move to Intel GPU
model = ipex.optimize(model, dtype=torch.float16)  # Optimize for Intel GPU

# Use torch.compile for additional speed
model = torch.compile(model)
"
```

## 📊 **Performance Comparison**

| Model | Size | Quality | Speed | Memory Usage |
|-------|------|---------|-------|--------------|
| Phi-2 | 2.7B | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ~6GB |
| DialoGPT-medium | 345M | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ~1GB |
| DistilGPT-2 | 82M | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ~300MB |

## 🚨 **Troubleshooting**

### **Intel GPU Issues:**
```bash
# Check GPU status
intel-gpu-top

# Test OpenCL
clinfo | grep -A5 "Platform"

# Verify MKL installation
python -c "import mkl; print('MKL version:', mkl.get_version_string())"
```

### **Model Loading Issues:**
```bash
# Clear cache
rm -rf ~/.cache/huggingface

# Use smaller model for testing
python -c "
from transformers import pipeline
pipe = pipeline('text-generation', model='distilgpt2', device=0)
result = pipe('Hello world', max_length=10)
print(result)
"
```

## 🎯 **Quick Start Commands**

```bash
# 1. Setup Intel GPU
sudo ./setup-intel-gpu-ml.sh

# 2. Download model
huggingface-cli download microsoft/phi-2 --local-dir ./models/phi-2

# 3. Test locally
python -c "
from transformers import pipeline
pipe = pipeline('text-generation', model='./models/phi-2', device='cpu')
print(pipe('Hello, how are you?'))
"

# 4. Use free GPU time on Lightning AI
# - Start GPU instance
# - Run: pip install transformers accelerate
# - Load and run your models
```

## 🤖 **Automated GPU Workflows**

### **One-Command GPU Jobs**
```bash
# Setup API keys first
cp .env.example .env
# Edit .env with your LIGHTNING_API_KEY, PAPERSPACE_API_KEY, HF_TOKEN

# Run inference on Lightning AI GPUs
./gpu-workflow.sh inference lightning microsoft/phi-2

# Fine-tune models on Paperspace GPUs
./gpu-workflow.sh train paperspace meta-llama/Llama-2-7b-chat-hf

# Use different models automatically
./gpu-workflow.sh inference paperspace google/gemma-7b-it
```

### **What Happens Automatically:**
1. ✅ **Credit Check** - Verifies you have free GPU time
2. ✅ **GPU Provisioning** - Spins up instances automatically
3. ✅ **Job Execution** - Runs your ML tasks
4. ✅ **Result Download** - Saves outputs to `./gpu_results/`
5. ✅ **Cleanup** - Shuts down instances to save credits

### **Workflow Types:**
- **`inference`** - Text generation testing
- **`train`** - Model fine-tuning on sample data
- **`finetune`** - Advanced training workflows

### **Supported Platforms:**
- **`lightning`** - Lightning AI (free A100/V100 GPUs)
- **`paperspace`** - Paperspace Gradient (free A4000 GPUs)

## 🚀 **Quick Start Commands**

```bash
# 1. Setup Intel GPU
sudo ./setup-intel-gpu-ml.sh

# 2. Setup API keys
cp .env.example .env
# Edit .env with your API keys

# 3. Download local model
huggingface-cli download microsoft/phi-2 --local-dir ./models/phi-2

# 4. Test locally
python -c "from transformers import pipeline; p = pipeline('text-generation', model='./models/phi-2'); print(p('Hello')[0]['generated_text'])"

# 5. AUTOMATED GPU WORKFLOW
./gpu-workflow.sh inference lightning microsoft/phi-2
./gpu-workflow.sh train paperspace microsoft/phi-2
```

## 💡 **Pro Tips**

1. **Start Small**: Test with `distilgpt2` first (82M params)
2. **Monitor Memory**: Use `nvidia-smi` or `intel-gpu-top` to watch GPU usage
3. **Use Mixed Precision**: `torch_dtype=torch.float16` saves memory
4. **Cache Models**: Keep downloaded models for reuse
5. **Batch Processing**: Process multiple requests together for efficiency
6. **Automation**: Use the workflow scripts to maximize your free GPU time

Your free GPU time on Lightning AI and Paperspace is perfect for training custom models or running inference on larger models that won't fit on your local Intel GPU!

**The automation scripts make it completely hands-off!** 🎉