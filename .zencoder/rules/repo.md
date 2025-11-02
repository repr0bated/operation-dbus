---
description: Repository Information Overview
alwaysApply: true
---

# Operation D-Bus (op-dbus) Information

## Summary
Operation D-Bus is a declarative system state management framework for Linux that integrates D-Bus protocol with optional MCP (Model Context Protocol) for AI-assisted system automation. Supports containerized deployments via LXC/Proxmox with Netmaker mesh networking, blockchain-based state tracking, and caching with BTRFS snapshots.

## Structure
**Core Modules**:
- `src/state/` - D-Bus plugin system and state management
- `src/mcp/` - Model Context Protocol server and agents
- `src/blockchain/` - State tracking and footprint management
- `src/cache/` - BTRFS snapshot and caching system
- `src/native/` - Native kernel networking (netlink, OVSDB JSON-RPC)
- `src/webui/` - Web UI server for system interaction
- `src/ml/` - Optional ML/transformer vectorization components

**Plugin System** (`src/state/plugins/`): systemd, network, LXC, login1, DNSResolver, PCI, session declaration

## Language & Runtime
**Language**: Rust (2021 edition)
**Compiler**: rustc 1.90.0
**Async Runtime**: Tokio 1.x (multi-threaded)
**Build System**: Cargo
**Package Manager**: Cargo

## Dependencies

**Core Dependencies**:
- zbus 4.x - D-Bus protocol communication
- tokio 1.x - Async runtime with multi-threading
- serde/serde_json - Serialization
- clap 4.x - CLI argument parsing
- axum 0.7 - Web server framework (optional: web feature)
- rtnetlink 0.13 - Native kernel networking
- rusqlite 0.32 - SQLite caching with bundled driver
- sha2, md5, aes-gcm, argon2 - Cryptography
- chrono 0.4 - Time handling

**Optional Dependencies**:
- **ml feature**: ort 2.0, tokenizers, ndarray, hf-hub - Transformer vectorization
- **web feature**: axum, tower, tower-http, mime_guess - Web UI
- **mcp feature**: uuid, toml - MCP server support

**Development Dependencies**: @types/node (Node.js 18+)

## Build & Installation

**Build Variants**:
```bash
# Full build with all features
cargo build --release --all-features

# MCP features only
cargo build --release --features mcp

# Minimal build
cargo build --release --features web
```

**Installation**:
```bash
# Full Proxmox mode (containers, blockchain, netmaker)
sudo ./install.sh

# Enterprise standalone (no containers)
sudo ./install.sh --no-proxmox

# Minimal agent (D-Bus only)
sudo ./install.sh --agent-only
```

## Main Binaries
- **op-dbus** (`src/main.rs`) - Core daemon with state management
- **dbus-mcp** (`src/mcp/main.rs`) - MCP server with D-Bus tools
- **dbus-orchestrator** (`src/mcp/orchestrator.rs`) - MCP orchestration
- **dbus-mcp-web** (`src/mcp/web_main.rs`) - Web UI server
- **Agents** (`src/mcp/agents/`): executor, systemd, file, monitor, network
- **dbus-mcp-discovery** - D-Bus service auto-discovery
- **mcp-chat** - Interactive MCP chat interface

## Testing
**Framework**: Cargo test framework
**Test Locations**: Inline tests in Rust modules; External test scripts
**Script Test Suite** (`*.sh` files):
- `test-introspection.sh` - D-Bus introspection validation
- `test-safe.sh` - Safe system tests
- `test_all_plugins.sh` - Plugin discovery and testing
- `test_system.sh` - Full system integration tests
- `test_discovery.sh` - Service discovery tests
- `test_web.sh` - Web UI functionality tests
- `test_advanced.sh` - Advanced feature tests

**Run Tests**:
```bash
cargo test --features mcp
./build-test.sh
```

## Configuration
**State File**: `/etc/op-dbus/state.json` - Declarative system state definition
**System Service**: systemd unit created by install.sh
**CLI Options**:
- `--state-file` - Path to state configuration
- `--enable-dhcp-server` - Enable DHCP functionality
- Commands: `run`, `apply`, `query`, `diff`, `validate`

## Key Files & Schemas
- **Cargo.toml** - Rust dependencies and binary definitions
- **package.json** - npm metadata (v1.0.0)
- **schemas/smart/** - JSON schema definitions for container specifications
- **mcp-configs/** - MCP server configurations for VSCode, Cursor, systemd
- **templates/** - LXC container templates

## Installation & Deployment
**Binary Installation**: Installed to `/usr/local/bin/op-dbus` by install.sh
**Systemd Service**: Auto-created by installer
**State File**: Auto-generated with system detection (OVS bridges, IPs, gateways)
**Netmaker Integration** (Optional): Auto-detects and configures netmaker interfaces
**Blockchain Storage**: Created for state tracking (unless `--agent-only`)

## Key Operations
```bash
op-dbus run                  # Run daemon
op-dbus query                # Query current state
op-dbus apply <file>         # Apply desired state
op-dbus diff <file>          # Show state differences
op-dbus validate <file>      # Validate state file
cargo run --release --bin dbus-mcp --features mcp  # Start MCP server
```
