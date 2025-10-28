#!/bin/bash
# Create branch protection ruleset via GitHub API

set -e

REPO="repr0bated/operation-dbus"

echo "Creating branch protection ruleset..."

gh api repos/$REPO/rulesets --method POST --input <(cat <<'EOF'
{
  "name": "Branch Protection",
  "target": "branch",
  "enforcement": "active",
  "conditions": {
    "ref_name": {
      "include": ["refs/heads/master", "refs/heads/main"]
    }
  },
  "rules": [
    {
      "type": "pull_request",
      "parameters": {
        "required_approving_review_count": 1,
        "dismiss_stale_reviews_on_push": true,
        "require_code_owner_review": false,
        "require_last_push_approval": false,
        "required_review_thread_resolution": true
      }
    },
    {
      "type": "required_linear_history"
    },
    {
      "type": "deletion",
      "parameters": {
        "allow_deletions": false
      }
    },
    {
      "type": "update",
      "parameters": {
        "allow_force_pushes": false,
        "allow_deletions": false
      }
    }
  ]
}
EOF
) && echo "✅ Ruleset created successfully!"
