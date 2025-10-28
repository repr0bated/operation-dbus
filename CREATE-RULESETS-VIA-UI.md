# Create Rulesets via GitHub UI

## Direct Link

**Open this URL:** https://github.com/repr0bated/operation-dbus/settings/rulesets

## Steps to Create Ruleset

### 1. Navigate to Rulesets
- Go to: Settings → Rules → Rulesets
- Click "New ruleset" button

### 2. Configure Main Branch Protection

**Step 1: Basic Info**
- **Ruleset name:** "Main Branch Protection"
- **Target:** Select "Branch"
- **Enforcement:** "Active"
- Click "Next"

**Step 2: Choose Branches**
- Select "Branch name"
- Enter: `master`
- Enter: `main`
- Click "Next"

**Step 3: Configure Rules**

Add these rules by clicking "Add rule" for each:

1. **Pull request**
   - ✅ Require pull request before merging
   - Required approvals: 1
   - ✅ Dismiss stale reviews when new commits are pushed
   - ✅ Require review thread resolution

2. **Status checks**
   - ✅ Require status checks to pass before merging
   - Select all checks (if any)
   - ✅ Require branches to be up to date

3. **History**
   - ✅ Require linear history

4. **Merge rules**
   - ❌ Do not allow force pushes
   - ❌ Do not allow deletions

5. **Conversation resolution**
   - ✅ Require conversation resolution before merging

**Step 4: Push Restriction**
- Create restriction: "No"
- Click "Next"

**Step 5: Protect matched branches**
- Enable: "Yes"
- Click "Create ruleset"

### 3. Test It

Try to push directly to master (should fail):
```bash
git push origin master
# Should say: "remote: error: changes must be made through a pull request"
```

### Alternative: Use Our JSON Files

We have pre-configured JSON files in `.github/rulesets/`:
- `main-branch-protection.json`
- `development-branch-protection.json`
- `feature-branch-protection.json`

If GitHub UI supports importing JSON, use these files.

## Current Status

✅ **Traditional branch protection is working**

You already have protection configured, but GitHub is prompting you to use the newer rulesets system.

## Recommendation

Since traditional branch protection is working, you can:
1. **Ignore the prompt** - Your protection is fine
2. **Create rulesets** - For better management and features
3. **Wait** - Until you have Enterprise features

The rulesets will give you a nicer UI to manage protection rules, but both systems provide the same protection.
