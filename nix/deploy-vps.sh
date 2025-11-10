#!/usr/bin/env bash
# Deploy op-dbus with Xray proxy to VPS
# VPS: 80.209.240.244
# Container: Xray proxy server only

set -e

VPS_IP="${VPS_IP:-80.209.240.244}"
SSH_KEY="${SSH_KEY:-$HOME/.ssh/ghostbridge_key}"

echo "=== Deploying Xray Proxy to VPS ==="
echo "VPS: $VPS_IP"
echo "Container: Xray only (no gateway/WARP)"
echo ""

# Step 1: Copy files to VPS
echo "[1/5] Copying files to VPS..."
ssh -i "$SSH_KEY" root@$VPS_IP "mkdir -p /tmp/op-dbus-deploy"
scp -i "$SSH_KEY" -r ../nix root@$VPS_IP:/tmp/op-dbus-deploy/

# Step 2: Install NixOS configuration
echo "[2/5] Installing NixOS configuration..."
ssh -i "$SSH_KEY" root@$VPS_IP << 'EOF'
cd /tmp/op-dbus-deploy/nix

# Backup existing config
cp /etc/nixos/configuration.nix /etc/nixos/configuration.nix.backup || true

# Copy our config
cp vps-config.nix /etc/nixos/configuration.nix

# Copy module files
cp module.nix /etc/nixos/
cp package.nix /etc/nixos/
cp flake.nix /etc/nixos/

echo "Configuration installed"
EOF

# Step 3: Build and switch
echo "[3/5] Building NixOS configuration (this may take a while)..."
ssh -i "$SSH_KEY" root@$VPS_IP << 'EOF'
nixos-rebuild switch
EOF

# Step 4: Check op-dbus status
echo "[4/5] Checking op-dbus status..."
ssh -i "$SSH_KEY" root@$VPS_IP << 'EOF'
systemctl status op-dbus --no-pager || true
ovs-vsctl show || true
lxc-ls -f || true
EOF

# Step 5: Show logs
echo "[5/5] Showing op-dbus logs..."
ssh -i "$SSH_KEY" root@$VPS_IP << 'EOF'
journalctl -u op-dbus -n 50 --no-pager
EOF

echo ""
echo "=== Deployment Complete! ==="
echo ""
echo "Next steps:"
echo "1. SSH to VPS: ssh -i $SSH_KEY root@$VPS_IP"
echo "2. Check container: lxc-ls -f"
echo "3. Configure Xray container:"
echo "   lxc-attach -n 102"
echo "   # Install Xray and configure (see PRIVACY-ROUTER.md)"
echo "4. Test proxy connectivity"
