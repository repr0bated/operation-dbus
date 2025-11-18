#!/bin/bash
# Cursor Bridge - Manual copy/paste bridge for chat-server
# Usage: cursor-bridge.sh "your question here"

REQUEST="$1"
REQUEST_FILE="/tmp/cursor-request.txt"
RESPONSE_FILE="/tmp/cursor-response.txt"

# Write request to file
echo "$REQUEST" > "$REQUEST_FILE"

echo "=========================================="
echo "COPY THIS TO CURSOR CHAT:"
echo "=========================================="
cat "$REQUEST_FILE"
echo ""
echo "=========================================="
echo "Waiting for response in: $RESPONSE_FILE"
echo "Paste Cursor's response there and press Enter..."
echo "=========================================="

# Clear old response
> "$RESPONSE_FILE"

# Wait for user to paste response
read -p "Press Enter after pasting response to $RESPONSE_FILE..."

# Check if response exists
if [ -s "$RESPONSE_FILE" ]; then
    cat "$RESPONSE_FILE"
else
    echo "ERROR: No response found in $RESPONSE_FILE"
    exit 1
fi
