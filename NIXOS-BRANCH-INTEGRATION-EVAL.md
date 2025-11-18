# NixOS Branch Integration Evaluation

## Overview
This document evaluates components from the nixos branch that should be integrated into master, **excluding NixOS-specific configuration files**.

## Components Already Present ✅

### 1. Hybrid Scanner (`src/mcp/hybrid_scanner.rs`)
- **Status**: Already exists in codebase
- **Purpose**: Discovers D-Bus services + non-D-Bus resources (filesystem, processes, hardware)
- **Action**: Verify it's fully integrated with MCP tools

### 2. Hybrid D-Bus Bridge (`src/mcp/hybrid_dbus_bridge.rs`)
- **Status**: Already exists in codebase
- **Purpose**: Exposes non-D-Bus resources via D-Bus interface
- **Action**: Check if it's registered as a D-Bus service and accessible

### 3. PackageKit Agent (`src/mcp/agents/packagekit.rs`)
- **Status**: Already exists in codebase
- **Purpose**: Package management via D-Bus PackageKit
- **Action**: Verify it's working and registered

## Components to Evaluate 🔍

### 1. Bridge Persistence via JSON-RPC + D-Bus
**User Mentioned**: "look at the original install script, how use the json/rpc and dbus to make bridges persistent"

**Current State**:
- OVSDB persistence is documented (`docs/OVS-PERSISTENCE-SETUP.md`)
- `src/native/ovsdb_jsonrpc.rs` has `datapath_type: "system"` for persistence
- Network plugin uses OVSDB JSON-RPC

**What to Check**:
- [ ] Is bridge persistence actually working in practice?
- [ ] Are there any install script patterns that should be preserved?
- [ ] Should we add D-Bus service registration for persistent bridges?
- [ ] Check if `nixos/` has any bridge persistence logic not in master

**Files to Review**:
- Any install scripts in nixos branch
- Bridge creation patterns
- D-Bus service registration for bridges

### 2. MCP Integration Improvements
**From nixos/module.nix** (lines 96-150):
```nix
mcp = {
  enable = mkEnableOption "MCP (Model Context Protocol) integration";
  introspection = mkOption { ... };
  hybridScanner = mkOption { ... };
  agents = { ... };
};
```

**What to Check**:
- [ ] Are there MCP configuration options that should be added?
- [ ] Is hybridScanner properly exposed as MCP tool?
- [ ] Are all agents properly registered?

### 3. System Introspection Enhancements
**From nixos/module.nix**:
- NUMA optimization options
- BTRFS configuration options
- Introspection caching options

**What to Check**:
- [ ] Are these configuration patterns useful for non-NixOS systems?
- [ ] Should we add CLI flags or config file options for these?
- [ ] Are there introspection improvements not in master?

### 4. Configuration Generation
**From nixos/module.nix** (lines 51-67):
```nix
stateFile = pkgs.writeText "op-dbus-state.json" (builtins.toJSON {
  version = 1;
  plugins = {
    net = { interfaces = cfg.network.interfaces; };
    systemd = { units = cfg.systemd.units; };
    packagekit = if cfg.packages.enable then { ... } else null;
  };
});
```

**What to Check**:
- [ ] Should we add a tool to generate state.json from system introspection?
- [ ] Is there a better pattern for state file generation?
- [ ] Can we extract the state generation logic (without Nix)?

## Files to Examine in nixos Branch

### Priority 1: Bridge Persistence
1. **Any install scripts** - Look for bridge persistence patterns
2. **OVSDB configuration** - Check for persistence settings
3. **D-Bus service files** - Look for bridge registration

### Priority 2: MCP Enhancements
1. **MCP configuration patterns** - Extract non-NixOS parts
2. **Agent registration** - Check if all agents are properly exposed
3. **Introspection tool registration** - Verify completeness

### Priority 3: Configuration & State Management
1. **State file generation** - Extract logic (without Nix)
2. **Configuration validation** - Check validation patterns
3. **Default state templates** - Useful examples?

## Integration Checklist

### Phase 1: Bridge Persistence ✅
- [ ] Review install scripts for bridge persistence patterns
- [ ] Verify OVSDB JSON-RPC persistence is working
- [ ] Check if D-Bus service registration needed
- [ ] Test bridge persistence across reboots
- [ ] Document any missing pieces

### Phase 2: MCP Integration ✅
- [ ] Verify hybrid_scanner is exposed as MCP tool
- [ ] Check hybrid_dbus_bridge is registered as D-Bus service
- [ ] Ensure all agents are properly registered
- [ ] Test MCP tool discovery

### Phase 3: Configuration Improvements ✅
- [ ] Extract state generation patterns (without Nix)
- [ ] Add configuration validation if missing
- [ ] Improve default state templates
- [ ] Add CLI options for advanced features

## Next Steps

1. **Examine nixos branch files** (excluding .nix configs):
   ```bash
   # Find non-NixOS files
   find nixos/ -type f ! -name "*.nix" ! -name "flake.nix"
   ```

2. **Compare with master**:
   - Check if hybrid components are fully integrated
   - Look for missing bridge persistence logic
   - Find any useful patterns to extract

3. **Test Integration**:
   - Verify bridge persistence works
   - Test MCP tools discovery
   - Validate configuration generation

## Notes

- **NixOS-specific files to IGNORE**:
  - `*.nix` files (module definitions)
  - `flake.nix` (Nix flake)
  - `configuration.nix` (NixOS config)
  - `validate-config.sh` (NixOS validation)

- **Files to EXAMINE**:
  - Any install/setup scripts
  - Documentation about bridge persistence
  - Code patterns in module.nix (extract logic, not Nix syntax)
  - Configuration examples (convert to JSON/YAML)

