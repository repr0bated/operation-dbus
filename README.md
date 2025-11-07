# op-dbus - Portable System State Management

**Declarative system state management via native protocols - runs on any Linux system**

op-dbus is a portable tool for declarative infrastructure management that automatically adapts to your system's capabilities.

---

## 🆕 Fresh NixOS Installation?

**Just wiped your drive and installed NixOS?** → Read **[START-HERE.md](START-HERE.md)** first!

```bash
# Get git, pull latest code, and bootstrap
nix-shell -p git
git pull origin claude/this-is-no-011CUr6iUZZC7YKhhLSY2MK6
./bootstrap.sh
```

Or just read the quick start: `cat START-HERE.md | less`

---

## ✨ Key Features

- **🔌 Dynamic Plugin Discovery** - Automatically detects and uses available system components
- **📋 Declarative State** - Define your desired system state in JSON
- **🔐 Blockchain Audit Trail** - Immutable log of all system changes (SHA-256 cryptographic footprints)
- **🚀 Native Protocols** - Direct OVSDB, Netlink, and D-Bus (no CLI wrappers)
- **💻 Truly Portable** - Works on any Linux system with systemd
- **🔍 Zero Config** - Introspects current state automatically

## 🎯 Works Everywhere

op-dbus dynamically discovers what's available on your system:

| Component | If Available | If Not Available |
|-----------|-------------|------------------|
| **OpenVSwitch** | Full network management via OVSDB | Gracefully skipped |
| **Proxmox VE** | Container orchestration (LXC) | Gracefully skipped |
| **systemd** | Service management | Always required |
| **D-Bus** | System introspection | Always required |

**The tool adapts to your system** - install once, works anywhere!

## 📦 Quick Start

### Cargo (Traditional)

#### 1. Build

```bash
cargo build --release
```

#### 2. Install

```bash
sudo ./install-portable.sh
```

The installer will:
- ✅ Check system requirements (systemd, D-Bus)
- ✅ Install binary to `/usr/local/bin/op-dbus`
- ✅ Introspect your current system state
- ✅ Create `/etc/op-dbus/state.json` with discovered configuration
- ✅ Set up systemd service (disabled by default)

### Nix / NixOS (Recommended)

#### With Flakes

```bash
# Try without installing
nix run github:repr0bated/operation-dbus -- query

# Install to user profile
nix profile install github:repr0bated/operation-dbus
```

#### NixOS Configuration

Add to `/etc/nixos/configuration.nix`:

```nix
{
  services.op-dbus = {
    enable = true;
    state = {
      version = 1;
      plugins = {
        systemd = {
          units = {
            "nginx.service" = {
              active_state = "active";
              enabled = true;
            };
          };
        };
      };
    };
  };
}
```

**See [NIX-INSTALL.md](NIX-INSTALL.md) for complete Nix documentation.**

### 3. Test

```bash
# View current system state
op-dbus query

# View available plugins
op-dbus query | jq '.plugins | keys'

# Run system diagnostics
op-dbus doctor
```

### 4. Manage State

```bash
# Edit desired state
sudo nano /etc/op-dbus/state.json

# Preview what would change
op-dbus diff /etc/op-dbus/state.json

# Apply changes (creates blockchain footprint)
sudo op-dbus apply /etc/op-dbus/state.json

# View change history
op-dbus blockchain list
```

## 🔌 Available Plugins

Plugins are **automatically discovered** when op-dbus starts:

### Core Plugins (Always Available)

- **systemd** - Service management (start/stop/enable/disable)
- **login1** - Session management (D-Bus)

### Optional Plugins (Auto-detected)

- **net** - Network management (requires OpenVSwitch)
  - OVS bridge management via OVSDB JSON-RPC
  - IP address configuration via rtnetlink
  - Route management

- **lxc** - Container orchestration (requires Proxmox VE)
  - LXC container lifecycle
  - Network attachment
  - Automatic OVS port cleanup

At startup, you'll see:
```
INFO Discovering available plugins...
INFO ✓ Registering plugin: systemd
INFO ✓ Registering plugin: login1
INFO ⊗ Skipping plugin: net - OpenVSwitch (ovs-vsctl) not found
INFO ⊗ Skipping plugin: lxc - Proxmox pct command not found
```

## 📝 Example State Files

### Minimal (Any Linux System)

```json
{
  "version": 1,
  "plugins": {
    "systemd": {
      "units": {
        "nginx.service": {
          "active_state": "active",
          "enabled": true
        },
        "postgresql.service": {
          "active_state": "active",
          "enabled": true
        }
      }
    }
  }
}
```

### With OpenVSwitch

```json
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": [{
        "name": "br0",
        "type": "ovs-bridge",
        "ports": ["eth0"],
        "ipv4": {
          "enabled": true,
          "dhcp": false,
          "address": [{"ip": "192.168.1.10", "prefix": 24}],
          "gateway": "192.168.1.1"
        }
      }]
    },
    "systemd": {
      "units": {
        "openvswitch-switch.service": {
          "active_state": "active",
          "enabled": true
        }
      }
    }
  }
}
```

### With Proxmox (Full Stack)

```json
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": [{
        "name": "mesh",
        "type": "ovs-bridge"
      }]
    },
    "lxc": {
      "containers": [{
        "id": "100",
        "veth": "vi100",
        "bridge": "mesh",
        "running": true
      }]
    },
    "systemd": {
      "units": {
        "openvswitch-switch.service": {
          "active_state": "active",
          "enabled": true
        }
      }
    }
  }
}
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────┐
│ op-dbus CLI                                     │
│ - query, apply, diff, verify                    │
└─────────────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────┐
│ Dynamic Plugin Discovery                        │
│ - Checks system for available components        │
│ - Registers only compatible plugins             │
└─────────────────────────────────────────────────┘
                     ↓
┌──────────────┬──────────────┬──────────────────┐
│ systemd      │ net (OVS)    │ lxc (Proxmox)    │
│ Always       │ Optional     │ Optional         │
└──────────────┴──────────────┴──────────────────┘
                     ↓
┌─────────────────────────────────────────────────┐
│ Native Protocols                                │
│ - D-Bus (zbus)                                  │
│ - OVSDB JSON-RPC (Unix socket)                  │
│ - Netlink (rtnetlink)                           │
└─────────────────────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────┐
│ Blockchain Audit Log                            │
│ - SHA-256 cryptographic footprints              │
│ - Immutable change history                      │
│ - Optional ML vectorization                     │
└─────────────────────────────────────────────────┘
```

## 🔐 Blockchain Audit Trail

Every `op-dbus apply` creates an immutable blockchain record:

```bash
# View audit trail
op-dbus blockchain list

# Search for changes
op-dbus blockchain search "nginx"

# Verify integrity
op-dbus blockchain verify --full

# Export for compliance
op-dbus blockchain export > audit-trail.json
```

Each block contains:
- **Timestamp** - When the change occurred
- **Plugin** - Which system component changed
- **Hash** - SHA-256 cryptographic footprint
- **Diff** - What changed (before/after)

## 🚀 Usage Examples

### Scenario 1: Manage Services (Any Linux)

```bash
# View current services
op-dbus query --plugin systemd

# Enable nginx
echo '{"version":1,"plugins":{"systemd":{"units":{"nginx.service":{"active_state":"active","enabled":true}}}}}' \
  | sudo tee /etc/op-dbus/state.json

# Apply
sudo op-dbus apply /etc/op-dbus/state.json
```

### Scenario 2: Network Management (With OVS)

```bash
# Create bridge
cat > /tmp/network.json <<EOF
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": [{
        "name": "br0",
        "type": "ovs-bridge",
        "ports": ["eth0"]
      }]
    }
  }
}
EOF

# Apply
sudo op-dbus apply /tmp/network.json
```

### Scenario 3: Infrastructure as Code

```bash
# Store state in Git
git init /etc/op-dbus
cd /etc/op-dbus
git add state.json
git commit -m "Initial infrastructure state"

# Make changes
vim state.json
git commit -am "Enable monitoring stack"

# Apply
sudo op-dbus apply state.json
```

## 📊 Commands

### Query

```bash
op-dbus query                    # All plugins
op-dbus query --plugin net       # Specific plugin
op-dbus query --plugin systemd   # Another plugin
```

### Apply

```bash
op-dbus apply state.json                # Apply all plugins
op-dbus apply state.json --plugin net   # Apply only net plugin
op-dbus apply state.json --dry-run      # Preview without applying
```

### Diff

```bash
op-dbus diff state.json                 # Show all differences
op-dbus diff state.json --plugin net    # Show net differences only
```

### Blockchain

```bash
op-dbus blockchain list                 # List all blocks
op-dbus blockchain show <hash>          # Show specific block
op-dbus blockchain export               # Export entire chain
op-dbus blockchain verify --full        # Verify integrity
op-dbus blockchain search "nginx"       # Search for changes
```

### Diagnostics

```bash
op-dbus doctor                          # System health check
op-dbus version --verbose               # Version info
op-dbus introspect                      # Show all databases
```

## 🛠️ System Requirements

### Required

- **Linux kernel** 4.4+ (for netlink)
- **systemd** (service management)
- **D-Bus** (system introspection)
- **Rust** 1.70+ (for building)

### Optional (Auto-detected)

- **OpenVSwitch** 2.9+ (for network management)
- **Proxmox VE** 7.0+ (for container orchestration)

## 📚 Documentation

- **[INSTALL.md](INSTALL.md)** - Detailed installation guide
- **[QUICKSTART.md](QUICKSTART.md)** - Get started in 5 minutes
- **[ARCHITECTURE-ANALYSIS.md](ARCHITECTURE-ANALYSIS.md)** - Technical deep dive
- **[PLUGIN-DEVELOPMENT-GUIDE.md](PLUGIN-DEVELOPMENT-GUIDE.md)** - Create custom plugins

## 🤝 Contributing

op-dbus is designed to be extensible. Want to add a plugin?

1. Implement the `StatePlugin` trait
2. Add `is_available()` to check for dependencies
3. Register in `main.rs` with conditional logic
4. Submit a PR!

## 📄 License

MIT License - See [LICENSE](LICENSE) for details

## 🎯 Philosophy

**Declarative over Imperative** - Describe what you want, not how to get there

**Native over Wrappers** - Direct protocol access, not CLI command parsing

**Portable over Specific** - Works everywhere, optimizes where possible

**Auditable over Opaque** - Every change logged cryptographically

**Adaptive over Rigid** - Discovers capabilities, doesn't assume them

---

**Made with ❤️ for infrastructure engineers who value portability and auditability**
