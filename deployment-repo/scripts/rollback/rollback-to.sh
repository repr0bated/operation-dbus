#!/bin/bash
set -euo pipefail

# Operation D-Bus Rollback Script
# Rolls back to a specific previous version

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
        log_error "Usage: $0 <target-version>"
        log_error "Example: $0 v1.2.3"
        exit 1
    fi

    TARGET_VERSION="$1"

    # Validate version format
    if [[ ! "$TARGET_VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+ ]]; then
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

validate_rollback_target() {
    log_step "Validating rollback target..."

    local target_snapshot="@op-dbus-$TARGET_VERSION"
    local snapshot_path="/var/lib/op-dbus/deploy/$target_snapshot"

    # Check target snapshot exists
    if ! btrfs subvolume list /var/lib/op-dbus/deploy | grep -q "$target_snapshot"; then
        log_error "Target snapshot $target_snapshot not found for rollback"
        log_error "Available snapshots:"
        btrfs subvolume list /var/lib/op-dbus/deploy | grep "@op-dbus-v" | awk '{print $9}'
        exit 1
    fi

    # Check overlay directories exist for target version
    local target_upper="/var/lib/op-dbus/overlays/upper-$TARGET_VERSION"
    local target_work="/var/lib/op-dbus/overlays/work-$TARGET_VERSION"

    if [[ ! -d "$target_upper" ]]; then
        log_warn "Overlay upper directory not found for $TARGET_VERSION"
        log_warn "Will create new overlay directories"
    fi

    log_info "Rollback target validation passed"
}

find_overlay_dirs() {
    local version="$1"
    local upper_dir="/var/lib/op-dbus/overlays/upper-$version"
    local work_dir="/var/lib/op-dbus/overlays/work-$version"

    # Create directories if they don't exist
    mkdir -p "$upper_dir" "$work_dir"

    echo "$upper_dir:$work_dir"
}

perform_rollback() {
    log_step "Performing rollback..."

    local overlay_dirs="$1"
    IFS=':' read -r upper_dir work_dir <<< "$overlay_dirs"

    local overlay_mount="/opt/op-dbus"
    local target_snapshot="/var/lib/op-dbus/deploy/@op-dbus-$TARGET_VERSION"

    # Stop service before rollback
    if systemctl is-active --quiet op-dbus; then
        log_info "Stopping op-dbus service for rollback..."
        systemctl stop op-dbus
    fi

    # Perform atomic overlay switch
    log_info "Switching overlay to target version..."
    if ! mount -t overlay overlay \
         -o remount,lowerdir="$target_snapshot",upperdir="$upper_dir",workdir="$work_dir" \
         "$overlay_mount"; then
        log_error "Failed to switch overlay for rollback"
        exit 1
    fi

    log_info "Overlay switch completed"
}

update_system_integration() {
    log_step "Updating system integration..."

    # The symlinks should automatically point to the new overlay content
    # but let's verify and recreate if needed

    local overlay_mount="/opt/op-dbus"

    # Verify binary symlink
    if [[ ! -x /usr/local/bin/op-dbus ]] || [[ ! -L /usr/local/bin/op-dbus ]]; then
        log_info "Recreating binary symlink..."
        rm -f /usr/local/bin/op-dbus
        ln -sf "$overlay_mount/usr/local/bin/op-dbus" /usr/local/bin/op-dbus
    fi

    # Verify systemd service symlink
    if [[ ! -f /lib/systemd/system/op-dbus.service ]] || [[ ! -L /lib/systemd/system/op-dbus.service ]]; then
        log_info "Recreating systemd service symlink..."
        rm -f /lib/systemd/system/op-dbus.service
        ln -sf "$overlay_mount/lib/systemd/system/op-dbus.service" /lib/systemd/system/op-dbus.service
    fi

    # Reload systemd daemon
    log_info "Reloading systemd daemon..."
    systemctl daemon-reload
}

restart_services() {
    log_step "Restarting services..."

    # Start service with rolled back version
    log_info "Starting op-dbus service with rolled back version..."
    systemctl start op-dbus

    # Verify service is running
    sleep 2
    if ! systemctl is-active --quiet op-dbus; then
        log_error "op-dbus service failed to start after rollback"
        exit 1
    fi

    log_info "Service restart completed"
}

update_deployment_record() {
    log_step "Updating deployment record..."

    # Get current deployment info
    local current_record="/etc/op-dbus/deployment.json"
    local previous_version
    previous_version=$(jq -r '.version' "$current_record" 2>/dev/null || echo "unknown")

    # Update deployment record
    local temp_file="/tmp/deployment.json"
    jq --arg version "$TARGET_VERSION" \
       --arg rolled_back_at "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
       --arg previous_version "$previous_version" \
       '.version = $version | .rolled_back_at = $rolled_back_at | .previous_version_before_rollback = $previous_version' \
       "$current_record" > "$temp_file"

    mv "$temp_file" "$current_record"

    log_info "Deployment record updated"
}

update_overlay_symlinks() {
    log_step "Updating overlay symlinks..."

    local old_upper="/var/lib/op-dbus/overlays/upper"
    local old_work="/var/lib/op-dbus/overlays/work"

    # Update symlinks to point to target version overlay dirs
    ln -sf "/var/lib/op-dbus/overlays/upper-$TARGET_VERSION" "$old_upper"
    ln -sf "/var/lib/op-dbus/overlays/work-$TARGET_VERSION" "$old_work"

    log_info "Overlay symlinks updated"
}

verify_rollback() {
    log_step "Verifying rollback..."

    # Test binary version
    local rolled_back_version
    rolled_back_version=$(/usr/local/bin/op-dbus --version 2>/dev/null || echo "unknown")

    if [[ "$rolled_back_version" == *"$TARGET_VERSION"* ]]; then
        log_info "Version verification passed: $rolled_back_version"
    else
        log_warn "Version verification uncertain: got $rolled_back_version, expected $TARGET_VERSION"
    fi

    # Test service status
    if systemctl is-active --quiet op-dbus; then
        log_info "Service status: running"
    else
        log_error "Service is not running after rollback"
        exit 1
    fi

    # Test basic functionality
    if /usr/local/bin/op-dbus --help >/dev/null 2>&1; then
        log_info "Basic functionality test passed"
    else
        log_error "Basic functionality test failed"
        exit 1
    fi

    log_info "Rollback verification completed successfully"
}

main() {
    log_info "Starting rollback to version $TARGET_VERSION..."

    check_args "$@"
    check_root
    validate_rollback_target

    local overlay_dirs
    overlay_dirs=$(find_overlay_dirs "$TARGET_VERSION")

    perform_rollback "$overlay_dirs"
    update_system_integration
    restart_services
    update_deployment_record
    update_overlay_symlinks
    verify_rollback

    log_info "✅ Rollback completed successfully!"
    log_info "System rolled back to $TARGET_VERSION"
}

main "$@"