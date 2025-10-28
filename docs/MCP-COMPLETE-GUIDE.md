# MCP (Model Context Protocol) Complete Guide

## Table of Contents
1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Components](#components)
4. [Installation](#installation)
5. [Configuration](#configuration)
6. [Usage](#usage)
7. [API Reference](#api-reference)
8. [Development](#development)
9. [Troubleshooting](#troubleshooting)
10. [Security](#security)

## Overview

The MCP (Model Context Protocol) integration in op-dbus provides a powerful bridge between D-Bus services and AI assistants, enabling automated Linux system management through natural language interfaces.

### Key Features

- **Automatic Service Discovery**: Automatically discovers and exposes all D-Bus services
- **Universal Bridge**: Any D-Bus service becomes MCP-accessible without code changes
- **Agent System**: Specialized agents for different system operations
- **Zero Configuration**: Works out-of-the-box with minimal setup
- **AI-Ready**: Designed for integration with Claude, Cursor, and other AI assistants

### What is MCP?

Model Context Protocol (MCP) is a standardized protocol for exposing tools and services to AI assistants. It enables:
- Structured tool definitions
- Type-safe parameter validation
- Consistent error handling
- Streaming responses
- Session management

## Architecture

### System Overview

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   AI Assistant  │────▶│   MCP Server     │────▶│   D-Bus System  │
│  (Claude/Cursor)│◀────│   (dbus-mcp)     │◀────│    Services     │
└─────────────────┘     └──────────────────┘     └─────────────────┘
         │                       │                         │
         │                       ▼                         │
         │              ┌──────────────────┐              │
         └──────────────│   Orchestrator   │──────────────┘
                        │ (Agent Manager)  │
                        └──────────────────┘
                                 │
                ┌────────────────┼────────────────┐
                ▼                ▼                ▼
         ┌──────────┐     ┌──────────┐    ┌──────────┐
         │  Agent   │     │  Agent   │    │  Agent   │
         │ (Systemd)│     │  (File)  │    │(Network) │
         └──────────┘     └──────────┘    └──────────┘
```

### Component Layers

1. **MCP Protocol Layer**
   - JSON-RPC communication
   - Tool discovery and invocation
   - Session management

2. **Bridge Layer**
   - D-Bus introspection
   - Dynamic tool generation
   - Type conversion

3. **Agent Layer**
   - Specialized system operations
   - Security boundaries
   - Resource management

4. **Discovery Layer**
   - Service enumeration
   - Capability detection
   - Configuration mapping

## Components

### Core Binaries

#### dbus-mcp
Main MCP server that handles protocol communication.

**Purpose**: Primary interface between AI assistants and D-Bus system
**Location**: `src/mcp/main.rs`
**Usage**: 
```bash
./dbus-mcp
```

#### dbus-orchestrator
Central agent management service.

**Purpose**: Spawns, manages, and coordinates agents
**Location**: `src/mcp/orchestrator.rs`
**Usage**:
```bash
./dbus-orchestrator
```

#### dbus-mcp-bridge
Generic bridge for any D-Bus service.

**Purpose**: Dynamically exposes D-Bus services as MCP tools
**Location**: `src/mcp/bridge.rs`
**Usage**:
```bash
./dbus-mcp-bridge <service-name> <object-path>
# Example:
./dbus-mcp-bridge org.freedesktop.systemd1 /org/freedesktop/systemd1
```

### Discovery Tools

#### dbus-mcp-discovery
Basic D-Bus service discovery.

**Purpose**: Finds available D-Bus services
**Location**: `src/mcp/discovery.rs`
**Usage**:
```bash
./dbus-mcp-discovery
```

#### dbus-mcp-discovery-enhanced
Advanced discovery with categorization.

**Purpose**: Discovers and categorizes services with detailed introspection
**Location**: `src/mcp/discovery_enhanced.rs`
**Usage**:
```bash
./dbus-mcp-discovery-enhanced
```

### Specialized Agents

#### dbus-agent-systemd
Systemd service management agent.

**Capabilities**:
- Start/stop/restart services
- Check service status
- Enable/disable services
- View logs

**Location**: `src/mcp/agents/systemd.rs`

#### dbus-agent-file
File system operations agent.

**Capabilities**:
- Read/write files
- Directory operations
- Permission management
- Path validation

**Location**: `src/mcp/agents/file.rs`

#### dbus-agent-network
Network configuration agent.

**Capabilities**:
- Interface management
- Connection configuration
- DNS settings
- Routing tables

**Location**: `src/mcp/agents/network.rs`

#### dbus-agent-monitor
System monitoring agent.

**Capabilities**:
- Resource usage
- Process monitoring
- System metrics
- Performance data

**Location**: `src/mcp/agents/monitor.rs`

#### dbus-agent-executor
Command execution agent.

**Capabilities**:
- Safe command execution
- Output streaming
- Environment management
- Process control

**Location**: `src/mcp/agents/executor.rs`

### Utility Components

#### introspection-parser
XML to JSON introspection converter.

**Purpose**: Parses D-Bus XML introspection data
**Location**: `src/mcp/introspection_parser.rs`

#### dbus-mcp-web
Web interface for MCP services.

**Purpose**: Browser-based MCP interaction
**Location**: `src/mcp/web_main.rs`
**Port**: 8080 (default)

## Installation

### Prerequisites

- Rust 1.70+ 
- D-Bus system (installed by default on most Linux distributions)
- systemd (for systemd agent)
- NetworkManager (for network agent)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/operation-dbus.git
cd operation-dbus

# Build with MCP support
cargo build --release --features mcp

# All MCP binaries will be in target/release/
ls target/release/dbus-*
```

### Quick Installation

```bash
# Run the installation script
./install-mcp-configs.sh

# This will:
# 1. Build all MCP components
# 2. Discover available D-Bus services
# 3. Generate MCP configurations
# 4. Set up your MCP client
```

### Manual Installation

1. **Build the binaries**:
```bash
cargo build --release --features mcp
```

2. **Install system-wide (optional)**:
```bash
sudo cp target/release/dbus-mcp* /usr/local/bin/
```

3. **Set up systemd service (optional)**:
```bash
sudo cp mcp-configs/systemd/dbus-orchestrator.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable dbus-orchestrator
sudo systemctl start dbus-orchestrator
```

4. **Configure MCP client**:
```bash
# For Cursor
cp mcp-configs/cursor/mcp.json ~/.config/Claude/

# For VSCode
cp mcp-configs/vscode/mcp.json ~/.vscode/
```

## Configuration

### Discovery Configuration

Location: `mcp-configs/discovery_config.toml`

```toml
# Service discovery configuration
[discovery]
# Enable automatic discovery
auto_discover = true
# Discovery interval in seconds
interval = 60
# Maximum services to discover
max_services = 100

# Service categories for organization
[categories]
system = ["systemd", "logind", "polkit", "upower"]
network = ["NetworkManager", "resolved", "avahi"]
desktop = ["gnome", "kde", "xfce", "cinnamon"]
audio = ["pulseaudio", "pipewire"]
storage = ["udisks2", "storaged"]

# Service filters
[filters]
# Ignore these services
ignore = ["org.freedesktop.DBus", "org.freedesktop.IBus"]
# Only include services matching these patterns
include_patterns = ["org.*", "com.*"]
```

### Agent Configuration

Location: `mcp-configs/agents/`

#### Orchestrator Configuration
`dbus-orchestrator.json`:
```json
{
  "max_agents": 10,
  "agent_timeout": 300,
  "allowed_agents": [
    "systemd",
    "file",
    "network",
    "monitor",
    "executor"
  ],
  "resource_limits": {
    "max_memory_mb": 100,
    "max_cpu_percent": 50
  }
}
```

#### Executor Agent Configuration
`dbus-executor.json`:
```json
{
  "allowed_commands": [
    "ls", "cat", "grep", "ps", "top", "df", "du"
  ],
  "forbidden_paths": [
    "/etc/shadow",
    "/etc/passwd",
    "/root"
  ],
  "max_output_size": 1048576,
  "command_timeout": 30
}
```

### MCP Client Configuration

#### Cursor Configuration
`mcp-configs/cursor/mcp.json`:
```json
{
  "mcpServers": {
    "dbus-mcp": {
      "command": "/usr/local/bin/dbus-mcp",
      "args": [],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

#### VSCode Configuration
`mcp-configs/vscode/mcp.json`:
```json
{
  "mcp.servers": [
    {
      "name": "dbus-mcp",
      "command": "/usr/local/bin/dbus-mcp",
      "args": [],
      "env": {
        "DBUS_SESSION_BUS_ADDRESS": "${env:DBUS_SESSION_BUS_ADDRESS}"
      }
    }
  ]
}
```

## Usage

### Basic Usage

1. **Start the MCP server**:
```bash
# Basic server
./dbus-mcp

# With debug logging
RUST_LOG=debug ./dbus-mcp
```

2. **With Orchestrator** (recommended):
```bash
# Start orchestrator first
./dbus-orchestrator &

# Then start MCP server
./dbus-mcp
```

3. **Web Interface**:
```bash
# Start web server
./dbus-mcp-web

# Access at http://localhost:8080
```

### Tool Discovery

List available tools:
```bash
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | ./dbus-mcp
```

### Tool Invocation

Call a tool:
```bash
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"systemd_status","arguments":{"service":"nginx"}},"id":2}' | ./dbus-mcp
```

### Using with AI Assistants

#### Claude/Cursor Integration

1. Ensure MCP server is running
2. Configure Claude/Cursor with the MCP endpoint
3. Tools will be automatically available to the assistant

Example prompts:
- "Check the status of the nginx service"
- "List all running systemd services"
- "Show network interfaces"
- "Monitor system resources"

### Advanced Usage

#### Custom Bridge for Specific Service

```bash
# Bridge for NetworkManager
./dbus-mcp-bridge org.freedesktop.NetworkManager /org/freedesktop/NetworkManager

# Bridge for custom service
./dbus-mcp-bridge com.mycompany.MyService /com/mycompany/MyService
```

#### Batch Operations

```bash
# Create a batch file
cat > batch.jsonl << EOF
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"systemd_status","arguments":{"service":"nginx"}},"id":1}
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"systemd_status","arguments":{"service":"postgresql"}},"id":2}
EOF

# Execute batch
cat batch.jsonl | ./dbus-mcp
```

## API Reference

### MCP Protocol

#### Initialize
```json
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {}
  },
  "id": 1
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": {}
    },
    "serverInfo": {
      "name": "dbus-mcp",
      "version": "0.1.0"
    }
  },
  "id": 1
}
```

#### List Tools
```json
{
  "jsonrpc": "2.0",
  "method": "tools/list",
  "id": 2
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "tools": [
      {
        "name": "systemd_status",
        "description": "Check systemd service status",
        "inputSchema": {
          "type": "object",
          "properties": {
            "service": {
              "type": "string",
              "description": "Service name"
            }
          },
          "required": ["service"]
        }
      }
    ]
  },
  "id": 2
}
```

#### Call Tool
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "systemd_status",
    "arguments": {
      "service": "nginx"
    }
  },
  "id": 3
}
```

### Agent D-Bus API

#### Orchestrator Interface
```
Interface: org.dbusmcp.Orchestrator
Path: /org/dbusmcp/Orchestrator

Methods:
  SpawnAgent(agent_type: String) -> agent_id: String
  SendTask(agent_id: String, task_json: String) -> result: String
  GetAgentStatus(agent_id: String) -> status: String
  ListAgents() -> agents: Array<String>
  KillAgent(agent_id: String) -> success: Boolean

Signals:
  AgentSpawned(agent_id: String, agent_type: String)
  AgentDied(agent_id: String, reason: String)
  TaskCompleted(agent_id: String, task_id: String, result: String)
```

#### Agent Interface
```
Interface: org.dbusmcp.Agent.<Type>
Path: /org/dbusmcp/Agent/<Type>/<ID>

Methods:
  ExecuteTask(task_json: String) -> result: String
  GetStatus() -> status: String
  Shutdown() -> success: Boolean

Signals:
  TaskStarted(task_id: String)
  TaskProgress(task_id: String, progress: Integer)
  TaskCompleted(task_id: String, result: String)
```

## Development

### Adding a New Agent

1. **Create agent source file**:
```rust
// src/mcp/agents/myagent.rs
use serde::{Deserialize, Serialize};
use zbus::{dbus_interface, ConnectionBuilder};

struct MyAgent {
    // Agent state
}

#[dbus_interface(name = "org.dbusmcp.Agent.MyAgent")]
impl MyAgent {
    async fn execute_task(&self, task_json: String) -> String {
        // Implementation
    }
}
```

2. **Add to Cargo.toml**:
```toml
[[bin]]
name = "dbus-agent-myagent"
path = "src/mcp/agents/myagent.rs"
required-features = ["mcp"]
```

3. **Register with orchestrator**:
```rust
// In orchestrator configuration
"allowed_agents": ["myagent", ...]
```

### Extending Discovery

1. **Add service patterns**:
```toml
# discovery_config.toml
[categories]
mycategory = ["myservice", "pattern*"]
```

2. **Custom introspection**:
```rust
// Add to discovery_enhanced.rs
fn categorize_service(name: &str) -> String {
    // Custom categorization logic
}
```

### Testing

#### Unit Tests
```bash
cargo test --features mcp
```

#### Integration Tests
```bash
# Test discovery
./test_discovery.sh

# Test web interface
./test_web.sh

# Test system integration
./test_system.sh
```

#### Manual Testing
```bash
# Test with netcat
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | nc -U /tmp/mcp.sock

# Test with curl (web interface)
curl -X POST http://localhost:8080/api/tools \
  -H "Content-Type: application/json" \
  -d '{"method":"list"}'
```

## Troubleshooting

### Common Issues

#### MCP Server Won't Start

**Problem**: `Error: Could not connect to D-Bus`

**Solution**:
1. Check D-Bus is running:
```bash
systemctl status dbus
```

2. Verify session bus:
```bash
echo $DBUS_SESSION_BUS_ADDRESS
```

3. Start with system bus:
```bash
DBUS_SESSION_BUS_ADDRESS=unix:path=/var/run/dbus/system_bus_socket ./dbus-mcp
```

#### No Tools Available

**Problem**: `tools/list` returns empty array

**Solution**:
1. Run discovery:
```bash
./dbus-mcp-discovery-enhanced
```

2. Check orchestrator:
```bash
busctl --user introspect org.dbusmcp.Orchestrator /org/dbusmcp/Orchestrator
```

3. Verify agents are allowed:
```bash
cat mcp-configs/agents/dbus-orchestrator.json
```

#### Agent Fails to Start

**Problem**: `Failed to spawn agent: <type>`

**Solution**:
1. Check agent binary exists:
```bash
ls target/release/dbus-agent-*
```

2. Run agent manually:
```bash
RUST_LOG=debug ./dbus-agent-systemd
```

3. Check permissions:
```bash
# Some agents need elevated permissions
sudo ./dbus-agent-network
```

#### Connection Timeout

**Problem**: `Timeout waiting for response`

**Solution**:
1. Increase timeout:
```bash
MCP_TIMEOUT=60 ./dbus-mcp
```

2. Check system load:
```bash
top
```

3. Enable debug logging:
```bash
RUST_LOG=debug ./dbus-mcp 2>debug.log
```

### Debug Logging

Enable detailed logging:
```bash
# All components
RUST_LOG=debug ./dbus-mcp

# Specific module
RUST_LOG=dbus_mcp=debug ./dbus-mcp

# Trace level
RUST_LOG=trace ./dbus-mcp
```

### Performance Tuning

#### Optimize Discovery
```toml
# discovery_config.toml
[discovery]
max_services = 50  # Reduce if slow
interval = 120     # Increase interval
```

#### Agent Resource Limits
```json
// dbus-orchestrator.json
{
  "resource_limits": {
    "max_memory_mb": 50,
    "max_cpu_percent": 25
  }
}
```

#### Connection Pooling
```bash
# Enable connection reuse
MCP_CONNECTION_POOL=true ./dbus-mcp
```

## Security

### Security Model

1. **Process Isolation**
   - Each agent runs in separate process
   - Limited inter-agent communication
   - Resource limits enforced

2. **D-Bus Security**
   - Uses D-Bus security policies
   - Session bus for user operations
   - System bus requires authentication

3. **Input Validation**
   - JSON schema validation
   - Parameter type checking
   - Command injection prevention

### Best Practices

#### Running in Production

1. **Use systemd service**:
```ini
[Service]
Type=simple
User=mcp-service
Group=mcp-service
PrivateTmp=true
NoNewPrivileges=true
```

2. **Restrict file access**:
```json
// dbus-executor.json
{
  "forbidden_paths": [
    "/etc/shadow",
    "/etc/passwd",
    "/root",
    "/home/*/.ssh"
  ]
}
```

3. **Limit network access**:
```bash
# Use firewall rules
iptables -A OUTPUT -m owner --uid-owner mcp-service -j DROP
```

#### Security Checklist

- [ ] Run as non-root user
- [ ] Enable systemd hardening options
- [ ] Configure resource limits
- [ ] Restrict file system access
- [ ] Use D-Bus policies
- [ ] Enable audit logging
- [ ] Regular security updates
- [ ] Monitor agent behavior

### Audit Logging

Enable audit logging:
```bash
# Set audit log path
MCP_AUDIT_LOG=/var/log/mcp-audit.log ./dbus-mcp

# Log format
# timestamp | user | action | result | details
```

### Security Vulnerabilities

Report security issues to: security@example.com

## Appendix

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level | `info` |
| `DBUS_SESSION_BUS_ADDRESS` | D-Bus session | System default |
| `MCP_CONFIG_DIR` | Configuration directory | `mcp-configs/` |
| `MCP_TIMEOUT` | Request timeout (seconds) | `30` |
| `MCP_MAX_AGENTS` | Maximum agents | `10` |
| `MCP_AUDIT_LOG` | Audit log path | None |
| `MCP_CONNECTION_POOL` | Enable connection pooling | `false` |

### File Locations

| File/Directory | Purpose |
|----------------|---------|
| `/usr/local/bin/dbus-mcp*` | Installed binaries |
| `/etc/dbus-mcp/` | System configuration |
| `~/.config/dbus-mcp/` | User configuration |
| `/var/log/dbus-mcp/` | Log files |
| `/run/dbus-mcp/` | Runtime data |

### Related Documentation

- [MCP Protocol Specification](https://github.com/anthropics/mcp)
- [D-Bus Specification](https://dbus.freedesktop.org/doc/dbus-specification.html)
- [systemd D-Bus API](https://www.freedesktop.org/wiki/Software/systemd/dbus/)
- [NetworkManager D-Bus API](https://developer.gnome.org/NetworkManager/stable/)

### Glossary

| Term | Definition |
|------|------------|
| **MCP** | Model Context Protocol - Protocol for AI assistant tool integration |
| **D-Bus** | Desktop Bus - IPC system for Linux |
| **Agent** | Specialized process for specific operations |
| **Orchestrator** | Central manager for agents |
| **Introspection** | D-Bus service self-description mechanism |
| **Bridge** | Component that translates between protocols |

### Version History

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2024-01 | Initial MCP integration |
| 0.2.0 | TBD | Enhanced discovery, web interface |

### License

MIT License - See LICENSE file for details