#!/bin/bash
# Automated GPU Job Runner for Lightning AI and Paperspace
# Uses their CLI tools and APIs to automate GPU workflows

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM="${1:-lightning}"
TASK="${2:-inference}"
MODEL="${3:-microsoft/phi-2}"

# Load environment
if [ -f "$SCRIPT_DIR/.env" ]; then
    source "$SCRIPT_DIR/.env"
fi

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    if [ "$PLATFORM" = "lightning" ]; then
        if ! command -v lightning >/dev/null 2>&1; then
            log_warning "Lightning CLI not found. Installing..."
            pip install lightning-sdk
        fi
        if [ -z "$LIGHTNING_API_KEY" ]; then
            log_error "LIGHTNING_API_KEY not set"
            exit 1
        fi
    elif [ "$PLATFORM" = "paperspace" ]; then
        if ! command -v gradient >/dev/null 2>&1; then
            log_warning "Paperspace CLI not found. Installing..."
            pip install paperspace
        fi
        if [ -z "$PAPERSPACE_API_KEY" ]; then
            log_error "PAPERSPACE_API_KEY not set"
            exit 1
        fi
    fi

    if [ -z "$HF_TOKEN" ]; then
        log_error "HF_TOKEN not set"
        exit 1
    fi

    log_success "Prerequisites OK"
}

# Generate job script
generate_job_script() {
    local script_path="$SCRIPT_DIR/gpu_job_${TASK}.py"

    if [ "$TASK" = "inference" ]; then
        cat > "$script_path" << 'EOF'
import os
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer, pipeline
import json
from datetime import datetime

def main():
    model_name = os.getenv("MODEL_NAME", "microsoft/phi-2")
    hf_token = os.getenv("HF_TOKEN")

    print(f"🚀 Running GPU inference for {model_name}")

    # Load model with GPU acceleration
    tokenizer = AutoTokenizer.from_pretrained(model_name, token=hf_token)
    model = AutoModelForCausalLM.from_pretrained(
        model_name,
        token=hf_token,
        torch_dtype=torch.float16,
        device_map="auto"
    )

    # Create pipeline
    pipe = pipeline(
        "text-generation",
        model=model,
        tokenizer=tokenizer,
        torch_dtype=torch.float16,
        device_map="auto"
    )

    # Test prompts
    prompts = [
        "Write a Python function to calculate factorial:",
        "Explain quantum computing simply:",
        "What is machine learning?",
        "Write a haiku about AI:",
        "Debug this Python code: print('hello')",
    ]

    results = []
    for i, prompt in enumerate(prompts, 1):
        print(f"\n📝 [{i}/{len(prompts)}] {prompt}")
        try:
            start_time = datetime.now()
            result = pipe(
                prompt,
                max_length=min(len(prompt) + 100, 200),
                num_return_sequences=1,
                temperature=0.7,
                do_sample=True
            )[0]['generated_text']

            duration = (datetime.now() - start_time).total_seconds()

            print(f"🤖 Response ({duration:.1f}s): {result[len(prompt):].strip()[:100]}...")

            results.append({
                "prompt": prompt,
                "response": result,
                "duration": duration,
                "model": model_name
            })

        except Exception as e:
            print(f"❌ Error: {e}")
            results.append({
                "prompt": prompt,
                "error": str(e),
                "model": model_name
            })

    # Save results
    output_file = f"gpu_results_{model_name.replace('/', '_')}_{int(datetime.now().timestamp())}.json"
    with open(output_file, "w") as f:
        json.dump({
            "model": model_name,
            "platform": os.getenv("PLATFORM", "unknown"),
            "timestamp": datetime.now().isoformat(),
            "results": results
        }, f, indent=2)

    print(f"\n✅ Results saved to {output_file}")
    print(f"📊 Completed {len([r for r in results if 'error' not in r])}/{len(prompts)} prompts successfully")

if __name__ == "__main__":
    main()
EOF

    elif [ "$TASK" = "train" ]; then
        cat > "$script_path" << 'EOF'
import os
import torch
from transformers import (
    AutoModelForCausalLM,
    AutoTokenizer,
    TrainingArguments,
    Trainer,
    DataCollatorForLanguageModeling
)
from datasets import load_dataset
from datetime import datetime

def main():
    model_name = os.getenv("MODEL_NAME", "microsoft/phi-2")
    hf_token = os.getenv("HF_TOKEN")

    print(f"🚀 Starting GPU training for {model_name}")

    # Load model and tokenizer
    model = AutoModelForCausalLM.from_pretrained(
        model_name,
        token=hf_token,
        torch_dtype=torch.float16,
        device_map="auto"
    )
    tokenizer = AutoTokenizer.from_pretrained(model_name, token=hf_token)

    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    # Load small dataset for demo
    dataset = load_dataset("wikitext", "wikitext-2-raw-v1", split="train[:1%]")

    def tokenize_function(examples):
        return tokenizer(
            examples["text"],
            truncation=True,
            padding="max_length",
            max_length=256
        )

    tokenized_dataset = dataset.map(tokenize_function, batched=True)
    tokenized_dataset = tokenized_dataset.remove_columns(["text"])

    # Training arguments
    training_args = TrainingArguments(
        output_dir="./gpu_training_results",
        num_train_epochs=1,
        per_device_train_batch_size=2,
        gradient_accumulation_steps=4,
        save_steps=100,
        logging_steps=10,
        learning_rate=2e-5,
        fp16=True,
        dataloader_pin_memory=False,
        report_to=[],  # Disable wandb etc.
    )

    # Trainer
    trainer = Trainer(
        model=model,
        args=training_args,
        train_dataset=tokenized_dataset,
        data_collator=DataCollatorForLanguageModeling(tokenizer=tokenizer, mlm=False),
    )

    start_time = datetime.now()
    trainer.train()
    training_duration = (datetime.now() - start_time).total_seconds()

    # Save model
    output_dir = f"./gpu_trained_{model_name.replace('/', '_')}_{int(datetime.now().timestamp())}"
    trainer.save_model(output_dir)
    tokenizer.save_pretrained(output_dir)

    # Save training stats
    stats = {
        "model": model_name,
        "platform": os.getenv("PLATFORM", "unknown"),
        "training_duration": training_duration,
        "samples_processed": len(tokenized_dataset),
        "output_dir": output_dir,
        "timestamp": datetime.now().isoformat()
    }

    with open(f"{output_dir}/training_stats.json", "w") as f:
        json.dump(stats, f, indent=2)

    print(f"\n✅ Training complete in {training_duration:.1f} seconds!")
    print(f"📁 Model saved to {output_dir}")

if __name__ == "__main__":
    main()
EOF
    fi

    echo "$script_path"
}

# Run Lightning AI job
run_lightning_job() {
    local script_path="$1"

    log_info "Starting Lightning AI job..."

    # Export environment
    export LIGHTNING_API_KEY="$LIGHTNING_API_KEY"
    export HF_TOKEN="$HF_TOKEN"
    export MODEL_NAME="$MODEL"
    export PLATFORM="lightning"

    # Use Lightning CLI
    lightning run app "$script_path" \
        --cloud \
        --name "gpu-job-$TASK-$MODEL-$(date +%s)" \
        --env "MODEL_NAME=$MODEL" \
        --env "HF_TOKEN=$HF_TOKEN" \
        --env "PLATFORM=lightning"
}

# Run Paperspace job
run_paperspace_job() {
    local script_path="$1"

    log_info "Starting Paperspace Gradient job..."

    # Export environment
    export PAPERSPACE_API_KEY="$PAPERSPACE_API_KEY"
    export HF_TOKEN="$HF_TOKEN"
    export MODEL_NAME="$MODEL"
    export PLATFORM="paperspace"

    # Use Paperspace CLI
    gradient jobs create \
        --name "gpu-job-$TASK-$MODEL-$(date +%s)" \
        --machineType "A4000" \
        --container "paperspace/gradient-base:pytorch" \
        --command "cd /app && pip install transformers accelerate datasets && python gpu_job.py" \
        --env "MODEL_NAME=$MODEL" \
        --env "HF_TOKEN=$HF_TOKEN" \
        --env "PLATFORM=paperspace" \
        --workspace "$script_path"
}

# Main execution
main() {
    log_info "GPU Job Automation - $PLATFORM"
    echo "Task: $TASK"
    echo "Model: $MODEL"
    echo

    check_prerequisites

    # Generate job script
    log_info "Generating job script..."
    script_path=$(generate_job_script)
    log_success "Job script created: $script_path"

    # Copy script to job file
    cp "$script_path" "$SCRIPT_DIR/gpu_job.py"

    # Run job based on platform
    case "$PLATFORM" in
        lightning)
            run_lightning_job "$script_path"
            ;;
        paperspace)
            run_paperspace_job "$script_path"
            ;;
        *)
            log_error "Unsupported platform: $PLATFORM"
            exit 1
            ;;
    esac

    log_success "GPU job submitted! Check your $PLATFORM dashboard for progress."
}

# Show usage if no arguments
if [ $# -eq 0 ]; then
    echo "GPU Job Automation Script"
    echo "Usage: $0 <platform> <task> [model]"
    echo ""
    echo "Platforms: lightning, paperspace"
    echo "Tasks: inference, train"
    echo "Models: microsoft/phi-2, microsoft/DialoGPT-medium, etc."
    echo ""
    echo "Examples:"
    echo "  $0 lightning inference microsoft/phi-2"
    echo "  $0 paperspace train meta-llama/Llama-2-7b-chat-hf"
    echo "  $0 lightning inference  # Uses default model"
    echo ""
    echo "Environment setup:"
    echo "  cp .env.example .env"
    echo "  # Edit .env with your API keys"
    exit 0
fi

main "$@"