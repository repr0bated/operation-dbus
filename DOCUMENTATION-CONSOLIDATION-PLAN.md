# Documentation Consolidation Plan

## Executive Summary

**Current State:** 45 markdown files, mostly accurate but needs organization  
**Goal:** Streamlined, accurate, well-organized documentation  
**Timeline:** Immediate cleanup + ongoing reorganization

---

## Phase 1: Immediate Cleanup (Do Now)

### 1.1 Remove Duplicate Files ✅

**Duplicate Content:**
- `CACHING-STRATEGY.md` (18K) - Duplicate of `BTRFS-CACHING-STRATEGY.md` (19K)
- **Action:** Archive `CACHING-STRATEGY.md`, keep `BTRFS-CACHING-STRATEGY.md`

**Reason:** `BTRFS-CACHING-STRATEGY.md` is more comprehensive and matches implemented code.

### 1.2 Fix Path References

**Files with incorrect paths:**
- `README.md` - References `/git/op-dbus` → Update to `/git/operation-dbus`
- `STATUS.md` - References `/git/op-dbus` → Update to `/git/operation-dbus`
- Various deployment docs

**Action:** Global find/replace `/git/op-dbus` → `/git/operation-dbus`

### 1.3 Archive Outdated Sync Docs

**Files to archive:** (They're about syncing to a fork that's been merged)
- `MCP-FORK-SYNC-STATUS.md` - Move to `docs/archive/`
- `MCP-SYNC-READY.md` - Move to `docs/archive/`
- `SYNC-MCP-FORK.md` - Move to `docs/archive/`

---

## Phase 2: Reorganization (Next Sprint)

### 2.1 Create Directory Structure

```
docs/
├── getting-started/
│   ├── README.md (overview)
│   ├── quickstart.md
│   └── installation.md
├── architecture/
│   ├── README.md (overview)
│   ├── blockchain-vector.md
│   ├── mcp-server.md
│   └── overall.md
├── deployment/
│   ├── README.md (overview)
│   ├── basic.md
│   ├── enterprise.md
│   └── containers.md
├── mcp/
│   ├── README.md (overview)
│   ├── integration.md
│   ├── benefits.md
│   ├── api-reference.md
│   └── developer-guide.md
├── networking/
│   ├── README.md (overview)
│   ├── dual-bridge.md
│   ├── netmaker.md
│   └── networkmanager.md
├── reference/
│   ├── cli-design.md
│   ├── security.md
│   └── troubleshooting.md
└── archive/
    └── (old sync docs, etc.)
```

### 2.2 File Mapping

**Getting Started:**
- `README.md` → Keep in root
- `QUICKSTART.md` → `docs/getting-started/quickstart.md`
- `INSTALL-COMPLETE.md` → `docs/getting-started/installation.md`

**Architecture:**
- `ARCHITECTURE-ANALYSIS.md` → `docs/architecture/blockchain-vector.md`
- `MCP-ARCHITECTURE-ANALYSIS.md` → `docs/architecture/mcp-server.md`
- `arch-review.md` → `docs/architecture/overall.md`

**Deployment:**
- `DEPLOYMENT.md` → `docs/deployment/basic.md`
- `ENTERPRISE-DEPLOYMENT.md` → `docs/deployment/enterprise.md`
- `CONTAINER-SETUP.md` → `docs/deployment/containers.md`

**MCP:**
- `MCP-README.md` → `docs/mcp/README.md`
- `MCP-INTEGRATION.md` → `docs/mcp/integration.md`
- `MCP-DBUS-BENEFITS.md` → `docs/mcp/benefits.md`
- `docs/MCP-*.md` → Keep in `docs/mcp/`

**Networking:**
- `DUAL-BRIDGE-ARCHITECTURE.md` → `docs/networking/dual-bridge.md`
- `NETMAKER-MESH-FIX.md` → `docs/networking/netmaker.md`
- `NETWORKMANAGER-PLUGIN-DESIGN.md` → `docs/networking/networkmanager.md`

**Reference:**
- `CLI-DESIGN.md` → `docs/reference/cli-design.md`
- `SECURITY-FIXES.md` → `docs/reference/security.md`

**Keep in Root:**
- `README.md` (main entry point)
- `DOCUMENTATION-INDEX.md` (this file)
- `STATUS.md` (project status)

---

## Phase 3: Content Updates (Ongoing)

### 3.1 Create Overview Files

Each subdirectory needs a `README.md` that explains:
- What's in this section
- Target audience
- Links to key documents

### 3.2 Cross-Reference Links

Update all files to use relative links:
- `[Quick Start](../getting-started/quickstart.md)`
- `[Architecture](../architecture/README.md)`

### 3.3 Update Broken Links

Scan for broken internal links and fix them.

---

## Actions Taken

### ✅ Completed
1. Created `DOCUMENTATION-INDEX.md` - Master index
2. Created `DOCUMENTATION-CONSOLIDATION-PLAN.md` - This file
3. Identified duplicate files
4. Identified outdated references

### 🔄 In Progress
1. Archive duplicate file (`CACHING-STRATEGY.md`)
2. Fix path references
3. Create directory structure

### 📋 Pending
1. Move files to new structure
2. Create overview files
3. Update cross-references
4. Fix broken links

---

## Implementation Steps

### Step 1: Archive Duplicate (Now)
```bash
mkdir -p docs/archive
mv CACHING-STRATEGY.md docs/archive/CACHING-STRATEGY.md.backup
echo "Archived as duplicate of BTRFS-CACHING-STRATEGY.md" > docs/archive/README.md
```

### Step 2: Fix Paths (Now)
```bash
# Update all references to /git/op-dbus
find . -name "*.md" -type f -exec sed -i 's|/git/op-dbus|/git/operation-dbus|g' {} \;
```

### Step 3: Create Structure (Next)
```bash
mkdir -p docs/{getting-started,architecture,deployment,mcp,networking,reference,archive}
```

### Step 4: Move Files (Next)
```bash
# After creating structure, move files as planned
```

---

## Verification Checklist

After reorganization:

- [ ] All files accessible via new paths
- [ ] No broken internal links
- [ ] Root README.md updated with new structure
- [ ] Each subdirectory has README.md
- [ ] Cross-references work
- [ ] Old paths redirected or updated
- [ ] Documentation still accurate after move

---

## Timeline

**Week 1:**
- ✅ Create index and consolidation plan
- Archive duplicate files
- Fix path references

**Week 2:**
- Create directory structure
- Move files systematically
- Create overview files

**Week 3:**
- Update cross-references
- Fix broken links
- Verify all documentation

**Week 4:**
- Final review
- Update main README
- Add new documentation as needed

---

## Notes

- Keep current working directory as `/git/operation-dbus`
- Documentation organization doesn't affect code
- Can be done incrementally without breaking anything
- All existing links should continue to work during transition
