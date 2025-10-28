# GitHub Rulesets

This directory contains GitHub rulesets for the operation-dbus repository.

## Rulesets

### main-branch-protection.json
**Target:** `master`, `main` branches  
**Enforcement:** Active

Protects the main branch with:
- ✅ Requires 1 approval before merge
- ✅ Requires status checks to pass (cargo test, clippy, rustfmt)
- ✅ Requires linear history (no merge commits)
- ✅ Blocks deletions
- ✅ Enforces conventional commit messages
- ✅ Blocks force pushes
- ✅ Maximum file size: 50MB

### development-branch-protection.json
**Target:** `develop`, `dev`, `staging` branches  
**Enforcement:** Active

More lenient rules for development branches:
- ✅ No approval required
- ✅ Requires cargo test to pass
- ✅ Allows force pushes
- ✅ Allows deletions
- ✅ Enforces conventional commit messages

### feature-branch-protection.json
**Target:** `feature/*`, `feat/*`, `fix/*`, `mcp/*` branches  
**Enforcement:** Active

Minimal protection for feature branches:
- ✅ No approval required
- ✅ Allows force pushes
- ✅ Allows deletions
- ⚠️ No status checks required

### workflow-rules.json
**Target:** Workflows  
**Enforcement:** Active

Workflow protections:
- ✅ Workflows only from this repository
- ✅ Requires approval for fork PRs in public repos

## Applying Rulesets

GitHub rulesets need to be applied via the GitHub API or UI:

### Via GitHub CLI:
```bash
gh api repos/:owner/:repo/rulesets \
  --method POST \
  --input .github/rulesets/main-branch-protection.json
```

### Via GitHub UI:
1. Go to repository Settings → Rules → Rulesets
2. Click "New ruleset"
3. Upload the JSON file
4. Configure bypass actors if needed

## Conventional Commits

All commits must follow the pattern:
```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Code style (formatting)
- `refactor`: Code refactoring
- `perf`: Performance improvement
- `test`: Adding tests
- `chore`: Maintenance tasks
- `ci`: CI/CD changes
- `build`: Build system changes
- `revert`: Revert a commit

**Examples:**
```
feat(net): add mesh bridge support
fix(lxc): handle veth rename correctly
docs: update installation guide
chore(deps): update dependencies
```

## Bypass Actors

Rulesets can be configured to allow certain actors to bypass rules:
- Repository admins
- Specific GitHub Apps
- Specific users

Configure in the `bypass_actors` array in each ruleset file.

## Testing

To test ruleset configurations locally:
```bash
# Validate JSON
cat .github/rulesets/main-branch-protection.json | jq .

# Check syntax
python -m json.tool .github/rulesets/main-branch-protection.json
```
