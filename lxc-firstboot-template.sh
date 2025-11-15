#!/bin/bash
# lxc-firstboot-template.sh - General-purpose LXC container first-boot initialization
# This script runs once on first boot, then disables itself
#
# Usage: This template should be customized per container and injected during creation
#        Copy to container's /usr/local/bin/lxc-firstboot.sh and create systemd service

set -euo pipefail

# Configuration - these will be replaced during container creation
CONTAINER_ID="{{CONTAINER_ID}}"
CONTAINER_HOSTNAME="{{CONTAINER_HOSTNAME}}"
NETWORK_TYPE="{{NETWORK_TYPE}}"  # bridge, socket, or netmaker
PACKAGES_TO_INSTALL="{{PACKAGES}}"  # Space-separated list

# Marker file to prevent re-running
MARKER_FILE="/var/lib/lxc-firstboot-complete"
LOG_FILE="/var/log/lxc-firstboot.log"

# Logging function
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

# Exit if already initialized
if [ -f "$MARKER_FILE" ]; then
    log "First-boot already completed, exiting"
    exit 0
fi

log "==================================="
log "LXC First-Boot Initialization"
log "==================================="
log "Container ID: $CONTAINER_ID"
log "Hostname: $CONTAINER_HOSTNAME"
log "Network Type: $NETWORK_TYPE"
log ""

# Wait for network to be ready (especially important for socket networking)
log "Waiting for network to be ready..."
for i in {1..30}; do
    if ip link show eth0 &>/dev/null || [ "$NETWORK_TYPE" = "socket" ]; then
        log "✓ Network interface ready"
        break
    fi
    sleep 1
done

# Update system (optional - can be disabled for faster boot)
if [ "${UPDATE_SYSTEM:-no}" = "yes" ]; then
    log "Updating system packages..."
    apt-get update -qq
    apt-get upgrade -y -qq
    log "✓ System updated"
fi

# Install requested packages
if [ -n "$PACKAGES_TO_INSTALL" ] && [ "$PACKAGES_TO_INSTALL" != "{{PACKAGES}}" ]; then
    log "Installing packages: $PACKAGES_TO_INSTALL"
    apt-get update -qq
    apt-get install -y -qq $PACKAGES_TO_INSTALL
    log "✓ Packages installed"
fi

# Configure hostname
if [ "$CONTAINER_HOSTNAME" != "{{CONTAINER_HOSTNAME}}" ]; then
    log "Setting hostname to: $CONTAINER_HOSTNAME"
    echo "$CONTAINER_HOSTNAME" > /etc/hostname
    hostname "$CONTAINER_HOSTNAME"

    # Update /etc/hosts
    sed -i "s/127.0.1.1.*/127.0.1.1\t$CONTAINER_HOSTNAME/" /etc/hosts || \
        echo "127.0.1.1\t$CONTAINER_HOSTNAME" >> /etc/hosts

    log "✓ Hostname configured"
fi

# Network-specific initialization
case "$NETWORK_TYPE" in
    socket)
        log "Configuring socket networking..."
        # Socket networking uses host network stack via OVS internal ports
        # No additional configuration needed - networking is handled by OVS
        log "✓ Socket networking ready (managed by op-dbus)"
        ;;

    netmaker)
        log "Configuring Netmaker networking..."

        # Check if netclient is installed
        if ! command -v netclient &>/dev/null; then
            log "Installing netclient..."
            apt-get update -qq
            apt-get install -y -qq curl wireguard-tools iptables

            # Download netclient
            NETCLIENT_VERSION="${NETCLIENT_VERSION:-v0.25.0}"
            NETCLIENT_URL="https://github.com/gravitl/netclient/releases/download/${NETCLIENT_VERSION}/netclient"

            curl -fsSL "$NETCLIENT_URL" -o /usr/local/bin/netclient
            chmod +x /usr/local/bin/netclient

            log "✓ Netclient installed: $(netclient --version)"
        fi

        # Check for enrollment token
        if [ -f /etc/netmaker/enrollment-token ]; then
            TOKEN=$(cat /etc/netmaker/enrollment-token)
            log "Found Netmaker enrollment token"

            # Enroll in Netmaker
            if netclient join --token "$TOKEN"; then
                log "✓ Successfully enrolled in Netmaker"

                # Create systemd service
                cat > /etc/systemd/system/netclient.service <<'SERVICE_EOF'
[Unit]
Description=Netclient
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/netclient daemon
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
SERVICE_EOF

                systemctl daemon-reload
                systemctl enable netclient.service
                systemctl start netclient.service

                log "✓ Netclient service enabled"
            else
                log "⚠ Failed to enroll in Netmaker"
            fi
        else
            log "⚠ No Netmaker token found at /etc/netmaker/enrollment-token"
        fi
        ;;

    bridge)
        log "Bridge networking (standard LXC networking)"
        log "✓ Using default bridge configuration"
        ;;

    *)
        log "Unknown network type: $NETWORK_TYPE"
        ;;
esac

# Run custom initialization script if exists
if [ -f /usr/local/bin/lxc-firstboot-custom.sh ]; then
    log "Running custom initialization script..."
    chmod +x /usr/local/bin/lxc-firstboot-custom.sh
    if bash /usr/local/bin/lxc-firstboot-custom.sh; then
        log "✓ Custom initialization completed"
    else
        log "⚠ Custom initialization failed (exit code: $?)"
    fi
fi

# Mark as complete
touch "$MARKER_FILE"
log "✓ First-boot initialization complete"
log "==================================="

exit 0
