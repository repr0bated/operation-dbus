# How to Create Rulesets in GitHub UI

## Quick Path

1. Go to: https://github.com/repr0bated/operation-dbus/settings/rules
2. Click "New ruleset"
3. Select "Branch ruleset" or "Tag ruleset"
4. Configure settings

## Step-by-Step Instructions

### Option 1: Via Settings → Rules

1. **Navigate to Settings**
   - Go to your repository: https://github.com/repr0bated/operation-dbus
   - Click "Settings" tab
   - Click "Rules" in left sidebar
   - Click "Rulesets"

2. **Create New Ruleset**
   - Click "New ruleset" button
   - Select "Branch ruleset"

3. **Configure Rules**
   - **Name:** "Main Branch Protection"
   - **Target branches:** Select "master" and "main"
   - **Rules to add:**
     - ✅ Require pull request review (1 approval)
     - ✅ Require status checks
     - ✅ Require linear history
     - ✅ Do not allow force pushes
     - ✅ Do not allow deletions

4. **Save**

### Option 2: Import from JSON

If the UI doesn't work, we have JSON files ready:

```bash
# These files exist in .github/rulesets/
- main-branch-protection.json
- development-branch-protection.json
- feature-branch-protection.json
- workflow-rules.json
```

**GitHub UI might have an "Import" option** to upload these JSON files.

## Current Status

✅ **Branch Protection is ACTIVE** via traditional API

You have:
- Require 1 approval
- Linear history enforced
- Force pushes blocked
- Deletions blocked

The "create rulesets" prompt is just GitHub suggesting the newer system, but your protection is already working!

## Why Create Rulesets?

Rulesets provide:
- More advanced features
- Multiple branches in one ruleset
- Better UI management
- Integration with other GitHub features

But traditional branch protection (what you have) works perfectly fine for most use cases.

## Recommendation

**Option A:** Keep traditional branch protection (it's working ✅)  
**Option B:** Try creating rulesets via UI (may require Enterprise features)

For now, your repository is protected!
