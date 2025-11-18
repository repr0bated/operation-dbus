# Plugin Generation Specification

## Your Mission
Generate drop-in plugins for the op-dbus system. Each plugin manages a specific domain of Linux system state (firewall rules, users, cron jobs, etc.).

## What You Must Deliver

A ZIP file containing exactly **3 files**:

1. `<pluginname>_plugin.rs` - The Rust source code
2. `<pluginname>_example.json` - Example configuration
3. `register.sh` - Auto-registration script

## File 1: `<pluginname>_plugin.rs`

### Required Imports
```rust
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Command;

use crate::state::plugin::{
    ApplyResult, Checkpoint, PluginCapabilities, StateAction, StateDiff, StatePlugin, DiffMetadata
};
```

### Structure Template
```rust
// 1. State structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyState {
    pub version: u32,
    pub items: Vec<MyItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyItem {
    pub id: String,
    // ... your fields
}

// 2. Plugin struct
pub struct MyPlugin;

impl MyPlugin {
    pub fn new() -> Self { Self }

    // Helper to query system (use Command, not D-Bus!)
    fn query_system() -> Vec<MyItem> {
        // Parse command output, return structured data
    }
}

// 3. Implement trait
#[async_trait]
impl StatePlugin for MyPlugin {
    fn name(&self) -> &str { "myplugin" }
    fn version(&self) -> &str { "1.0.0" }

    async fn query_current_state(&self) -> Result<Value> {
        let items = Self::query_system();
        Ok(serde_json::to_value(MyState { version: 1, items })?)
    }

    async fn calculate_diff(&self, _current: &Value, desired: &Value) -> Result<StateDiff> {
        let desired_state: MyState = serde_json::from_value(desired.clone())?;
        let current = Self::query_system();

        let mut actions = Vec::new();
        // Calculate diffs, populate actions

        Ok(StateDiff {
            plugin: self.name().to_string(),
            actions,
            metadata: DiffMetadata {
                timestamp: chrono::Utc::now().timestamp(),
                current_hash: format!("{:x}", md5::compute("cur")),
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
                    // Create logic
                }
                StateAction::Modify { resource, changes } => {
                    // Modify logic
                }
                StateAction::Delete { resource } => {
                    // Delete logic
                }
                StateAction::NoOp { resource } => {
                    changes_applied.push(format!("{}: no action", resource));
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

    async fn verify_state(&self, _desired: &Value) -> Result<bool> {
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

### CRITICAL Rules

1. **StateAction has NO `reason` field**
   ```rust
   // WRONG:
   StateAction::NoOp { resource, reason: Some("...") }

   // RIGHT:
   StateAction::NoOp { resource }
   ```

2. **Use Command, NOT D-Bus**
   ```rust
   // RIGHT:
   let output = Command::new("tool").arg("list").output()?;

   // WRONG:
   let conn = zbus::Connection::system().await?;
   ```

3. **Make enums Copy if pattern-matched**
   ```rust
   #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
   pub enum Mode { Enforce, ObserveOnly }
   ```

4. **Handle failures gracefully**
   ```rust
   let Ok(output) = Command::new("tool").output() else {
       return Ok(Vec::new());  // Don't panic!
   };
   ```

## File 2: `<pluginname>_example.json`

Show the JSON schema users will write:

```json
{
  "version": 1,
  "items": [
    {
      "id": "example-1",
      "enabled": true,
      "description": "Example item"
    }
  ]
}
```

## File 3: `register.sh`

**MUST BE EXECUTABLE** (`chmod +x`)

Replace `PLUGIN_NAME` and `PLUGIN_STRUCT` with actual values:

```bash
#!/bin/bash
set -e

PLUGIN_NAME="myplugin"        # lowercase, matches filename
PLUGIN_STRUCT="MyPlugin"      # PascalCase, matches struct name

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=== op-dbus Plugin Registration ==="
echo "Plugin: ${PLUGIN_NAME}"
echo "Struct: ${PLUGIN_STRUCT}"
echo ""

MOD_FILE="src/state/plugins/mod.rs"
echo "→ Updating ${MOD_FILE}..."

if ! grep -q "pub mod ${PLUGIN_NAME};" "${MOD_FILE}"; then
    LINE=$(grep -n "^pub use" "${MOD_FILE}" | head -1 | cut -d: -f1)
    if [ -n "$LINE" ]; then
        sed -i "${LINE}i pub mod ${PLUGIN_NAME};" "${MOD_FILE}"
        echo -e "  ${GREEN}✓${NC} Added module"
    fi
else
    echo -e "  ${YELLOW}→${NC} Module exists"
fi

if ! grep -q "pub use ${PLUGIN_NAME}::${PLUGIN_STRUCT};" "${MOD_FILE}"; then
    echo "pub use ${PLUGIN_NAME}::${PLUGIN_STRUCT};" >> "${MOD_FILE}"
    echo -e "  ${GREEN}✓${NC} Added use"
else
    echo -e "  ${YELLOW}→${NC} Use exists"
fi

MAIN_FILE="src/main.rs"
echo "→ Updating ${MAIN_FILE}..."

if ! grep -q "plugins::${PLUGIN_STRUCT}::new" "${MAIN_FILE}"; then
    LAST_LINE=$(grep -n "\.register_plugin" "${MAIN_FILE}" | tail -1 | cut -d: -f1)
    if [ -n "$LAST_LINE" ]; then
        sed -i "${LAST_LINE}a\\    state_manager\\n        .register_plugin(Box::new(state::plugins::${PLUGIN_STRUCT}::new()))\\n        .await;" "${MAIN_FILE}"
        echo -e "  ${GREEN}✓${NC} Registered"
    fi
else
    echo -e "  ${YELLOW}→${NC} Already registered"
fi

echo ""
echo -e "${GREEN}✓ Complete!${NC}"
echo "Build: cargo build --release"
```

## Naming Conventions

- **Plugin name**: lowercase, no underscores (e.g., `firewall`, `cronman`)
- **Struct name**: PascalCase ending in "Plugin" (e.g., `FirewallPlugin`, `CronManPlugin`)
- **File names**:
  - Source: `<pluginname>_plugin.rs`
  - Example: `<pluginname>_example.json`
  - Script: `register.sh` (same for all plugins)
- **Zip name**: `<pluginname>_pack_YYYYMMDD_HHMM.zip`

## Testing Checklist

Before delivering, verify:

1. ✅ All structs derive `Debug, Clone, Serialize, Deserialize`
2. ✅ Enums that are pattern-matched derive `Copy`
3. ✅ No `reason` field on `StateAction::NoOp`
4. ✅ Uses `Command` not D-Bus for system calls
5. ✅ `PLUGIN_NAME` and `PLUGIN_STRUCT` are correctly set in `register.sh`
6. ✅ Example JSON matches the state structures

## Installation Flow (User Side)

```bash
unzip myplugin_pack_20251031_1200.zip
cp myplugin_plugin.rs src/state/plugins/myplugin.rs
chmod +x register.sh && ./register.sh
cargo build --release
```

Done! Plugin is now usable.

## Example Plugin Ideas

- **firewall**: UFW/iptables rules
- **cronman**: Crontab entry management
- **users**: Local user/group management
- **mounts**: Filesystem mount management
- **packages**: Package installation state
- **sysctl**: Kernel parameter management
- **hosts**: /etc/hosts file entries
- **timers**: Systemd timer management
- **ssh**: SSH key and authorized_keys management
- **selinux**: SELinux policy management

## Reference Implementation

See `src/state/plugins/sessdecl.rs` for a complete, working example that:
- Manages systemd-logind sessions
- Uses `loginctl` command (not D-Bus)
- Implements enforcement modes
- Handles selectors for flexible matching
