#!/bin/bash
set -e
echo "=== VPS Privacy Router Setup ==="
echo ""
echo "[1/5] Fixing WARP config (172->10)..."
sed -i 's/172.16.0.2/10.16.0.2/' /etc/wireguard/wgcf.conf
chmod 600 /etc/wireguard/wgcf.conf
echo "✓ Config fixed!"
echo ""
echo "[2/5] Creating socket bridge..."
ovs-vsctl add-br socket 2>/dev/null || echo "Bridge exists"
ovs-vsctl set bridge socket datapath_type=system
echo "✓ Bridge ready!"
echo ""
echo "[3/5] Stopping old wgcf if running..."
ip link show wgcf &>/dev/null && wg-quick down wgcf || echo "Not running"
echo ""
echo "[4/5] Starting WARP tunnel..."
wg-quick up wgcf
sleep 2
echo "✓ WARP tunnel up!"
echo ""
echo "[5/5] Adding wgcf to socket bridge..."
ovs-vsctl add-port socket wgcf 2>/dev/null || echo "Already added"
echo ""
echo "=== Setup Complete! ==="
echo ""
echo "Status:"
wg show wgcf
echo ""
ovs-vsctl show
echo ""
ip addr show wgcf | grep inet
