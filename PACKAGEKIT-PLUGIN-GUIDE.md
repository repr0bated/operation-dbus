# PackageKit Plugin Guide

## Overview

The PackageKit plugin provides declarative package management for op-dbus. It integrates with the system's PackageKit D-Bus service to manage packages across different distributions (apt, yum, dnf, zypper, etc.).

## Features

- **Declarative Package State**: Define desired packages in JSON/YAML
- **Cross-Distribution**: Works with any package manager via PackageKit
- **Transaction-Based**: Safe, atomic package operations
- **MCP Integration**: Exposes package operations as MCP tools for AI assistants
- **Introspection**: Query installed packages and available updates

## Architecture

```
┌─────────────────────────────────────────┐
│   State Manager (op-dbus)               │
│   - Reads desired state from JSON       │
│   - Calculates diff (install/remove)    │
└──────────────┬──────────────────────────┘
               │
               ↓
┌─────────────────────────────────────────┐
│   PackageKit Plugin                     │
│   - Implements StatePlugin trait        │
│   - Creates D-Bus transactions          │
└──────────────┬──────────────────────────┘
               │
               ↓
┌─────────────────────────────────────────┐
│   PackageKit D-Bus Service              │
│   org.freedesktop.PackageKit            │
│   - Creates transactions                │
│   - Emits signals (progress, package)   │
└──────────────┬──────────────────────────┘
               │
               ↓
┌─────────────────────────────────────────┐
│   Backend (apt/yum/dnf/zypper)          │
│   - Executes actual package operations   │
└─────────────────────────────────────────┘
```

## Configuration

### State File Format

```json
{
  "version": 1,
  "plugins": {
    "packagekit": {
      "installed": [
        "nginx",
        "postgresql-14",
        "git",
        "htop"
      ],
      "removed": [
        "apache2"
      ],
      "version_pinned": {
        "docker-ce": "24.0.7"
      },
      "auto_update": false
    }
  }
}
```

### NixOS Configuration

```nix
services.op-dbus.packages = {
  enable = true;
  installed = [ "nginx" "postgresql" "git" ];
  removed = [ "apache2" ];
  autoUpdate = false;
};
```

## Usage

### Query Current Packages

```bash
op-dbus query --plugin packagekit
```

Output:
```json
{
  "installed": ["nginx", "git", "htop"],
  "version_pinned": {
    "nginx": "1.18.0-6ubuntu14"
  }
}
```

### Apply Package State

```bash
op-dbus apply state.json
```

This will:
1. Query currently installed packages
2. Calculate diff (packages to install/remove)
3. Create PackageKit transaction
4. Install missing packages
5. Remove unwanted packages
6. Report results

### Diff State

```bash
op-dbus diff state.json
```

Shows what changes would be applied:
```
Actions to perform:
  + Install: postgresql-14
  - Remove: apache2
  ✓ No change: nginx, git, htop
```

## MCP Tools

The PackageKit MCP agent exposes these tools:

### package_search

Search for packages:
```json
{
  "type": "package",
  "action": "search",
  "query": "nginx"
}
```

### package_install

Install packages:
```json
{
  "type": "package",
  "action": "install",
  "packages": ["nginx", "postgresql"]
}
```

### package_remove

Remove packages:
```json
{
  "type": "package",
  "action": "remove",
  "packages": ["apache2"]
}
```

### package_list

List installed packages:
```json
{
  "type": "package",
  "action": "list"
}
```

### package_updates

Get available updates:
```json
{
  "type": "package",
  "action": "updates"
}
```

### package_refresh

Refresh package cache:
```json
{
  "type": "package",
  "action": "refresh"
}
```

### package_details

Get package details:
```json
{
  "type": "package",
  "action": "details",
  "packages": ["nginx"]
}
```

## D-Bus Interface

### Manual Testing

```bash
# Create transaction
busctl call org.freedesktop.PackageKit /org/freedesktop/PackageKit \
  org.freedesktop.PackageKit CreateTransaction

# Get transaction path (e.g., /45_dafeca)

# Search for package
busctl call org.freedesktop.PackageKit /45_dafeca \
  org.freedesktop.PackageKit.Transaction SearchNames t 0 as 1 "nginx"

# Monitor signals
busctl monitor org.freedesktop.PackageKit
```

## Implementation Details

### Transaction Workflow

1. **Create Transaction**:
   ```rust
   let path = proxy.call("CreateTransaction", &()).await?;
   ```

2. **Resolve Package Names**:
   ```rust
   proxy.call("Resolve", &(filter, package_names)).await?;
   ```

3. **Install Packages**:
   ```rust
   proxy.call("InstallPackages", &(flags, package_ids)).await?;
   ```

4. **Monitor Signals** (not yet implemented):
   - `Package`: Emitted for each package found
   - `Progress`: Operation progress
   - `Error`: Errors during operation
   - `Finished`: Transaction complete

### Package ID Format

PackageKit uses composite IDs:
```
name;version;arch;repo
Example: nginx;1.18.0-6ubuntu14;amd64;Ubuntu
```

### Filter Bitfield

Common filters:
- `0` - No filter (all packages)
- `1 << 2` - Installed packages only
- `1 << 3` - Available packages only

## Security

### PolicyKit Permissions

Package operations require PolicyKit authorization:

- `org.freedesktop.packagekit.install-untrusted` - Install packages
- `org.freedesktop.packagekit.remove` - Remove packages
- `org.freedesktop.packagekit.update-package` - Update packages
- `org.freedesktop.packagekit.refresh-cache` - Refresh cache

### Sandboxing

The NixOS module configures security:

```nix
serviceConfig = {
  PrivateTmp = true;
  ProtectSystem = "strict";
  ProtectHome = true;
  NoNewPrivileges = false; # Need privileges for package management
};
```

## Troubleshooting

### "Permission denied" Error

Check PolicyKit rules:
```bash
pkaction --verbose | grep packagekit
```

Grant permission:
```bash
sudo polkit add-rule /etc/polkit-1/rules.d/99-packagekit.rules
```

### "Transaction already exists" Error

PackageKit limits concurrent transactions. Wait for previous transaction to complete or cancel it:

```bash
busctl call org.freedesktop.PackageKit /transaction/path \
  org.freedesktop.PackageKit.Transaction Cancel
```

### Package Not Found

Refresh cache first:
```bash
op-dbus refresh-cache
# or manually
sudo pkcon refresh
```

### Signal Handling Not Working

Full signal handling will be implemented in future version. Current version provides basic transaction creation and method calls.

## Future Enhancements

- [ ] Complete signal handling (Package, Progress, Error, Finished)
- [ ] Progress reporting to user
- [ ] Package dependency resolution visualization
- [ ] Automatic rollback on failure
- [ ] Package verification (checksums, signatures)
- [ ] Offline package installation

## Examples

### Example 1: Web Server Setup

State file:
```json
{
  "plugins": {
    "packagekit": {
      "installed": ["nginx", "certbot", "python3-certbot-nginx"],
      "removed": ["apache2", "apache2-utils"]
    }
  }
}
```

Apply:
```bash
op-dbus apply webserver-state.json
```

### Example 2: Development Environment

```json
{
  "plugins": {
    "packagekit": {
      "installed": [
        "git",
        "build-essential",
        "python3",
        "python3-pip",
        "nodejs",
        "npm",
        "docker.io"
      ],
      "auto_update": true
    }
  }
}
```

### Example 3: Minimal System

```json
{
  "plugins": {
    "packagekit": {
      "installed": ["vim", "tmux", "htop", "curl"],
      "removed": ["*games*", "*doc*"]
    }
  }
}
```

## References

- [PackageKit D-Bus API](https://www.freedesktop.org/software/PackageKit/gtk-doc/api-reference.html)
- [PackageKit GitHub](https://github.com/PackageKit/PackageKit)
- [PACKAGEKIT-RESEARCH.md](./PACKAGEKIT-RESEARCH.md) - Detailed API documentation

## See Also

- [HYBRID-SCANNER-GUIDE.md](./HYBRID-SCANNER-GUIDE.md) - System introspection
- [nixos/README.md](./nixos/README.md) - NixOS deployment
- [MCP-DBUS-BENEFITS.md](./MCP-DBUS-BENEFITS.md) - MCP integration benefits
