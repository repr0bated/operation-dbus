# Plugin Development Guide for op-dbus

This guide provides requirements and best practices for developing plugins for the op-dbus system.

## Overview

Plugins are the core extension mechanism in op-dbus. Each plugin manages a specific domain of system state (sessions, containers, systemd units, etc.) using native protocols.

## Plugin Architecture

### Location
All plugins must be placed in: `src/state/plugins/`

### Core Trait
Every plugin must implement the `StatePlugin` trait from `crate::state::plugin`:

```rust
use crate::state::plugin::{
    StatePlugin, StateDiff, StateAction, ApplyResult,
    Checkpoint, PluginCapabilities, DiffMetadata
};
```

## Required Imports

```rust
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Command;  // For system commands
```

## StateAction Enum

Plugins must work with this fixed enum (cannot add fields):

```rust
pub enum StateAction {
    Create { resource: String, config: Value },
    Modify { resource: String, changes: Value },
    Delete { resource: String },
    NoOp { resource: String },  // NOTE: No 'reason' field!
}
```

## Plugin Structure Template

```rust
// 1. Define your state structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyPluginState {
    pub items: Vec<MyItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyItem {
    pub id: String,
    // ... other fields
}

// 2. Create the plugin struct
pub struct MyPlugin;

impl MyPlugin {
    pub fn new() -> Self { Self }

    // Helper methods (can be private or public)
    fn query_system() -> Vec<MyItem> {
        // Use Command::new() for system calls
        // Parse output and return structured data
    }
}

// 3. Implement StatePlugin trait
#[async_trait]
impl StatePlugin for MyPlugin {
    fn name(&self) -> &str { "myplugin" }
    fn version(&self) -> &str { "1.0.0" }

    async fn query_current_state(&self) -> Result<Value> {
        // Query actual system state
        let items = Self::query_system();
        Ok(serde_json::to_value(MyPluginState { items })?)
    }

    async fn calculate_diff(&self, _current: &Value, desired: &Value) -> Result<StateDiff> {
        // Parse desired state
        let desired_state: MyPluginState = serde_json::from_value(desired.clone())?;

        // Query current state
        let current_items = Self::query_system();

        // Calculate actions needed
        let mut actions = Vec::new();
        // ... diff logic here

        Ok(StateDiff {
            plugin: self.name().to_string(),
            actions,
            metadata: DiffMetadata {
                timestamp: chrono::Utc::now().timestamp(),
                current_hash: format!("{:x}", md5::compute("current-data")),
                desired_hash: format!("{:x}", md5::compute(serde_json::to_string(&desired)?)),
            },
        })
    }

    async fn apply_state(&self, diff: &StateDiff) -> Result<ApplyResult> {
        let mut changes_applied = Vec::new();
        let mut errors = Vec::new();

        for action in &diff.actions {
            match action {
                StateAction::Create { resource, config } => {
                    // Implementation
                }
                StateAction::Modify { resource, changes } => {
                    // Implementation
                }
                StateAction::Delete { resource } => {
                    // Implementation
                }
                StateAction::NoOp { resource } => {
                    changes_applied.push(format!("{}: no action required", resource));
                }
            }
        }

        Ok(ApplyResult {
            success: errors.is_empty(),
            changes_applied,
            errors,
            checkpoint: None,
        })
    }

    async fn verify_state(&self, desired: &Value) -> Result<bool> {
        // Verify current state matches desired
        Ok(true)
    }

    async fn create_checkpoint(&self) -> Result<Checkpoint> {
        Ok(Checkpoint {
            id: format!("{}-{}", self.name(), chrono::Utc::now().timestamp()),
            plugin: self.name().to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            state_snapshot: serde_json::json!({}),
            backend_checkpoint: None,
        })
    }

    async fn rollback(&self, _checkpoint: &Checkpoint) -> Result<()> {
        Ok(())
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            supports_rollback: false,
            supports_checkpoints: true,
            supports_verification: true,
            atomic_operations: false,
        }
    }
}
```

## Integration Steps

1. **Add plugin file**: Create `src/state/plugins/myplugin.rs`

2. **Export in mod.rs**: Edit `src/state/plugins/mod.rs`:
```rust
pub mod myplugin;
pub use myplugin::MyPlugin;
```

3. **Register in main.rs**: Add to `src/main.rs` in the `main()` function:
```rust
state_manager
    .register_plugin(Box::new(state::plugins::MyPlugin::new()))
    .await;
```

## Best Practices

### 1. Use System Commands, Not D-Bus Calls
```rust
// GOOD: Use Command for system interaction
let output = Command::new("loginctl")
    .arg("list-sessions")
    .output()?;

// AVOID: Direct D-Bus calls in plugin methods
// (D-Bus is used internally by StateManager, not plugins)
```

### 2. Parse Command Output Carefully
```rust
fn parse_output(output: &str) -> Vec<MyItem> {
    let mut results = Vec::new();
    for line in output.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 2 {
            results.push(MyItem {
                id: cols[0].to_string(),
                // ...
            });
        }
    }
    results
}
```

### 3. Handle Errors Gracefully
```rust
// Don't panic - return errors or empty results
let Ok(output) = Command::new("tool").output() else {
    return Ok(Vec::new());  // Fallback to empty state
};

if !output.status.success() {
    return Ok(Vec::new());
}
```

### 4. Make Enums Copyable When Needed
```rust
// If you pattern match on enums, derive Copy
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Mode { Enforce, ObserveOnly }
```

### 5. Document State Schema
```rust
/// Plugin state structure for MyPlugin
///
/// # Example JSON
/// ```json
/// {
///   "version": 1,
///   "items": [
///     {
///       "id": "example-1",
///       "enabled": true
///     }
///   ]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyPluginState {
    pub version: u32,
    pub items: Vec<MyItem>,
}
```

## Common Patterns

### Selector Pattern (for matching resources)
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Selector {
    pub user: Option<String>,
    pub name: Option<String>,
    // Optional fields allow flexible matching
}

fn matches(selector: &Selector, item: &Item) -> bool {
    if let Some(u) = &selector.user {
        if &item.user != u { return false; }
    }
    if let Some(n) = &selector.name {
        if &item.name != n { return false; }
    }
    true
}
```

### Enforcement Modes
```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Enforce,      // Take action
    ObserveOnly,  // Just report
}
```

## Testing Your Plugin

### 1. Build
```bash
cargo build --release
```

### 2. Query Plugin State
```bash
./target/release/op-dbus query --plugin myplugin
```

### 3. Create Example Config
Create `examples/myplugin_example.json`:
```json
{
  "version": 1,
  "items": [
    {
      "id": "test-1",
      "enabled": true
    }
  ]
}
```

### 4. Test with Full State File
Create `/tmp/test-state.json`:
```json
{
  "version": 1,
  "plugins": {
    "myplugin": {
      "version": 1,
      "items": [...]
    }
  }
}
```

Then test:
```bash
# Dry run (show what would change)
./target/release/op-dbus apply /tmp/test-state.json --dry-run --plugin myplugin

# Actually apply
./target/release/op-dbus apply /tmp/test-state.json --plugin myplugin
```

## Plugin Ideas

Here are some plugin ideas that would fit the architecture:

1. **Firewall Plugin** (`ufw`, `iptables`, `nftables`)
2. **User Plugin** (manage local users/groups)
3. **Cron Plugin** (manage crontab entries)
4. **Mount Plugin** (manage filesystem mounts)
5. **Package Plugin** (manage installed packages)
6. **Service Timer Plugin** (systemd timers)
7. **Sysctl Plugin** (kernel parameters)
8. **Hostname Plugin** (system hostname/hosts file)
9. **Time Plugin** (timezone, NTP configuration)
10. **SSH Plugin** (SSH keys, authorized_keys)

## Deliverable Format

When ChatGPT generates a plugin, provide **THREE** files packaged in a zip:

### 1. `<pluginname>_plugin.rs`
The plugin source code implementing StatePlugin trait.

### 2. `<pluginname>_example.json`
Example configuration showing the JSON schema that users can use as a template.

### 3. `register.sh`
**CRITICAL FOR DROP-IN SUPPORT**: Auto-registration script with these exact values:

```bash
#!/bin/bash
# Drop-in plugin registration script
# This script auto-registers a plugin with op-dbus

set -e

PLUGIN_NAME="<pluginname>"           # Must match: lowercase, no underscores
PLUGIN_STRUCT="<PluginStruct>"       # Must match: PascalCase struct name

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=== op-dbus Plugin Registration ==="
echo "Plugin: ${PLUGIN_NAME}"
echo "Struct: ${PLUGIN_STRUCT}"
echo ""

# 1. Update src/state/plugins/mod.rs
MOD_FILE="src/state/plugins/mod.rs"
echo "→ Updating ${MOD_FILE}..."

# Add module declaration (before "pub use" statements)
if ! grep -q "pub mod ${PLUGIN_NAME};" "${MOD_FILE}"; then
    LINE=$(grep -n "^pub use" "${MOD_FILE}" | head -1 | cut -d: -f1)
    if [ -n "$LINE" ]; then
        sed -i "${LINE}i pub mod ${PLUGIN_NAME};" "${MOD_FILE}"
        echo -e "  ${GREEN}✓${NC} Added module declaration"
    fi
else
    echo -e "  ${YELLOW}→${NC} Module already declared"
fi

# Add use statement
if ! grep -q "pub use ${PLUGIN_NAME}::${PLUGIN_STRUCT};" "${MOD_FILE}"; then
    echo "pub use ${PLUGIN_NAME}::${PLUGIN_STRUCT};" >> "${MOD_FILE}"
    echo -e "  ${GREEN}✓${NC} Added use statement"
else
    echo -e "  ${YELLOW}→${NC} Use statement already exists"
fi

# 2. Update src/main.rs
MAIN_FILE="src/main.rs"
echo "→ Updating ${MAIN_FILE}..."

if ! grep -q "plugins::${PLUGIN_STRUCT}::new" "${MAIN_FILE}"; then
    LAST_LINE=$(grep -n "\.register_plugin" "${MAIN_FILE}" | tail -1 | cut -d: -f1)

    if [ -n "$LAST_LINE" ]; then
        sed -i "${LAST_LINE}a\\    state_manager\\n        .register_plugin(Box::new(state::plugins::${PLUGIN_STRUCT}::new()))\\n        .await;" "${MAIN_FILE}"
        echo -e "  ${GREEN}✓${NC} Registered plugin in main.rs"
    fi
else
    echo -e "  ${YELLOW}→${NC} Plugin already registered"
fi

echo ""
echo -e "${GREEN}✓ Registration complete!${NC}"
echo ""
echo "Next steps:"
echo "  1. cargo build --release"
echo "  2. ./target/release/op-dbus query --plugin ${PLUGIN_NAME}"
```

### Package Format
**Filename**: `<pluginname>_pack_YYYYMMDD_HHMM.zip`

**Example**: `firewall_pack_20251031_1430.zip`

**Contents**:
```
firewall_pack_20251031_1430.zip
├── firewall_plugin.rs      (source code)
├── firewall_example.json   (example config)
└── register.sh             (registration script)
```

## Installation Instructions (For End Users)

Once ChatGPT provides the zip file, installation is 3 commands:

```bash
# 1. Extract zip in project root
unzip firewall_pack_20251031_1430.zip

# 2. Copy plugin and register
cp firewall_plugin.rs src/state/plugins/firewall.rs
chmod +x register.sh && ./register.sh

# 3. Build
cargo build --release
```

That's it! The plugin is now integrated.

## Questions?

Reference the existing plugins for working examples:
- `src/state/plugins/sessdecl.rs` - Session management
- `src/state/plugins/lxc.rs` - Container management
- `src/state/plugins/systemd.rs` - Systemd units
- `src/state/plugins/net.rs` - Network interfaces
