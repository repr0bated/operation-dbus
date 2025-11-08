#!/bin/bash
# install.sh - op-dbus Installation (Bootstrap + Declarative Apply)
#
# Philosophy:
#   - Imperative bootstrap: Generic prereqs (handled by install-dependencies.sh)
#   - Declarative core: All op-dbus-specific config via state.json + apply
#
# TODO: Evolving script - will be enhanced as we discover requirements
#   - BTRFS subvolume creation for cache storage
#   - NUMA CPU pinning configuration
#   - Advanced MCP component setup

set -euo pipefail

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  op-dbus Installation"
echo "  Minimal Bootstrap + Declarative Apply"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Configuration
BINARY_PATH="/usr/local/bin/op-dbus"
CONFIG_DIR="/etc/op-dbus"
DATA_DIR="/var/lib/op-dbus"
CACHE_DIR="${DATA_DIR}/@cache"
BLOCKCHAIN_DIR="${DATA_DIR}/blockchain"
RUNTIME_DIR="/run/op-dbus"
STATE_FILE="${CONFIG_DIR}/state.json"
SERVICE_FILE="/etc/systemd/system/op-dbus.service"

# Deployment mode (can be overridden with flags)
MODE=""

# Parse command-line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --full)
                MODE="full"
                shift
                ;;
            --standalone)
                MODE="standalone"
                shift
                ;;
            --agent-only)
                MODE="agent"
                shift
                ;;
            --help)
                show_help
                exit 0
                ;;
            *)
                echo "❌ Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

show_help() {
    cat <<EOF
Usage: $0 [MODE]

Deployment Modes:
  --full         Full deployment (Proxmox): D-Bus + Blockchain + LXC + Netmaker
  --standalone   Standalone: D-Bus + Blockchain (no containers)
  --agent-only   Agent only: D-Bus plugins only (minimal)

If no mode is specified, the script will prompt interactively.

Examples:
  sudo ./install.sh --full
  sudo ./install.sh --standalone
  sudo ./install.sh                    # Interactive mode selection

EOF
}

# Check prerequisites
check_prerequisites() {
    echo "━━━ Phase 0: Preflight Checks ━━━"
    echo ""

    # Check root
    if [ "$EUID" -ne 0 ]; then
        echo "❌ This script must be run as root"
        echo "   Run: sudo $0"
        exit 1
    fi

    # Check if binary exists
    if [ ! -f "target/release/op-dbus" ]; then
        echo "❌ Binary not found: target/release/op-dbus"
        echo ""
        echo "Please build first:"
        echo "  ./build.sh"
        echo ""
        echo "Or with features:"
        echo "  cargo build --release --features mcp"
        exit 1
    fi

    echo "✅ op-dbus binary found"

    # Check if openvswitch is installed
    if ! command -v ovs-vsctl &> /dev/null; then
        echo "❌ OpenVSwitch not found"
        echo ""
        echo "Please install dependencies first:"
        echo "  sudo ./install-dependencies.sh"
        exit 1
    fi

    echo "✅ OpenVSwitch installed"

    # Verify OVS is running
    if ! ovs-vsctl show &> /dev/null; then
        echo "⚠️  OVS not responding, attempting to start..."
        systemctl start openvswitch-switch 2>/dev/null || systemctl start openvswitch 2>/dev/null || true
        sleep 2

        if ! ovs-vsctl show &> /dev/null; then
            echo "❌ OpenVSwitch is not running"
            echo "   Please check: systemctl status openvswitch-switch"
            exit 1
        fi
    fi

    echo "✅ OpenVSwitch running"
    echo ""
}

# Select deployment mode
select_mode() {
    if [ -n "$MODE" ]; then
        echo "━━━ Deployment Mode: $MODE (from command line) ━━━"
        echo ""
        return
    fi

    echo "━━━ Phase 1: Deployment Mode Selection ━━━"
    echo ""
    echo "Select deployment mode:"
    echo ""
    echo "  [1] Full (Proxmox)"
    echo "      D-Bus + Blockchain + LXC/Proxmox + Netmaker"
    echo "      For container-based deployments with mesh networking"
    echo ""
    echo "  [2] Privacy Client (WireGuard + Warp + XRay)"
    echo "      3-container privacy router (client side)"
    echo "      WireGuard gateway → Warp tunnel → XRay client → VPS"
    echo "      Socket OpenFlow networking, Level 3 obfuscation"
    echo ""
    echo "  [3] Privacy VPS (XRay Server only)"
    echo "      Single XRay server container (VPS side)"
    echo "      Receives encrypted traffic from privacy clients"
    echo "      Socket OpenFlow networking, Level 2 obfuscation"
    echo ""
    echo "  [4] Standalone"
    echo "      D-Bus + Blockchain (no containers)"
    echo "      OVS bridge + security flows only"
    echo ""
    echo "  [5] Agent Only"
    echo "      D-Bus plugins only (minimal)"
    echo "      For lightweight plugin-only deployments"
    echo ""

    while true; do
        read -rp "Enter choice [1-5]: " CHOICE
        case $CHOICE in
            1)
                MODE="full"
                break
                ;;
            2)
                MODE="privacy-client"
                break
                ;;
            3)
                MODE="privacy-vps"
                break
                ;;
            4)
                MODE="standalone"
                break
                ;;
            5)
                MODE="agent"
                break
                ;;
            *)
                echo "Invalid choice. Please enter 1, 2, 3, 4, or 5."
                ;;
        esac
    done

    echo ""
    echo "Selected mode: $MODE"
    echo ""
}

# Install binary
install_binary() {
    echo "━━━ Phase 2: Binary Installation ━━━"
    echo ""

    echo "⏳ Installing op-dbus binary..."
    cp target/release/op-dbus "$BINARY_PATH"
    chmod 755 "$BINARY_PATH"
    echo "✅ Binary installed: $BINARY_PATH"

    # Verify binary works
    if "$BINARY_PATH" --version &> /dev/null; then
        VERSION=$("$BINARY_PATH" --version)
        echo "✅ Binary verified: $VERSION"
    else
        echo "❌ Binary verification failed"
        exit 1
    fi

    # TODO: Install MCP binaries if built with --features mcp
    # Check for and install: dbus-mcp, dbus-orchestrator, dbus-mcp-web, mcp-chat

    echo ""
}

# Create directory structure
create_directories() {
    echo "━━━ Phase 3: Directory Structure ━━━"
    echo ""

    # Create config directory
    if [ ! -d "$CONFIG_DIR" ]; then
        echo "⏳ Creating config directory: $CONFIG_DIR"
        mkdir -p "$CONFIG_DIR"
        chmod 755 "$CONFIG_DIR"
        echo "✅ Config directory created"
    else
        echo "✅ Config directory exists: $CONFIG_DIR"
    fi

    # Create data directory
    if [ ! -d "$DATA_DIR" ]; then
        echo "⏳ Creating data directory: $DATA_DIR"
        mkdir -p "$DATA_DIR"
        chmod 755 "$DATA_DIR"
        echo "✅ Data directory created"
    else
        echo "✅ Data directory exists: $DATA_DIR"
    fi

    # Create blockchain subdirectories (always created, usage is optional)
    echo "⏳ Creating blockchain storage structure..."
    mkdir -p "${BLOCKCHAIN_DIR}/timing"
    mkdir -p "${BLOCKCHAIN_DIR}/vectors"
    mkdir -p "${BLOCKCHAIN_DIR}/snapshots"
    echo "✅ Blockchain directories created"

    # Create cache directory
    # TODO: Enhance with BTRFS subvolume creation for L3 cache
    # TODO: Add NUMA-aware configuration
    if [ ! -d "$CACHE_DIR" ]; then
        echo "⏳ Creating cache directory: $CACHE_DIR"
        mkdir -p "$CACHE_DIR"
        chmod 755 "$CACHE_DIR"
        echo "✅ Cache directory created"
        echo "   TODO: Convert to BTRFS subvolume for performance"
    else
        echo "✅ Cache directory exists: $CACHE_DIR"
    fi

    # Create runtime directory
    if [ ! -d "$RUNTIME_DIR" ]; then
        echo "⏳ Creating runtime directory: $RUNTIME_DIR"
        mkdir -p "$RUNTIME_DIR"
        chmod 755 "$RUNTIME_DIR"
        echo "✅ Runtime directory created"
    else
        echo "✅ Runtime directory exists: $RUNTIME_DIR"
    fi

    # TODO: Create MCP agent specs directory if MCP feature enabled
    # mkdir -p /etc/op-dbus/agents

    echo ""
}

# Generate declarative state file
generate_state_file() {
    echo "━━━ Phase 4: State File Generation ━━━"
    echo ""

    if [ -f "$STATE_FILE" ]; then
        echo "⚠️  State file already exists: $STATE_FILE"
        read -rp "Overwrite? [y/N]: " OVERWRITE
        if [[ ! "$OVERWRITE" =~ ^[Yy]$ ]]; then
            echo "⏹️  Keeping existing state file"
            echo ""
            return
        fi
    fi

    echo "Generating state file for mode: $MODE"
    echo ""

    # Option 1: Use introspection to auto-detect current system
    read -rp "Use introspection to auto-detect system state? [Y/n]: " USE_INTROSPECT

    if [[ ! "$USE_INTROSPECT" =~ ^[Nn]$ ]]; then
        echo "⏳ Running introspection..."
        if "$BINARY_PATH" init --introspect --output "$STATE_FILE" 2>/dev/null; then
            echo "✅ State file generated via introspection"
        else
            echo "⚠️  Introspection failed, using template instead"
            USE_INTROSPECT="n"
        fi
    fi

    # Option 2: Generate template based on mode
    if [[ "$USE_INTROSPECT" =~ ^[Nn]$ ]]; then
        echo "⏳ Generating template state file..."
        generate_state_template "$MODE" > "$STATE_FILE"
        echo "✅ Template state file created"
    fi

    # Validate JSON
    if jq empty "$STATE_FILE" 2>/dev/null; then
        echo "✅ State file is valid JSON"
    else
        echo "⚠️  State file may have JSON syntax errors"
    fi

    echo "📄 State file: $STATE_FILE"
    echo ""
}

# Generate state template based on mode
generate_state_template() {
    local mode=$1

    case "$mode" in
        full)
            cat <<'EOF'
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": [
        {
          "name": "ovsbr0",
          "type": "ovs-bridge",
          "ports": [],
          "ipv4": {
            "enabled": true,
            "dhcp": false,
            "address": [],
            "gateway": null
          }
        },
        {
          "name": "mesh",
          "type": "ovs-bridge",
          "ports": [],
          "ipv4": {
            "enabled": true,
            "dhcp": false,
            "address": [],
            "gateway": null
          }
        }
      ]
    },
    "systemd": {
      "units": {
        "openvswitch-switch.service": {
          "enabled": true,
          "active_state": "active"
        }
      }
    },
    "lxc": {
      "containers": []
    }
  }
}
EOF
            ;;
        standalone)
            cat <<'EOF'
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": [
        {
          "name": "ovsbr0",
          "type": "ovs-bridge",
          "ports": [],
          "ipv4": {
            "enabled": true,
            "dhcp": false,
            "address": [],
            "gateway": null
          }
        }
      ]
    },
    "systemd": {
      "units": {
        "openvswitch-switch.service": {
          "enabled": true,
          "active_state": "active"
        }
      }
    }
  }
}
EOF
            ;;
        privacy-client)
            cat <<'EOF'
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": [
        {
          "name": "ovsbr0",
          "type": "ovs-bridge",
          "ports": [],
          "ipv4": {
            "enabled": true,
            "dhcp": false,
            "address": ["10.0.0.1/24"],
            "gateway": null
          }
        }
      ]
    },
    "systemd": {
      "units": {
        "openvswitch-switch.service": {
          "enabled": true,
          "active_state": "active"
        }
      }
    },
    "lxc": {
      "container_profile": "privacy-client",
      "containers": [
        {
          "id": 100,
          "name": "wireguard-gateway",
          "template": "debian-12",
          "autostart": true,
          "network": {
            "bridge": "ovsbr0",
            "veth": false,
            "socket_networking": true,
            "port_name": "internal_100",
            "ipv4": "10.0.0.100/24"
          }
        },
        {
          "id": 101,
          "name": "warp-tunnel",
          "template": "debian-12",
          "autostart": true,
          "network": {
            "bridge": "ovsbr0",
            "veth": false,
            "socket_networking": false,
            "wg_tunnel": true,
            "port_name": "wg-warp",
            "ipv4": "10.0.0.101/24"
          }
        },
        {
          "id": 102,
          "name": "xray-client",
          "template": "debian-12",
          "autostart": true,
          "network": {
            "bridge": "ovsbr0",
            "veth": false,
            "socket_networking": true,
            "port_name": "internal_102",
            "ipv4": "10.0.0.102/24"
          }
        }
      ]
    },
    "openflow": {
      "enable_security_flows": true,
      "obfuscation_level": 3,
      "auto_discover_containers": true,
      "flow_policies": [
        {
          "name": "wireguard-to-warp",
          "selector": "container:100",
          "template": {
            "table": 10,
            "priority": 1000,
            "actions": [{"type": "output", "port": "wg-warp"}]
          }
        },
        {
          "name": "warp-to-xray",
          "selector": "container:101",
          "template": {
            "table": 10,
            "priority": 1000,
            "actions": [{"type": "output", "port": "internal_102"}]
          }
        }
      ],
      "bridges": [{
        "name": "ovsbr0",
        "flows": [],
        "socket_ports": [
          {"name": "internal_100", "container_id": "100"},
          {"name": "wg-warp", "container_id": "101"},
          {"name": "internal_102", "container_id": "102"}
        ]
      }]
    }
  }
}
EOF
            ;;
        privacy-vps)
            cat <<'EOF'
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": [
        {
          "name": "ovsbr0",
          "type": "ovs-bridge",
          "ports": [],
          "ipv4": {
            "enabled": true,
            "dhcp": false,
            "address": ["10.0.0.1/24"],
            "gateway": null
          }
        }
      ]
    },
    "systemd": {
      "units": {
        "openvswitch-switch.service": {
          "enabled": true,
          "active_state": "active"
        }
      }
    },
    "lxc": {
      "container_profile": "privacy-vps",
      "containers": [
        {
          "id": 100,
          "name": "xray-server",
          "template": "debian-12",
          "autostart": true,
          "network": {
            "bridge": "ovsbr0",
            "veth": false,
            "socket_networking": true,
            "port_name": "internal_100",
            "ipv4": "10.0.0.100/24"
          }
        }
      ]
    },
    "openflow": {
      "enable_security_flows": true,
      "obfuscation_level": 2,
      "auto_discover_containers": true,
      "flow_policies": [
        {
          "name": "xray-server-forwarding",
          "selector": "container:100",
          "template": {
            "table": 10,
            "priority": 1000,
            "actions": [{"type": "normal"}]
          }
        }
      ],
      "bridges": [{
        "name": "ovsbr0",
        "flows": [],
        "socket_ports": [
          {"name": "internal_100", "container_id": "100"}
        ]
      }]
    }
  }
}
EOF
            ;;
        agent)
            cat <<'EOF'
{
  "version": 1,
  "plugins": {
    "systemd": {
      "units": {}
    }
  }
}
EOF
            ;;
    esac
}

# Create systemd service
create_systemd_service() {
    echo "━━━ Phase 5: Systemd Service ━━━"
    echo ""

    echo "⏳ Creating systemd service file..."

    # Adjust service dependencies based on mode
    local after_clause="After=network-online.target"
    local requires_clause=""

    if [ "$MODE" = "full" ] || [ "$MODE" = "standalone" ] || [ "$MODE" = "privacy-client" ] || [ "$MODE" = "privacy-vps" ]; then
        after_clause="After=network-online.target openvswitch-switch.service"
        requires_clause="Requires=openvswitch-switch.service"
    fi

    cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=op-dbus - Declarative system state management
Documentation=https://github.com/ghostbridge/op-dbus
$after_clause
Wants=network-online.target
$requires_clause

[Service]
Type=simple
ExecStart=$BINARY_PATH run --state-file $STATE_FILE
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=false
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/etc/network/interfaces /run /var/run /etc/dnsmasq.d $DATA_DIR

# Network capabilities
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW

# TODO: Add NUMA CPU pinning configuration
# CPUAffinity=...
# NUMAPolicy=...

[Install]
WantedBy=multi-user.target
EOF

    echo "✅ Service file created: $SERVICE_FILE"

    # Reload systemd
    echo "⏳ Reloading systemd..."
    systemctl daemon-reload
    echo "✅ Systemd reloaded"

    echo ""
}

# Apply declarative state
apply_state() {
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  DECLARATIVE STATE APPLICATION"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "This is where op-dbus installs itself declaratively!"
    echo ""

    # Show what will be applied
    echo "📄 State file: $STATE_FILE"
    echo ""
    echo "━━━ State Preview ━━━"
    if command -v jq &> /dev/null; then
        jq . "$STATE_FILE" 2>/dev/null || cat "$STATE_FILE"
    else
        cat "$STATE_FILE"
    fi
    echo "━━━━━━━━━━━━━━━━━━━━"
    echo ""

    read -rp "Apply this state now? [Y/n]: " APPLY_NOW

    if [[ "$APPLY_NOW" =~ ^[Nn]$ ]]; then
        echo "⏹️  Skipping state application"
        echo ""
        echo "To apply later, run:"
        echo "  sudo $BINARY_PATH apply $STATE_FILE"
        echo ""
        return
    fi

    echo ""
    echo "⏳ Applying declarative state..."
    echo ""

    # Run op-dbus apply
    if "$BINARY_PATH" apply "$STATE_FILE"; then
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "  ✅ STATE APPLIED SUCCESSFULLY"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        echo "This demonstrates the power of declarative state management:"
        echo "  - OVS bridges created"
        echo "  - Network configuration applied"
        echo "  - Services configured"
        echo "  - All from a single state.json file!"
        echo ""
    else
        echo ""
        echo "❌ State application failed"
        echo ""
        echo "Check logs for details:"
        echo "  sudo journalctl -xeu op-dbus.service"
        echo ""
        echo "You can try manual apply:"
        echo "  sudo $BINARY_PATH apply $STATE_FILE"
        echo ""
        return 1
    fi
}

# Service management
manage_service() {
    echo "━━━ Phase 6: Service Management ━━━"
    echo ""

    # Enable service
    read -rp "Enable op-dbus service to start at boot? [Y/n]: " ENABLE_SERVICE

    if [[ ! "$ENABLE_SERVICE" =~ ^[Nn]$ ]]; then
        echo "⏳ Enabling op-dbus service..."
        systemctl enable op-dbus.service
        echo "✅ Service enabled"
    else
        echo "⏹️  Service not enabled"
    fi

    # Start service
    echo ""
    read -rp "Start op-dbus service now? [y/N]: " START_SERVICE

    if [[ "$START_SERVICE" =~ ^[Yy]$ ]]; then
        echo "⏳ Starting op-dbus service..."
        systemctl start op-dbus.service
        sleep 2

        # Check status
        if systemctl is-active --quiet op-dbus.service; then
            echo "✅ Service is running"
        else
            echo "⚠️  Service failed to start"
            echo "   Check: sudo systemctl status op-dbus.service"
        fi
    else
        echo "⏹️  Service not started"
        echo "   To start later: sudo systemctl start op-dbus.service"
    fi

    echo ""
}

# Installation summary
show_summary() {
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  ✅ INSTALLATION COMPLETE"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "Installation Summary:"
    echo "  Mode:          $MODE"
    echo "  Binary:        $BINARY_PATH"
    echo "  Config:        $CONFIG_DIR"
    echo "  State file:    $STATE_FILE"
    echo "  Data:          $DATA_DIR"
    echo "  Service:       $SERVICE_FILE"
    echo ""

    # Show service status
    if systemctl is-enabled --quiet op-dbus.service 2>/dev/null; then
        echo "  Service:       enabled"
    else
        echo "  Service:       disabled"
    fi

    if systemctl is-active --quiet op-dbus.service 2>/dev/null; then
        echo "  Status:        running ✅"
    else
        echo "  Status:        stopped"
    fi

    echo ""
    echo "Useful commands:"
    echo "  Query state:         sudo op-dbus query"
    echo "  Check differences:   sudo op-dbus diff $STATE_FILE"
    echo "  Apply state:         sudo op-dbus apply $STATE_FILE"
    echo "  Service status:      sudo systemctl status op-dbus"
    echo "  View logs:           sudo journalctl -fu op-dbus"
    echo "  Run diagnostics:     sudo op-dbus doctor"
    echo ""

    if [ "$MODE" = "full" ]; then
        echo "Container commands:"
        echo "  List containers:     sudo op-dbus container list"
        echo "  Create container:    sudo op-dbus container create 101"
        echo ""
    fi

    echo "Next steps:"
    echo "  1. Review state file: $STATE_FILE"
    echo "  2. Verify installation: sudo ./verify-installation.sh"
    if [ "$MODE" = "full" ]; then
        echo "  3. Configure Netmaker (optional): echo \"NETMAKER_TOKEN=...\" | sudo tee /etc/op-dbus/netmaker.env"
    fi
    echo ""
    echo "Documentation: README.md, ENTERPRISE-DEPLOYMENT.md"
    echo ""
}

# Main installation flow
main() {
    parse_args "$@"
    check_prerequisites
    select_mode
    install_binary
    create_directories
    generate_state_file
    create_systemd_service
    apply_state
    manage_service
    show_summary
}

# Run installation
main "$@"
