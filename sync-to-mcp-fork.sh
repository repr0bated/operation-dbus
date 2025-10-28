#!/bin/bash

# Sync MCP-related files to a fork repository
# Usage: ./sync-to-mcp-fork.sh [fork-repo-url]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}MCP Fork Sync Tool${NC}"
echo "==================="

# Get fork repository URL from argument or environment variable
FORK_REPO="${1:-${MCP_FORK_REPO}}"

if [ -z "$FORK_REPO" ]; then
    echo -e "${RED}Error: No fork repository specified${NC}"
    echo "Usage: $0 <fork-repo-url>"
    echo "Or set MCP_FORK_REPO environment variable"
    exit 1
fi

echo -e "${YELLOW}Syncing to fork: ${FORK_REPO}${NC}"

# Create temporary directory
TEMP_DIR=$(mktemp -d)
echo "Using temp directory: $TEMP_DIR"

# Function to cleanup on exit
cleanup() {
    echo "Cleaning up..."
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

# Clone the fork
echo -e "${YELLOW}Cloning fork repository...${NC}"
git clone "$FORK_REPO" "$TEMP_DIR/mcp-fork"
cd "$TEMP_DIR/mcp-fork"

# Get the default branch
DEFAULT_BRANCH=$(git symbolic-ref refs/remotes/origin/HEAD | sed 's@^refs/remotes/origin/@@')
git checkout "$DEFAULT_BRANCH"

# Go back to main repo
cd - > /dev/null

# Files and directories to sync
echo -e "${YELLOW}Collecting MCP files...${NC}"

# Core MCP files
MCP_DIRS=(
    "src/mcp"
    "mcp-configs"
    "src/plugin_system"
    "src/event_bus"
)

MCP_FILES=(
    "package.json"
    "mcp.json"
    "claude_desktop_config.json"
    "MCP-README.md"
    "MCP-INTEGRATION.md"
    "MCP-WEB-IMPROVEMENTS.md"
    "COUPLING-FIXES.md"
    "SECURITY-FIXES.md"
)

MCP_DOCS=(
    "docs/MCP-COMPLETE-GUIDE.md"
    "docs/MCP-API-REFERENCE.md"
    "docs/MCP-DEVELOPER-GUIDE.md"
)

# Create structure in fork
echo -e "${YELLOW}Syncing files...${NC}"

# Copy directories
for dir in "${MCP_DIRS[@]}"; do
    if [ -d "$dir" ]; then
        echo "  Syncing $dir..."
        mkdir -p "$TEMP_DIR/mcp-fork/$(dirname "$dir")"
        cp -r "$dir" "$TEMP_DIR/mcp-fork/$(dirname "$dir")/"
    fi
done

# Copy files
for file in "${MCP_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "  Syncing $file..."
        cp "$file" "$TEMP_DIR/mcp-fork/"
    fi
done

# Copy documentation
mkdir -p "$TEMP_DIR/mcp-fork/docs"
for doc in "${MCP_DOCS[@]}"; do
    if [ -f "$doc" ]; then
        echo "  Syncing $doc..."
        cp "$doc" "$TEMP_DIR/mcp-fork/docs/"
    fi
done

# Create a fork-specific README if it doesn't exist
if [ ! -f "$TEMP_DIR/mcp-fork/README.md" ]; then
    echo -e "${YELLOW}Creating fork README...${NC}"
    cat > "$TEMP_DIR/mcp-fork/README.md" << 'EOF'
# MCP D-Bus Server - Fork

This is a fork of the MCP (Model Context Protocol) components from the [operation-dbus](https://github.com/yourusername/operation-dbus) project.

## Overview

A powerful MCP server that provides comprehensive Linux system automation through D-Bus interfaces.

## Features

- 🚀 100+ Auto-discovered Tools
- 🔒 Secure by Default
- 📊 Real-time Monitoring
- 🎯 Multi-Agent System
- 🌐 Web Interface

## Quick Start

```bash
# Build with MCP support
cargo build --release --features mcp

# Run the MCP server
./target/release/dbus-mcp
```

## Documentation

- [Complete Guide](docs/MCP-COMPLETE-GUIDE.md)
- [API Reference](docs/MCP-API-REFERENCE.md)
- [Developer Guide](docs/MCP-DEVELOPER-GUIDE.md)

## Integration

For Claude Desktop, add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "dbus": {
      "command": "dbus-mcp",
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

## Synced from Main Repository

This fork is automatically synced from the main operation-dbus repository.
Last sync: $(date)

## License

MIT License - See LICENSE file for details
EOF
fi

# Create a proper Cargo.toml for the fork
echo -e "${YELLOW}Creating fork Cargo.toml...${NC}"
cat > "$TEMP_DIR/mcp-fork/Cargo.toml" << 'EOF'
[package]
name = "mcp-dbus"
version = "1.0.0"
edition = "2021"
description = "MCP server for D-Bus system automation on Linux"
license = "MIT"
authors = ["Operation D-Bus Team"]

[dependencies]
# Include all MCP-related dependencies from main Cargo.toml
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
zbus = { version = "4", features = ["tokio"] }
uuid = { version = "1.6", features = ["v4"] }
toml = "0.8"
axum = { version = "0.7", features = ["ws"] }
tower-http = { version = "0.5", features = ["fs", "cors"] }
anyhow = "1"
async-trait = "0.1"
once_cell = "1.19"
log = "0.4"
env_logger = "0.10"
tracing = "0.1"
chrono = "0.4"
gethostname = "0.5"
num_cpus = "1.16"
base64 = "0.22"

[features]
default = ["mcp"]
mcp = []

[[bin]]
name = "dbus-mcp"
path = "src/mcp/main.rs"

[[bin]]
name = "dbus-orchestrator"
path = "src/mcp/orchestrator.rs"

[[bin]]
name = "dbus-mcp-web"
path = "src/mcp/web_main.rs"

[[bin]]
name = "dbus-mcp-discovery"
path = "src/mcp/discovery.rs"

[[bin]]
name = "dbus-mcp-discovery-enhanced"
path = "src/mcp/discovery_enhanced.rs"

[[bin]]
name = "dbus-mcp-bridge"
path = "src/mcp/bridge.rs"

# Agent binaries
[[bin]]
name = "dbus-agent-executor"
path = "src/mcp/agents/executor.rs"

[[bin]]
name = "dbus-agent-file"
path = "src/mcp/agents/file.rs"

[[bin]]
name = "dbus-agent-network"
path = "src/mcp/agents/network.rs"

[[bin]]
name = "dbus-agent-systemd"
path = "src/mcp/agents/systemd.rs"

[[bin]]
name = "dbus-agent-monitor"
path = "src/mcp/agents/monitor.rs"
EOF

# Create lib.rs for the fork
echo -e "${YELLOW}Creating fork lib.rs...${NC}"
cat > "$TEMP_DIR/mcp-fork/src/lib.rs" << 'EOF'
//! MCP D-Bus Server Library

pub mod mcp;
pub mod plugin_system;
pub mod event_bus;

// Re-exports
pub use mcp::{agent_registry, tool_registry};
pub use plugin_system::{Plugin, PluginRegistry};
pub use event_bus::{EventBus, Event};
EOF

# Go to fork directory
cd "$TEMP_DIR/mcp-fork"

# Check for changes
if git diff --quiet && git diff --staged --quiet; then
    echo -e "${GREEN}No changes to sync${NC}"
    exit 0
fi

# Add all changes
git add -A

# Show what changed
echo -e "${YELLOW}Changes to be synced:${NC}"
git status --short

# Commit changes
COMMIT_MSG="Sync from main operation-dbus repository $(date +%Y-%m-%d)"
git commit -m "$COMMIT_MSG"

# Ask for confirmation before pushing
echo -e "${YELLOW}Ready to push changes to fork${NC}"
echo -e "Repository: ${FORK_REPO}"
echo -e "Branch: ${DEFAULT_BRANCH}"
read -p "Do you want to push these changes? (y/n) " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    git push origin "$DEFAULT_BRANCH"
    echo -e "${GREEN}Successfully synced to fork!${NC}"
    
    # Show the fork URL
    echo -e "${GREEN}Fork updated at: ${FORK_REPO}${NC}"
else
    echo -e "${YELLOW}Push cancelled${NC}"
fi

# Create a sync marker file
echo "$(date): Synced to $FORK_REPO" >> "$HOME/.mcp-sync-log"

echo -e "${GREEN}Sync complete!${NC}"