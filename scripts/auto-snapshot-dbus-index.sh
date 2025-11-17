#!/bin/bash
# Automatic D-Bus Index Snapshot Script
# This script rebuilds the D-Bus index and creates a snapshot with rolling-3 retention

set -euo pipefail

LOCKFILE="/var/run/op-dbus-index.lock"
LOG_FILE="/var/log/op-dbus/auto-index.log"

# Ensure log directory exists
mkdir -p /var/log/op-dbus

# Function to log with timestamp
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

# Acquire lock to prevent concurrent runs
exec 200>"$LOCKFILE"
if ! flock -n 200; then
    log "ERROR: Another instance is already running"
    exit 1
fi

log "Starting D-Bus index update"

# Build the index (auto-creates snapshot with rolling-3 retention)
if /usr/local/bin/op-dbus index build >> "$LOG_FILE" 2>&1; then
    log "✅ Index build and snapshot successful"
else
    log "❌ Index build failed"
    exit 1
fi

# Optional: Send notification
# notify-send "D-Bus Index" "Index updated successfully"

log "Completed D-Bus index update"

# Release lock
flock -u 200
