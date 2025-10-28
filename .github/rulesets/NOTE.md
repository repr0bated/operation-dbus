# GitHub Rulesets Configuration

## Status: ⚠️ Rulesets API May Not Be Available

The GitHub Rulesets API may require:
- GitHub Enterprise Cloud account
- Organization-level access
- Or specific GitHub features

## Alternative: Traditional Branch Protection

For now, configure branch protection via the GitHub UI:

1. Go to: Settings → Branches
2. Add rule for `master` branch
3. Configure:
   - ✅ Require pull request reviews (1 approval)
   - ✅ Require status checks to pass
   - ✅ Require branches to be up to date
   - ✅ Require linear history
   - ❌ Do not allow force pushes
   - ❌ Do not allow deletions

## Commands

View existing rulesets:
```bash
gh api repos/repr0bated/operation-dbus/rulesets
```

View branch protection:
```bash
gh api repos/repr0bated/operation-dbus/branches/master/protection
```

## Documentation

- [GitHub REST API - Rulesets](https://docs.github.com/rest/repos/rules)
- [GitHub Branch Protection](https://docs.github.com/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)
