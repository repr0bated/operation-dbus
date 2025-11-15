# plugin.toml Format Specification

## Overview

The `plugin.toml` file contains metadata for op-dbus plugins. It supports both:
- **Hand-written plugins** (Rust code in src/state/plugins/)
- **Auto-generated plugins** (from D-Bus introspection)

## Format

### Basic Metadata

```toml
[plugin]
name = "lxc"                    # Plugin identifier (must match directory name)
version = "1.2.0"               # Semantic version
author = "repr0bated"           # Author/maintainer
description = "Proxmox LXC container management"
source = "hand-written"         # "hand-written" | "auto-generated"
requires_proxmox = true         # Optional: Proxmox-specific plugin
requires_root = true            # Optional: Requires root privileges
```

### For Auto-Generated Plugins

```toml
[plugin]
name = "packagekit"
version = "1.0.0"
author = "community"
description = "System package management via PackageKit D-Bus"
source = "auto-generated"
dbus_service = "org.freedesktop.PackageKit"     # D-Bus service name
dbus_path = "/org/freedesktop/PackageKit"       # D-Bus object path
dbus_interface = "org.freedesktop.PackageKit"   # Primary interface

# Optional: Mark as read-only if no semantic mapping exists
read_only = true
```

### Capabilities

```toml
[capabilities]
query = true                # Can query current state
apply = true                # Can apply desired state
rollback = true             # Can rollback changes
checkpoint = true           # Can create checkpoints
verify = true               # Can verify state
```

### Dependencies

```toml
[dependencies]
# Other plugins this plugin depends on
plugins = ["net", "systemd"]

# System requirements
packages = ["lxc", "bridge-utils"]

# D-Bus services required
dbus_services = ["org.freedesktop.NetworkManager"]
```

### Configuration

```toml
[config]
# Plugin-specific configuration
default_bridge = "vmbr0"
default_template = "debian-13-standard"

# Cache settings (optional override)
cache_ttl_seconds = 60
cache_enabled = true
```

### Metadata

```toml
[metadata]
homepage = "https://github.com/repr0bated/operation-dbus"
repository = "https://github.com/repr0bated/operation-dbus"
license = "MIT"
keywords = ["lxc", "containers", "proxmox"]
```

## Examples

### Hand-Written Plugin (LXC)

```toml
[plugin]
name = "lxc"
version = "1.2.0"
author = "repr0bated"
description = "Proxmox LXC container management with golden images"
source = "hand-written"
requires_proxmox = true
requires_root = true

[capabilities]
query = true
apply = true
rollback = true
checkpoint = true
verify = true

[dependencies]
plugins = ["net"]
packages = ["lxc", "pct"]

[config]
default_bridge = "vmbr0"
default_storage = "local-btrfs"
default_template = "debian-13-standard"
golden_image_dir = "/var/lib/pve/local-btrfs/templates/subvol"

[metadata]
homepage = "https://github.com/repr0bated/operation-dbus"
license = "MIT"
keywords = ["lxc", "containers", "proxmox", "btrfs"]
```

### Auto-Generated Plugin (PackageKit)

```toml
[plugin]
name = "packagekit"
version = "1.0.0"
author = "community"
description = "System package management via PackageKit D-Bus"
source = "auto-generated"
dbus_service = "org.freedesktop.PackageKit"
dbus_path = "/org/freedesktop/PackageKit"
dbus_interface = "org.freedesktop.PackageKit"
requires_root = true

# Initially read-only (no semantic mapping yet)
read_only = false  # semantic-mapping.toml exists!

[capabilities]
query = true
apply = true        # Enabled via semantic mapping!
rollback = false    # PackageKit doesn't support transactions
checkpoint = false
verify = true

[dependencies]
dbus_services = ["org.freedesktop.PackageKit"]

[config]
cache_ttl_seconds = 300  # 5 minutes (package list changes slowly)

[metadata]
homepage = "https://www.freedesktop.org/software/PackageKit/"
license = "GPL-2.0"
keywords = ["packages", "apt", "dnf", "zypper", "dbus"]
```

### Auto-Generated Plugin (NetworkManager) - Read-Only

```toml
[plugin]
name = "networkmanager"
version = "1.0.0"
author = "auto-generated"
description = "Network configuration via NetworkManager D-Bus (read-only)"
source = "auto-generated"
dbus_service = "org.freedesktop.NetworkManager"
dbus_path = "/org/freedesktop/NetworkManager"
dbus_interface = "org.freedesktop.NetworkManager"

# No semantic mapping yet = read-only
read_only = true

[capabilities]
query = true
apply = false       # Disabled (no semantic mapping)
rollback = false
checkpoint = false
verify = false

[dependencies]
dbus_services = ["org.freedesktop.NetworkManager"]

[config]
cache_ttl_seconds = 10  # Network state changes frequently

[metadata]
note = "Read-only auto-generated plugin. Contribute semantic-mapping.toml to enable writes!"
```

### Netmaker Plugin (Hand-Written, Mesh Networking)

```toml
[plugin]
name = "netmaker"
version = "2.1.0"
author = "repr0bated"
description = "Netmaker mesh network management"
source = "hand-written"
requires_root = true

[capabilities]
query = true
apply = true
rollback = true
checkpoint = true
verify = true

[dependencies]
plugins = ["net"]
packages = ["wireguard-tools"]

[config]
netclient_binary = "/usr/local/bin/netclient"
config_dir = "/etc/netmaker"
enrollment_token_file = "/etc/netmaker/enrollment-token"

[metadata]
homepage = "https://www.netmaker.io/"
repository = "https://github.com/gravitl/netmaker"
license = "SSPL"
keywords = ["wireguard", "mesh", "vpn", "netmaker"]
```

## Validation Rules

### Required Fields

- `plugin.name` - Must match @plugin-{name} directory
- `plugin.version` - Must be valid semver
- `plugin.source` - Must be "hand-written" or "auto-generated"

### Auto-Generated Specific

If `source = "auto-generated"`:
- **Required**: `dbus_service`, `dbus_interface`
- **Optional**: `dbus_path` (defaults to /{service/path/from/name})
- **Must exist**: `introspection.xml` file in same directory

### Hand-Written Specific

If `source = "hand-written"`:
- **Must exist**: Corresponding Rust file in src/state/plugins/{name}.rs
- **Must register**: In src/main.rs via register_plugin()

## Discovery Process

When op-dbus starts:

1. **Scan** `/var/lib/op-dbus/` for `@plugin-*` directories
2. **Load** `plugin.toml` from each directory
3. **Validate** metadata and dependencies
4. **Check source type**:
   - If `hand-written`: Load from compiled binary
   - If `auto-generated`: Generate plugin from introspection.xml
5. **Load semantic mapping** (if exists): semantic-mapping.toml
6. **Register** plugin with StateManager

## Versioning

Plugins use semantic versioning:

```
MAJOR.MINOR.PATCH
1.2.0

MAJOR: Breaking changes (incompatible state format)
MINOR: New features (backward compatible)
PATCH: Bug fixes (backward compatible)
```

Plugin snapshots include version:
```
@plugin-snapshots/
├── lxc@v1.0.0
├── lxc@v1.2.0       ← Can have multiple versions
└── lxc@v2.0.0
```

Active plugin (symlink or directory):
```
@plugin-lxc/  → points to latest compatible version
```

## Distribution

When distributing plugins via BTRFS snapshots:

1. **Create snapshot**: `btrfs subvolume snapshot -r @plugin-lxc @plugin-snapshots/lxc@v1.2.0`
2. **Send**: `btrfs send @plugin-snapshots/lxc@v1.2.0 | zstd > lxc-v1.2.0.btrfs.zst`
3. **Distribute**: Upload to community repository or GitHub releases
4. **Install**: `zstd -d lxc-v1.2.0.btrfs.zst | btrfs receive /var/lib/op-dbus/`
5. **Activate**: `mv @plugin-snapshots/lxc@v1.2.0 @plugin-lxc`

The snapshot includes:
- plugin.toml (metadata)
- semantic-mapping.toml (if exists)
- introspection.xml (for auto-generated)
- examples/ (example configs)
- README.md (documentation)

**NOT included in snapshot**: Compiled Rust code (security)

Hand-written plugins still require:
- Source code distribution via Git
- Compilation on target system
- Or: Pre-compiled binaries (with checksums)

Auto-generated plugins work immediately (no compilation needed!)

## Security Considerations

### Trusted Sources

Only install plugins from trusted sources:
- Official op-dbus repository
- Verified community contributors
- Your own plugins

### Signature Verification (Future)

```toml
[plugin]
name = "lxc"
version = "1.2.0"

[signature]
gpg_key = "0x1234567890ABCDEF"
signature_file = "plugin.toml.sig"
```

### Sandboxing (Future)

```toml
[security]
allow_network = true
allow_filesystem = ["/var/lib/lxc", "/etc/pve"]
allow_dbus = ["org.freedesktop.PackageKit"]
allow_capabilities = ["CAP_NET_ADMIN"]
```

## Migration Path

For existing plugins without plugin.toml:

1. Auto-generate plugin.toml from Rust code
2. Populate metadata from Cargo.toml
3. Migrate to @plugin-{name}/ structure

## See Also

- [semantic-mapping.toml Format](SEMANTIC-MAPPING-FORMAT.md)
- [Plugin Development Guide](PLUGIN-DEVELOPMENT.md)
- [Auto-Plugin Integration](AUTO-PLUGIN-INTEGRATION.md)
