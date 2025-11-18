# Branch Comparison Guide

## Three Branches to Compare

### 1. `master`
**Current main branch**

**Key Features**:
- Basic MCP server implementation
- D-Bus introspection
- Plugin system
- Basic agent registry

### 2. `origin/claude/ghostbfridge-non-nixos-improvements-018yb8CpSXQQuGGr8W1zz3TZ`
**NixOS improvements branch (non-NixOS components)**

**Key Features**:
- Staged deployment system with BTRFS snapshots
- Visual workflow builder for D-Bus automation
- D-Bus discovery web interface
- OVS bridge creation tools (`create_ovs_bridge_with_uplink.rs`)
- OVS bridge templates (JSON)
- Deployment documentation
- Workflow scripts

**Commits not in master**:
- `f34a0f6` - Add quick reference card for fast deployment lookup
- `47bd98b` - Add comprehensive deployment documentation and audit
- `da7979f` - Add staged deployment system with BTRFS snapshots
- `81a9fc9` - Build visual workflow builder for D-Bus automation
- `3ef654c` - Build comprehensive D-Bus discovery web interface
- `c401069` - Add D-Bus discovery tools for self-documenting API exploration

### 3. `claude/chatbot-data-improvements` (NEW)
**Chatbot data access improvements**

**Key Features**:
- Agent tools exposing embedded Rust agents as MCP tools
- Granular D-Bus introspection tools (services, methods, properties, signals)
- SQLite cache integration (XML→JSON conversion)
- Workflow nodes system (plugins/services/agents as nodes)
- Error handling for non-introspectable objects
- Deployment image system

**New Files**:
- `src/mcp/tools/agents.rs` - Agent tools
- `src/mcp/tools/dbus_granular.rs` - Granular D-Bus tools
- `src/mcp/workflow_nodes.rs` - Workflow node discovery
- `src/deployment/image_manager.rs` - Deployment images
- `MCP-ARCHITECTURE-ANALYSIS.md` - Architecture documentation
- `CHATBOT-DATA-IMPROVEMENTS.md` - Chatbot improvements guide
- `DEPLOYMENT-IMAGES.md` - Deployment image documentation

## Comparison Commands

### Compare All Three Branches
```bash
# See what's unique to each branch
git log master..origin/claude/ghostbfridge-non-nixos-improvements-018yb8CpSXQQuGGr8W1zz3TZ --oneline
git log master..claude/chatbot-data-improvements --oneline
git log origin/claude/ghostbfridge-non-nixos-improvements-018yb8CpSXQQuGGr8W1zz3TZ..claude/chatbot-data-improvements --oneline
```

### File Differences
```bash
# Files in nixos branch but not in master
git diff master origin/claude/ghostbfridge-non-nixos-improvements-018yb8CpSXQQuGGr8W1zz3TZ --name-only

# Files in chatbot branch but not in master
git diff master claude/chatbot-data-improvements --name-only

# Files in chatbot branch but not in nixos branch
git diff origin/claude/ghostbfridge-non-nixos-improvements-018yb8CpSXQQuGGr8W1zz3TZ claude/chatbot-data-improvements --name-only
```

### Key Differences

#### NixOS Branch Has (not in master):
- `staging/STAGING-LAYOUT.md` - Staged deployment architecture
- `src/bin/create_ovs_bridge_with_uplink.rs` - OVS bridge creation tool
- `templates/ovs-*.json` - OVS bridge templates
- Enhanced web interface for workflow builder
- Deployment documentation

#### Chatbot Branch Has (not in master):
- `src/mcp/tools/agents.rs` - Agent MCP tools
- `src/mcp/tools/dbus_granular.rs` - Granular D-Bus tools
- `src/mcp/workflow_nodes.rs` - Workflow node discovery
- `src/deployment/image_manager.rs` - Deployment images
- SQLite cache integration for D-Bus introspection

#### Overlap:
- Both branches have deployment-related features
- Both branches have workflow-related features
- Both branches have web interface improvements

## Integration Strategy

### Phase 1: Merge NixOS Branch Components
1. **OVS Bridge Tools** - Extract `create_ovs_bridge_with_uplink.rs` and templates
2. **Staging System** - Already in master, verify alignment
3. **Web Interface** - Merge workflow builder enhancements
4. **Documentation** - Integrate deployment docs

### Phase 2: Merge Chatbot Branch Components
1. **Agent Tools** - Register in both MCP servers
2. **Granular D-Bus Tools** - Register in both MCP servers
3. **Workflow Nodes** - Integrate with web interface workflow builder
4. **SQLite Cache** - Already exists, verify integration

### Phase 3: Unified Integration
1. **Combine workflow systems** - Merge workflow builder from nixos with workflow nodes from chatbot
2. **Unify deployment** - Combine staging system with deployment images
3. **Complete web interface** - Merge all UI improvements
4. **Documentation** - Consolidate all docs

## Next Steps

1. **Compare branches** using commands above
2. **Identify conflicts** and resolve
3. **Extract non-NixOS components** from nixos branch
4. **Merge chatbot improvements** into master
5. **Integrate nixos improvements** (non-NixOS parts)
6. **Test unified system**

## Branch Status

- ✅ `master` - Stable base
- ✅ `origin/claude/ghostbfridge-non-nixos-improvements-018yb8CpSXQQuGGr8W1zz3TZ` - NixOS improvements (remote)
- ✅ `claude/chatbot-data-improvements` - Chatbot improvements (local, ready to push)

