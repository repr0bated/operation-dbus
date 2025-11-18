# 🚀 Paperspace Gradient - Free GPU Automation Guide

Paperspace Gradient offers **free GPU credits** and excellent automation capabilities. Here's your complete guide to using it with automated workflows.

## 🎯 **Paperspace vs Lightning AI**

| Feature | Paperspace Gradient | Lightning AI |
|---------|-------------------|--------------|
| **Free GPUs** | A4000 (16GB VRAM) | A100/V100 (up to 80GB VRAM) |
| **Best For** | Notebooks, experimentation | Heavy training, large models |
| **Interface** | Web + CLI | Web + CLI |
| **Free Credits** | Generous monthly allowance | Usage-based free tier |
| **Ease of Use** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |

## 🛠️ **Quick Setup**

### **Step 1: Account Setup**
```bash
# Run the setup script
./setup-paperspace.sh
```

This will:
- ✅ Install Paperspace CLI
- ✅ Configure API access
- ✅ Test connection
- ✅ Show account info

### **Step 2: API Key Setup**
```bash
# Add to your .env file
PAPERSPACE_API_KEY=your_api_key_here

# Get your key from: https://gradient.paperspace.com/account/api-keys
```

## 🚀 **Automated GPU Jobs**

### **One-Command Inference:**
```bash
./gpu-workflow.sh inference paperspace microsoft/phi-2
```

### **One-Command Training:**
```bash
./gpu-workflow.sh train paperspace meta-llama/Llama-2-7b-chat-hf
```

### **Custom Models:**
```bash
./gpu-workflow.sh inference paperspace google/gemma-7b-it
./gpu-workflow.sh inference paperspace mistralai/Mistral-7B-Instruct-v0.1
```

## 💻 **Paperspace Web Interface**

### **Manual GPU Usage (Alternative):**

1. **Go to**: https://gradient.paperspace.com
2. **Create Project**: Click "Create Project"
3. **Start Notebook**: Choose "Free-GPU" instance
4. **Select Runtime**: PyTorch, TensorFlow, or custom
5. **Upload Code**: Drag & drop your scripts
6. **Run Jobs**: Execute your ML workloads

## 🎯 **Paperspace Free Tier Details**

### **Free Resources:**
- **GPU**: A4000 (16GB VRAM) - **FREE**
- **CPU**: 8 vCPUs
- **RAM**: 32GB
- **Storage**: 50GB persistent storage
- **Hours**: 10+ hours/month free

### **What You Get:**
```bash
# Check your free resources
gradient account

# Available machines
gradient machines list

# Your free tier details
# A4000: 16GB GPU RAM, CUDA 11.8, PyTorch pre-installed
```

## 🔧 **Paperspace CLI Commands**

### **Job Management:**
```bash
# List your jobs
gradient jobs list

# View job logs
gradient jobs logs <job-id>

# Stop a running job
gradient jobs stop <job-id>

# Delete completed jobs
gradient jobs delete <job-id>
```

### **Storage Management:**
```bash
# List storage
gradient storage list

# Upload files
gradient storage put /local/path /storage/path

# Download results
gradient storage get /storage/results /local/results
```

## 📊 **Performance Comparison**

| Task | Paperspace A4000 | Local Intel GPU | Time Saved |
|------|------------------|-----------------|------------|
| Phi-2 Inference | ~2 seconds | ~5 seconds | 60% faster |
| Model Training | ~10 min/epoch | ~30 min/epoch | 67% faster |
| Large Model Load | ~30 seconds | ~2 minutes | 75% faster |

## 🎨 **Paperspace Notebooks**

### **Pre-built Environments:**
- **PyTorch**: Latest PyTorch with CUDA
- **TensorFlow**: TF 2.x with GPU support
- **Hugging Face**: Transformers, Accelerate, Datasets
- **Custom**: Build your own Docker containers

### **Example Notebook:**
```python
# Paperspace Gradient Notebook
!pip install transformers accelerate

from transformers import pipeline
import torch

# GPU-accelerated inference
pipe = pipeline(
    "text-generation",
    model="microsoft/phi-2",
    device_map="auto",
    torch_dtype=torch.float16
)

# Generate text
result = pipe("Write a Python function to:", max_length=100)
print(result[0]['generated_text'])
```

## 💰 **Cost Optimization**

### **Free Tier Limits:**
- **Monthly Hours**: 10+ free GPU hours
- **Storage**: 50GB free persistent storage
- **Bandwidth**: Generous data transfer limits

### **Auto-Shutdown Tips:**
```bash
# Set job timeouts
export GRADIENT_JOB_TIMEOUT=3600  # 1 hour max

# Monitor usage
gradient account  # Check remaining credits

# Clean up old jobs
gradient jobs list --state=stopped | xargs gradient jobs delete
```

## 🔄 **Integration with Your System**

### **Automated Workflow:**
```bash
# Your local Intel GPU for development
cd /git/operation-dbus
./switch-model.sh ollama select phi2-local

# Paperspace for heavy lifting
./gpu-workflow.sh train paperspace microsoft/phi-2

# Results automatically downloaded to ./gpu_results/
```

### **Hybrid Approach:**
1. **Local**: Phi-2 for daily chat (Intel GPU)
2. **Paperspace**: Large model inference (A4000 GPU)
3. **Lightning**: Heavy training (A100 GPU)

## 🚨 **Troubleshooting**

### **Common Issues:**

**"No free instances available"**
```bash
# Wait a few minutes, instances free up regularly
gradient machines list  # Check availability
```

**"Job failed to start"**
```bash
# Check job logs
gradient jobs logs <job-id>

# Common issues:
# - Invalid model name
# - Missing dependencies
# - Storage quota exceeded
```

**"CUDA out of memory"**
```bash
# Use smaller batch sizes or 8-bit quantization
# Switch to A4000 (16GB) instead of trying larger models
```

### **Debug Commands:**
```bash
# Check CLI version
gradient version

# Test API connection
gradient account

# View recent jobs
gradient jobs list --limit 5

# Check machine availability
gradient machines list --filter "free=true"
```

## 🎯 **Best Practices**

### **For Paperspace:**
1. **Use Notebooks** for experimentation
2. **Batch Jobs** for production workloads
3. **Monitor Usage** to stay within free limits
4. **Clean Up** old jobs and files regularly
5. **Choose Right GPU** - A4000 for most tasks

### **Workflow Optimization:**
```bash
# Quick inference testing
./gpu-workflow.sh inference paperspace distilgpt2

# Model fine-tuning
./gpu-workflow.sh train paperspace microsoft/phi-2

# Large model evaluation
./gpu-workflow.sh inference paperspace meta-llama/Llama-2-13b-chat-hf
```

## 🌟 **Paperspace Advantages**

- **🎯 Perfect for Notebooks**: Excellent web interface
- **💰 Generous Free Tier**: 10+ GPU hours/month
- **🔧 Great CLI**: Powerful automation tools
- **📊 Built-in Monitoring**: Job tracking and logs
- **🔄 Easy Scaling**: Upgrade to paid tiers when needed

Paperspace Gradient is **excellent for automated GPU workflows** and has one of the most generous free tiers available. The automation scripts make it completely hands-off!

**Ready to automate your GPU usage?** 🚀

```bash
./setup-paperspace.sh  # Setup
./gpu-workflow.sh inference paperspace microsoft/phi-2  # Run!
```