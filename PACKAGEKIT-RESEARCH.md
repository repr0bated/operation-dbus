# PackageKit D-Bus API Research

## Overview

PackageKit is a D-Bus abstraction layer for package management across Linux distributions. It provides a unified API for apt, yum, dnf, zypper, and other package managers.

## D-Bus Interfaces

### 1. org.freedesktop.PackageKit (Main Daemon)

**Service Name**: `org.freedesktop.PackageKit`
**Object Path**: `/org/freedesktop/PackageKit`

#### Key Methods

| Method | Parameters | Returns | Description |
|--------|-----------|---------|-------------|
| `CreateTransaction` | - | `o` (object path) | Creates a new transaction object |
| `GetTimeSinceAction` | `u` (role) | `u` (seconds) | Time since last action of type |
| `GetTransactionList` | - | `ao` (array of paths) | List of active transactions |
| `GetPackageHistory` | `as` (names), `u` (count) | Complex dict | Package installation history |
| `CanAuthorize` | `s` (action_id) | `u` (result) | Check authorization for action |
| `SetProxy` | 6x `s` (proxy settings) | - | Configure HTTP/HTTPS/FTP proxies |

#### Signals

- `TransactionListChanged` - emitted when transactions change
- `UpdatesChanged` - emitted when updates are available
- `RepoListChanged` - emitted when repositories change
- `InstalledChanged` - emitted when installed packages change

### 2. org.freedesktop.PackageKit.Transaction

**Service Name**: `org.freedesktop.PackageKit`
**Object Path**: Dynamic (e.g., `/45_dafeca`)

#### Key Methods

| Method | Input Signature | Output | Description |
|--------|----------------|--------|-------------|
| `SearchNames` | `t` (filter), `as` (search terms) | Signals | Search packages by name |
| `InstallPackages` | `t` (flags), `as` (package_ids) | Signals | Install packages with deps |
| `RemovePackages` | `t` (flags), `as` (package_ids), `b` (deps), `b` (autoremove) | Signals | Remove packages |
| `UpdatePackages` | `t` (flags), `as` (package_ids) | Signals | Update packages |
| `GetPackages` | `t` (filter) | Signals | List all packages |
| `GetUpdates` | `t` (filter) | Signals | List available updates |
| `GetDetails` | `as` (package_ids) | Signals | Get package details |
| `GetFiles` | `as` (package_ids) | Signals | Get package files |
| `Resolve` | `t` (filter), `as` (names) | Signals | Resolve package names to IDs |
| `Cancel` | - | - | Cancel running transaction |

#### Properties

| Property | Type | Description |
|----------|------|-------------|
| `Status` | `u` | Current status (enum) |
| `Percentage` | `u` | Progress 0-100 (101 = unknown) |
| `ElapsedTime` | `u` | Seconds elapsed |
| `RemainingTime` | `u` | Estimated seconds remaining |
| `AllowCancel` | `b` | Can transaction be cancelled |
| `Uid` | `u` | User ID running transaction |
| `Role` | `u` | Transaction role (enum) |

#### Signals

- `Package` - emitted for each package (info, package_id, summary)
- `Progress` - progress updates
- `Error` - error messages
- `Files` - file list results
- `Finished` - transaction complete

## Transaction Workflow

```
1. CreateTransaction() → /transaction/45_dafeca
2. Call method on transaction (e.g., SearchNames)
3. Listen for signals (Package, Progress, Error)
4. Wait for Finished signal
5. Transaction auto-destructs after timeout
```

## Package ID Format

PackageKit uses a composite package ID:
```
name;version;arch;repo
Example: nginx;1.18.0-6ubuntu14;amd64;Ubuntu
```

## Filter Bitfield

Common filter values (can be combined with `;`):
- `installed` - only installed packages
- `~installed` - not installed
- `devel` - development packages
- `~devel` - not development
- `none` - no filter

Examples:
- `installed;~devel` - installed non-dev packages
- `none` - all packages

## Transaction Flags

Bitfield for transaction behavior:
- `simulate` (1) - dry run
- `only-trusted` (2) - only install trusted packages
- `only-download` (4) - download only, don't install

## Required PolicyKit Permissions

- `org.freedesktop.packagekit.install-untrusted` - install packages
- `org.freedesktop.packagekit.remove` - remove packages
- `org.freedesktop.packagekit.update-package` - update packages
- `org.freedesktop.packagekit.refresh-cache` - refresh package cache

## Implementation Notes

### For State Plugin

1. Use transaction-based model (create transaction per operation)
2. Query current state with `GetPackages(installed)`
3. Apply state by comparing desired vs actual packages
4. Install missing packages with `InstallPackages`
5. Remove extra packages with `RemovePackages`

### For MCP Agent

1. Each MCP tool maps to a PackageKit transaction method
2. Handle async signals with tokio channels
3. Provide progress updates to MCP client
4. Parse package IDs into structured data
5. Handle PolicyKit authentication prompts

## Example D-Bus Commands

```bash
# List all services
busctl list | grep -i package

# Introspect main interface
busctl introspect org.freedesktop.PackageKit /org/freedesktop/PackageKit

# Create transaction
busctl call org.freedesktop.PackageKit /org/freedesktop/PackageKit \
  org.freedesktop.PackageKit CreateTransaction

# Search for package (on transaction)
busctl call org.freedesktop.PackageKit /45_dafeca \
  org.freedesktop.PackageKit.Transaction SearchNames t 0 as 1 "nginx"
```

## References

- Main interface XML: https://github.com/PackageKit/PackageKit/blob/main/src/org.freedesktop.PackageKit.xml
- Transaction interface XML: https://github.com/PackageKit/PackageKit/blob/main/src/org.freedesktop.PackageKit.Transaction.xml
- Official docs: https://www.freedesktop.org/software/PackageKit/gtk-doc/api-reference.html
