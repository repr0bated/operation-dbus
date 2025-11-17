#!/bin/bash
# AI Chat Script
# Usage: ./chat.sh "Your message here"

if [ $# -eq 0 ]; then
    echo "Usage: $0 \"Your message here\""
    exit 1
fi

MESSAGE="$1"
API_KEY="1e4ffc3e35d14302ae8c38a3b88afbdf.6rcSE8GW_DsKPquVev9o7obK"

echo "🤖 You: $MESSAGE"
echo "🤖 AI:"

curl -s -X POST https://ollama.com/api/chat \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d "{
    \"model\": \"ai-v3.1:671b-cloud\",
    \"messages\": [{\"role\": \"user\", \"content\": \"$MESSAGE\"}],
    \"stream\": false
  }" | jq -r '.message.content'

echo -e "\n---"