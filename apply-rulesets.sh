#!/bin/bash
# Apply GitHub rulesets to the repository

set -e

REPO="repr0bated/operation-dbus"
RULESETS_DIR=".github/rulesets"

echo "=== Applying GitHub Rulesets ==="
echo "Repository: $REPO"
echo ""

# Check if gh CLI is installed
if ! command -v gh &> /dev/null; then
    echo "Error: GitHub CLI (gh) is not installed"
    echo "Install it from: https://cli.github.com/"
    exit 1
fi

# Check if authenticated
if ! gh auth status &> /dev/null; then
    echo "Error: Not authenticated with GitHub CLI"
    echo "Run: gh auth login"
    exit 1
fi

# Apply each ruleset
for ruleset_file in "$RULESETS_DIR"/*.json; do
    if [ -f "$ruleset_file" ]; then
        filename=$(basename "$ruleset_file")
        echo "Applying: $filename"
        
        # Validate JSON
        if ! jq empty "$ruleset_file" 2>/dev/null; then
            echo "  ❌ Invalid JSON in $filename"
            continue
        fi
        
        # Check if ruleset already exists
        ruleset_name=$(jq -r '.name' "$ruleset_file")
        existing=$(gh api repos/$REPO/rulesets 2>/dev/null | jq -r ".[] | select(.name == \"$ruleset_name\") | .id" || echo "")
        
        if [ -n "$existing" ]; then
            echo "  ⚠️  Ruleset '$ruleset_name' already exists (ID: $existing)"
            echo "  To update: gh api repos/$REPO/rulesets/$existing --method PUT --input $ruleset_file"
        else
            # Create new ruleset
            if gh api repos/$REPO/rulesets --method POST --input "$ruleset_file" &> /dev/null; then
                echo "  ✅ Successfully created ruleset: $ruleset_name"
            else
                echo "  ❌ Failed to create ruleset: $ruleset_name"
            fi
        fi
        echo ""
    fi
done

echo "=== Summary ==="
echo "Rulesets in repository:"
gh api repos/$REPO/rulesets | jq -r '.[] | "  - \(.name) (ID: \(.id), Enforcement: \(.enforcement))"'

echo ""
echo "✅ Done!"
echo ""
echo "To view or manage rulesets:"
echo "  gh repo view $REPO --web"
echo "  → Settings → Rules → Rulesets"
