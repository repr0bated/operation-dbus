#!/bin/bash
set -euo pipefail

# Operation D-Bus Rolling Upgrade Script
# Performs zero-downtime upgrade between versions using Btrfs

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
    if [[ $# -ne 2 ]]; then
        log_error "Usage: $0 <current-version> <target-version>"
        log_error "Example: $0 v1.2.3 v1.3.0"
        exit 1
    fi

    CURRENT_VERSION="$1"
    TARGET_VERSION="$2"

    # Validate version formats
    for version in "$CURRENT_VERSION" "$TARGET_VERSION"; do
        if [[ ! "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+ ]]; then
            log_error "Version must be in format vX.Y.Z (e.g., v1.2.3): $version"
            exit 1
        fi
    done

    # Ensure versions are different
    if [[ "$CURRENT_VERSION" == "$TARGET_VERSION" ]]; then
        log_error "Current and target versions must be different"
        exit 1
    fi
}

check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root"
        exit 1
    fi
}

validate_current_deployment() {
    log_step "Validating current deployment..."

    # Check deployment record exists
    if [[ ! -f /etc/op-dbus/deployment.json ]]; then
        log_error "No deployment record found. Not a deployed system?"
        exit 1
    fi

    # Check current version matches
    local deployed_version
    deployed_version=$(jq -r '.version' /etc/op-dbus/deployment.json 2>/dev/null || echo "")

    if [[ "$deployed_version" != "$CURRENT_VERSION" ]]; then
        log_error "Deployed version ($deployed_version) doesn't match specified current version ($CURRENT_VERSION)"
        exit 1
    fi

    # Check overlay mount is active
    if ! mount | grep -q "/opt/op-dbus"; then
        log_error "Deployment overlay mount not active"
        exit 1
    fi

    # Check service is running
    if ! systemctl is-active --quiet op-dbus; then
        log_warn "op-dbus service is not running - will attempt upgrade anyway"
    fi

    log_info "Current deployment validation passed"
}

download_target_version() {
    log_step "Downloading target version..."

    local send_stream_url="https://github.com/repr0bated/operation-dbus-deployment/releases/download/$TARGET_VERSION/op-dbus-$TARGET_VERSION.send"
    local send_stream_file="/tmp/op-dbus-$TARGET_VERSION.send"

    log_info "Downloading send stream from: $send_stream_url"

    if ! wget -q -O "$send_stream_file" "$send_stream_url"; then
        log_error "Failed to download send stream for $TARGET_VERSION"
        exit 1
    fi

    # Verify download
    if [[ ! -s "$send_stream_file" ]]; then
        log_error "Downloaded send stream is empty"
        rm -f "$send_stream_file"
        exit 1
    fi

    log_info "Send stream downloaded successfully"
    echo "$send_stream_file"
}

receive_target_snapshot() {
    log_step "Receiving target snapshot..."

    local send_stream_file="$1"
    local target_snapshot="@op-dbus-$TARGET_VERSION"
    local deploy_dir="/var/lib/op-dbus/deploy"

    # Remove existing target snapshot if it exists
    if btrfs subvolume list "$deploy_dir" | grep -q "$target_snapshot"; then
        log_warn "Removing existing target snapshot..."
        btrfs subvolume delete "$deploy_dir/$target_snapshot" || {
            log_error "Failed to remove existing target snapshot"
            exit 1
        }
    fi

    # Receive new snapshot
    log_info "Receiving snapshot $target_snapshot..."
    if ! btrfs receive "$deploy_dir" < "$send_stream_file"; then
        log_error "Failed to receive target snapshot"
        exit 1
    fi

    # Set as read-only
    btrfs property set "$deploy_dir/$target_snapshot" ro true

    # Cleanup send stream file
    rm -f "$send_stream_file"

    log_info "Target snapshot received successfully"
}

prepare_upgrade_environment() {
    log_step "Preparing upgrade environment..."

    # Create new overlay directories for target version
    local new_upper="/var/lib/op-dbus/overlays/upper-$TARGET_VERSION"
    local new_work="/var/lib/op-dbus/overlays/work-$TARGET_VERSION"

    rm -rf "$new_upper" "$new_work"
    mkdir -p "$new_upper" "$new_work"

    # Copy current upper directory contents to preserve any local changes
    if [[ -d /var/lib/op-dbus/overlays/upper ]]; then
        log_info "Preserving local changes from current overlay..."
        cp -r /var/lib/op-dbus/overlays/upper/* "$new_upper/" 2>/dev/null || true
    fi

    echo "$new_upper:$new_work"
}

perform_atomic_switch() {
    log_step "Performing atomic overlay switch..."

    local overlay_dirs="$1"
    IFS=':' read -r new_upper new_work <<< "$overlay_dirs"

    local current_mount="/opt/op-dbus"
    local temp_mount="/opt/op-dbus-new"
    local target_snapshot="/var/lib/op-dbus/deploy/@op-dbus-$TARGET_VERSION"

    # Create temporary mount point
    mkdir -p "$temp_mount"

    # Mount new overlay temporarily
    log_info "Mounting new overlay temporarily..."
    if ! mount -t overlay overlay \
         -o lowerdir="$target_snapshot",upperdir="$new_upper",workdir="$new_work" \
         "$temp_mount"; then
        log_error "Failed to mount new overlay"
        rmdir "$temp_mount" 2>/dev/null || true
        exit 1
    fi

    # Test new overlay functionality
    log_info "Testing new overlay functionality..."
    if ! "$temp_mount/usr/local/bin/op-dbus" --help >/dev/null 2>&1; then
        log_error "New overlay op-dbus binary not functional"
        umount "$temp_mount"
        rmdir "$temp_mount"
        exit 1
    fi

    # Atomic switch: remount current overlay with new lowerdir
    log_info "Performing atomic switch..."
    mount -t overlay overlay \
         -o remount,lowerdir="$target_snapshot",upperdir="$new_upper",workdir="$new_work" \
         "$current_mount"

    # Clean up temporary mount
    umount "$temp_mount" 2>/dev/null || true
    rmdir "$temp_mount" 2>/dev/null || true

    log_info "Atomic switch completed"
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

    # Stop service gracefully
    if systemctl is-active --quiet op-dbus; then
        log_info "Stopping op-dbus service..."
        systemctl stop op-dbus
    fi

    # Start service with new version
    log_info "Starting op-dbus service with new version..."
    systemctl start op-dbus

    # Enable if not already enabled
    if ! systemctl is-enabled --quiet op-dbus; then
        systemctl enable op-dbus
    fi

    # Verify service is running
    sleep 2
    if ! systemctl is-active --quiet op-dbus; then
        log_error "op-dbus service failed to start after upgrade"
        exit 1
    fi

    log_info "Service restart completed"
}

update_deployment_record() {
    log_step "Updating deployment record..."

    # Update deployment record
    local temp_file="/tmp/deployment.json"
    jq --arg version "$TARGET_VERSION" \
       --arg upgraded_at "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
       --arg previous_version "$CURRENT_VERSION" \
       '.version = $version | .upgraded_at = $upgraded_at | .previous_version = $previous_version' \
       /etc/op-dbus/deployment.json > "$temp_file"

    mv "$temp_file" /etc/op-dbus/deployment.json

    log_info "Deployment record updated"
}

cleanup_old_overlays() {
    log_step "Cleaning up old overlays..."

    # Keep one previous version for rollback
    local old_upper="/var/lib/op-dbus/overlays/upper"
    local old_work="/var/lib/op-dbus/overlays/work"

    # Move current to backup
    if [[ -d "$old_upper" ]]; then
        mv "$old_upper" "/var/lib/op-dbus/overlays/upper-$CURRENT_VERSION"
        mv "$old_work" "/var/lib/op-dbus/overlays/work-$CURRENT_VERSION"
    fi

    # Update symlinks to new overlay dirs
    ln -sf "/var/lib/op-dbus/overlays/upper-$TARGET_VERSION" "$old_upper"
    ln -sf "/var/lib/op-dbus/overlays/work-$TARGET_VERSION" "$old_work"

    log_info "Old overlays cleaned up"
}

rollback_on_failure() {
    log_error "Upgrade failed! Initiating rollback..."

    # Attempt rollback to previous version
    if [[ -f "$REPO_ROOT/scripts/rollback/rollback-to.sh" ]]; then
        log_info "Attempting automatic rollback..."
        bash "$REPO_ROOT/scripts/rollback/rollback-to.sh" "$CURRENT_VERSION" || {
            log_error "Automatic rollback failed. Manual intervention required."
            exit 1
        }
    else
        log_error "Rollback script not found. Manual rollback required."
        exit 1
    fi
}

verify_upgrade() {
    log_step "Verifying upgrade..."

    # Test binary version
    local new_version
    new_version=$(/usr/local/bin/op-dbus --version 2>/dev/null || echo "unknown")

    if [[ "$new_version" == *"$TARGET_VERSION"* ]]; then
        log_info "Version verification passed: $new_version"
    else
        log_warn "Version verification uncertain: got $new_version, expected $TARGET_VERSION"
    fi

    # Test service status
    if systemctl is-active --quiet op-dbus; then
        log_info "Service status: running"
    else
        log_error "Service is not running after upgrade"
        exit 1
    fi

    # Test basic functionality
    if /usr/local/bin/op-dbus --help >/dev/null 2>&1; then
        log_info "Basic functionality test passed"
    else
        log_error "Basic functionality test failed"
        exit 1
    fi

    log_info "Upgrade verification completed successfully"
}

main() {
    log_info "Starting rolling upgrade from $CURRENT_VERSION to $TARGET_VERSION..."

    check_args "$@"
    check_root
    validate_current_deployment

    # Set trap for rollback on failure
    trap rollback_on_failure ERR

    local send_stream_file
    send_stream_file=$(download_target_version)
    receive_target_snapshot "$send_stream_file"

    local overlay_dirs
    overlay_dirs=$(prepare_upgrade_environment)

    perform_atomic_switch "$overlay_dirs"
    update_system_integration
    restart_services
    update_deployment_record
    cleanup_old_overlays
    verify_upgrade

    # Clear trap since upgrade succeeded
    trap - ERR

    log_info "✅ Rolling upgrade completed successfully!"
    log_info "System upgraded from $CURRENT_VERSION to $TARGET_VERSION"
}

main "$@"