# Quick Reference Card - Operation-DBus Deployment

## Current Status
✅ **In Git:** 6,134 lines across 5 commits (all pushed)
❌ **Deployed:** Nothing running on live system

## Repository
**Single repo:** `operation-dbus`
**Branch:** `claude/ghostbfridge-non-nixos-improvements-018yb8CpSXQQuGGr8W1zz3TZ`

## What Was Built

| Commit | Feature | Files | Purpose |
|--------|---------|-------|---------|
| c401069 | Discovery Tools | introspection.rs | Backend API for D-Bus exploration |
| 3ef654c | Discovery UI | HTML/JS/CSS | Tree-view web interface |
| 81a9fc9 | Workflow Builder | HTML/JS/CSS | Visual programming for automation |
| da7979f | Staging System | op-dbus-stage-v2 | Deployment tool with snapshots |
| 47bd98b | Documentation | 3 markdown files | This audit and guides |

## 3-Minute Deploy

\`\`\`bash
cd /home/user/operation-dbus
sudo staging/op-dbus-stage-v2 interactive  # Select option 4
# Web UI: http://localhost:8080
\`\`\`

## Documentation Files

| File | Purpose | Size |
|------|---------|------|
| **DEPLOYMENT-AUDIT.md** | What's in git vs what's deployed | 19K |
| **DEPLOYMENT-GUIDE.md** | How to deploy (3 methods) | 24K |
| **SESSION-SUMMARY.md** | Complete overview of all work | 20K |
| **QUICK-REFERENCE.md** | This file (fast lookup) | 3K |

## Key Commands

### Deploy Everything (Automated)
\`\`\`bash
sudo staging/op-dbus-stage-v2 interactive
# Select: 4 (Install all)
\`\`\`

### Deploy Everything (Manual)
\`\`\`bash
cargo build --release --bin dbus-mcp --features mcp
sudo staging/op-dbus-stage-v2 deploy core-dbus
sudo staging/op-dbus-stage-v2 deploy introspection-cache
sudo staging/op-dbus-stage-v2 deploy mcp-server
sudo dbus-mcp --port 8080 --enable-web --web-root /var/www/op-dbus-mcp &
\`\`\`

### Quick Test (No Install)
\`\`\`bash
cargo build --release --bin dbus-mcp --features mcp
sudo target/release/dbus-mcp --port 8080 --enable-web --web-root src/mcp/web
\`\`\`

### Check Status
\`\`\`bash
staging/op-dbus-stage-v2 status        # Deployed stages + snapshots
ps aux | grep dbus-mcp                 # MCP server running?
ss -tlnp | grep 8080                   # Port listening?
curl -I http://localhost:8080          # Web UI accessible?
\`\`\`

### Rollback
\`\`\`bash
sudo staging/op-dbus-stage-v2 rollback mcp-server
sudo staging/op-dbus-stage-v2 rollback introspection-cache
sudo staging/op-dbus-stage-v2 rollback core-dbus
\`\`\`

## Access Points After Deploy

| Feature | Access Method |
|---------|--------------|
| **Web UI** | http://localhost:8080 |
| **Discovery Tools** | MCP JSON-RPC API on port 8080 |
| **Core Binary** | \`/usr/local/bin/op-dbus\` |
| **Cache Warmup** | Systemd timer (runs every 15 min) |

## One-Liner Deploy

\`\`\`bash
sudo staging/op-dbus-stage-v2 interactive
\`\`\`

**Then select option 4 and you're done!**
