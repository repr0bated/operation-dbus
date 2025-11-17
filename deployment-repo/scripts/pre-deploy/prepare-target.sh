#!/bin/bash
set -euo pipefail

# Operation D-Bus Target System Preparation Script
# Prepares a target system for receiving Btrfs deployment

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root"
        exit 1
    fi
}

check_btrfs() {
    log_step "Checking Btrfs filesystem..."

    # Check if root filesystem is Btrfs
    if ! mount | grep -q "on / type btrfs"; then
        log_error "Root filesystem is not Btrfs. Required for deployment."
        log_error "Please convert to Btrfs or use a different deployment method."
        exit 1
    fi

    # Check btrfs tools
    if ! command -v btrfs &> /dev/null; then
        log_error "btrfs-progs not installed. Installing..."
        apt update && apt install -y btrfs-progs
    fi

    log_info "Btrfs filesystem OK"
}

create_deployment_directories() {
    log_step "Creating deployment directories..."

    # Create main deployment directory
    mkdir -p /var/lib/op-dbus/deploy

    # Create overlay directories for writable areas
    mkdir -p /var/lib/op-dbus/overlays/{upper,work}

    # Create state directory
    mkdir -p /var/lib/op-dbus/state

    # Create config directory
    mkdir -p /etc/op-dbus

    log_info "Deployment directories created"
}

check_disk_space() {
    log_step "Checking available disk space..."

    # Check available space (need at least 5GB free for deployment + operations)
    local available_kb
    available_kb=$(df / | tail -1 | awk '{print $4}')

    if (( available_kb < 5242880 )); then  # 5GB in KB
        log_error "Insufficient disk space. Need at least 5GB free for deployment."
        exit 1
    fi

    log_info "Disk space OK ($(numfmt --to=iec-i --suffix=B $((available_kb * 1024)))) available"
}

check_dependencies() {
    log_step "Checking system dependencies..."

    local deps=("mount" "umount" "chmod" "chown" "mkdir" "rmdir" "systemctl")

    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null; then
            log_error "Required command '$dep' not found"
            exit 1
        fi
    done

    # Check for overlay filesystem support
    if ! grep -q overlay /proc/filesystems; then
        log_error "Overlay filesystem not supported by kernel"
        exit 1
    fi

    log_info "Dependencies OK"
}

backup_existing_installation() {
    log_step "Checking for existing Operation D-Bus installation..."

    # Check if op-dbus is already installed
    if command -v op-dbus &> /dev/null; then
        log_warn "Existing op-dbus installation detected"

        # Get current version if possible
        local current_version
        current_version=$(op-dbus --version 2>/dev/null || echo "unknown")

        log_warn "Current version: $current_version"

        # Backup existing configuration
        if [[ -d /etc/op-dbus ]]; then
            log_info "Backing up existing configuration..."
            cp -r /etc/op-dbus "/etc/op-dbus.backup.$(date +%Y%m%d_%H%M%S)"
        fi

        # Stop service if running
        if systemctl is-active --quiet op-dbus 2>/dev/null; then
            log_info "Stopping existing op-dbus service..."
            systemctl stop op-dbus
        fi

        # Disable service
        if systemctl is-enabled --quiet op-dbus 2>/dev/null; then
            log_info "Disabling existing op-dbus service..."
            systemctl disable op-dbus
        fi
    else
        log_info "No existing op-dbus installation found"
    fi
}

setup_systemd_service() {
    log_step "Setting up systemd service infrastructure..."

    # Ensure systemd is available
    if ! command -v systemctl &> /dev/null; then
        log_error "systemctl not available. Systemd required for op-dbus."
        exit 1
    fi

    # Create systemd override directory
    mkdir -p /etc/systemd/system/op-dbus.service.d

    log_info "Systemd service infrastructure ready"
}

configure_network_requirements() {
    log_step "Checking network requirements..."

    # Check if OpenVSwitch is available (for net plugin)
    if ! command -v ovs-vsctl &> /dev/null; then
        log_warn "OpenVSwitch not detected. Net plugin will be limited."
        log_warn "Install with: apt install openvswitch-switch"
    fi

    # Check D-Bus system bus
    if ! pgrep -f "dbus-daemon.*system" >/dev/null; then
        log_warn "D-Bus system daemon not running"
        log_warn "This may affect system integration"
    fi

    log_info "Network requirements checked"
}

create_deployment_marker() {
    log_step "Creating deployment marker..."

    # Create a marker file to indicate deployment preparation
    cat > /etc/op-dbus/deployment-prep.json << EOF
{
  "prepared_at": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "prepared_by": "prepare-target.sh",
  "deployment_method": "btrfs-send-receive"
}
EOF

    log_info "Deployment preparation marker created"
}

show_next_steps() {
    log_info "Target system preparation complete!"
    echo
    log_info "Next steps:"
    echo "1. Download the send stream:"
    echo "   wget https://github.com/repr0bated/operation-dbus-deployment/releases/download/vX.Y.Z/op-dbus-vX.Y.Z.send"
    echo
    echo "2. Receive the deployment:"
    echo "   sudo btrfs receive /var/lib/op-dbus/deploy < op-dbus-vX.Y.Z.send"
    echo
    echo "3. Mount the deployment:"
    echo "   sudo ./scripts/post-deploy/mount-deployment.sh vX.Y.Z"
    echo
    echo "4. Run post-deployment setup:"
    echo "   sudo ./scripts/post-deploy/integrate-system.sh"
}

main() {
    log_info "Starting Operation D-Bus target system preparation..."

    check_root
    check_btrfs
    check_disk_space
    check_dependencies
    create_deployment_directories
    backup_existing_installation
    setup_systemd_service
    configure_network_requirements
    create_deployment_marker

    show_next_steps
}

main "$@"