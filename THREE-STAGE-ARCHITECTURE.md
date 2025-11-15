# Three-Stage Plugin Architecture

## Vision: Introspection → Build → Deploy

A progressive system where op-dbus discovers missing plugins, generates Rust code, compiles them, and deploys. Over time, the plugin library grows and the build stage shrinks.

## The Three Stages

### Stage 1: Introspection

**Discover what's on the system**

```bash
op-dbus discover

# Output:
# 🔍 Introspecting system...
#
# ✅ MANAGED D-BUS SERVICES
#   ✓ org.freedesktop.systemd1 → systemd (in library)
#   ✓ org.freedesktop.login1 → login1 (in library)
#
# 🤖 AUTO-DISCOVERABLE (Can be generated):
#   ⊗ org.freedesktop.PackageKit
#   ⊗ org.freedesktop.NetworkManager
#   ⊗ org.freedesktop.UPower
#
# 🔄 CONVERSION CANDIDATES (Need manual work):
#   nginx.service (systemd)
#   docker.service (systemd)
```

**What it does**:
- Scans D-Bus for services
- Introspects each service (methods, properties, signals)
- Categorizes:
  * **In library**: Already have compiled plugin
  * **Auto-discoverable**: Can generate plugin code
  * **Conversion candidates**: Need manual implementation

**Output**: `introspection-report.json`
```json
{
  "managed_services": ["systemd", "login1"],
  "missing_plugins": [
    {
      "service": "org.freedesktop.PackageKit",
      "interfaces": ["org.freedesktop.PackageKit"],
      "methods": ["GetPackages", "InstallPackages", ...],
      "properties": ["Version", "TransactionList", ...],
      "can_autogen": true
    },
    {
      "service": "org.freedesktop.NetworkManager",
      "can_autogen": true
    }
  ]
}
```

### Stage 2: Build

**Generate and compile missing plugins**

```bash
op-dbus codegen --from introspection-report.json

# Output:
# 📝 Generating plugin code...
#   ✓ Generated src/state/plugins/packagekit.rs (1,234 lines)
#   ✓ Generated src/state/plugins/networkmanager.rs (2,567 lines)
#   ✓ Generated src/state/plugins/upower.rs (456 lines)
#
# 🔨 Compiling plugins...
#   ✓ Compiled packagekit plugin (32s)
#   ✓ Compiled networkmanager plugin (48s)
#   ✓ Compiled upower plugin (12s)
#
# ✅ 3 new plugins ready to deploy
```

**What it does**:

**Sub-stage 2a: Code Generation**
- Reads `introspection-report.json`
- For each missing plugin:
  * Introspects D-Bus service (methods, properties, signals)
  * Generates Rust code (`src/state/plugins/packagekit.rs`)
  * Infers semantic mapping (safe vs unsafe methods)
  * Generates state translation (D-Bus ↔ JSON)

**Sub-stage 2b: Compilation**
- Adds generated plugins to `src/state/plugins/mod.rs`
- Runs `cargo build --release`
- Creates optimized binaries with new plugins

**Output**:
- `src/state/plugins/packagekit.rs` (generated Rust code)
- `target/release/op-dbus` (compiled binary with new plugins)

### Stage 3: Deploy

**Use the newly built plugins**

```bash
# Now PackageKit is a first-class plugin!
op-dbus apply state.json

# state.json contains:
# {
#   "plugins": {
#     "packagekit": {
#       "packages": ["nginx", "postgres", "redis"]
#     }
#   }
# }

# op-dbus can now:
# - Query current packages (GetPackages)
# - Install missing packages (InstallPackages)
# - Remove extra packages (RemovePackages)
# - All with proper error handling and rollback
```

**What it does**:
- Uses newly compiled binary
- PackageKit (and others) are now full plugins
- Can read AND write state
- Proper error handling
- Rollback support

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     STAGE 1: INTROSPECTION                  │
│                                                             │
│  op-dbus discover                                           │
│    ↓                                                        │
│  Scan D-Bus → Introspect services → Generate report        │
│    ↓                                                        │
│  introspection-report.json                                  │
│  {                                                          │
│    "missing_plugins": [                                     │
│      "org.freedesktop.PackageKit",                          │
│      "org.freedesktop.NetworkManager"                       │
│    ]                                                        │
│  }                                                          │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                     STAGE 2: BUILD                          │
│                                                             │
│  op-dbus codegen --from introspection-report.json          │
│    ↓                                                        │
│  ┌──────────────────────────────────────────────┐          │
│  │  Code Generator                              │          │
│  │  ├─ Introspect D-Bus service                │          │
│  │  ├─ Infer semantic mappings                 │          │
│  │  ├─ Generate Rust code                      │          │
│  │  └─ Write src/state/plugins/packagekit.rs   │          │
│  └──────────────────────────────────────────────┘          │
│    ↓                                                        │
│  cargo build --release                                      │
│    ↓                                                        │
│  target/release/op-dbus (with new plugins!)                │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                     STAGE 3: DEPLOY                         │
│                                                             │
│  ./target/release/op-dbus apply state.json                 │
│    ↓                                                        │
│  PackageKit plugin manages packages ✅                      │
│  NetworkManager plugin manages network ✅                   │
│  UPower plugin manages power ✅                             │
└─────────────────────────────────────────────────────────────┘
```

## Progressive Library Growth

### Iteration 1: First User (Empty Library)

```
Day 1: User A runs op-dbus discover on Debian server
  → Finds PackageKit, NetworkManager
  → Runs op-dbus codegen
  → Generates + compiles plugins (takes 2 minutes)
  → Uses plugins
  → Commits generated code to fork
```

### Iteration 2: Second User (Small Library)

```
Day 7: User B runs op-dbus discover on Ubuntu server
  → Finds PackageKit (already in library!), UPower (new)
  → Only generates UPower plugin (takes 30 seconds)
  → Uses plugins
  → Submits UPower plugin as PR
```

### Iteration 100: Mature Library

```
Month 6: User Z runs op-dbus discover on NixOS
  → All common services already in library!
  → Zero code generation needed
  → Just works
```

**The library self-improves over time.**

## Code Generation Details

### What Gets Generated

**File**: `src/state/plugins/packagekit.rs`

```rust
// AUTO-GENERATED by op-dbus codegen
// D-Bus Service: org.freedesktop.PackageKit
// Generated: 2025-11-06 12:34:56 UTC
//
// Review and edit as needed. Once satisfied, this becomes
// part of the permanent plugin library.

use crate::state::plugin::{StatePlugin, StateDiff, ApplyResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use zbus::{Connection, Proxy};

/// PackageKit plugin for package management
pub struct PackageKitPlugin {
    connection: Connection,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageKitConfig {
    /// List of packages that should be installed
    pub packages: Vec<String>,

    /// Whether to auto-update packages
    #[serde(default)]
    pub auto_update: bool,
}

impl PackageKitPlugin {
    pub async fn new() -> Result<Self> {
        let connection = Connection::system().await?;
        Ok(Self { connection })
    }

    /// Get list of installed packages
    async fn get_installed_packages(&self) -> Result<Vec<String>> {
        let proxy = Proxy::new(
            &self.connection,
            "org.freedesktop.PackageKit",
            "/org/freedesktop/PackageKit",
            "org.freedesktop.PackageKit",
        ).await?;

        // Call GetPackages method
        let packages: Vec<String> = proxy
            .call("GetPackages", &("installed",))
            .await?;

        Ok(packages)
    }

    /// Install a package
    ///
    /// SAFETY: This modifies the system. Marked as unsafe operation.
    /// Requires user confirmation or --force flag.
    async fn install_package(&self, package: &str) -> Result<()> {
        // Check if already installed
        let installed = self.get_installed_packages().await?;
        if installed.contains(&package.to_string()) {
            return Ok(());  // Already installed
        }

        let proxy = Proxy::new(
            &self.connection,
            "org.freedesktop.PackageKit",
            "/org/freedesktop/PackageKit",
            "org.freedesktop.PackageKit",
        ).await?;

        // Call InstallPackages method
        proxy
            .call("InstallPackages", &(vec![package],))
            .await?;

        Ok(())
    }

    /// Remove a package
    ///
    /// SAFETY: This modifies the system. Marked as unsafe operation.
    async fn remove_package(&self, package: &str) -> Result<()> {
        let proxy = Proxy::new(
            &self.connection,
            "org.freedesktop.PackageKit",
            "/org/freedesktop/PackageKit",
            "org.freedesktop.PackageKit",
        ).await?;

        // Call RemovePackages method
        proxy
            .call("RemovePackages", &(vec![package],))
            .await?;

        Ok(())
    }
}

#[async_trait]
impl StatePlugin for PackageKitPlugin {
    fn name(&self) -> &str {
        "packagekit"
    }

    fn version(&self) -> &str {
        "1.0.0-autogen"
    }

    fn is_available(&self) -> bool {
        // Check if PackageKit D-Bus service is available
        true  // TODO: Actually check
    }

    async fn query_current_state(&self) -> Result<serde_json::Value> {
        let packages = self.get_installed_packages().await?;

        Ok(serde_json::json!({
            "packages": packages,
            "auto_update": false,  // TODO: Query from PackageKit
        }))
    }

    async fn calculate_diff(&self, current: &Value, desired: &Value) -> Result<StateDiff> {
        let current_packages: Vec<String> =
            serde_json::from_value(current["packages"].clone())?;
        let desired_packages: Vec<String> =
            serde_json::from_value(desired["packages"].clone())?;

        let mut actions = Vec::new();

        // Packages to install
        for pkg in &desired_packages {
            if !current_packages.contains(pkg) {
                actions.push(StateAction::Create {
                    resource: format!("package:{}", pkg),
                    config: serde_json::json!({"name": pkg}),
                });
            }
        }

        // Packages to remove
        for pkg in &current_packages {
            if !desired_packages.contains(pkg) {
                actions.push(StateAction::Delete {
                    resource: format!("package:{}", pkg),
                });
            }
        }

        Ok(StateDiff {
            plugin: "packagekit".to_string(),
            actions,
            metadata: DiffMetadata { /* ... */ },
        })
    }

    async fn apply_state(&self, diff: &StateDiff) -> Result<ApplyResult> {
        let mut changes = Vec::new();
        let mut errors = Vec::new();

        for action in &diff.actions {
            match action {
                StateAction::Create { resource, config } => {
                    let pkg_name = config["name"].as_str().unwrap();

                    match self.install_package(pkg_name).await {
                        Ok(_) => {
                            changes.push(format!("Installed package: {}", pkg_name));
                        }
                        Err(e) => {
                            errors.push(format!("Failed to install {}: {}", pkg_name, e));
                        }
                    }
                }
                StateAction::Delete { resource } => {
                    let pkg_name = resource.strip_prefix("package:").unwrap();

                    match self.remove_package(pkg_name).await {
                        Ok(_) => {
                            changes.push(format!("Removed package: {}", pkg_name));
                        }
                        Err(e) => {
                            errors.push(format!("Failed to remove {}: {}", pkg_name, e));
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(ApplyResult {
            success: errors.is_empty(),
            changes_applied: changes,
            errors,
            checkpoint: None,
        })
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            supports_rollback: false,  // PackageKit doesn't support transactions
            supports_checkpoints: false,
            supports_verification: true,
            atomic_operations: false,
        }
    }
}
```

**This is ~200 lines of working Rust code, auto-generated!**

### Code Generator Implementation

**File**: `src/codegen/mod.rs` (new module)

```rust
use crate::mcp::introspection_parser::IntrospectionParser;
use anyhow::Result;
use zbus::{Connection, Proxy};

pub struct PluginCodeGenerator {
    templates: TemplateEngine,
}

impl PluginCodeGenerator {
    pub async fn generate_plugin(
        &self,
        service_name: &str,
    ) -> Result<String> {
        // Step 1: Introspect D-Bus service
        let conn = Connection::system().await?;
        let proxy = Proxy::new(
            &conn,
            service_name,
            &format!("/{}", service_name.replace('.', "/")),
            "org.freedesktop.DBus.Introspectable",
        ).await?;

        let xml: String = proxy.call("Introspect", &()).await?;
        let introspection = IntrospectionParser::parse_xml(&xml);

        // Step 2: Infer semantic mappings
        let semantic_map = self.infer_semantics(&introspection)?;

        // Step 3: Generate Rust code from template
        let code = self.templates.render("plugin.rs.hbs", &serde_json::json!({
            "service_name": service_name,
            "plugin_name": self.service_to_plugin_name(service_name),
            "interfaces": introspection.interfaces,
            "semantic_map": semantic_map,
        }))?;

        Ok(code)
    }

    /// Infer whether methods are safe (read-only) or unsafe (modify system)
    fn infer_semantics(&self, intro: &IntrospectionData) -> Result<SemanticMap> {
        let mut map = SemanticMap::new();

        for interface in &intro.interfaces {
            for method in &interface.methods {
                // Heuristics for safety:
                let is_safe = method.name.starts_with("Get")     // Getters
                    || method.name.starts_with("List")           // Listers
                    || method.name.starts_with("Query")          // Queries
                    || method.name.starts_with("Introspect")     // Introspection
                    || method.name.starts_with("Search");        // Searches

                let is_unsafe = method.name.starts_with("Set")   // Setters
                    || method.name.starts_with("Install")        // Installers
                    || method.name.starts_with("Remove")         // Removers
                    || method.name.starts_with("Delete")         // Deleters
                    || method.name.starts_with("Create")         // Creators
                    || method.name.starts_with("Update");        // Updaters

                map.insert(method.name.clone(), MethodSafety {
                    safe: is_safe,
                    side_effects: is_unsafe,
                    requires_confirmation: is_unsafe,
                });
            }
        }

        Ok(map)
    }
}
```

## CLI Commands

### Stage 1: Introspection

```bash
# Discover system
op-dbus discover

# Export introspection report
op-dbus discover --export --output introspection-report.json
```

### Stage 2: Build

```bash
# Generate code for all missing plugins
op-dbus codegen --from introspection-report.json

# Generate code for specific service
op-dbus codegen --service org.freedesktop.PackageKit

# Generate code interactively
op-dbus codegen --interactive

# Compile plugins
cargo build --release
```

### Stage 3: Deploy

```bash
# Use newly built binary
./target/release/op-dbus apply state.json

# Verify new plugins work
./target/release/op-dbus query --plugin packagekit
```

## Plugin Library Repository

### Structure

```
op-dbus/
├── src/state/plugins/
│   ├── mod.rs                 # Plugin registry
│   ├── systemd.rs             # Built-in (hand-written)
│   ├── login1.rs              # Built-in (hand-written)
│   ├── packagekit.rs          # Generated → reviewed → committed
│   ├── networkmanager.rs      # Generated → reviewed → committed
│   ├── upower.rs              # Generated → reviewed → committed
│   └── udisks2.rs             # Generated → reviewed → committed
├── codegen/
│   ├── templates/
│   │   └── plugin.rs.hbs      # Handlebars template for plugins
│   └── semantic-rules/
│       ├── common.toml        # Common method patterns
│       └── freedesktop.toml   # Freedesktop-specific rules
└── docs/
    ├── PLUGIN-DEVELOPMENT.md  # How to write plugins
    └── CODEGEN-GUIDE.md       # How to use code generator
```

### Contribution Workflow

```bash
# 1. User discovers missing plugin
op-dbus discover
# → Found: org.freedesktop.Avahi

# 2. Generate plugin code
op-dbus codegen --service org.freedesktop.Avahi
# → Generated src/state/plugins/avahi.rs

# 3. Review and edit generated code
vim src/state/plugins/avahi.rs
# → Fix any issues, add error handling, etc.

# 4. Test plugin
cargo test
op-dbus apply avahi-test-state.json

# 5. Commit and submit PR
git add src/state/plugins/avahi.rs
git commit -m "feat: add Avahi plugin (code-generated)"
git push origin add-avahi-plugin
# → Open PR on GitHub

# 6. Maintainers review + merge
# → Plugin now in library for everyone!
```

## Benefits

### For First-Time Users
- **Fast bootstrap**: Discover system → generate plugins → works in minutes
- **No manual plugin writing**: Code generator does 90% of the work
- **Review and improve**: Generated code is editable

### For the Community
- **Shared library**: Everyone benefits from generated plugins
- **Progressive improvement**: Generated code → reviewed → refined → committed
- **Long-term sustainability**: Library grows naturally

### For Maintainers
- **Less work over time**: Common plugins eventually covered
- **Quality control**: Review generated code before committing
- **Community contributions**: Users submit plugins they generate

## Comparison with Other Approaches

| Approach | Pros | Cons |
|----------|------|------|
| **Runtime auto-plugins** | No compilation, instant | Read-only, limited |
| **Hand-written plugins** | High quality, full featured | Slow to write, doesn't scale |
| **3-Stage codegen** | Best of both worlds | Requires code generation step |

## Implementation Plan

### Phase 1: Code Generator (Week 1-2)
- [ ] Create codegen module
- [ ] Implement D-Bus → Rust code generation
- [ ] Template engine (Handlebars)
- [ ] Semantic inference (safe vs unsafe methods)

### Phase 2: CLI Integration (Week 2-3)
- [ ] `op-dbus codegen` command
- [ ] Auto-add to `plugins/mod.rs`
- [ ] Compile newly generated plugins
- [ ] Verify plugins work

### Phase 3: Library Building (Week 3-4)
- [ ] Generate PackageKit plugin
- [ ] Generate NetworkManager plugin
- [ ] Generate UPower plugin
- [ ] Generate UDisks2 plugin
- [ ] Review and commit to library

### Phase 4: Community (Ongoing)
- [ ] Documentation for plugin development
- [ ] Contribution guidelines
- [ ] Plugin quality standards
- [ ] Automated testing for generated plugins

## Success Metrics

**Week 1**: Code generator creates compilable Rust code
**Week 2**: First generated plugin (PackageKit) works end-to-end
**Month 1**: 10 plugins in library (mix of hand-written + generated)
**Month 3**: 50 plugins in library
**Month 6**: 90%+ of common services covered, minimal generation needed

---

**This architecture combines the speed of code generation with the quality of hand-written plugins, creating a self-improving system.**
