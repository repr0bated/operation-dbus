# Quick Guide: Syncing MCP Fork

## 🚀 Quick Start

To sync your MCP fork with the latest changes (including the new chat interface):

```bash
# Option 1: Provide fork URL directly
./sync-to-mcp-fork.sh https://github.com/username/mcp-dbus-fork

# Option 2: Use environment variable
export MCP_FORK_REPO="https://github.com/username/mcp-dbus-fork"
./sync-to-mcp-fork.sh
```

## 📦 What Gets Synced

### New in This Sync (2025-10-27)

**🎉 Chat Interface** - Complete conversational UI for MCP
- `src/mcp/chat_server.rs` - Backend WebSocket server with NLP
- `src/mcp/chat_main.rs` - Standalone chat application
- `src/mcp/web/chat.html` - Modern chat UI
- `src/mcp/web/chat.js` - Interactive chat client
- `src/mcp/web/chat-styles.css` - Beautiful dark/light themes
- `MCP-CHAT-INTERFACE.md` - Complete documentation
- `test-mcp-chat.sh` - Test script

**Features:**
- Natural language commands ("run systemd status nginx")
- Real-time WebSocket communication
- Smart command suggestions
- Tool templates with forms
- Agent management
- Dark/Light theme

### Always Synced

**Source Code:**
- `src/mcp/*` - All MCP components (25+ files)
- `src/plugin_system/` - Plugin architecture
- `src/event_bus/` - Event system
- `mcp-configs/` - Configurations

**Documentation:**
- `MCP-README.md` - Fork README
- `MCP-INTEGRATION.md` - Integration guide
- `MCP-CHAT-INTERFACE.md` - Chat guide ⭐ NEW
- `MCP-WEB-IMPROVEMENTS.md` - Web UI docs
- `SECURITY-FIXES.md` - Security info
- `COUPLING-FIXES.md` - Architecture docs
- `docs/MCP-*.md` - Complete guides

**Config Files:**
- `package.json` - NPM metadata
- `mcp.json` - MCP configuration
- `claude_desktop_config.json` - Claude Desktop config

## 🔍 Pre-Sync Verification

The sync script has been updated to include:

✅ Chat server components  
✅ Chat web interface  
✅ Chat documentation  
✅ Updated dependencies (futures, tower-http trace, chrono serde)  
✅ New binary target (mcp-chat)  
✅ Test script  

## 📝 Step-by-Step Sync Process

### 1. Set Fork URL

Choose one method:

```bash
# Method A: Export as environment variable (persists in session)
export MCP_FORK_REPO="https://github.com/username/mcp-dbus-fork"

# Method B: Pass as argument (one-time)
./sync-to-mcp-fork.sh https://github.com/username/mcp-dbus-fork
```

### 2. Run Sync Script

```bash
./sync-to-mcp-fork.sh
```

The script will:
1. Create a temporary directory
2. Clone your fork
3. Copy all MCP files
4. Create/update fork-specific files (README, Cargo.toml)
5. Show changes to be synced
6. Ask for confirmation
7. Push changes to fork

### 3. Confirm Push

When prompted:
```
Ready to push changes to fork
Repository: https://github.com/username/mcp-dbus-fork
Branch: main
Do you want to push these changes? (y/n)
```

Type `y` to push.

## 🧪 Testing the Synced Fork

After syncing, test the fork:

```bash
# Clone your fork
git clone https://github.com/username/mcp-dbus-fork
cd mcp-dbus-fork

# Build everything
cargo build --release

# Test the new chat interface
cargo run --release --bin mcp-chat
# Open http://localhost:8080/chat.html in browser

# Test other binaries
cargo run --release --bin dbus-mcp
cargo run --release --bin dbus-mcp-web
```

## 🤖 Automated Sync (GitHub Actions)

For automatic syncing on every push:

### Setup:

1. **Go to repository settings:**
   ```
   https://github.com/repr0bated/operation-dbus/settings/secrets/actions
   ```

2. **Add repository secret:**
   - Name: `MCP_FORK_REPO`
   - Value: `https://github.com/username/mcp-dbus-fork` or `username/mcp-dbus-fork`

3. **The workflow will:**
   - Trigger on pushes to `main`/`master`
   - Copy all MCP files
   - Commit with sync message
   - Push to fork
   - Create PR (if configured)

### Workflow Status:

Check: `https://github.com/repr0bated/operation-dbus/actions`

## 🆘 Troubleshooting

### Issue: "No fork repository specified"

**Solution:**
```bash
export MCP_FORK_REPO="https://github.com/username/mcp-dbus-fork"
```

### Issue: Permission denied when pushing

**Solutions:**
1. Use SSH URL: `git@github.com:username/mcp-dbus-fork.git`
2. Check authentication: `gh auth status`
3. Generate PAT: GitHub Settings → Developer settings → Personal access tokens

### Issue: Merge conflicts in fork

**Solution:**
```bash
# In fork directory
git fetch origin
git reset --hard origin/main
# Re-run sync
```

### Issue: Files not syncing

**Check:**
1. File exists in main repo
2. File listed in sync script arrays
3. Script has read permissions

## 📊 Sync History

Sync operations are logged to:
```
~/.mcp-sync-log
```

View history:
```bash
cat ~/.mcp-sync-log
```

## 🎯 What's Different in Fork

The fork includes a customized `Cargo.toml` with:
- Simplified dependencies
- All MCP binaries (including new `mcp-chat`)
- Fork-specific metadata
- Optimized for standalone use

The fork includes a custom `README.md` with:
- Fork-specific instructions
- Quick start guide
- Integration examples
- Links to documentation

## 🔄 Regular Sync Schedule

**Recommended:** Sync after major changes

Major changes include:
- New features (like chat interface)
- Security fixes
- API changes
- Documentation updates
- Bug fixes

## 📚 Related Documentation

After syncing, these docs are available in fork:
- `README.md` - Fork overview
- `MCP-CHAT-INTERFACE.md` - Chat guide ⭐
- `docs/MCP-COMPLETE-GUIDE.md` - Comprehensive guide
- `docs/MCP-API-REFERENCE.md` - API documentation
- `docs/MCP-DEVELOPER-GUIDE.md` - Development guide

## ✅ Post-Sync Checklist

After syncing, verify:

- [ ] Fork repository updated
- [ ] All files present
- [ ] `cargo build --release` succeeds
- [ ] Chat interface works (`mcp-chat` binary)
- [ ] Web interfaces accessible
- [ ] Agents start correctly
- [ ] Documentation readable
- [ ] Examples run

## 🎉 Success!

Your fork is now synced with:
- ✨ New chat interface
- 🔧 Updated tool registry
- 📱 Modern web UI
- 🔒 Security enhancements
- 📚 Complete documentation

## 💡 Next Steps

1. **Test the chat interface**: Most exciting new feature!
   ```bash
   cargo run --release --bin mcp-chat
   ```

2. **Update your integrations**: If you have existing integrations

3. **Review documentation**: Check `MCP-CHAT-INTERFACE.md`

4. **Submit to MCP registry**: If ready for public use

---

**Need help?** See `MCP-FORK-SYNC-STATUS.md` for detailed status.

**Found an issue?** Report at: https://github.com/repr0bated/operation-dbus/issues