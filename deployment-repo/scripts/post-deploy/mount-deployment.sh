#!/bin/bash
set -euo pipefail

# Operation D-Bus Deployment Mounting Script
# Mounts a received Btrfs snapshot for system integration

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

check_args() {
    if [[ $# -ne 1 ]]; then
        log_error "Usage: $0 <version>"
        log_error "Example: $0 v1.2.3"
        exit 1
    fi

    VERSION="$1"

    # Validate version format
    if [[ ! "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+ ]]; then
        log_error "Version must be in format vX.Y.Z (e.g., v1.2.3)"
        exit 1
    fi
}

check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root"
        exit 1
    fi
}

validate_deployment_snapshot() {
    log_step "Validating deployment snapshot..."

    local snapshot_name="@op-dbus-$VERSION"
    local snapshot_path="/var/lib/op-dbus/deploy/$snapshot_name"

    # Check snapshot exists
    if ! btrfs subvolume list /var/lib/op-dbus/deploy | grep -q "$snapshot_name"; then
        log_error "Deployment snapshot $snapshot_name not found in /var/lib/op-dbus/deploy"
        log_error "Receive it first with: sudo btrfs receive /var/lib/op-dbus/deploy < op-dbus-$VERSION.send"
        exit 1
    fi

    # Check it's read-only
    local ro_status
    ro_status=$(btrfs property get "$snapshot_path" ro 2>/dev/null || echo "ro=false")
    if [[ "$ro_status" != "ro=true" ]]; then
        log_warn "Snapshot is not read-only. Setting as read-only..."
        btrfs property set "$snapshot_path" ro true
    fi

    # Quick mount test
    local test_mount="/tmp/op-dbus-test-mount"
    mkdir -p "$test_mount"

    if mount -o subvol="$snapshot_name",ro "$snapshot_path" "$test_mount" 2>/dev/null; then
        # Check key files exist
        if [[ ! -x "$test_mount/usr/local/bin/op-dbus" ]]; then
            log_error "op-dbus binary not found in deployment snapshot"
            umount "$test_mount"
            exit 1
        fi

        if [[ ! -f "$test_mount/lib/systemd/system/op-dbus.service" ]]; then
            log_error "Systemd service file not found in deployment snapshot"
            umount "$test_mount"
            exit 1
        fi

        umount "$test_mount"
        rmdir "$test_mount"
    else
        log_error "Cannot mount deployment snapshot for validation"
        exit 1
    fi

    log_info "Deployment snapshot validation passed"
}

create_overlay_mounts() {
    log_step "Creating overlay mounts..."

    local snapshot_name="@op-dbus-$VERSION"
    local snapshot_path="/var/lib/op-dbus/deploy/$snapshot_name"
    local overlay_mount="/opt/op-dbus"

    # Create mount point
    mkdir -p "$overlay_mount"

    # Create overlay mount
    log_info "Setting up overlay filesystem..."

    # Mount options for overlay
    local lowerdir="$snapshot_path"
    local upperdir="/var/lib/op-dbus/overlays/upper"
    local workdir="/var/lib/op-dbus/overlays/work"

    # Ensure upper and work directories exist and are clean
    rm -rf "$upperdir"/* "$workdir"/* 2>/dev/null || true
    mkdir -p "$upperdir" "$workdir"

    # Create overlay mount
    if ! mount -t overlay overlay \
         -o lowerdir="$lowerdir",upperdir="$upperdir",workdir="$workdir" \
         "$overlay_mount"; then
        log_error "Failed to create overlay mount"
        exit 1
    fi

    log_info "Overlay mount created at $overlay_mount"
}

create_symbolic_links() {
    log_step "Creating system integration links..."

    local overlay_mount="/opt/op-dbus"

    # Create symlink for binary
    if [[ ! -L /usr/local/bin/op-dbus ]]; then
        ln -sf "$overlay_mount/usr/local/bin/op-dbus" /usr/local/bin/op-dbus
        log_info "Created symlink: /usr/local/bin/op-dbus"
    fi

    # Create symlink for systemd service
    if [[ ! -L /lib/systemd/system/op-dbus.service ]]; then
        ln -sf "$overlay_mount/lib/systemd/system/op-dbus.service" /lib/systemd/system/op-dbus.service
        log_info "Created symlink: /lib/systemd/system/op-dbus.service"
    fi

    # Create symlink for configuration directory (but don't overwrite existing)
    if [[ ! -e /etc/op-dbus/config ]]; then
        ln -sf "$overlay_mount/etc/op-dbus" /etc/op-dbus/config
        log_info "Created symlink: /etc/op-dbus/config"
    fi

    # Create symlink for libraries if needed
    if [[ -d "$overlay_mount/usr/lib" ]]; then
        # Check if we need to add to ldconfig
        if [[ ! -f /etc/ld-musl-x86_64.path ]] && ! ldconfig -p | grep -q "$overlay_mount"; then
            log_info "Adding overlay lib directory to ldconfig search path..."
            echo "$overlay_mount/usr/lib" >> /etc/ld-musl-x86_64.path 2>/dev/null || \
            echo "$overlay_mount/usr/lib" >> /etc/ld.so.conf.d/op-dbus.conf
            ldconfig
        fi
    fi
}

configure_permissions() {
    log_step "Configuring permissions..."

    local overlay_mount="/opt/op-dbus"

    # Ensure binary is executable
    chmod +x "$overlay_mount/usr/local/bin/op-dbus"

    # Ensure configuration directories have proper permissions
    chmod 755 /etc/op-dbus 2>/dev/null || true
    chmod 644 /etc/op-dbus/*.json 2>/dev/null || true

    log_info "Permissions configured"
}

test_mount_functionality() {
    log_step "Testing mount functionality..."

    # Test binary execution
    if ! /usr/local/bin/op-dbus --help >/dev/null 2>&1; then
        log_error "op-dbus binary not functional after mounting"
        exit 1
    fi

    # Test version
    local version
    version=$(/usr/local/bin/op-dbus --version 2>/dev/null || echo "unknown")
    log_info "Operation D-Bus version: $version"

    # Test basic functionality (if we have a config)
    if [[ -f /etc/op-dbus/state.json ]]; then
        if ! /usr/local/bin/op-dbus diff /etc/op-dbus/state.json >/dev/null 2>&1; then
            log_warn "op-dbus diff test failed - this may be expected if plugins require system access"
        else
            log_info "op-dbus diff test passed"
        fi
    fi

    log_info "Mount functionality tests passed"
}

create_deployment_record() {
    log_step "Creating deployment record..."

    # Update deployment marker
    cat > /etc/op-dbus/deployment.json << EOF
{
  "version": "$VERSION",
  "deployed_at": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "deployment_method": "btrfs-send-receive",
  "mount_type": "overlay",
  "overlay_mount": "/opt/op-dbus",
  "snapshot_path": "/var/lib/op-dbus/deploy/@op-dbus-$VERSION"
}
EOF

    log_info "Deployment record created"
}

show_next_steps() {
    log_info "Deployment mounting complete!"
    echo
    log_info "Next steps:"
    echo "1. Run system integration:"
    echo "   sudo ./scripts/post-deploy/integrate-system.sh"
    echo
    echo "2. Test the deployment:"
    echo "   sudo ./validation/post-deploy-check.sh"
    echo
    echo "3. Start the service:"
    echo "   sudo systemctl daemon-reload"
    echo "   sudo systemctl enable op-dbus"
    echo "   sudo systemctl start op-dbus"
}

cleanup_on_failure() {
    local overlay_mount="/opt/op-dbus"

    # Unmount overlay if it exists
    if mount | grep -q "$overlay_mount"; then
        log_warn "Cleaning up overlay mount due to failure..."
        umount "$overlay_mount" || true
    fi

    # Remove symlinks
    rm -f /usr/local/bin/op-dbus 2>/dev/null || true
    rm -f /lib/systemd/system/op-dbus.service 2>/dev/null || true
}

trap cleanup_on_failure ERR

main() {
    log_info "Starting Operation D-Bus deployment mounting for version $VERSION..."

    check_args "$@"
    check_root
    validate_deployment_snapshot
    create_overlay_mounts
    create_symbolic_links
    configure_permissions
    test_mount_functionality
    create_deployment_record

    show_next_steps
}

main "$@"