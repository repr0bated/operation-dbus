#!/bin/bash
# netmaker-firstboot.sh - Netmaker enrollment script for containers
# This script should be executed inside the container on first boot

set -euo pipefail

echo "🌐 Netmaker Firstboot Setup"
echo "=========================="

# Check if already enrolled
if command -v netclient >/dev/null 2>&1; then
    if netclient list 2>/dev/null | grep -q "connected.*true"; then
        echo "✅ Already enrolled in Netmaker"
        netclient list
        exit 0
    fi
fi

# Install dependencies
echo "📦 Installing dependencies..."
apt-get update -qq
apt-get install -y -qq curl wget wireguard-tools iptables

# Download and install netclient
echo "📥 Downloading netclient..."
NETCLIENT_VERSION="v0.25.0"
NETCLIENT_URL="https://github.com/gravitl/netclient/releases/download/${NETCLIENT_VERSION}/netclient"

wget -q -O /usr/local/bin/netclient "$NETCLIENT_URL"
chmod +x /usr/local/bin/netclient

echo "✅ Netclient installed: $(netclient --version)"

# Check for enrollment token
if [ -f /etc/netmaker/enrollment-token ]; then
    TOKEN=$(cat /etc/netmaker/enrollment-token)
    echo "🔑 Found enrollment token"
else
    echo "⚠️  No enrollment token found at /etc/netmaker/enrollment-token"
    echo "⚠️  Please provide token for manual enrollment"
    exit 1
fi

# Enroll in Netmaker
echo "🔗 Enrolling in Netmaker..."
if netclient join --token "$TOKEN"; then
    echo "✅ Successfully enrolled in Netmaker"

    # Show network status
    echo ""
    echo "Network status:"
    netclient list

    # Create systemd service for netclient
    echo "🔧 Creating systemd service..."
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

    echo "✅ Netclient service enabled and started"
else
    echo "❌ Failed to enroll in Netmaker"
    exit 1
fi

echo ""
echo "=========================="
echo "✅ Netmaker setup complete"
echo "=========================="
