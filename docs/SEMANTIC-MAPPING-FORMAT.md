# semantic-mapping.toml Format Specification

## Overview

The `semantic-mapping.toml` file teaches auto-generated D-Bus plugins how to **apply state changes safely**. Without this file, auto-generated plugins are **read-only**.

## Purpose

Auto-generated plugins can introspect D-Bus services and read their state, but they don't know:
- Which methods modify system state (safe vs unsafe)
- How to map declarative state to D-Bus method calls
- What arguments methods expect
- Whether confirmation is needed

The semantic mapping provides this knowledge, enabling **write operations**.

## Format

### Method Safety Classification

```toml
[methods.method_name]
safe = true | false                    # Does it modify system state?
side_effects = true | false            # Does it have side effects?
requires_confirmation = true | false   # Ask user before calling?
idempotent = true | false             # Safe to call multiple times?
```

### Method Argument Mapping

```toml
[methods.install_packages]
safe = false
requires_confirmation = true
side_effects = true

# Map declarative state properties to D-Bus method arguments
args_mapping = ["transaction_flags", "package_ids"]

# Argument types (for validation)
arg_types = ["u", "as"]  # DBus signature: u = uint32, as = array of strings

# Optional: Default values
defaults = { transaction_flags = 0 }
```

### Property Setters

```toml
[properties.hostname]
# How to set this property
setter_method = "SetHostname"
getter_method = "GetHostname"
property_type = "s"  # DBus signature: string

# Does setting require restart?
requires_restart = false

# Validation
validation = { min_length = 1, max_length = 63, pattern = "^[a-zA-Z0-9-]+$" }
```

### State Mapping

```toml
[state_mapping]
# Map declarative state keys to D-Bus operations

# Simple property mapping
"hostname" = { property = "Hostname", writable = true }
"timezone" = { property = "Timezone", writable = true }

# Method-based mapping
"packages" = {
    query_method = "GetPackages",
    apply_method = "InstallPackages",
    diff_strategy = "set_difference"  # How to calculate diff
}

# Complex mapping with transformation
"repositories" = {
    query_method = "GetRepos",
    apply_method = "AddRepo",
    remove_method = "RemoveRepo",
    diff_strategy = "merge_strategy",
    transform = "repo_list_to_config"  # Custom transform function
}
```

## Examples

### PackageKit (Package Management)

```toml
# semantic-mapping.toml for org.freedesktop.PackageKit

[service]
name = "org.freedesktop.PackageKit"
version = "1.0"
description = "Semantic mapping for PackageKit D-Bus service"

# === READ OPERATIONS (Safe) ===

[methods.get_packages]
safe = true
side_effects = false
requires_confirmation = false
description = "Get list of installed packages"
returns = "as"  # array of strings

[methods.get_updates]
safe = true
side_effects = false
requires_confirmation = false
description = "Get available updates"
returns = "as"

[methods.get_repos]
safe = true
side_effects = false
requires_confirmation = false
description = "Get repository list"
returns = "as"

# === WRITE OPERATIONS (Unsafe) ===

[methods.install_packages]
safe = false
side_effects = true
requires_confirmation = true
idempotent = false
description = "Install packages"

# Argument mapping
args_mapping = ["transaction_flags", "package_ids"]
arg_types = ["u", "as"]
defaults = { transaction_flags = 0 }

# Safety checks
pre_check = "check_disk_space"
dry_run_supported = true

[methods.remove_packages]
safe = false
side_effects = true
requires_confirmation = true
idempotent = false
description = "Remove packages"

args_mapping = ["transaction_flags", "package_ids", "allow_deps"]
arg_types = ["u", "as", "b"]
defaults = { transaction_flags = 0, allow_deps = false }

[methods.update_packages]
safe = false
side_effects = true
requires_confirmation = true
idempotent = false
description = "Update packages"

args_mapping = ["transaction_flags", "package_ids"]
arg_types = ["u", "as"]
defaults = { transaction_flags = 0 }

[methods.refresh_cache]
safe = true  # Just updates package cache
side_effects = false
requires_confirmation = false
idempotent = true
description = "Refresh package cache"

# === STATE MAPPING ===

[state_mapping]
# Map declarative state to D-Bus operations

"packages" = {
    query_method = "GetPackages",
    apply_method = "InstallPackages",
    remove_method = "RemovePackages",
    diff_strategy = "set_difference",
    description = "Installed packages list"
}

"repositories" = {
    query_method = "GetRepos",
    apply_method = "RepoSetData",
    diff_strategy = "merge_strategy",
    description = "Package repositories"
}

# === SAFETY POLICIES ===

[safety]
# Always refresh cache before operations
pre_apply_hook = "RefreshCache"

# Maximum packages to install in one transaction
max_batch_size = 50

# Require disk space (in MB) before install
min_disk_space_mb = 1024

# Backup package list before changes
create_backup = true
backup_path = "/var/lib/op-dbus/backups/packagekit"

# Timeout for operations (seconds)
operation_timeout = 3600  # 1 hour for large package installs
```

### NetworkManager (Read-Only Example)

```toml
# semantic-mapping.toml for org.freedesktop.NetworkManager
# NOTE: This is read-only for now (no write methods defined)

[service]
name = "org.freedesktop.NetworkManager"
version = "1.0"
description = "Semantic mapping for NetworkManager (read-only)"

# === READ OPERATIONS ONLY ===

[methods.get_devices]
safe = true
side_effects = false
requires_confirmation = false
description = "Get network devices"
returns = "ao"  # array of object paths

[methods.get_connections]
safe = true
side_effects = false
requires_confirmation = false
description = "Get network connections"
returns = "ao"

[methods.get_active_connections]
safe = true
side_effects = false
requires_confirmation = false
description = "Get active connections"
returns = "ao"

# === STATE MAPPING (Read-Only) ===

[state_mapping]
"devices" = {
    query_method = "GetDevices",
    # No apply_method = read-only!
    diff_strategy = "none",
    description = "Network devices (read-only)"
}

"connections" = {
    query_method = "GetConnections",
    diff_strategy = "none",
    description = "Network connections (read-only)"
}

[safety]
# This plugin is read-only
allow_writes = false
read_only_reason = "NetworkManager API is complex, manual plugin recommended"
```

### UPower (Power Management)

```toml
# semantic-mapping.toml for org.freedesktop.UPower

[service]
name = "org.freedesktop.UPower"
version = "1.0"
description = "Semantic mapping for UPower D-Bus service"

# === READ OPERATIONS ===

[methods.enumerate_devices]
safe = true
side_effects = false
requires_confirmation = false
description = "List power devices"
returns = "ao"

[methods.get_critical_action]
safe = true
side_effects = false
requires_confirmation = false
description = "Get critical battery action"
returns = "s"

# === WRITE OPERATIONS ===

[methods.set_critical_action]
safe = false
side_effects = true
requires_confirmation = true
idempotent = true
description = "Set critical battery action"

args_mapping = ["action"]
arg_types = ["s"]
validation = { allowed_values = ["hibernate", "shutdown", "hybrid-sleep"] }

# === PROPERTIES ===

[properties.on_battery]
getter_method = "OnBattery"
writable = false
property_type = "b"
description = "Whether system is on battery power"

[properties.lid_is_closed]
getter_method = "LidIsClosed"
writable = false
property_type = "b"
description = "Whether laptop lid is closed"

# === STATE MAPPING ===

[state_mapping]
"critical_action" = {
    query_method = "GetCriticalAction",
    apply_method = "SetCriticalAction",
    diff_strategy = "value_comparison",
    description = "Action on critical battery"
}

"devices" = {
    query_method = "EnumerateDevices",
    diff_strategy = "none",  # Read-only
    description = "Power devices (read-only)"
}
```

### systemd (System Management) - Partial Mapping

```toml
# semantic-mapping.toml for org.freedesktop.systemd1

[service]
name = "org.freedesktop.systemd1"
version = "1.0"
description = "Semantic mapping for systemd (services only)"

# === READ OPERATIONS ===

[methods.list_units]
safe = true
side_effects = false
requires_confirmation = false
description = "List all units"
returns = "a(ssssssouso)"  # Complex struct array

[methods.get_unit]
safe = true
side_effects = false
requires_confirmation = false
description = "Get unit by name"
args_mapping = ["unit_name"]
arg_types = ["s"]
returns = "o"  # object path

# === WRITE OPERATIONS ===

[methods.start_unit]
safe = false
side_effects = true
requires_confirmation = true
idempotent = false
description = "Start a systemd unit"

args_mapping = ["unit_name", "mode"]
arg_types = ["s", "s"]
defaults = { mode = "replace" }
validation = { mode = { allowed_values = ["replace", "fail", "isolate"] } }

[methods.stop_unit]
safe = false
side_effects = true
requires_confirmation = true
idempotent = true  # Stopping already-stopped service is safe
description = "Stop a systemd unit"

args_mapping = ["unit_name", "mode"]
arg_types = ["s", "s"]
defaults = { mode = "replace" }

[methods.restart_unit]
safe = false
side_effects = true
requires_confirmation = true
idempotent = false
description = "Restart a systemd unit"

args_mapping = ["unit_name", "mode"]
arg_types = ["s", "s"]
defaults = { mode = "replace" }

[methods.enable_unit_files]
safe = false
side_effects = true
requires_confirmation = false  # Just enables, doesn't start
idempotent = true
description = "Enable unit files"

args_mapping = ["unit_files", "runtime", "force"]
arg_types = ["as", "b", "b"]
defaults = { runtime = false, force = false }

[methods.disable_unit_files]
safe = false
side_effects = true
requires_confirmation = false
idempotent = true
description = "Disable unit files"

args_mapping = ["unit_files", "runtime"]
arg_types = ["as", "b"]
defaults = { runtime = false }

# === STATE MAPPING ===

[state_mapping]
"services" = {
    query_method = "ListUnits",
    apply_enabled_method = "EnableUnitFiles",
    apply_disabled_method = "DisableUnitFiles",
    apply_started_method = "StartUnit",
    apply_stopped_method = "StopUnit",
    diff_strategy = "service_state_diff",
    description = "Systemd services configuration"
}

# === SAFETY POLICIES ===

[safety]
# Don't allow stopping critical services
protected_services = [
    "sshd.service",
    "systemd-logind.service",
    "dbus.service"
]

# Warn before restarting these
warning_services = [
    "networking.service",
    "docker.service",
    "lxc.service"
]

# Timeout for service operations
operation_timeout = 300  # 5 minutes
```

## Diff Strategies

The `diff_strategy` determines how to calculate differences between current and desired state:

### set_difference
```toml
diff_strategy = "set_difference"
```
Treats state as a set. Calculate:
- **To install**: desired - current
- **To remove**: current - desired

Example: Package lists

### value_comparison
```toml
diff_strategy = "value_comparison"
```
Simple value comparison. If different, replace.

Example: Hostname, timezone

### merge_strategy
```toml
diff_strategy = "merge_strategy"
```
Merge desired into current, preserving unspecified keys.

Example: Repository configurations

### service_state_diff
```toml
diff_strategy = "service_state_diff"
```
Compare service state (enabled/disabled, running/stopped).

Example: systemd services

### none
```toml
diff_strategy = "none"
```
Read-only, no diff calculation.

## Validation

### Argument Validation

```toml
[methods.set_hostname]
args_mapping = ["hostname"]
arg_types = ["s"]

# Validation rules
validation = {
    hostname = {
        min_length = 1,
        max_length = 63,
        pattern = "^[a-zA-Z0-9-]+$",
        not_starts_with = "-",
        not_ends_with = "-"
    }
}
```

### Pre-Flight Checks

```toml
[methods.install_packages]
# Run these checks before executing method
pre_check = "check_disk_space"
pre_check_args = { min_mb = 1024 }
```

Built-in pre-checks:
- `check_disk_space` - Ensure sufficient disk space
- `check_network` - Ensure network connectivity
- `check_service_running` - Ensure D-Bus service is running
- `check_root` - Ensure running as root

## Interactive Confirmation

When `requires_confirmation = true`:

```
About to call D-Bus method:
  Service: org.freedesktop.PackageKit
  Method: InstallPackages
  Arguments:
    - transaction_flags: 0
    - package_ids: ["nginx", "postgresql", "redis"]

  This will:
    - Install 3 packages
    - Download ~150 MB
    - Require ~500 MB disk space

  Proceed? [y/N]:
```

User can:
- **y** - Proceed with operation
- **N** - Cancel (default)
- **d** - Dry-run (if supported)

## Dry-Run Support

```toml
[methods.install_packages]
dry_run_supported = true
dry_run_method = "SimulateInstallPackages"  # Alternative method for dry-run
```

If supported:
```bash
sudo op-dbus apply state.json --dry-run --plugin packagekit
```

Shows what would happen without actually doing it.

## Safety Hooks

### Pre-Apply Hook

```toml
[safety]
pre_apply_hook = "RefreshCache"
```

Runs before any apply operation (e.g., refresh package cache).

### Post-Apply Hook

```toml
[safety]
post_apply_hook = "Cleanup"
```

Runs after successful apply (e.g., clean package cache).

### Rollback Hook

```toml
[safety]
rollback_supported = true
rollback_method = "RestoreBackup"
```

## Community Contributions

Users can contribute semantic mappings to the community repository:

```bash
# Clone community repo
git clone https://github.com/repr0bated/op-dbus-plugins

# Create semantic mapping
cd op-dbus-plugins/packagekit/
vim semantic-mapping.toml

# Test
sudo op-dbus apply test-state.json --plugin packagekit

# Contribute
git add semantic-mapping.toml
git commit -m "Add semantic mapping for PackageKit InstallPackages method"
git push origin main
```

## Versioning

Semantic mappings should be versioned alongside plugins:

```toml
[service]
name = "org.freedesktop.PackageKit"
version = "1.0"  # Mapping version
api_version = "1.2.45"  # D-Bus API version (if known)
```

Breaking changes in D-Bus API may require new mapping versions.

## Security Considerations

### Trusted Mappings Only

Only use semantic mappings from:
- Official op-dbus repository
- Verified community contributors
- Your own mappings

### Signature Verification (Future)

```toml
[signature]
gpg_key = "0x1234567890ABCDEF"
signature_file = "semantic-mapping.toml.sig"
signed_by = "repr0bated <email@example.com>"
```

### Sandboxing (Future)

Restrict what methods can do:

```toml
[sandbox]
allow_network = true
allow_filesystem_write = ["/var/cache/apt"]
allow_exec = ["/usr/bin/apt-get"]
max_execution_time = 3600
```

## Migration from Read-Only

To enable writes on a read-only auto-generated plugin:

1. **Create semantic-mapping.toml** in plugin directory
2. **Define safe methods** (query operations)
3. **Define unsafe methods** (apply operations)
4. **Map state** to D-Bus operations
5. **Test thoroughly** before distribution
6. **Set `read_only = false`** in plugin.toml

Example:
```bash
cd /var/lib/op-dbus/@plugin-packagekit/
vim semantic-mapping.toml  # Add mapping
sudo op-dbus verify --plugin packagekit  # Test
```

## See Also

- [plugin.toml Format](PLUGIN-TOML-FORMAT.md)
- [Auto-Plugin Integration](AUTO-PLUGIN-INTEGRATION.md)
- [Plugin Development Guide](PLUGIN-DEVELOPMENT.md)
