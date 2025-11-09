#!/usr/bin/env bash
# Privacy Router Installation Script
# Deploys op-dbus privacy router on NixOS systems

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Detect deployment target
detect_target() {
    log_info "Detecting deployment target..."

    local hostname=$(hostname)
    local has_proxmox=false

    # Check for Proxmox
    if systemctl list-units --full -all | grep -q "pve-"; then
        has_proxmox=true
    fi

    # Check for DigitalOcean
    local is_digitalocean=false
    if [ -f /sys/class/dmi/id/sys_vendor ]; then
        local vendor=$(cat /sys/class/dmi/id/sys_vendor)
        if [[ "$vendor" == "DigitalOcean" ]]; then
            is_digitalocean=true
        fi
    fi

    echo "hostname=$hostname;proxmox=$has_proxmox;digitalocean=$is_digitalocean"
}

# Select configuration based on target
select_config() {
    local target_info=$1

    log_info "Selecting configuration..."

    if [[ "$target_info" == *"digitalocean=true"* ]]; then
        echo "digitalocean-netmaker-socks5.nix"
    elif [[ "$target_info" == *"hostname=oo1424oo"* ]] || [[ "$target_info" == *"proxmox=true"* ]]; then
        # Ask user which variant they want
        echo ""
        log_info "Select oo1424oo configuration:"
        echo "  1) Simple (local only) - Gateway on host + Xray container"
        echo "  2) Full (3 containers) - Gateway + WARP + Xray containers"
        echo "  3) Netmaker + WARP - For NAT traversal with DO droplet"
        echo ""
        read -p "Choice [1-3]: " choice

        case $choice in
            1) echo "oo1424oo-config-simple.nix" ;;
            2) echo "oo1424oo-config.nix" ;;
            3) echo "oo1424oo-netmaker-warp.nix" ;;
            *) log_error "Invalid choice"; exit 1 ;;
        esac
    else
        log_error "Unknown target system"
        exit 1
    fi
}

# Install NixOS dependencies
install_dependencies() {
    log_info "Installing dependencies..."

    # Check if nix is available
    if ! command -v nix &> /dev/null; then
        log_error "Nix is not installed. This script requires NixOS."
        exit 1
    fi

    log_success "Nix is available"
}

# Backup existing configuration
backup_config() {
    local config_path="/etc/nixos/configuration.nix"

    if [ -f "$config_path" ]; then
        local backup_path="$config_path.backup.$(date +%Y%m%d_%H%M%S)"
        log_info "Backing up existing configuration to $backup_path"
        cp "$config_path" "$backup_path"
        log_success "Backup created"
    fi
}

# Copy hardware configuration
ensure_hardware_config() {
    local nix_dir="$1"

    if [ ! -f "$nix_dir/hardware-configuration.nix" ]; then
        log_info "Copying hardware configuration..."
        if [ -f "/etc/nixos/hardware-configuration.nix" ]; then
            cp /etc/nixos/hardware-configuration.nix "$nix_dir/"
            log_success "Hardware configuration copied"
        else
            log_warn "No hardware configuration found. You may need to generate one."
        fi
    fi
}

# Deploy configuration
deploy_config() {
    local config_file=$1
    local repo_dir=$2

    log_info "Deploying configuration: $config_file"

    # Copy module.nix and config to /etc/nixos
    cp "$repo_dir/nix/module.nix" /etc/nixos/
    cp "$repo_dir/nix/$config_file" /etc/nixos/configuration.nix

    log_success "Configuration files copied to /etc/nixos"

    # Build configuration
    log_info "Building NixOS configuration (this may take a while)..."
    if nixos-rebuild build; then
        log_success "Build successful"
    else
        log_error "Build failed"
        return 1
    fi

    # Ask for confirmation
    echo ""
    log_warn "Ready to switch to new configuration."
    log_warn "This will restart services and may cause brief downtime."
    read -p "Continue? [y/N]: " confirm

    if [[ "$confirm" =~ ^[Yy]$ ]]; then
        log_info "Switching to new configuration..."
        if nixos-rebuild switch; then
            log_success "Configuration activated"
            return 0
        else
            log_error "Switch failed"
            return 1
        fi
    else
        log_info "Deployment cancelled. Run 'nixos-rebuild switch' manually when ready."
        return 0
    fi
}

# Test deployment
test_deployment() {
    local config_file=$1

    log_info "Testing deployment..."

    # Check if op-dbus service is running
    if systemctl is-active --quiet op-dbus.service; then
        log_success "op-dbus service is running"
    else
        log_warn "op-dbus service is not running"
        systemctl status op-dbus.service || true
    fi

    # Check for OVS bridge
    if command -v ovs-vsctl &> /dev/null; then
        if ovs-vsctl br-exists vmbr0 2>/dev/null; then
            log_success "OVS bridge vmbr0 exists"

            # List ports
            log_info "Bridge ports:"
            ovs-vsctl list-ports vmbr0 | sed 's/^/  - /'
        else
            log_warn "OVS bridge vmbr0 not found"
        fi
    fi

    # Check for WireGuard interfaces
    if command -v wg &> /dev/null; then
        local wg_interfaces=$(wg show interfaces 2>/dev/null || echo "")
        if [ -n "$wg_interfaces" ]; then
            log_success "WireGuard interfaces: $wg_interfaces"
        else
            log_warn "No WireGuard interfaces found"
        fi
    fi

    # Check for LXC containers (if applicable)
    if [[ "$config_file" != *"simple"* ]]; then
        if command -v lxc-ls &> /dev/null; then
            local containers=$(lxc-ls --running 2>/dev/null || echo "")
            if [ -n "$containers" ]; then
                log_success "Running containers: $containers"
            else
                log_warn "No running containers found"
            fi
        fi
    fi
}

# Generate WireGuard client config
generate_client_config() {
    log_info "Would you like to generate a WireGuard client configuration?"
    read -p "[y/N]: " gen_config

    if [[ "$gen_config" =~ ^[Yy]$ ]]; then
        echo ""
        log_info "Client configuration will be generated via op-dbus API."
        log_info "Run the following to generate a config:"
        echo ""
        echo "  curl -X POST http://localhost:9573/api/vpn/provision"
        echo ""
        log_info "Or use the MCP tool (if configured):"
        echo ""
        echo "  mcp__op_dbus__provision_vpn_peer '{\"name\": \"laptop\"}'"
        echo ""
    fi
}

# Main installation flow
main() {
    local repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

    echo ""
    log_info "═══════════════════════════════════════════════════════"
    log_info "  op-dbus Privacy Router Installation"
    log_info "═══════════════════════════════════════════════════════"
    echo ""

    # Detect target
    local target_info=$(detect_target)
    log_success "Target detected: $target_info"
    echo ""

    # Select config
    local config_file=$(select_config "$target_info")
    log_success "Selected configuration: $config_file"
    echo ""

    # Install dependencies
    install_dependencies
    echo ""

    # Backup existing config
    backup_config
    echo ""

    # Ensure hardware config
    ensure_hardware_config "$repo_dir/nix"
    echo ""

    # Deploy
    if deploy_config "$config_file" "$repo_dir"; then
        echo ""
        test_deployment "$config_file"
        echo ""
        generate_client_config
        echo ""
        log_success "═══════════════════════════════════════════════════════"
        log_success "  Installation complete!"
        log_success "═══════════════════════════════════════════════════════"
    else
        echo ""
        log_error "═══════════════════════════════════════════════════════"
        log_error "  Installation failed"
        log_error "═══════════════════════════════════════════════════════"
        exit 1
    fi
}

# Run main
main "$@"
