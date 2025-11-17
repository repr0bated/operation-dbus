#!/bin/bash
set -euo pipefail

# Operation D-Bus Source Environment Validation Script
# Ensures the golden environment is ready for deployment snapshot

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
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

check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root"
        exit 1
    fi
}

check_btrfs() {
    log_info "Checking Btrfs filesystem..."

    # Check if root filesystem is Btrfs
    if ! mount | grep -q "on / type btrfs"; then
        log_error "Root filesystem is not Btrfs. Required for deployment snapshots."
        exit 1
    fi

    # Check btrfs tools
    if ! command -v btrfs &> /dev/null; then
        log_error "btrfs tools not installed"
        exit 1
    fi

    # Check golden subvolume exists
    if ! btrfs subvolume list / | grep -q "@op-dbus-golden"; then
        log_error "Golden subvolume @op-dbus-golden not found"
        log_info "Create it with: sudo btrfs subvolume create /@op-dbus-golden"
        exit 1
    fi

    log_info "Btrfs filesystem OK"
}

check_op_dbus_installation() {
    log_info "Checking Operation D-Bus installation..."

    local mount_point="/mnt/op-dbus-golden"

    # Mount golden environment if not already mounted
    if ! mount | grep -q "$mount_point"; then
        log_info "Mounting golden environment..."
        mkdir -p "$mount_point"
        mount -o subvol=@op-dbus-golden /dev/mapper/vg0-root "$mount_point" 2>/dev/null || \
        mount -o subvol=@op-dbus-golden /dev/vda1 "$mount_point" 2>/dev/null || {
            log_error "Failed to mount golden subvolume. Check your device mapping."
            exit 1
        }
    fi

    # Check binary exists and is executable
    if [[ ! -x "$mount_point/usr/local/bin/op-dbus" ]]; then
        log_error "op-dbus binary not found in golden environment"
        log_error "Install with: sudo make install DESTDIR=$mount_point"
        exit 1
    fi

    # Check version
    local version
    version="$("$mount_point/usr/local/bin/op-dbus" --version 2>/dev/null || echo "unknown")"
    log_info "Operation D-Bus version: $version"

    # Check configuration directory
    if [[ ! -d "$mount_point/etc/op-dbus" ]]; then
        log_warn "Configuration directory /etc/op-dbus not found in golden environment"
    fi

    # Check systemd service
    if [[ ! -f "$mount_point/lib/systemd/system/op-dbus.service" ]]; then
        log_error "Systemd service file not found"
        exit 1
    fi

    # Unmount if we mounted it
    if mount | grep -q "$mount_point"; then
        umount "$mount_point"
        rmdir "$mount_point"
    fi

    log_info "Operation D-Bus installation OK"
}

check_dependencies() {
    log_info "Checking system dependencies..."

    local mount_point="/mnt/op-dbus-golden"
    local deps=("libssl.so" "libcrypto.so" "libdbus-1.so")

    # Mount golden environment
    mkdir -p "$mount_point"
    mount -o subvol=@op-dbus-golden /dev/mapper/vg0-root "$mount_point" 2>/dev/null || \
    mount -o subvol=@op-dbus-golden /dev/vda1 "$mount_point"

    # Check shared libraries
    for dep in "${deps[@]}"; do
        if ! find "$mount_point/usr/lib" -name "*$dep*" 2>/dev/null | grep -q .; then
            log_error "Required library $dep not found in golden environment"
            exit 1
        fi
    done

    # Check OpenSSL version compatibility
    local openssl_version
    openssl_version="$("$mount_point/usr/bin/openssl" version 2>/dev/null || echo "not found")"
    if [[ "$openssl_version" == "not found" ]]; then
        log_error "OpenSSL not found in golden environment"
        exit 1
    fi
    log_info "OpenSSL version: $openssl_version"

    # Unmount
    umount "$mount_point"
    rmdir "$mount_point"

    log_info "Dependencies OK"
}

check_network_connectivity() {
    log_info "Checking network connectivity..."

    # Test basic connectivity
    if ! ping -c 1 -W 5 8.8.8.8 &>/dev/null; then
        log_warn "No internet connectivity detected"
        log_warn "This may affect dependency downloads during deployment"
    else
        log_info "Network connectivity OK"
    fi
}

check_disk_space() {
    log_info "Checking disk space..."

    # Check available space (need at least 2GB free)
    local available_kb
    available_kb=$(df / | tail -1 | awk '{print $4}')

    if (( available_kb < 2097152 )); then  # 2GB in KB
        log_error "Insufficient disk space. Need at least 2GB free."
        exit 1
    fi

    log_info "Disk space OK ($(numfmt --to=iec-i --suffix=B $((available_kb * 1024)))) available"
}

validate_op_dbus_functionality() {
    log_info "Validating Operation D-Bus functionality..."

    local mount_point="/mnt/op-dbus-golden"
    local test_config="/tmp/test-config.json"

    # Create test config
    cat > "$test_config" << 'EOF'
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": []
    }
  }
}
EOF

    # Mount and test
    mkdir -p "$mount_point"
    mount -o subvol=@op-dbus-golden /dev/mapper/vg0-root "$mount_point" 2>/dev/null || \
    mount -o subvol=@op-dbus-golden /dev/vda1 "$mount_point"

    # Test basic functionality (query should work even without configs)
    if ! chroot "$mount_point" /usr/local/bin/op-dbus --help &>/dev/null; then
        log_error "op-dbus binary not functional in golden environment"
        exit 1
    fi

    # Test with config file
    if ! chroot "$mount_point" /usr/local/bin/op-dbus diff "$test_config" &>/dev/null; then
        log_error "op-dbus diff command failed in golden environment"
        exit 1
    fi

    # Cleanup
    umount "$mount_point"
    rmdir "$mount_point"
    rm -f "$test_config"

    log_info "Operation D-Bus functionality OK"
}

main() {
    log_info "Starting Operation D-Bus source environment validation..."

    check_root
    check_btrfs
    check_op_dbus_installation
    check_dependencies
    check_network_connectivity
    check_disk_space
    validate_op_dbus_functionality

    log_info "✅ Source environment validation complete!"
    log_info "Ready to create deployment snapshot."
}

main "$@"