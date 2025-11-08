#!/bin/bash
# op-dbus Complete Infrastructure Installation Script
# Supports: Proxmox, Standalone, Agent-only modes
# Creates: BTRFS subvolumes, OVS bridges, container templates, Netmaker mesh

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/op-dbus"
DATA_DIR="/var/lib/op-dbus"
LOG_FILE="/var/log/op-dbus-install.log"

# Deployment modes
MODE="full"  # full, standalone, agent-only

# Component flags
SETUP_BTRFS=true
SETUP_OVS=true
SETUP_CONTAINERS=true
SETUP_NETMAKER=false
SETUP_NIXOS=false

# OVS Configuration
MESH_BRIDGE="mesh"
OPENFLOW_CONTROLLER="tcp:127.0.0.1:6653"

# Logging
log() {
    echo -e "${2:-$NC}$1${NC}" | tee -a "$LOG_FILE"
}

header() {
    echo "" | tee -a "$LOG_FILE"
    log "================================================" "$BLUE"
    log "$1" "$BLUE"
    log "================================================" "$BLUE"
}

success() {
    log "✓ $1" "$GREEN"
}

warn() {
    log "⚠ $1" "$YELLOW"
}

error() {
    log "✗ $1" "$RED"
}

# Check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        error "Must run as root"
        exit 1
    fi
}

# Parse arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --mode=*)
                MODE="${1#*=}"
                shift
                ;;
            --no-proxmox)
                MODE="standalone"
                shift
                ;;
            --agent-only)
                MODE="agent-only"
                shift
                ;;
            --enable-netmaker)
                SETUP_NETMAKER=true
                shift
                ;;
            --enable-nixos)
                SETUP_NIXOS=true
                shift
                ;;
            --help)
                show_help
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done

    # Adjust component flags based on mode
    case $MODE in
        agent-only)
            SETUP_BTRFS=false
            SETUP_OVS=false
            SETUP_CONTAINERS=false
            ;;
        standalone)
            SETUP_CONTAINERS=false
            ;;
        full)
            # All enabled by default
            ;;
    esac
}

show_help() {
    cat << EOF
op-dbus Installation Script

Usage: $0 [OPTIONS]

Deployment Modes:
  --mode=full           Full Proxmox mode (default): D-Bus + Blockchain + LXC + Netmaker
  --mode=standalone     Enterprise mode: D-Bus + Blockchain (no containers)
  --mode=agent-only     Minimal mode: D-Bus plugins only
  --no-proxmox          Alias for --mode=standalone
  --agent-only          Alias for --mode=agent-only

Optional Features:
  --enable-netmaker     Enable Netmaker mesh networking
  --enable-nixos        Enable NixOS integration paths

Examples:
  $0                              # Full Proxmox installation
  $0 --no-proxmox                 # Enterprise standalone
  $0 --agent-only                 # Minimal D-Bus agent
  $0 --enable-netmaker            # Full with Netmaker mesh

EOF
}

# Detect system capabilities
detect_system() {
    header "Detecting System Capabilities"

    # Check for Proxmox
    if command -v pct &>/dev/null; then
        success "Proxmox detected (pct available)"
        export HAS_PROXMOX=true
    else
        warn "Proxmox not detected (pct not found)"
        export HAS_PROXMOX=false
        if [ "$MODE" = "full" ]; then
            warn "Full mode requested but Proxmox not available, switching to standalone"
            MODE="standalone"
            SETUP_CONTAINERS=false
        fi
    fi

    # Check for BTRFS
    if df -T "$DATA_DIR" 2>/dev/null | grep -q btrfs || df -T /var/lib 2>/dev/null | grep -q btrfs; then
        success "BTRFS filesystem detected"
        export HAS_BTRFS=true
    else
        warn "BTRFS not detected, will use regular directories"
        export HAS_BTRFS=false
        SETUP_BTRFS=false
    fi

    # Check for OVS
    if command -v ovs-vsctl &>/dev/null; then
        success "Open vSwitch detected"
        export HAS_OVS=true
    else
        warn "Open vSwitch not found (install: apt install openvswitch-switch)"
        export HAS_OVS=false
        if [ "$SETUP_OVS" = true ]; then
            warn "OVS setup requested but not available, skipping"
            SETUP_OVS=false
        fi
    fi

    # Check for Netmaker client
    if command -v netclient &>/dev/null; then
        success "Netmaker netclient detected"
        export HAS_NETCLIENT=true
    else
        log "Netclient not found (optional)"
        export HAS_NETCLIENT=false
    fi

    log ""
    log "Deployment Mode: $MODE" "$BLUE"
    log "BTRFS Setup: $SETUP_BTRFS"
    log "OVS Setup: $SETUP_OVS"
    log "Container Setup: $SETUP_CONTAINERS"
    log "Netmaker Setup: $SETUP_NETMAKER"
}

# Create directory structure
create_directories() {
    header "Creating Directory Structure"

    mkdir -p "$CONFIG_DIR"
    mkdir -p "$DATA_DIR"
    mkdir -p "$INSTALL_DIR"

    success "Created base directories"
}

# Setup BTRFS subvolumes
setup_btrfs_subvolumes() {
    if [ "$SETUP_BTRFS" != true ]; then
        return
    fi

    header "Setting Up BTRFS Subvolumes"

    # Ensure parent directory exists
    mkdir -p "$DATA_DIR"

    # Helper function to create subvolume if it doesn't exist
    create_subvolume() {
        local path="$1"
        local description="$2"

        if [ -d "$path" ]; then
            # Check if it's already a subvolume
            if btrfs subvolume show "$path" &>/dev/null; then
                warn "Subvolume already exists: $path"
                return 0
            else
                # It's a regular directory, back it up and convert
                warn "Converting regular directory to subvolume: $path"
                local backup="${path}.backup.$(date +%s)"
                mv "$path" "$backup"
                btrfs subvolume create "$path"
                if [ -d "$backup" ] && [ "$(ls -A $backup)" ]; then
                    cp -a "$backup"/* "$path"/
                    rm -rf "$backup"
                fi
            fi
        else
            btrfs subvolume create "$path"
        fi

        success "$description: $path"
    }

    # Create main subvolumes
    create_subvolume "${DATA_DIR}/@blockchain" "Blockchain data subvolume"
    create_subvolume "${DATA_DIR}/@blockchain/timing" "Blockchain timing subvolume"
    create_subvolume "${DATA_DIR}/@blockchain/vectors" "Blockchain vectors subvolume"

    create_subvolume "${DATA_DIR}/@cache" "Cache subvolume"
    create_subvolume "${DATA_DIR}/@cache/embeddings" "Embeddings cache"
    create_subvolume "${DATA_DIR}/@cache/blocks" "Blocks cache"
    create_subvolume "${DATA_DIR}/@cache/queries" "Queries cache"

    create_subvolume "${DATA_DIR}/@plugins" "Plugins data subvolume"
    create_subvolume "${DATA_DIR}/@plugins/lxc" "LXC plugin data"
    create_subvolume "${DATA_DIR}/@plugins/net" "Network plugin data"
    create_subvolume "${DATA_DIR}/@plugins/systemd" "Systemd plugin data"

    # Set compression on cache (zstd:3 for good compression/speed balance)
    log "Setting compression on cache subvolumes..."
    btrfs property set "${DATA_DIR}/@cache" compression zstd
    btrfs property set "${DATA_DIR}/@blockchain/vectors" compression zstd
    success "Compression enabled (zstd)"

    # Create templates subvolume if in container mode
    if [ "$SETUP_CONTAINERS" = true ]; then
        create_subvolume "${DATA_DIR}/@templates" "Container templates subvolume"
        success "Container template infrastructure ready"
    fi
}

# Setup regular directories (non-BTRFS)
setup_regular_directories() {
    if [ "$SETUP_BTRFS" = true ]; then
        return
    fi

    header "Setting Up Regular Directories (non-BTRFS)"

    mkdir -p "${DATA_DIR}/blockchain/timing"
    mkdir -p "${DATA_DIR}/blockchain/vectors"
    mkdir -p "${DATA_DIR}/cache/embeddings"
    mkdir -p "${DATA_DIR}/cache/blocks"
    mkdir -p "${DATA_DIR}/cache/queries"
    mkdir -p "${DATA_DIR}/plugins/lxc"
    mkdir -p "${DATA_DIR}/plugins/net"
    mkdir -p "${DATA_DIR}/plugins/systemd"

    if [ "$SETUP_CONTAINERS" = true ]; then
        mkdir -p "${DATA_DIR}/templates"
    fi

    success "Regular directory structure created"
}

# Setup OVS bridges
setup_ovs_bridges() {
    if [ "$SETUP_OVS" != true ]; then
        return
    fi

    header "Setting Up Open vSwitch Bridges"

    # Start OVS services
    log "Starting OVS services..."
    systemctl start ovsdb-server ovs-vswitchd 2>/dev/null || true
    systemctl enable ovsdb-server ovs-vswitchd 2>/dev/null || true
    sleep 2

    # Verify OVS is responding
    if ! ovs-vsctl show &>/dev/null; then
        error "OVS not responding, restarting..."
        systemctl restart ovsdb-server ovs-vswitchd
        sleep 3
        if ! ovs-vsctl show &>/dev/null; then
            error "OVS still not responding, cannot continue"
            return 1
        fi
    fi

    success "OVS services running"

    # Create mesh bridge for container networking
    log "Creating mesh bridge..."
    if ovs-vsctl list-br | grep -q "^${MESH_BRIDGE}$"; then
        warn "Bridge already exists: $MESH_BRIDGE"
    else
        ovs-vsctl add-br "$MESH_BRIDGE" -- set bridge "$MESH_BRIDGE" datapath_type=system
        success "Created bridge: $MESH_BRIDGE"
    fi

    # Configure mesh bridge
    ovs-vsctl set bridge "$MESH_BRIDGE" stp_enable=false
    ovs-vsctl set-controller "$MESH_BRIDGE" "$OPENFLOW_CONTROLLER"
    success "Configured mesh bridge (STP disabled, OpenFlow enabled)"

    # Wait for bridge to appear in kernel
    for i in {1..10}; do
        if ip link show "$MESH_BRIDGE" &>/dev/null; then
            success "Bridge visible in kernel: $MESH_BRIDGE"
            break
        fi
        sleep 1
    done

    # Initialize OpenFlow tables for socket networking
    log "Setting up socket networking OpenFlow tables..."

    # Table 0: Ingress classification
    # Default: drop unknown traffic
    ovs-ofctl add-flow "$MESH_BRIDGE" "table=0,priority=0,actions=drop" 2>/dev/null || \
        ovs-ofctl mod-flows "$MESH_BRIDGE" "table=0,priority=0,actions=drop"

    # Table 1: Policy/routing (populated per-container)
    ovs-ofctl add-flow "$MESH_BRIDGE" "table=1,priority=0,actions=drop" 2>/dev/null || \
        ovs-ofctl mod-flows "$MESH_BRIDGE" "table=1,priority=0,actions=drop"

    # Allow LOCAL port to send/receive (for host-container communication)
    ovs-ofctl add-flow "$MESH_BRIDGE" "table=0,priority=100,in_port=LOCAL,actions=normal" 2>/dev/null || \
        ovs-ofctl mod-flows "$MESH_BRIDGE" "table=0,priority=100,in_port=LOCAL,actions=normal"

    success "Socket networking flows initialized"
}

# Setup Netmaker integration
setup_netmaker() {
    if [ "$SETUP_NETMAKER" != true ]; then
        return
    fi

    header "Setting Up Netmaker Integration"

    # Check for enrollment token
    local token_file="${CONFIG_DIR}/netmaker.env"
    if [ -f "$token_file" ]; then
        source "$token_file"
        if [ -n "${NETMAKER_TOKEN:-}" ]; then
            success "Found Netmaker enrollment token"

            # Join host to netmaker if not already joined
            if [ "$HAS_NETCLIENT" = true ]; then
                if netclient list 2>/dev/null | grep -q "Connected networks"; then
                    success "Host already joined to Netmaker"
                else
                    log "Joining host to Netmaker..."
                    if netclient join -t "$NETMAKER_TOKEN"; then
                        success "Host joined to Netmaker"
                    else
                        error "Failed to join Netmaker"
                    fi
                fi

                # Detect and add netmaker interfaces to mesh bridge
                if [ "$HAS_OVS" = true ]; then
                    log "Detecting Netmaker interfaces..."
                    for iface in $(ip -br link | grep "^nm-" | awk '{print $1}'); do
                        if ! ovs-vsctl list-ports "$MESH_BRIDGE" | grep -q "^${iface}$"; then
                            ovs-vsctl add-port "$MESH_BRIDGE" "$iface"
                            success "Added Netmaker interface to mesh: $iface"
                        else
                            warn "Netmaker interface already on mesh: $iface"
                        fi
                    done
                fi
            else
                warn "Netclient not installed, skipping host enrollment"
            fi
        else
            warn "NETMAKER_TOKEN not set in $token_file"
        fi
    else
        warn "Netmaker token file not found: $token_file"
        log "To enable Netmaker: echo 'NETMAKER_TOKEN=your-token' > $token_file"
    fi
}

# Create container template subvolume
setup_container_templates() {
    if [ "$SETUP_CONTAINERS" != true ] || [ "$SETUP_BTRFS" != true ]; then
        return
    fi

    header "Setting Up Container Template Infrastructure"

    local template_dir="${DATA_DIR}/@templates"

    # Create base template marker
    log "Creating container template markers..."
    mkdir -p "${template_dir}/base"
    cat > "${template_dir}/base/README" << 'EOF'
Container Template Subvolume

This subvolume serves as the base for container templates.
LXC containers can be cloned from snapshots of this subvolume.

Template variants:
- netmaker: Pre-configured for Netmaker mesh networking
- socket: Optimized for socket networking
- nixos: NixOS container support

When creating containers via op-dbus, these templates are used
for instant CoW (copy-on-write) container creation.
EOF

    success "Container template infrastructure created"

    # Create template variants directory structure
    mkdir -p "${template_dir}/netmaker"
    mkdir -p "${template_dir}/socket"

    if [ "$SETUP_NIXOS" = true ]; then
        mkdir -p "${template_dir}/nixos"
    fi

    # Create netmaker enrollment script template
    log "Creating Netmaker enrollment script template..."
    cat > "${template_dir}/netmaker/enrollment.sh" << 'EOF'
#!/bin/bash
# Container Netmaker Enrollment Script
# This script runs on first boot to join the container to Netmaker

set -e

TOKEN_FILE="/etc/netmaker/enrollment-token"
MARKER_FILE="/var/lib/netmaker-enrolled"

# Exit if already enrolled
[ -f "$MARKER_FILE" ] && exit 0

# Wait for network
sleep 5

# Read token
if [ -f "$TOKEN_FILE" ]; then
    TOKEN=$(cat "$TOKEN_FILE")
    if [ -n "$TOKEN" ] && command -v netclient &>/dev/null; then
        if netclient join -t "$TOKEN"; then
            touch "$MARKER_FILE"
            echo "Successfully joined Netmaker network"
        fi
    fi
fi
EOF
    chmod +x "${template_dir}/netmaker/enrollment.sh"

    success "Template enrollment scripts created"
}

# Install op-dbus binary
install_binary() {
    header "Installing op-dbus Binary"

    if [ -f "target/release/op-dbus" ]; then
        cp "target/release/op-dbus" "$INSTALL_DIR/op-dbus"
        chmod +x "$INSTALL_DIR/op-dbus"
        success "Installed: $INSTALL_DIR/op-dbus"
    else
        warn "Binary not found at target/release/op-dbus"
        warn "Build first: cargo build --release"
    fi
}

# Create systemd service
install_systemd_service() {
    header "Installing Systemd Service"

    local service_file="/etc/systemd/system/op-dbus.service"

    cat > "$service_file" << EOF
[Unit]
Description=op-dbus - Declarative System State Management
Documentation=https://github.com/repr0bated/operation-dbus
After=network-online.target
Wants=network-online.target
EOF

    # Add OVS dependency if enabled
    if [ "$SETUP_OVS" = true ]; then
        cat >> "$service_file" << EOF
After=openvswitch-switch.service
Requires=openvswitch-switch.service
EOF
    fi

    cat >> "$service_file" << EOF

[Service]
Type=simple
ExecStart=$INSTALL_DIR/op-dbus run --state-file $CONFIG_DIR/state.json
Restart=on-failure
RestartSec=10s
StandardOutput=journal
StandardError=journal

# Environment
Environment="OP_DBUS_VECTOR_LEVEL=none"
Environment="OPDBUS_MAX_CACHE_SNAPSHOTS=24"
Environment="OPDBUS_SNAPSHOT_INTERVAL=every-15-minutes"

# Security hardening
NoNewPrivileges=false
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes

# Required paths
ReadWritePaths=$DATA_DIR $CONFIG_DIR /run /var/run

# Network capabilities
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_SYS_ADMIN
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW CAP_SYS_ADMIN

[Install]
WantedBy=multi-user.target
EOF

    success "Created systemd service: $service_file"

    # Reload systemd
    systemctl daemon-reload
    success "Systemd reloaded"

    # Enable service
    systemctl enable op-dbus.service
    success "Service enabled for boot"
}

# Create initial state.json
create_initial_state() {
    header "Creating Initial State Configuration"

    local state_file="${CONFIG_DIR}/state.json"

    if [ -f "$state_file" ]; then
        warn "State file already exists: $state_file"
        return
    fi

    log "Generating state.json..."

    cat > "$state_file" << 'EOF'
{
  "version": 1,
  "plugins": {
    "systemd": {
      "units": {
        "openvswitch-switch.service": {
          "active_state": "active",
          "enabled": true
        }
      }
    }
  }
}
EOF

    success "Created initial state: $state_file"
    log "Edit $state_file to customize your infrastructure"
}

# Setup NixOS integration
setup_nixos_integration() {
    if [ "$SETUP_NIXOS" != true ]; then
        return
    fi

    header "Setting Up NixOS Integration"

    # Create NixOS-compatible paths
    mkdir -p /nix/var/nix/profiles
    mkdir -p /nix/store

    # Create op-dbus profile link
    if [ -f "$INSTALL_DIR/op-dbus" ]; then
        local nix_profile="/nix/var/nix/profiles/op-dbus"
        ln -sf "$INSTALL_DIR/op-dbus" "$nix_profile"
        success "Created NixOS profile link"
    fi

    # Create flake-compatible metadata
    cat > "${CONFIG_DIR}/flake-info.json" << EOF
{
  "version": "0.1.0",
  "installed": "$(date -Iseconds)",
  "mode": "$MODE",
  "features": {
    "btrfs": $SETUP_BTRFS,
    "ovs": $SETUP_OVS,
    "containers": $SETUP_CONTAINERS,
    "netmaker": $SETUP_NETMAKER
  }
}
EOF

    success "NixOS integration paths created"
}

# Print summary
print_summary() {
    header "Installation Summary"

    log "Deployment Mode: $MODE" "$BLUE"
    log ""
    log "Components Installed:"
    [ "$SETUP_BTRFS" = true ] && success "BTRFS subvolumes"
    [ "$SETUP_OVS" = true ] && success "OVS bridges and flows"
    [ "$SETUP_CONTAINERS" = true ] && success "Container template infrastructure"
    [ "$SETUP_NETMAKER" = true ] && success "Netmaker integration"
    [ "$SETUP_NIXOS" = true ] && success "NixOS integration paths"

    log ""
    log "Paths:"
    log "  Binary: $INSTALL_DIR/op-dbus"
    log "  Config: $CONFIG_DIR/state.json"
    log "  Data: $DATA_DIR"
    log "  Service: /etc/systemd/system/op-dbus.service"

    log ""
    log "Next Steps:" "$YELLOW"
    log "1. Edit configuration: $CONFIG_DIR/state.json"
    log "2. Start service: systemctl start op-dbus"
    log "3. Check status: systemctl status op-dbus"
    log "4. View logs: journalctl -u op-dbus -f"

    if [ "$SETUP_NETMAKER" = true ] && [ ! -f "${CONFIG_DIR}/netmaker.env" ]; then
        log ""
        warn "To enable Netmaker mesh networking:"
        log "  echo 'NETMAKER_TOKEN=your-token' > ${CONFIG_DIR}/netmaker.env"
        log "  ./install.sh --enable-netmaker"
    fi

    log ""
    success "Installation complete!"
    log ""
    log "This script is idempotent - safe to run multiple times for upgrades" "$BLUE"
}

# Main installation flow
main() {
    # Initialize log
    mkdir -p "$(dirname $LOG_FILE)"
    echo "=== op-dbus Installation Started: $(date) ===" > "$LOG_FILE"

    header "op-dbus Infrastructure Installation"
    log "Version: 0.1.0"
    log "Date: $(date)"

    check_root
    parse_args "$@"
    detect_system
    create_directories

    # Storage layer
    if [ "$SETUP_BTRFS" = true ]; then
        setup_btrfs_subvolumes
    else
        setup_regular_directories
    fi

    # Network layer
    if [ "$SETUP_OVS" = true ]; then
        setup_ovs_bridges
    fi

    # Container layer
    if [ "$SETUP_CONTAINERS" = true ]; then
        setup_container_templates
    fi

    # Integration layers
    setup_netmaker
    setup_nixos_integration

    # Application layer
    install_binary
    install_systemd_service
    create_initial_state

    print_summary

    echo "=== Installation Completed: $(date) ===" >> "$LOG_FILE"
}

# Run main
main "$@"
