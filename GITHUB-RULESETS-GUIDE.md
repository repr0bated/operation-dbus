# GitHub Rulesets Guide

## Overview

This repository uses GitHub rulesets to enforce branch protection, code quality, and security policies.

## What Are Rulesets?

GitHub rulesets are policies that enforce how contributors interact with the repository. They provide:
- Branch protection (prevent accidental deletion, force pushes)
- Pull request requirements (reviews, status checks)
- Commit message standards
- File size limits
- Workflow security

## Configured Rulesets

### 1. Main Branch Protection
**Files:** `master`, `main`  
**Strictness:** High

**Requirements:**
- ✅ At least 1 approval before merge
- ✅ All status checks must pass
- ✅ Linear history (no merge commits)
- ✅ Conventional commit messages
- ❌ No force pushes
- ❌ No branch deletion
- ❌ No direct pushes (must use PRs)

**Status Checks Required:**
- `cargo test` - Unit tests must pass
- `cargo clippy` - Code quality checks
- `rustfmt check` - Code formatting

### 2. Development Branch Protection
**Files:** `develop`, `dev`, `staging`  
**Strictness:** Medium

**Requirements:**
- ✅ `cargo test` must pass
- ✅ Conventional commit messages
- ✅ Linear history
- ✅ Allows force pushes
- ✅ Allows deletion

### 3. Feature Branch Protection
**Files:** `feature/*`, `feat/*`, `fix/*`, `mcp/*`  
**Strictness:** Low

**Requirements:**
- ✅ Minimal restrictions
- ✅ Allows force pushes
- ✅ Allows deletion
- ⚠️ No required checks

### 4. Workflow Protection
**Target:** All workflows  
**Strictness:** High

**Requirements:**
- ✅ Workflows only from this repository
- ✅ Requires approval for fork PRs in public repos

## Applying Rulesets

### Option 1: Via Script (Recommended)
```bash
./apply-rulesets.sh
```

### Option 2: Via GitHub CLI
```bash
# View existing rulesets
gh api repos/:owner/:repo/rulesets

# Create a ruleset
gh api repos/:owner/:repo/rulesets \
  --method POST \
  --input .github/rulesets/main-branch-protection.json

# Update a ruleset
gh api repos/:owner/:repo/rulesets/:ruleset_id \
  --method PUT \
  --input .github/rulesets/main-branch-protection.json

# Delete a ruleset
gh api repos/:owner/:repo/rulesets/:ruleset_id \
  --method DELETE
```

### Option 3: Via GitHub UI
1. Go to repository Settings
2. Click Rules → Rulesets
3. Click "New ruleset"
4. Select "Branch" or "Workflow"
5. Upload JSON file or configure manually

## Conventional Commits

All commits must follow the conventional commit format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Examples

✅ **Good:**
```
feat(net): add mesh bridge support
fix(lxc): handle veth rename correctly
docs: update installation guide
chore(deps): update dependencies
ci: add GitHub Actions workflow
```

❌ **Bad:**
```
fix stuff
update docs
changes
WIP
```

### Commit Types

| Type | Use For |
|------|---------|
| `feat` | New features |
| `fix` | Bug fixes |
| `docs` | Documentation changes |
| `style` | Code formatting (no logic changes) |
| `refactor` | Code refactoring |
| `perf` | Performance improvements |
| `test` | Adding or updating tests |
| `chore` | Maintenance tasks |
| `ci` | CI/CD changes |
| `build` | Build system changes |
| `revert` | Reverting commits |

## Enforcement Levels

### Active Enforcement
Rules are enforced immediately. Violations will block actions.

### Evaluate Enforcement
Rules are evaluated but don't block actions. Useful for testing.

## Bypass Actors

Rulesets can allow certain actors to bypass rules:
- Repository admins
- Specific GitHub Apps
- Specific users

Configure in the `bypass_actors` array in each ruleset file.

## Troubleshooting

### "Branch is not up to date"
**Cause:** Main branch has changes you don't have locally  
**Fix:** Pull latest changes before pushing
```bash
git pull origin master
```

### "Required status check not met"
**Cause:** CI checks haven't passed yet  
**Fix:** Wait for checks to complete or fix failing tests

### "Commit message doesn't match pattern"
**Cause:** Commit message doesn't follow conventional commits  
**Fix:** Amend the commit message
```bash
git commit --amend -m "feat: proper commit message"
```

### "Branch cannot be deleted"
**Cause:** Branch is protected  
**Fix:** Use GitHub UI to delete if you have admin permissions

## Current Status

View active rulesets:
```bash
gh api repos/:owner/:repo/rulesets | jq -r '.[] | "\(.name) - \(.enforcement)"'
```

## Benefits

- 🔒 **Security:** Prevents accidental force pushes to main
- 🎯 **Quality:** Ensures code is tested before merge
- 📝 **Standards:** Enforces consistent commit messages
- 🤝 **Collaboration:** Requires peer review for main branch
- 📊 **Visibility:** Clear process for contributions

## Questions?

- See `.github/rulesets/README.md` for technical details
- Open an issue for questions or suggestions
- Contact repository admins for bypass requests
