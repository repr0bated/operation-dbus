#!/bin/bash
set -euo pipefail

# Operation D-Bus Btrfs Snapshot Creation Script
# Creates a deployment-ready snapshot of the golden environment

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

validate_golden_environment() {
    log_step "Validating golden environment..."

    # Run the validation script
    if [[ -f "$REPO_ROOT/scripts/pre-deploy/validate-source.sh" ]]; then
        bash "$REPO_ROOT/scripts/pre-deploy/validate-source.sh"
    else
        log_error "Validation script not found: $REPO_ROOT/scripts/pre-deploy/validate-source.sh"
        exit 1
    fi
}

prepare_golden_environment() {
    log_step "Preparing golden environment for snapshot..."

    local mount_point="/mnt/op-dbus-golden"

    # Mount golden environment
    log_info "Mounting golden environment..."
    mkdir -p "$mount_point"
    if ! mount -o subvol=@op-dbus-golden /dev/mapper/vg0-root "$mount_point" 2>/dev/null; then
        if ! mount -o subvol=@op-dbus-golden /dev/vda1 "$mount_point" 2>/dev/null; then
            log_error "Failed to mount golden subvolume"
            exit 1
        fi
    fi

    # Clean up any temporary files
    log_info "Cleaning up temporary files..."
    chroot "$mount_point" /bin/bash -c "
        # Clean package cache
        apt clean 2>/dev/null || true
        apt autoclean 2>/dev/null || true

        # Clean old logs
        find /var/log -name '*.gz' -delete 2>/dev/null || true
        find /var/log -name '*.old' -delete 2>/dev/null || true
        truncate -s 0 /var/log/*.log 2>/dev/null || true

        # Clean systemd journal
        journalctl --vacuum-time=1d 2>/dev/null || true

        # Clean op-dbus temporary data
        rm -rf /tmp/op-dbus-* 2>/dev/null || true
        rm -rf /var/tmp/op-dbus-* 2>/dev/null || true
    "

    # Ensure proper permissions
    log_info "Setting proper permissions..."
    chroot "$mount_point" /bin/bash -c "
        # Fix permissions
        chown -R root:root /etc/op-dbus 2>/dev/null || true
        chmod 755 /usr/local/bin/op-dbus 2>/dev/null || true
        chmod 644 /lib/systemd/system/op-dbus.service 2>/dev/null || true
    "

    # Create version file
    log_info "Creating version file..."
    echo "$VERSION" | chroot "$mount_point" /bin/bash -c "cat > /etc/op-dbus/version"

    # Unmount
    log_info "Unmounting golden environment..."
    umount "$mount_point"
    rmdir "$mount_point"
}

create_snapshot() {
    log_step "Creating Btrfs snapshot..."

    local snapshot_name="@op-dbus-$VERSION"
    local snapshot_path="/$snapshot_name"

    # Remove existing snapshot if it exists
    if btrfs subvolume list / | grep -q "$snapshot_name"; then
        log_warn "Snapshot $snapshot_name already exists, removing..."
        btrfs subvolume delete "$snapshot_path" || {
            log_error "Failed to delete existing snapshot"
            exit 1
        }
    fi

    # Create new snapshot
    log_info "Creating snapshot $snapshot_name..."
    btrfs subvolume snapshot /@op-dbus-golden "$snapshot_path" || {
        log_error "Failed to create snapshot"
        exit 1
    }

    # Set snapshot as read-only
    log_info "Setting snapshot as read-only..."
    btrfs property set "$snapshot_path" ro true || {
        log_error "Failed to set snapshot as read-only"
        exit 1
    }

    log_info "Snapshot created successfully: $snapshot_path"
}

generate_metadata() {
    log_step "Generating snapshot metadata..."

    local snapshot_name="@op-dbus-$VERSION"
    local snapshot_path="/$snapshot_name"
    local metadata_file="$REPO_ROOT/btrfs/snapshots/$VERSION.json"

    # Create snapshots directory
    mkdir -p "$REPO_ROOT/btrfs/snapshots"

    # Gather snapshot information
    local created_at
    created_at=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    local uuid
    uuid=$(btrfs subvolume show "$snapshot_path" | grep "UUID:" | awk '{print $2}')

    local size_kb
    size_kb=$(btrfs qgroup show / | grep "$snapshot_name" | awk '{print $2}' | sed 's/\..*//')

    # Get op-dbus version from within snapshot
    local mount_point="/mnt/op-dbus-snapshot"
    mkdir -p "$mount_point"
    mount -o subvol="$snapshot_name",ro /dev/mapper/vg0-root "$mount_point" 2>/dev/null || \
    mount -o subvol="$snapshot_name",ro /dev/vda1 "$mount_point"

    local op_dbus_version
    op_dbus_version=$(chroot "$mount_point" /usr/local/bin/op-dbus --version 2>/dev/null || echo "unknown")

    umount "$mount_point"
    rmdir "$mount_point"

    # Create metadata JSON
    cat > "$metadata_file" << EOF
{
  "version": "$VERSION",
  "op_dbus_version": "$op_dbus_version",
  "snapshot_name": "$snapshot_name",
  "uuid": "$uuid",
  "created_at": "$created_at",
  "size_kb": $size_kb,
  "source": "golden-environment"
}
EOF

    log_info "Metadata saved to: $metadata_file"
}

verify_snapshot() {
    log_step "Verifying snapshot..."

    local snapshot_name="@op-dbus-$VERSION"
    local snapshot_path="/$snapshot_name"

    # Check snapshot exists
    if ! btrfs subvolume list / | grep -q "$snapshot_name"; then
        log_error "Snapshot $snapshot_name not found after creation"
        exit 1
    fi

    # Check it's read-only
    local ro_status
    ro_status=$(btrfs property get "$snapshot_path" ro)
    if [[ "$ro_status" != "ro=true" ]]; then
        log_error "Snapshot is not read-only"
        exit 1
    fi

    # Quick mount test
    local mount_point="/mnt/op-dbus-verify"
    mkdir -p "$mount_point"

    if ! mount -o subvol="$snapshot_name",ro /dev/mapper/vg0-root "$mount_point" 2>/dev/null; then
        if ! mount -o subvol="$snapshot_name",ro /dev/vda1 "$mount_point" 2>/dev/null; then
            log_error "Cannot mount snapshot for verification"
            exit 1
        fi
    fi

    # Verify key files exist
    if [[ ! -x "$mount_point/usr/local/bin/op-dbus" ]]; then
        log_error "op-dbus binary not found in snapshot"
        exit 1
    fi

    if [[ ! -f "$mount_point/lib/systemd/system/op-dbus.service" ]]; then
        log_error "Systemd service file not found in snapshot"
        exit 1
    fi

    # Test op-dbus basic functionality
    if ! chroot "$mount_point" /usr/local/bin/op-dbus --help >/dev/null 2>&1; then
        log_error "op-dbus binary not functional in snapshot"
        exit 1
    fi

    umount "$mount_point"
    rmdir "$mount_point"

    log_info "Snapshot verification passed"
}

cleanup_old_snapshots() {
    log_step "Cleaning up old snapshots..."

    # Keep last 5 snapshots
    local snapshots
    mapfile -t snapshots < <(btrfs subvolume list / | grep "@op-dbus-v" | awk '{print $9}' | sort -V | head -n -5)

    for snapshot in "${snapshots[@]}"; do
        log_info "Removing old snapshot: $snapshot"
        btrfs subvolume delete "/$snapshot" || log_warn "Failed to remove $snapshot"
    done
}

main() {
    log_info "Starting Operation D-Bus snapshot creation for version $VERSION..."

    check_args "$@"
    check_root
    validate_golden_environment
    prepare_golden_environment
    create_snapshot
    generate_metadata
    verify_snapshot
    cleanup_old_snapshots

    log_info "✅ Snapshot creation complete!"
    log_info "Snapshot: @op-dbus-$VERSION"
    log_info "Ready to generate send stream with: ./btrfs/scripts/send-snapshot.sh $VERSION"
}

main "$@"