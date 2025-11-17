#!/bin/bash
set -euo pipefail

# Operation D-Bus Btrfs Send Stream Generation Script
# Generates a send stream from a snapshot for distribution

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

validate_snapshot() {
    log_step "Validating snapshot..."

    local snapshot_name="@op-dbus-$VERSION"
    local snapshot_path="/$snapshot_name"

    # Check snapshot exists
    if ! btrfs subvolume list / | grep -q "$snapshot_name"; then
        log_error "Snapshot $snapshot_name not found"
        log_error "Create it first with: ./btrfs/scripts/create-snapshot.sh $VERSION"
        exit 1
    fi

    # Check it's read-only
    local ro_status
    ro_status=$(btrfs property get "$snapshot_path" ro)
    if [[ "$ro_status" != "ro=true" ]]; then
        log_error "Snapshot is not read-only"
        exit 1
    fi

    log_info "Snapshot validation passed"
}

find_parent_snapshot() {
    log_step "Finding parent snapshot for incremental send..."

    # Look for previous version snapshots
    local current_major current_minor current_patch
    IFS='.' read -r _ current_major current_minor current_patch <<< "${VERSION#v}"

    local parent_snapshot=""

    # Try to find the most recent previous snapshot
    local snapshots
    mapfile -t snapshots < <(btrfs subvolume list / | grep "@op-dbus-v" | awk '{print $9}' | sort -V)

    for snapshot in "${snapshots[@]}"; do
        local snap_version="${snapshot#@op-dbus-}"

        if [[ "$snap_version" < "$VERSION" ]]; then
            parent_snapshot="$snapshot"
        else
            break
        fi
    done

    if [[ -n "$parent_snapshot" ]]; then
        log_info "Using parent snapshot: $parent_snapshot"
        echo "$parent_snapshot"
    else
        log_info "No parent snapshot found - creating full send stream"
        echo ""
    fi
}

generate_send_stream() {
    log_step "Generating send stream..."

    local snapshot_name="@op-dbus-$VERSION"
    local parent_snapshot="$1"

    log_info "Generating send stream for $snapshot_name..."

    if [[ -n "$parent_snapshot" ]]; then
        log_info "Creating incremental send stream from $parent_snapshot"
        btrfs send -p "/$parent_snapshot" "/$snapshot_name"
    else
        log_info "Creating full send stream"
        btrfs send "/$snapshot_name"
    fi
}

calculate_checksum() {
    log_step "Calculating checksum..."

    local temp_file="/tmp/op-dbus-$VERSION.send"
    local checksum_file="$REPO_ROOT/btrfs/snapshots/$VERSION.send.sha256"

    # Generate checksum from stdin (send stream output)
    sha256sum > "$checksum_file"

    log_info "Checksum saved to: $checksum_file"
}

generate_metadata() {
    log_step "Updating metadata with send stream info..."

    local metadata_file="$REPO_ROOT/btrfs/snapshots/$VERSION.json"

    if [[ ! -f "$metadata_file" ]]; then
        log_error "Metadata file not found: $metadata_file"
        exit 1
    fi

    # Calculate send stream size (from checksum file which contains the stream)
    local checksum_file="$REPO_ROOT/btrfs/snapshots/$VERSION.send.sha256"
    local stream_size
    stream_size=$(stat -c%s "$checksum_file" 2>/dev/null || echo "0")

    # Update metadata
    local temp_file="/tmp/metadata.json"
    jq --arg size "$stream_size" \
       --arg generated_at "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
       '.send_stream_size = ($size | tonumber) | .generated_at = $generated_at' \
       "$metadata_file" > "$temp_file"

    mv "$temp_file" "$metadata_file"

    log_info "Metadata updated with send stream information"
}

show_usage_instructions() {
    log_info "Send stream generation complete!"
    echo
    log_info "To deploy this version:"
    echo "1. Copy the send stream to target system:"
    echo "   scp op-dbus-$VERSION.send target-system:"
    echo
    echo "2. On target system, receive the snapshot:"
    echo "   sudo btrfs receive /var/lib/op-dbus/deploy < op-dbus-$VERSION.send"
    echo
    echo "3. Mount the deployment:"
    echo "   sudo ./scripts/post-deploy/mount-deployment.sh $VERSION"
}

main() {
    log_info "Starting Operation D-Bus send stream generation for version $VERSION..."

    check_args "$@"
    check_root
    validate_snapshot

    local parent_snapshot
    parent_snapshot=$(find_parent_snapshot)

    # Generate send stream to stdout, calculate checksum, then update metadata
    {
        generate_send_stream "$parent_snapshot"
    } | tee >(calculate_checksum) | cat

    generate_metadata
    show_usage_instructions
}

main "$@"