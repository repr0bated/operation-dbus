#!/usr/bin/env bash
# Deploy op-dbus FULL privacy router to oo1424oo
# Privacy router: Gateway + WARP + Xray containers

set -e

SERVER="oo1424oo"
SSH_KEY="${SSH_KEY:-$HOME/.ssh/ghostbridge_key}"

echo "=== Deploying Full Privacy Router to $SERVER ==="
echo "Containers: Gateway (NAT) + WARP (Cloudflare) + Xray (Proxy)"
echo ""

# Check if we can connect
if ! ssh -i "$SSH_KEY" root@$SERVER "echo 'Connection OK'" 2>/dev/null; then
    echo "ERROR: Cannot connect to $SERVER"
    echo "Usage: SSH_KEY=/path/to/key $0"
    exit 1
fi

# Step 1: Backup existing configuration
echo "[1/6] Backing up existing configuration..."
ssh -i "$SSH_KEY" root@$SERVER << 'EOF'
mkdir -p /root/nixos-backups
if [ -f /etc/nixos/configuration.nix ]; then
    cp /etc/nixos/configuration.nix /root/nixos-backups/configuration.nix.$(date +%Y%m%d-%H%M%S)
    echo "Backup created"
else
    echo "No existing config to backup"
fi
EOF

# Step 2: Copy files to server
echo "[2/6] Copying files to $SERVER..."
ssh -i "$SSH_KEY" root@$SERVER "mkdir -p /tmp/op-dbus-deploy"
scp -i "$SSH_KEY" -r ../nix root@$SERVER:/tmp/op-dbus-deploy/

# Step 3: Install NixOS configuration
echo "[3/6] Installing NixOS configuration..."
ssh -i "$SSH_KEY" root@$SERVER << 'EOF'
cd /tmp/op-dbus-deploy/nix

# Copy configuration
cp oo1424oo-config.nix /etc/nixos/configuration.nix

# Copy module files
cp module.nix /etc/nixos/
cp package.nix /etc/nixos/
cp flake.nix /etc/nixos/

echo "Configuration installed"
EOF

# Step 4: Build and switch
echo "[4/6] Building NixOS configuration..."
echo "NOTE: This may take 10-20 minutes for first build"
ssh -i "$SSH_KEY" root@$SERVER << 'EOF'
nixos-rebuild switch 2>&1 | tee /tmp/nixos-rebuild.log
EOF

# Step 5: Check status
echo "[5/6] Checking deployment status..."
ssh -i "$SSH_KEY" root@$SERVER << 'EOF'
echo "=== op-dbus status ==="
systemctl status op-dbus --no-pager || true

echo ""
echo "=== OVS bridges ==="
ovs-vsctl show || echo "OVS not ready yet"

echo ""
echo "=== LXC containers ==="
lxc-ls -f || echo "No containers yet"

echo ""
echo "=== OpenFlow rules ==="
ovs-ofctl dump-flows ovsbr0 2>/dev/null || echo "No flows yet"
EOF

# Step 6: Show logs
echo "[6/6] Recent op-dbus logs..."
ssh -i "$SSH_KEY" root@$SERVER << 'EOF'
journalctl -u op-dbus -n 50 --no-pager
EOF

echo ""
echo "=== Deployment Complete! ==="
echo ""
echo "Next steps:"
echo "1. SSH to server: ssh -i $SSH_KEY root@$SERVER"
echo "2. Verify containers: lxc-ls -f"
echo "3. Configure containers:"
echo "   - Gateway: Setup NAT/firewall"
echo "   - WARP: Install wgcf and configure"
echo "   - Xray: Install and configure proxy"
echo ""
echo "See nix/PRIVACY-ROUTER.md for detailed setup instructions"
