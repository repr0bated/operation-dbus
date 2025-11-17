#!/bin/bash
set -euo pipefail

# Local RustRover SSH Setup Script for Operation D-Bus
# Run this script on your LOCAL machine (where RustRover is installed)
# This configures SSH access to connect to the remote Operation D-Bus server

echo "=========================================="
echo "🛠️  RustRover SSH Setup for Operation D-Bus"
echo "=========================================="
echo
echo "This script configures your LOCAL machine for SSH access to Operation D-Bus"
echo "Remote server: proxmox.ghostbridge.tech (80.209.240.244)"
echo

# Check if SSH keys exist
echo "Checking SSH keys..."
if [[ ! -f ~/.ssh/id_rsa ]]; then
    echo "Generating SSH key pair..."
    ssh-keygen -t rsa -b 4096 -C "rustrover-operation-dbus-$(date +%Y%m%d)"
fi

# Create SSH config
echo "Creating SSH configuration..."
cat >> ~/.ssh/config << 'EOF'

# Operation D-Bus Development Server
Host operation-dbus
    HostName proxmox.ghostbridge.tech
    User jeremy
    Port 22
    IdentityFile ~/.ssh/id_rsa
    IdentitiesOnly yes
    StrictHostKeyChecking no
    UserKnownHostsFile ~/.ssh/known_hosts_operation_dbus
    ServerAliveInterval 60
    ServerAliveCountMax 3
    ForwardAgent no
    ForwardX11 no

# Alternative connection using IP address
Host operation-dbus-ip
    HostName 80.209.240.244
    User jeremy
    Port 22
    IdentityFile ~/.ssh/id_rsa
    IdentitiesOnly yes
    StrictHostKeyChecking no
    UserKnownHostsFile ~/.ssh/known_hosts_operation_dbus
    ServerAliveInterval 60
    ServerAliveCountMax 3
    ForwardAgent no
    ForwardX11 no
EOF

echo "SSH configuration created"

# Try to install public key
echo "Installing public key on remote server..."
if ssh-copy-id -i ~/.ssh/id_rsa.pub -o StrictHostKeyChecking=no jeremy@proxmox.ghostbridge.tech 2>/dev/null; then
    echo "✅ Public key installed automatically!"
else
    echo "❌ Automatic key installation failed"
    echo
    echo "Please manually add this public key to ~/.ssh/authorized_keys on the server:"
    echo "--------------------------------------------------------------------------------"
    cat ~/.ssh/id_rsa.pub
    echo "--------------------------------------------------------------------------------"
    echo
    read -p "Press Enter after you've installed the public key on the server..."
fi

# Test connection
echo "Testing SSH connection..."
if ssh -o ConnectTimeout=10 -o BatchMode=yes operation-dbus 'echo "SSH connection successful!" && ls -ld /git/operation-dbus' 2>/dev/null; then
    echo "✅ SSH connection test passed!"
else
    echo "❌ SSH connection test failed"
    echo "Please check that the public key is installed on the server"
    exit 1
fi

# Create instructions
echo "Creating setup instructions..."
cat > ~/RUSTROVER-SETUP-INSTRUCTIONS.md << 'EOF2'
# RustRover SSH Setup Instructions - Operation D-Bus

## ✅ Local Setup Complete!

Your local machine is now configured to connect to the Operation D-Bus development server.

## SSH Configuration Summary

- **Host**: operation-dbus (proxmox.ghostbridge.tech)
- **User**: jeremy
- **Port**: 22
- **SSH Key**: ~/.ssh/id_rsa
- **Project Path**: /git/operation-dbus

## RustRover Configuration

### Step 1: Configure SSH in RustRover

1. **Open RustRover**
2. **Go to Settings**: File → Settings → Tools → SSH Configurations
3. **Click the + button** to add new configuration
4. **Fill in connection details**:
   - **Host**: proxmox.ghostbridge.tech
   - **Port**: 22
   - **User name**: jeremy
   - **Authentication type**: Key pair (OpenSSH or PuTTY)
   - **Private key file**: ~/.ssh/id_rsa
   - **Passphrase**: (leave empty if no passphrase set)
5. **Click Test Connection** - should show "Successfully connected!"
6. **Click OK** to save

### Step 2: Open the Project

#### Option A: Remote Development (Recommended)
1. In RustRover: File → Open
2. Select "Remote Development"
3. Choose "SSH" as connection type
4. Select "operation-dbus" from the dropdown
5. Set **Project path** to: /git/operation-dbus
6. Click **Download and Open**

#### Option B: Direct Open
1. In RustRover: File → Open
2. Enter: ssh://jeremy@proxmox.ghostbridge.tech/git/operation-dbus
3. Click OK

## Testing the Setup

### Test SSH Connection
```bash
ssh operation-dbus
ls -la /git/operation-dbus
```

### Test in RustRover
1. Open any file from the project (e.g., src/main.rs)
2. Try building: Right-click Cargo.toml → Run "Cargo Build"
3. Check if code completion works

## Troubleshooting

### Connection Issues
```bash
# Test basic connectivity
ssh -v operation-dbus

# Test with IP address
ssh -v operation-dbus-ip

# Check SSH key
ssh-keygen -l -f ~/.ssh/id_rsa
```

### Permission Issues
```bash
# Check local key permissions
ls -la ~/.ssh/id_rsa*

# Check if key is in authorized_keys on server
ssh operation-dbus 'cat ~/.ssh/authorized_keys | grep "$(cat ~/.ssh/id_rsa.pub)"'
```

## Quick Reference

**SSH Command**: `ssh operation-dbus`
**Project Path**: /git/operation-dbus
**Build**: `cargo build --release`
**Test**: `cargo test`

---
**Setup completed on**: '$(date)'
**Remote server**: proxmox.ghostbridge.tech (80.209.240.244)
**Local user**: '$(whoami)'
EOF2

echo "=========================================="
echo "🎉 Setup Complete!"
echo "=========================================="
echo
echo "Instructions saved to: ~/RUSTROVER-SETUP-INSTRUCTIONS.md"
echo
echo "Quick test: ssh operation-dbus 'ls /git/operation-dbus'
echo
echo "Next: Configure RustRover using the instructions above"

