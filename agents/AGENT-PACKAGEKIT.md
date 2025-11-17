# PackageKit Agent Specification

**D-Bus Interface**: `org.dbusmcp.Agent.PackageKit`
**Agent Type**: `packagekit`
**Purpose**: Package management via D-Bus PackageKit interface

## Task Format

```json
{
  "type": "packagekit",
  "operation": "search",
  "package": "nginx",
  "filter": "installed"
}
```

## Supported Operations

### search
Search for packages by name

**Parameters**:
- `package` (required): Package name or pattern
- `filter` (optional): "installed", "available", "newest"

### install
Install packages

**Parameters**:
- `packages` (required): Array of package names
- `simulate` (optional): Dry-run simulation

### remove
Remove/uninstall packages

**Parameters**:
- `packages` (required): Array of package names
- `simulate` (optional): Dry-run simulation

### update
Update installed packages

**Parameters**:
- `packages` (optional): Specific packages to update (empty = all)

### get_updates
List available package updates

### get_details
Get detailed package information

**Parameters**:
- `package_id` (required): PackageKit package ID

### refresh_cache
Update package cache from repositories

**Parameters**:
- `force` (optional): Force refresh even if cache is recent

## PackageKit D-Bus Integration

This agent wraps the standard `org.freedesktop.PackageKit` D-Bus interface:

- **Service**: `org.freedesktop.PackageKit`
- **Transaction Objects**: `/org/freedesktop/PackageKit/Transaction/*`
- **Methods**: SearchNames, InstallPackages, RemovePackages, UpdatePackages, etc.

## Usage Examples

```bash
# Search for nginx
busctl call org.dbusmcp.Agent.PackageKit.{id} /org/dbusmcp/Agent/PackageKit/{id} \
  org.dbusmcp.Agent.PackageKit Execute s \
  '{"type":"packagekit","operation":"search","package":"nginx"}'

# Install package (simulation)
busctl call org.dbusmcp.Agent.PackageKit.{id} /org/dbusmcp/Agent/PackageKit/{id} \
  org.dbusmcp.Agent.PackageKit Execute s \
  '{"type":"packagekit","operation":"install","packages":["nginx"],"simulate":true}'

# Check for updates
busctl call org.dbusmcp.Agent.PackageKit.{id} /org/dbusmcp/Agent/PackageKit/{id} \
  org.dbusmcp.Agent.PackageKit Execute s \
  '{"type":"packagekit","operation":"get_updates"}'
```

## Response Format

```json
{
  "success": true,
  "operation": "search",
  "results": [
    {
      "package_id": "nginx;1.18.0-0ubuntu1;amd64;Ubuntu",
      "summary": "small, powerful, scalable web/proxy server",
      "info": "installed"
    }
  ]
}
```

## PackageKit Transaction Flow

1. Create transaction: `CreateTransaction()`
2. Set up signal handlers for: `Package`, `Finished`, `ErrorCode`
3. Execute operation: `SearchNames`, `InstallPackages`, etc.
4. Monitor signals for progress and results
5. Transaction completes with `Finished` signal

## Error Handling

- **Package not found**: Empty results array
- **Permission denied**: Requires PolicyKit authorization
- **Transaction failed**: Check `ErrorCode` signal details
- **Network error**: Unable to reach package repositories

## Security

- Operations require PolicyKit authorization
- Install/remove operations need `org.freedesktop.packagekit.package-install` privilege
- User may be prompted for authentication

## Supported Package Managers

PackageKit supports multiple backends:
- **apt** (Debian/Ubuntu)
- **dnf** (Fedora/RHEL)
- **zypper** (openSUSE)
- **pacman** (Arch)
- **apk** (Alpine)
