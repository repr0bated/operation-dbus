#!/bin/bash
# Run openclaw-indexer with failover chain
# Usage:
#   ./run.sh full              # Index all repos
#   ./run.sh full --repo NAME  # Index single repo
#   ./run.sh status            # Check index stats
#   ./run.sh search "query"    # Search indexed code
#   ./run.sh crawl-only        # Just list repos/files

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Source secrets
source ~/.bash_secrets 2>/dev/null

# Ensure Qdrant is running
if ! curl -s http://127.0.0.1:6333/collections > /dev/null 2>&1; then
    echo "Starting Qdrant..."
    nohup ~/.local/bin/qdrant > /tmp/qdrant.log 2>&1 &
    sleep 3
fi

# Run indexer
exec .venv/bin/python scripts/run_index.py "$@"
