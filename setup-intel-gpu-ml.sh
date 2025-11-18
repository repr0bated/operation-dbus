#!/bin/bash
# Setup script for Intel GPU ML optimization with MKL
# This script helps you utilize your free GPU time on Lightning AI/Paperspace

set -e

echo "🚀 Intel GPU ML Setup for Local AI Models"
echo "=========================================="
echo ""

# Check if running on Intel GPU system
if ! lspci | grep -i intel | grep -i display > /dev/null 2>&1; then
    echo "⚠️  No Intel GPU detected. This script is optimized for Intel Arc GPUs."
    echo "   Continuing anyway for CPU optimization..."
fi

# Install Intel GPU drivers and oneAPI toolkit
echo "📦 Installing Intel GPU Drivers and oneAPI Toolkit..."
echo "   This provides MKL (Math Kernel Library) for optimized matrix operations"
echo ""

# Add Intel repository
wget -qO - https://repositories.intel.com/gpu/intel-graphics.key | sudo apt-key add -
sudo apt-add-repository 'deb [arch=amd64] https://repositories.intel.com/gpu/ubuntu jammy unified'

# Install Intel GPU drivers
sudo apt update
sudo apt install -y \
    intel-opencl-icd \
    intel-level-zero-gpu \
    level-zero \
    intel-gpu-tools

# Install Intel oneAPI toolkit (includes MKL)
wget -O- https://apt.repos.intel.com/intel-gpg-keys/GPG-PUB-KEY-INTEL-SW-PRODUCTS.PUB | gpg --dearmor | sudo tee /usr/share/keyrings/oneapi-archive-keyring.gpg > /dev/null
echo 'deb [signed-by=/usr/share/keyrings/oneapi-archive-keyring.gpg] https://apt.repos.intel.com/oneapi all main' | sudo tee /etc/apt/sources.list.d/oneapi.list

sudo apt update
sudo apt install -y intel-oneapi-mkl intel-oneapi-dnn

# Set up Intel GPU environment
echo "🔧 Configuring Intel GPU environment..."
echo 'export ONEAPI_ROOT=/opt/intel/oneapi' >> ~/.bashrc
echo 'export LD_LIBRARY_PATH=$ONEAPI_ROOT/compiler/latest/linux/lib:$LD_LIBRARY_PATH' >> ~/.bashrc
echo 'export LIBRARY_PATH=$ONEAPI_ROOT/compiler/latest/linux/lib:$LIBRARY_PATH' >> ~/.bashrc
echo 'export CPATH=$ONEAPI_ROOT/compiler/latest/linux/include:$CPATH' >> ~/.bashrc

source ~/.bashrc

# Install Python ML libraries with Intel optimizations
echo "🐍 Installing optimized Python ML libraries..."
pip install \
    torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cpu
pip install \
    intel-extension-for-pytorch \
    oneccl-bind-pt \
    torch-ccl \
    transformers \
    accelerate \
    optimum[intel] \
    intel-openmp

# Test Intel GPU
echo "🧪 Testing Intel GPU setup..."
if command -v clinfo >/dev/null 2>&1; then
    echo "Intel GPU devices found:"
    clinfo | grep -i "device name" | head -3
else
    echo "clinfo not available, but Intel drivers should be working"
fi

echo ""
echo "✅ Intel GPU ML setup complete!"
echo ""
echo "🎯 Next Steps:"
echo "1. Download a model for local use:"
echo "   huggingface-cli download microsoft/phi-2 --local-dir ./models/phi-2"
echo ""
echo "2. Test with a simple script:"
echo "   python -c \"import torch; print('GPU available:', torch.cuda.is_available())\""
echo ""
echo "3. Use your free GPU time on Lightning AI:"
echo "   - Go to lightning.ai"
echo "   - Start a GPU instance"
echo "   - Run: pip install transformers accelerate"
echo "   - Load and run your local models"
echo ""
echo "4. Use your free GPU time on Paperspace:"
echo "   - Go to paperspace.com"
echo "   - Start Gradient instance"
echo "   - Use pre-built ML templates"
echo ""
echo "💡 Recommended models for local GPU use:"
echo "   • microsoft/phi-2 (2.7B params, fast, good quality)"
echo "   • microsoft/DialoGPT-medium (345M params, conversational)"
echo "   • distilbert-base-uncased (smaller BERT model)"
echo ""
echo "🔗 Useful links:"
echo "   • Lightning AI: https://lightning.ai"
echo "   • Paperspace Gradient: https://gradient.paperspace.com"
echo "   • Intel GPU drivers: https://www.intel.com/content/www/us/en/download/726609/intel-arc-graphics-driver.html"