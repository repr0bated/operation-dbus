#!/bin/bash
# LXC hook to automatically join container to netmaker on start
# This should be placed at /usr/share/lxc/hooks/netmaker-join
# and configured in container config as: lxc.hook.start-host

# Get container ID from LXC environment
CT_ID="${LXC_NAME##*-}"  # Extract ID from name like "pve-container-100"

# Paths
NETMAKER_ENV="/etc/op-dbus/netmaker.env"
LOG_FILE="/var/log/lxc-netmaker-hook.log"

# Logging function
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [CT$CT_ID] $1" >> "$LOG_FILE"
}

log "Hook triggered for container $CT_ID"

# Check if this is a netmaker-enabled container
# Look for network_type=netmaker in container metadata
CONTAINER_CONFIG="/etc/pve/lxc/${CT_ID}.conf"
if [ ! -f "$CONTAINER_CONFIG" ]; then
    log "Container config not found, skipping"
    exit 0
fi

# Check if container should use netmaker (look for mesh bridge)
if ! grep -q "bridge=mesh" "$CONTAINER_CONFIG" 2>/dev/null; then
    log "Container not using mesh bridge, skipping netmaker join"
    exit 0
fi

log "Container uses mesh bridge, proceeding with netmaker join"

# Load netmaker token from host
if [ ! -f "$NETMAKER_ENV" ]; then
    log "ERROR: Netmaker env file not found at $NETMAKER_ENV"
    exit 0
fi

source "$NETMAKER_ENV"

if [ -z "$NETMAKER_TOKEN" ]; then
    log "WARNING: NETMAKER_TOKEN not set in $NETMAKER_ENV"
    exit 0
fi

# Wait for container to be fully started
log "Waiting for container to be ready..."
sleep 3

# Check if netclient is installed in container
if ! pct exec "$CT_ID" -- which netclient >/dev/null 2>&1; then
    log "WARNING: netclient not found in container (use netmaker-ready template)"
    exit 0
fi

log "netclient found in container"

# Check if already joined
if pct exec "$CT_ID" -- netclient list 2>/dev/null | grep -q "Connected networks:"; then
    log "Container already joined to netmaker"
    exit 0
fi

# Join container to netmaker
log "Joining container to netmaker network..."
if pct exec "$CT_ID" -- netclient join -t "$NETMAKER_TOKEN" >> "$LOG_FILE" 2>&1; then
    log "SUCCESS: Container joined netmaker network"

    # Get netmaker interface name
    sleep 2
    NETMAKER_IFACE=$(pct exec "$CT_ID" -- ip -j link show | jq -r '.[] | select(.ifname | startswith("nm-")) | .ifname' | head -1)

    if [ -n "$NETMAKER_IFACE" ]; then
        log "Netmaker interface created: $NETMAKER_IFACE"
    fi
else
    log "ERROR: Failed to join netmaker network"
    exit 1
fi

log "Hook completed successfully"
exit 0
