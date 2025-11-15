# Proxmox VE 9 → PackageKit D-Bus Extractor (Rust)

**Extract the Proxmox VE 9 ISO installation plan and translate it to PackageKit D-Bus calls for reproducible, auditable installation.**

---

## Overview

This Rust toolkit provides a complete pipeline for:

1. **Extracting** package lists from Proxmox VE 9 ISO
2. **Parsing** Debian package files with full dependency analysis
3. **Generating** a declarative JSON manifest
4. **Translating** to PackageKit D-Bus operations via `zbus`
5. **Installing** Proxmox VE via D-Bus (fully auditable and reproducible)

`★ Insight ─────────────────────────────────────`
**Why Rust for this task:**
1. **Type safety** - Package dependencies are complex, Rust's type system prevents errors
2. **D-Bus integration** - `zbus` provides excellent async D-Bus support
3. **Performance** - Parsing large package lists is fast
4. **Integration** - Matches op-dbus architecture (Rust + zbus)
`─────────────────────────────────────────────────`

---

## Prerequisites

### System Requirements

```bash
# Install dependencies
sudo apt-get install -y \
    squashfs-tools \
    packagekit \
    packagekit-tools

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the tools
cd tools/proxmox-extractor
cargo build --release
```

### Download Proxmox VE 9 ISO

```bash
wget https://enterprise.proxmox.com/iso/proxmox-ve_9.0-1.iso
```

---

## Quick Start

```bash
# 1. Extract package lists from ISO
./extract-iso.sh proxmox-ve_9.0-1.iso ./extracted

# 2. Generate manifest from packages
cargo run --release --bin proxmox-manifest -- \
    --packages ./extracted/packages/Packages.txt \
    --output ./manifest.json

# 3. Install via PackageKit D-Bus (dry-run first)
cargo run --release --bin proxmox-packagekit -- \
    ./manifest.json --dry-run

# 4. Actually install
sudo cargo run --release --bin proxmox-packagekit -- \
    ./manifest.json
```

---

## Architecture

```
┌─────────────────┐
│  Proxmox VE 9   │
│      ISO        │
└────────┬────────┘
         │ extract-iso.sh (bash)
         ▼
┌─────────────────┐
│  Packages.txt   │
└────────┬────────┘
         │ proxmox-manifest (Rust)
         │ - Parser module
         │ - Dependency resolution
         │ - Stage categorization
         ▼
┌─────────────────┐
│  manifest.json  │
└────────┬────────┘
         │ proxmox-packagekit (Rust + zbus)
         │ - D-Bus proxy generation
         │ - Batch installation
         │ - Retry policies
         ▼
┌─────────────────┐
│  PackageKit     │
│   D-Bus API     │
└────────┬────────┘
         ▼
┌─────────────────┐
│   Installed     │
│  Proxmox VE 9   │
└─────────────────┘
```

---

## Binaries

### `proxmox-manifest`

Parses Debian package lists and generates PackageKit manifest.

**Usage:**
```bash
proxmox-manifest \
    --packages path/to/Packages.txt \
    --output manifest.json \
    --version 9.0
```

**Features:**
- Parses Debian package format
- Extracts dependencies with version constraints
- Topological sorting by dependencies
- Categorizes into stages (essential, required, proxmox, etc.)
- Generates retry policies per stage

### `proxmox-packagekit`

Translates manifest to PackageKit D-Bus calls using `zbus`.

**Usage:**
```bash
proxmox-packagekit manifest.json [OPTIONS]

Options:
  --dry-run          Don't actually install packages
  --log-file <PATH>  Log file path [default: packagekit-install.log]
```

**Features:**
- Async D-Bus operations via `zbus`
- Batch installation with configurable sizes
- Retry policies (abort, retry_transient, skip)
- Comprehensive logging
- Error recovery

---

## Manifest Format

```json
{
  "version": "1.0",
  "format": "proxmox-packagekit-manifest",
  "target": {
    "distribution": "proxmox-ve",
    "version": "9.0",
    "architecture": "amd64"
  },
  "metadata": {
    "total_packages": 1234,
    "stages": 6,
    "generated_at": "2025-11-15T12:34:56Z"
  },
  "configuration": {
    "default_batch_size": 20,
    "default_retry_policy": "retry_transient",
    "max_retries": 3,
    "retry_delay": 5,
    "continue_on_error": false
  },
  "stages": [
    {
      "name": "essential",
      "description": "ESSENTIAL packages",
      "priority": 1,
      "package_count": 45,
      "batch_size": 10,
      "retry_policy": "abort",
      "continue_on_error": false,
      "packages": [...]
    },
    ...
  ]
}
```

---

## D-Bus Interface

The PackageKit translator uses these D-Bus interfaces:

### `org.freedesktop.PackageKit`
- `CreateTransaction()` - Create new transaction
- `GetDaemonState()` - Query daemon state

### `org.freedesktop.PackageKit.Transaction`
- `InstallPackages(flags, package_ids)` - Install packages
- `RefreshCache(force)` - Update cache
- `Resolve(filters, packages)` - Resolve package names
- `GetDetails(package_ids)` - Get package details

**Signals:**
- `Finished(exit_code, runtime)` - Transaction completed
- `ErrorCode(code, details)` - Error occurred
- `Package(info, package_id, summary)` - Package status update

---

## Installation Stages

1. **Essential** (Priority 1)
   - Core system packages (libc6, bash, coreutils)
   - Batch size: 10
   - Policy: Abort on any failure

2. **Required** (Priority 2)
   - Base Debian system packages
   - Batch size: 20
   - Policy: Retry transient errors

3. **Important** (Priority 3)
   - Important system utilities
   - Batch size: 30
   - Policy: Retry transient errors

4. **Standard** (Priority 4)
   - Standard Debian packages
   - Batch size: 50
   - Policy: Continue on errors

5. **Proxmox** (Priority 5)
   - Proxmox-specific packages (pve-*, proxmox-*)
   - Batch size: 20
   - Policy: Retry transient errors

6. **Optional** (Priority 6)
   - Optional packages
   - Batch size: 50
   - Policy: Skip on errors

---

## Integration with op-dbus

Use the manifest with op-dbus PackageKit plugin:

```json
{
  "version": 1,
  "plugins": {
    "packagekit": {
      "manifest": "/path/to/manifest.json"
    }
  }
}
```

Then apply with:
```bash
op-dbus apply state.json
```

---

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Lint
cargo clippy -- -D warnings
```

---

## Files

- `src/types.rs` - Data structures (Package, Manifest, etc.)
- `src/parser.rs` - Debian package file parser
- `src/manifest.rs` - Manifest generation binary
- `src/packagekit.rs` - PackageKit D-Bus translator binary
- `extract-iso.sh` - Shell script to extract from ISO
- `Cargo.toml` - Rust package configuration

---

## See Also

- [op-dbus PackageKit Integration](../../PACKAGEKIT-INTEGRATION.md)
- [PackageKit D-Bus API](https://www.freedesktop.org/software/PackageKit/gtk-doc/)
- [zbus Documentation](https://docs.rs/zbus/)

---

**License:** MIT
