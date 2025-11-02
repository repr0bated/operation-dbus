#!/bin/bash
set -euo pipefail

# Script to configure git authentication with a GitHub personal access token

if [ $# -eq 0 ]; then
    echo "Usage: $0 <github-token>"
    echo ""
    echo "This script will configure git to use your GitHub token for authentication."
    echo ""
    echo "If you don't have a token yet, create one at:"
    echo "https://github.com/settings/tokens"
    echo ""
    echo "For HTTPS repositories, use a 'repo' scoped token."
    exit 1
fi

TOKEN="$1"
REPO_URL=$(git remote get-url origin 2>/dev/null || echo "https://github.com/repr0bated/operation-dbus.git")

# Extract user/repo from URL
if [[ "$REPO_URL" =~ github.com[:/]([^/]+)/([^/]+)\.git ]]; then
    GITHUB_USER="${BASH_REMATCH[1]}"
    REPO_NAME="${BASH_REMATCH[2]}"
    
    echo "Configuring git authentication for: $GITHUB_USER/$REPO_NAME"
    
    # Method 1: Update remote URL with token (immediate effect)
    NEW_URL="https://${TOKEN}@github.com/${GITHUB_USER}/${REPO_NAME}.git"
    git remote set-url origin "$NEW_URL"
    echo "✓ Updated remote URL with token"
    
    # Method 2: Also store in credential helper for other repos
    CREDENTIAL_FILE="$HOME/.git-credentials"
    CREDENTIAL_LINE="https://${TOKEN}@github.com"
    
    # Remove old GitHub entries if any
    if [ -f "$CREDENTIAL_FILE" ]; then
        grep -v "github.com" "$CREDENTIAL_FILE" > "${CREDENTIAL_FILE}.tmp" || true
        mv "${CREDENTIAL_FILE}.tmp" "$CREDENTIAL_FILE"
    fi
    
    # Add new credential
    echo "$CREDENTIAL_LINE" >> "$CREDENTIAL_FILE"
    chmod 600 "$CREDENTIAL_FILE"
    echo "✓ Stored credential in $CREDENTIAL_FILE"
    
    echo ""
    echo "Git authentication configured successfully!"
    echo "You can now push/pull without entering credentials."
    
else
    echo "Error: Could not parse repository URL: $REPO_URL"
    exit 1
fi
