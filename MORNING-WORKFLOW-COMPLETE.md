# Complete Morning Workflow - Plugins & Schemas
**op-dbus Automated Integration System**

ChatGPT is generating **two types** of artifacts hourly:
1. **State Management Plugins** - Rust plugins for system state tracking
2. **Container Schemas** - Production-grade LXC container specifications

This guide covers processing both types after overnight generation.

---

## Quick Start (5 minutes)

```bash
cd /git/operation-dbus

# Process all plugins
./install_all_plugins.sh

# Process all schemas
./install_all_schemas.sh

# Test everything
./test_all_plugins.sh
cargo build --release
```

---

## Part 1: State Management Plugins

### Expected Artifacts
- **Format**: `*_pack_*.zip` (e.g., `sessdecl_pack_20251031_0639.zip`)
- **Contents**: Rust plugin file + register.sh script + example JSON
- **Location**: `/home/jeremy/`

### Batch Installation
```bash
cd /git/operation-dbus
./install_all_plugins.sh
```

**Expected Output:**
```
🔍 Searching for plugin packs in: /home/jeremy
📦 Found 8 plugin pack(s)

Processing: sessdecl_pack_20251031_0639.zip
✅ Plugin installed: sessdecl
✅ Build successful

...

📊 Batch Installation Summary
   ✅ Successful: 7
   ❌ Failed: 1
   ⏭️  Skipped: 0
   📦 Total: 8
```

###Handling Failures

**Common Issue #1: Python f-strings** (10% of plugins)
```bash
Error: this method takes 1 argument but 2 arguments were supplied
  --> src/state/plugins/example.rs:94
   |
94 |     .arg(f"command {variable}")
   |          ^^^^^^^^^^^^^^^^^^^^^^
```

**Fix:**
```rust
// BEFORE (Python syntax)
.arg(f"mv -f {path} /dest")

// AFTER (Rust syntax)
let cmd = format!("mv -f {} /dest", path);
.arg(&cmd)
```

**Common Issue #2: Python booleans** (auto-fixed)
- `False` → `false`
- `True` → `true`

**Common Issue #3: Invalid json! syntax** (rare)
```rust
// BEFORE
json!({"items": [] as [Value; 0]})

// AFTER
let empty: Vec<Value> = Vec::new();
json!({"items": empty})
```

### Manual Plugin Installation
If batch fails on a specific plugin:
```bash
./install_plugin.sh /home/jeremy/plugin_pack_*.zip
```

### Testing Plugins
```bash
# Test all installed plugins
./test_all_plugins.sh

# Test specific plugin
./target/release/op-dbus query -p plugin-name
```

---

## Part 2: Container Schemas

### Expected Artifacts
- **Format**: `Production_Container_Spec_<Domain>_<timestamp>.zip`
- **Example**: `Production_Container_Spec_Smart_Aquarium_20251031-062835.zip`
- **Contents**:
  - `schema/production.container.schema.json` - Production overlay
  - `LXC-CONFIGURATION-SCHEMA.json` - Base LXC schema
  - `mapping/legacy_to_production.csv` - Legacy migration mappings
  - `examples/*.json` - Valid container configs
  - `tests/invalid*.json` - Validation test cases
  - `docs/README.md` - Domain documentation

### Batch Installation
```bash
cd /git/operation-dbus
./install_all_schemas.sh
```

**Expected Output:**
```
🔍 Searching for schema bundles in: /home/jeremy
📦 Found 6 schema bundle(s)

Processing: Production_Container_Spec_Smart_Aquarium_20251031-062835.zip
🏢 Domain: smart-aquarium
📋 Installing production overlay schema...
📋 Installing base LXC schema...
🗺️  Installing legacy migration mapping...
📝 Installing examples... Found 3 example files
🧪 Installing test cases... Found 6 test files
📖 Installing documentation...
✅ Production schema JSON is valid
✅ Base LXC schema JSON is valid

📦 Schema Bundle Summary
   Domain: smart-aquarium
   Production Schema: ✅
   Base LXC Schema: ✅
   Legacy Mapping: ✅
   Examples: 3 files
   Tests: 6 files
   Documentation: ✅

🎉 Schema installation complete!
```

### Schema Validation (Optional)

Install ajv-cli for full schema validation:
```bash
npm install -g ajv-cli
```

Then re-run installation - it will automatically validate:
- Examples must pass schema validation
- Invalid test cases must fail schema validation

### Manual Schema Installation
```bash
./install_schema.sh /home/jeremy/Production_Container_Spec_*.zip
```

### Validating Container Specs
```bash
# Validate a container config against domain schema
./validate_container_spec.sh smart-aquarium my-container.json
```

---

## Complete Workflow Timeline

### Overnight (Automated)
- **Plugins**: 1 per hour × 8 hours = 8 plugins
- **Schemas**: 1 per hour × 8 hours = 8 schemas
- **Total**: 16 artifacts waiting

### Morning (5-15 minutes)
1. **Install Plugins** (3-10 min)
   ```bash
   ./install_all_plugins.sh
   ```
   - 90% success rate with auto-fixes
   - 1-2 may need manual f-string fixes (2 min each)

2. **Install Schemas** (1-2 min)
   ```bash
   ./install_all_schemas.sh
   ```
   - Nearly 100% success rate
   - Schemas are JSON, no Rust compilation needed

3. **Test & Build** (2-3 min)
   ```bash
   ./test_all_plugins.sh
   cargo build --release
   ```

---

## Directory Structure After Installation

```
/git/operation-dbus/
├── src/state/plugins/
│   ├── sessdecl.rs          # Session management
│   ├── dnsresolver.rs       # DNS configuration
│   ├── pcidecl.rs           # PCI devices
│   ├── filesystemdecl.rs    # Filesystem state
│   ├── kernelparams.rs      # Kernel parameters
│   ├── clocksync.rs         # Time synchronization
│   └── ...                  # More generated plugins
│
└── schemas/
    ├── smart-aquarium/
    │   ├── production.schema.json
    │   ├── lxc-base.schema.json
    │   ├── legacy-mapping.csv
    │   ├── README.md
    │   ├── examples/
    │   │   ├── gateway.prod.json
    │   │   ├── analytics.stage.json
    │   │   └── api.dev.json
    │   └── tests/
    │       ├── invalid_privileged_no_ticket.json
    │       └── ...
    │
    ├── healthcare-telehealth/
    ├── financial-trading/
    ├── smart-manufacturing/
    ├── energy-grid/
    └── ...
```

---

## Verification Checklist

### Plugins
- [ ] All plugin packs processed
- [ ] Build successful (`cargo build --release`)
- [ ] All tests pass (`./test_all_plugins.sh`)
- [ ] Plugins registered in `src/state/plugins/mod.rs`
- [ ] Query commands work for each plugin

### Schemas
- [ ] All schema bundles processed
- [ ] JSON validation passes for all schemas
- [ ] Examples validate against production schemas
- [ ] Invalid test cases correctly rejected
- [ ] Documentation readable

---

## Troubleshooting

### Plugin Build Failures

**Issue**: Duplicate plugin registration
```
error[E0428]: the name `PluginName` is defined multiple times
```

**Fix**: Check `src/main.rs` and `src/state/plugins/mod.rs` for duplicate entries, remove extras.

**Issue**: Missing .await
```
error: expected `;`, found `state_manager`
```

**Fix**: Add `.await;` to plugin registration in `src/main.rs`

### Schema Installation Failures

**Issue**: Corrupt ZIP file
```
❌ Bundle not found or corrupt
```

**Fix**: Re-download or regenerate schema bundle

**Issue**: Invalid JSON
```
❌ Production schema is not valid JSON!
```

**Fix**: Check schema file with `python3 -m json.tool`, fix syntax errors

---

## Statistics & Expectations

### Plugin Generation
- **Success Rate**: ~90% with auto-fixes
- **Manual Fixes**: 1-2 plugins per batch
- **Time Per Plugin**:
  - Auto-fixed: <1 minute
  - Manual fix: 2-3 minutes
- **Total Time (8 plugins)**: 5-10 minutes

### Schema Generation
- **Success Rate**: ~98%
- **Manual Fixes**: Rare (JSON syntax only)
- **Time Per Schema**: <1 minute
- **Total Time (8 schemas)**: 2-3 minutes

### Combined Morning Workflow
- **Total Artifacts**: 16 (8 plugins + 8 schemas)
- **Fully Automated**: ~85% (14/16)
- **Need Manual Fix**: ~15% (2/16)
- **Total Time**: 7-15 minutes

---

## Available Tools

### Plugin Management
- `install_plugin.sh <zip>` - Install single plugin
- `install_all_plugins.sh` - Batch install all plugins
- `test_all_plugins.sh` - Test all installed plugins

### Schema Management
- `install_schema.sh <zip>` - Install single schema bundle
- `install_all_schemas.sh` - Batch install all schemas
- `validate_container_spec.sh <domain> <json>` - Validate container config

### Documentation
- `PLUGIN-DEVELOPMENT-GUIDE.md` - Plugin development guide
- `CHATGPT-PLUGIN-SPEC.md` - ChatGPT integration spec
- `CHATGPT-HOURLY-PROMPT.txt` - Prompt template with 60 plugin ideas
- `PLUGIN-STATUS.md` - Current plugin inventory
- `MORNING-WORKFLOW-COMPLETE.md` - This document

---

## Integration with op-dbus

### Using Plugins
```bash
# Query plugin state
./target/release/op-dbus query -p <plugin-name>

# Apply desired state
./target/release/op-dbus apply -p <plugin-name> -f config.json

# Verify state
./target/release/op-dbus verify -p <plugin-name> -f config.json
```

### Using Schemas
Schemas are documentation and validation artifacts. Use them to:
1. Validate LXC container configurations before deployment
2. Generate container configs from templates
3. Migrate legacy LXC configs to production format
4. Understand domain-specific requirements

**Example validation:**
```bash
# Validate before deployment
./validate_container_spec.sh smart-aquarium containers/reef-gateway.json

# If valid, deploy with your LXC tooling
lxc-create -f containers/reef-gateway.json -n reef-gateway
```

---

## Success Metrics

After a successful morning workflow:

✅ **8 New Plugins** operational and tested
✅ **8 New Schema Domains** validated and documented
✅ **Clean Build** with no warnings or errors
✅ **All Tests Passing** across plugins
✅ **Documentation Complete** for all new artifacts

**System Ready** for declarative state management across 16 new domains! 🎉
