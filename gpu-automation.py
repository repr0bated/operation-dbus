#!/usr/bin/env python3
"""
GPU Automation Script for Lightning AI and Paperspace
Automatically utilizes free GPU credits for AI model training and inference

Usage:
    python gpu_automation.py --platform lightning --task train --model phi-2
    python gpu_automation.py --platform paperspace --task inference --model llama-2-7b
"""

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Dict, Any, Optional

import requests
from dotenv import load_dotenv

# Load environment variables
load_dotenv()

class GPUAutomation:
    def __init__(self):
        self.lightning_api_key = os.getenv('LIGHTNING_API_KEY')
        self.paperspace_api_key = os.getenv('PAPERSPACE_API_KEY')
        self.hf_token = os.getenv('HF_TOKEN')

    def run_lightning_job(self, task: str, model: str, **kwargs) -> Dict[str, Any]:
        """Run a job on Lightning AI"""
        if not self.lightning_api_key:
            raise ValueError("LIGHTNING_API_KEY not found in environment")

        # Lightning AI API endpoints
        base_url = "https://api.lightning.ai/v1"

        # Create training script
        training_script = self._generate_training_script(task, model, **kwargs)

        # Job configuration
        job_config = {
            "name": f"{task}-{model}-{int(time.time())}",
            "image": "pytorch/pytorch:2.1.0-cuda12.1-cudnn8-runtime",
            "command": ["python", "train.py"],
            "files": {
                "train.py": training_script
            },
            "resources": {
                "gpu": "A100",  # Use free tier GPU
                "cpu": 4,
                "memory": "16GB"
            },
            "env": {
                "HF_TOKEN": self.hf_token,
                "MODEL_NAME": model,
                "TASK": task
            }
        }

        # Submit job
        headers = {"Authorization": f"Bearer {self.lightning_api_key}"}
        response = requests.post(f"{base_url}/jobs", json=job_config, headers=headers)

        if response.status_code == 201:
            job_id = response.json()["id"]
            print(f"✅ Lightning AI job submitted: {job_id}")

            # Monitor job
            return self._monitor_lightning_job(job_id)
        else:
            raise Exception(f"Lightning AI job failed: {response.text}")

    def run_paperspace_job(self, task: str, model: str, **kwargs) -> Dict[str, Any]:
        """Run a job on Paperspace Gradient"""
        if not self.paperspace_api_key:
            raise ValueError("PAPERSPACE_API_KEY not found in environment")

        # Paperspace Gradient API
        base_url = "https://api.paperspace.com/v1"

        # Create notebook/job
        job_config = {
            "name": f"{task}-{model}-{int(time.time())}",
            "machineType": "A4000",  # Free tier GPU
            "container": "paperspace/gradient-base: pytorch",
            "command": self._generate_gradient_command(task, model, **kwargs),
            "env": {
                "HF_TOKEN": self.hf_token,
                "MODEL_NAME": model,
                "TASK": task
            }
        }

        headers = {"Authorization": f"Bearer {self.paperspace_api_key}"}
        response = requests.post(f"{base_url}/jobs", json=job_config, headers=headers)

        if response.status_code == 201:
            job_id = response.json()["id"]
            print(f"✅ Paperspace job submitted: {job_id}")
            return self._monitor_paperspace_job(job_id)
        else:
            raise Exception(f"Paperspace job failed: {response.text}")

    def _generate_training_script(self, task: str, model: str, **kwargs) -> str:
        """Generate Python training script"""
        if task == "train":
            return f'''
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

def main():
    model_name = "{model}"
    hf_token = os.getenv("HF_TOKEN")

    print(f"🚀 Starting training for {{model_name}}")

    # Load model and tokenizer
    model = AutoModelForCausalLM.from_pretrained(
        model_name,
        token=hf_token,
        torch_dtype=torch.float16,
        device_map="auto"
    )
    tokenizer = AutoTokenizer.from_pretrained(model_name, token=hf_token)

    # Load dataset
    dataset = load_dataset("wikitext", "wikitext-2-raw-v1")
    def tokenize_function(examples):
        return tokenizer(examples["text"], truncation=True, padding="max_length", max_length=512)

    tokenized_datasets = dataset.map(tokenize_function, batched=True)

    # Training arguments
    training_args = TrainingArguments(
        output_dir="./results",
        num_train_epochs=1,
        per_device_train_batch_size=4,
        save_steps=500,
        logging_steps=100,
        learning_rate=2e-5,
        weight_decay=0.01,
        fp16=True,
        dataloader_pin_memory=False,
    )

    # Trainer
    trainer = Trainer(
        model=model,
        args=training_args,
        train_dataset=tokenized_datasets["train"],
        data_collator=DataCollatorForLanguageModeling(tokenizer=tokenizer, mlm=False),
    )

    # Train
    trainer.train()

    # Save model
    trainer.save_model("./fine-tuned-model")
    tokenizer.save_pretrained("./fine-tuned-model")

    print("✅ Training complete!")

if __name__ == "__main__":
    main()
'''
        elif task == "inference":
            return f'''
import os
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer, pipeline

def main():
    model_name = "{model}"
    hf_token = os.getenv("HF_TOKEN")

    print(f"🚀 Starting inference for {{model_name}}")

    # Load model
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
        "Write a Python function to calculate fibonacci:",
        "Explain machine learning in simple terms:",
        "What is the capital of France?"
    ]

    results = []
    for prompt in prompts:
        print(f"\\n📝 Prompt: {{prompt}}")
        result = pipe(prompt, max_length=100, num_return_sequences=1)[0]['generated_text']
        print(f"🤖 Response: {{result}}")
        results.append({{"prompt": prompt, "response": result}})

    # Save results
    with open("inference_results.json", "w") as f:
        json.dump(results, f, indent=2)

    print("✅ Inference complete!")

if __name__ == "__main__":
    main()
'''

    def _generate_gradient_command(self, task: str, model: str, **kwargs) -> str:
        """Generate Paperspace Gradient command"""
        if task == "train":
            return f"""
cd /app &&
pip install transformers accelerate datasets &&
python -c "
{kwargs.get('script', self._generate_training_script(task, model))}
"
"""
        else:
            return f"""
cd /app &&
pip install transformers accelerate &&
python -c "
{kwargs.get('script', self._generate_training_script(task, model))}
"
"""

    def _monitor_lightning_job(self, job_id: str) -> Dict[str, Any]:
        """Monitor Lightning AI job status"""
        base_url = "https://api.lightning.ai/v1"
        headers = {"Authorization": f"Bearer {self.lightning_api_key}"}

        while True:
            response = requests.get(f"{base_url}/jobs/{job_id}", headers=headers)
            if response.status_code == 200:
                job_data = response.json()
                status = job_data.get("status")

                print(f"🔄 Job {job_id} status: {status}")

                if status == "completed":
                    # Download results
                    return self._download_lightning_results(job_id)
                elif status in ["failed", "cancelled"]:
                    raise Exception(f"Job {job_id} {status}")
                else:
                    time.sleep(30)  # Check every 30 seconds
            else:
                raise Exception(f"Failed to check job status: {response.text}")

    def _monitor_paperspace_job(self, job_id: str) -> Dict[str, Any]:
        """Monitor Paperspace job status"""
        base_url = "https://api.paperspace.com/v1"
        headers = {"Authorization": f"Bearer {self.paperspace_api_key}"}

        while True:
            response = requests.get(f"{base_url}/jobs/{job_id}", headers=headers)
            if response.status_code == 200:
                job_data = response.json()
                status = job_data.get("state")

                print(f"🔄 Job {job_id} status: {status}")

                if status == "stopped":
                    # Download results
                    return self._download_paperspace_results(job_id)
                elif status == "error":
                    raise Exception(f"Job {job_id} failed")
                else:
                    time.sleep(30)
            else:
                raise Exception(f"Failed to check job status: {response.text}")

    def _download_lightning_results(self, job_id: str) -> Dict[str, Any]:
        """Download results from Lightning AI"""
        # This would implement downloading model artifacts, logs, etc.
        print(f"📥 Downloading results for job {job_id}")
        return {"job_id": job_id, "platform": "lightning", "status": "completed"}

    def _download_paperspace_results(self, job_id: str) -> Dict[str, Any]:
        """Download results from Paperspace"""
        print(f"📥 Downloading results for job {job_id}")
        return {"job_id": job_id, "platform": "paperspace", "status": "completed"}

    def check_credits(self, platform: str) -> Dict[str, Any]:
        """Check available credits on platform"""
        if platform == "lightning":
            if not self.lightning_api_key:
                return {"error": "No Lightning API key"}
            # Check Lightning credits
            headers = {"Authorization": f"Bearer {self.lightning_api_key}"}
            response = requests.get("https://api.lightning.ai/v1/account", headers=headers)
            return response.json() if response.status_code == 200 else {"error": "Failed to get credits"}

        elif platform == "paperspace":
            if not self.paperspace_api_key:
                return {"error": "No Paperspace API key"}
            # Check Paperspace credits
            headers = {"Authorization": f"Bearer {self.paperspace_api_key}"}
            response = requests.get("https://api.paperspace.com/v1/account", headers=headers)
            return response.json() if response.status_code == 200 else {"error": "Failed to get credits"}

        return {"error": "Unknown platform"}


def main():
    parser = argparse.ArgumentParser(description="GPU Automation for AI Training")
    parser.add_argument("--platform", choices=["lightning", "paperspace"],
                       required=True, help="GPU platform to use")
    parser.add_argument("--task", choices=["train", "inference", "finetune"],
                       required=True, help="Task to perform")
    parser.add_argument("--model", required=True,
                       help="Model name (e.g., microsoft/phi-2)")
    parser.add_argument("--check-credits", action="store_true",
                       help="Just check available credits")

    args = parser.parse_args()

    automation = GPUAutomation()

    if args.check_credits:
        credits = automation.check_credits(args.platform)
        print(json.dumps(credits, indent=2))
        return

    try:
        if args.platform == "lightning":
            result = automation.run_lightning_job(args.task, args.model)
        elif args.platform == "paperspace":
            result = automation.run_paperspace_job(args.task, args.model)

        print("✅ Job completed successfully!")
        print(json.dumps(result, indent=2))

    except Exception as e:
        print(f"❌ Error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()