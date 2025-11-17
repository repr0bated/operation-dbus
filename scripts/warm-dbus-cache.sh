#!/bin/bash
# D-Bus Introspection Cache Warmup Script
# Proactively populates the SQLite cache with common D-Bus services
# Run this on system startup or via cron for optimal performance

set -euo pipefail

CACHE_DIR="/var/cache"
CACHE_FILE="${CACHE_DIR}/dbus-introspection.db"
LOG_FILE="/var/log/dbus-cache-warmup.log"

# Priority D-Bus services to cache (most frequently queried)
PRIORITY_SERVICES=(
    "org.freedesktop.systemd1"
    "org.freedesktop.NetworkManager"
    "org.freedesktop.login1"
    "org.freedesktop.PackageKit"
    "org.freedesktop.UDisks2"
    "org.freedesktop.UPower"
    "org.freedesktop.Notifications"
    "org.freedesktop.Accounts"
    "org.freedesktop.PolicyKit1"
    "org.bluez"
)

log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

check_dbus_running() {
    if ! systemctl is-active --quiet dbus.service; then
        log "ERROR: D-Bus service is not running"
        exit 1
    fi
}

ensure_cache_dir() {
    if [ ! -d "$CACHE_DIR" ]; then
        log "Creating cache directory: $CACHE_DIR"
        sudo mkdir -p "$CACHE_DIR"
        sudo chmod 755 "$CACHE_DIR"
    fi
}

introspect_service() {
    local service_name="$1"
    local start_time=$(date +%s%3N)

    # Derive default object path from service name
    local object_path="/${service_name//.//}"

    log "Introspecting: $service_name at $object_path"

    # Use dbus-send to introspect the service
    if timeout 5s dbus-send --system \
        --print-reply \
        --dest="$service_name" \
        "$object_path" \
        org.freedesktop.DBus.Introspectable.Introspect \
        >/dev/null 2>&1; then

        local end_time=$(date +%s%3N)
        local duration=$((end_time - start_time))
        log "  ✓ Cached $service_name (${duration}ms)"
        return 0
    else
        log "  ✗ Failed to introspect $service_name"
        return 1
    fi
}

main() {
    log "========================================="
    log "D-Bus Introspection Cache Warmup Started"
    log "========================================="

    check_dbus_running
    ensure_cache_dir

    local success_count=0
    local fail_count=0
    local total=${#PRIORITY_SERVICES[@]}

    log "Warming cache for $total priority services..."

    for service in "${PRIORITY_SERVICES[@]}"; do
        if introspect_service "$service"; then
            ((success_count++))
        else
            ((fail_count++))
        fi
    done

    log "========================================="
    log "Cache Warmup Complete"
    log "  Total services: $total"
    log "  Successfully cached: $success_count"
    log "  Failed: $fail_count"
    if [ -f "$CACHE_FILE" ]; then
        local cache_size=$(du -h "$CACHE_FILE" | cut -f1)
        log "  Cache size: $cache_size"
    fi
    log "========================================="

    exit 0
}

main "$@"
