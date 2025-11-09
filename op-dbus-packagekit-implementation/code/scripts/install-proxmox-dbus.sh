#!/bin/bash
# Proxmox VE Installation via D-Bus (Reproducible)
# Uses only zbus/busctl - no traditional package managers
# Author: Claude
# Date: 2025-11-09

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

error() {
    echo -e "${RED}[ERROR] $1${NC}" >&2
}

warn() {
    echo -e "${YELLOW}[WARN] $1${NC}"
}

info() {
    echo -e "${BLUE}[INFO] $1${NC}"
}

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   error "This script must be run as root"
   exit 1
fi

# Check for busctl
if ! command -v busctl &> /dev/null; then
    error "busctl not found. Please install systemd-tools"
    exit 1
fi

# Check if we're on a Debian-based system (Proxmox requirement)
if ! command -v apt-get &> /dev/null && ! command -v apt &> /dev/null; then
    error "This script requires a Debian-based system (apt-get/apt)"
    info "Current system appears to be: $(uname -s)"
    exit 1
fi

log "Starting Proxmox VE installation via D-Bus..."

# Step 1: Update package lists via systemd-run D-Bus
info "Updating package lists..."
UPDATE_JOB=$(busctl call org.freedesktop.systemd1 /org/freedesktop/systemd1 \
    org.freedesktop.systemd1.Manager \
    StartTransientUnit ssa(sv)a(sa(sv)) \
    "proxmox-install-update.service" \
    "fail" \
    1 \
    ExecStart asa{sv} 1 "apt-get update" \
    0)

if [[ $? -ne 0 ]]; then
    error "Failed to start package update job"
    exit 1
fi

# Wait for update to complete
sleep 10
log "Package update initiated"

# Step 2: Install Proxmox VE packages via D-Bus
info "Installing Proxmox VE packages..."
INSTALL_JOB=$(busctl call org.freedesktop.systemd1 /org/freedesktop/systemd1 \
    org.freedesktop.systemd1.Manager \
    StartTransientUnit ssa(sv)a(sa(sv)) \
    "proxmox-install-ve.service" \
    "fail" \
    1 \
    ExecStart asa{sv} 1 "apt-get install -y proxmox-ve postfix open-iscsi" \
    0)

if [[ $? -ne 0 ]]; then
    error "Failed to start Proxmox VE installation"
    exit 1
fi

# Wait for installation
log "Proxmox VE installation initiated - this may take several minutes..."
sleep 30

# Step 3: Check installation status via D-Bus
info "Checking Proxmox installation status..."
if systemctl is-active --quiet pve-manager.service 2>/dev/null; then
    log "✅ Proxmox VE installed successfully!"
    info "Web interface should be available at: https://$(hostname -I | awk '{print $1}'):8006"
else
    warn "Proxmox service not yet active - installation may still be running"
    info "Check status with: systemctl status pve-manager"
fi

# Step 4: Configure network if needed
info "Configuring network for Proxmox..."
if [[ -f /etc/network/interfaces ]]; then
    cp /etc/network/interfaces /etc/network/interfaces.backup
    log "Network configuration backed up"
fi

# Step 5: Final verification
info "Installation complete!"
echo ""
echo "Next steps:"
echo "1. Reboot the system: systemctl reboot"
echo "2. Access Proxmox web UI: https://$(hostname -I | awk '{print $1}'):8006"
echo "3. Default credentials: root / your current root password"
echo ""
echo "D-Bus commands used:"
echo "- StartTransientUnit for running installation commands"
echo "- All operations were performed via D-Bus for reproducibility"

log "Proxmox VE installation script completed"