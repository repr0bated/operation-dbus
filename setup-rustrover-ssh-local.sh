#!/bin/bash
set -euo pipefail

# Local RustRover SSH Setup Script for Operation D-Bus
# Run this script on your LOCAL machine (where RustRover is installed)
# This configures SSH access to connect to the remote Operation D-Bus server

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

# Configuration - Update these if needed
REMOTE_HOST="proxmox.ghostbridge.tech"
REMOTE_IP="80.209.240.244"
REMOTE_USER="jeremy"
SSH_PORT="22"
PROJECT_PATH="/git/operation-dbus"

check_ssh_keys() {
    log_step "Checking SSH keys..."

    local ssh_dir="$HOME/.ssh"
    local key_file="$ssh_dir/id_rsa"
    local pub_key_file="$key_file.pub"

    # Create SSH directory if it doesn't exist
    if [[ ! -d "$ssh_dir" ]]; then
        log_info "Creating SSH directory..."
        mkdir -p "$ssh_dir"
        chmod 700 "$ssh_dir"
    fi

    # Check if SSH key pair exists
    if [[ ! -f "$key_file" ]] || [[ ! -f "$pub_key_file" ]]; then
        log_warn "SSH key pair not found"
        echo
        log_info "Generating new SSH key pair..."
        echo "You'll be prompted for:"
        echo "  - Key location (press Enter for default: $key_file)"
        echo "  - Passphrase (recommended: press Enter for no passphrase)"
        echo

        ssh-keygen -t rsa -b 4096 -C "rustrover-operation-dbus-$(date +%Y%m%d)"

        if [[ ! -f "$key_file" ]]; then
            log_error "SSH key generation failed or was cancelled"
            exit 1
        fi

        log_info "SSH key pair generated successfully"
    else
        log_info "SSH key pair found: $key_file"
    fi

    # Verify key permissions
    local key_perms
    key_perms=$(stat -c '%a' "$key_file" 2>/dev/null || stat -f '%Lp' "$key_file" 2>/dev/null || echo "unknown")
    if [[ "$key_perms" != "600" ]]; then
        log_warn "Fixing SSH private key permissions..."
        chmod 600 "$key_file"
    fi

    local pub_perms
    pub_perms=$(stat -c '%a' "$pub_key_file" 2>/dev/null || stat -f '%Lp' "$pub_key_file" 2>/dev/null || echo "unknown")
    if [[ "$pub_perms" != "644" ]]; then
        chmod 644 "$pub_key_file"
    fi

    log_info "SSH keys are ready"
}

create_ssh_config() {
    log_step "Creating SSH configuration..."

    local config_file="$HOME/.ssh/config"
    local backup_file="$HOME/.ssh/config.backup.$(date +%Y%m%d_%H%M%S)"

    # Backup existing config if it exists
    if [[ -f "$config_file" ]]; then
        log_info "Backing up existing SSH config to $backup_file"
        cp "$config_file" "$backup_file"
    fi

    # Check if operation-dbus config already exists
    if grep -q "^Host operation-dbus$" "$config_file" 2>/dev/null; then
        log_warn "Operation D-Bus SSH config already exists in $config_file"
        log_warn "Please review and update manually if needed"
        return 0
    fi

    # Add operation-dbus configuration
    cat >> "$config_file" << EOF

# Operation D-Bus Development Server - Added by setup-rustrover-ssh-local.sh on $(date)
Host operation-dbus
    HostName $REMOTE_HOST
    User $REMOTE_USER
    Port $SSH_PORT
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
    HostName $REMOTE_IP
    User $REMOTE_USER
    Port $SSH_PORT
    IdentityFile ~/.ssh/id_rsa
    IdentitiesOnly yes
    StrictHostKeyChecking no
    UserKnownHostsFile ~/.ssh/known_hosts_operation_dbus
    ServerAliveInterval 60
    ServerAliveCountMax 3
    ForwardAgent no
    ForwardX11 no
EOF

    log_info "SSH configuration added to $config_file"
}

install_public_key() {
    log_step "Installing public key on remote server..."

    local pub_key_file="$HOME/.ssh/id_rsa.pub"
    local pub_key
    pub_key=$(cat "$pub_key_file")

    log_info "Attempting to install public key automatically..."

    # Try to install automatically
    if ssh-copy-id -i "$pub_key_file" -p "$SSH_PORT" -o StrictHostKeyChecking=no "$REMOTE_USER@$REMOTE_HOST" 2>/dev/null; then
        log_info "✅ Public key installed automatically!"
        return 0
    fi

    # If automatic installation failed, provide manual instructions
    log_warn "Automatic installation failed - you'll need to install manually"
    echo
    log_info "Please run this command on the SERVER (not your local machine):"
    echo "echo '$pub_key' >> ~/.ssh/authorized_keys"
    echo
    log_info "Or manually add this public key to ~/.ssh/authorized_keys on the server:"
    echo "--------------------------------------------------------------------------------"
    echo "$pub_key"
    echo "--------------------------------------------------------------------------------"
    echo
    read -p "Press Enter after you've installed the public key on the server..."
}

test_ssh_connection() {
    log_step "Testing SSH connection..."

    log_info "Testing connection to operation-dbus..."

    # Test with hostname
    if ssh -o ConnectTimeout=10 -o BatchMode=yes -o StrictHostKeyChecking=no operation-dbus "echo 'SSH connection successful!' && echo 'Project path: $PROJECT_PATH' && ls -ld '$PROJECT_PATH'" 2>/dev/null; then
        log_info "✅ SSH connection test passed!"
    else
        log_error "❌ SSH connection test failed"
        log_error "Please check:"
        log_error "  1. Public key is installed on server"
        log_error "  2. SSH service is running on server"
        log_error "  3. Network connectivity to $REMOTE_HOST:$SSH_PORT"
        log_error "  4. Firewall settings"
        exit 1
    fi
}

create_rustrover_instructions() {
    log_step "Creating RustRover setup instructions..."

    local instructions_file="$HOME/RUSTROVER-SETUP-INSTRUCTIONS.md"

    cat > "$instructions_file" << EOF
# RustRover SSH Setup Instructions - Operation D-Bus

## ✅ Local Setup Complete!

Your local machine is now configured to connect to the Operation D-Bus development server.

## SSH Configuration Summary

- **Host**: operation-dbus ($REMOTE_HOST)
- **User**: $REMOTE_USER
- **Port**: $SSH_PORT
- **SSH Key**: ~/.ssh/id_rsa
- **Project Path**: $PROJECT_PATH

## RustRover Configuration

### Step 1: Configure SSH in RustRover

1. **Open RustRover**
2. **Go to Settings**: File → Settings → Tools → SSH Configurations
3. **Click the + button** to add new configuration
4. **Fill in connection details**:
   - **Host**: $REMOTE_HOST
   - **Port**: $SSH_PORT
   - **User name**: $REMOTE_USER
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
5. Set **Project path** to: $PROJECT_PATH
6. Click **Download and Open**

#### Option B: Direct Open
1. In RustRover: File → Open
2. Enter: ssh://$REMOTE_USER@$REMOTE_HOST$PROJECT_PATH
3. Click OK

## Testing the Setup

### Test SSH Connection
\`\`\`bash
ssh operation-dbus
ls -la $PROJECT_PATH
\`\`\`

### Test in RustRover
1. Open any file from the project (e.g., src/main.rs)
2. Try building: Right-click Cargo.toml → Run 'Cargo Build'
3. Check if code completion works

## Troubleshooting

### Connection Issues
\`\`\`bash
# Test basic connectivity
ssh -v operation-dbus

# Test with IP address
ssh -v operation-dbus-ip

# Check SSH key
ssh-keygen -l -f ~/.ssh/id_rsa
\`\`\`

### Permission Issues
\`\`\`bash
# Check local key permissions
ls -la ~/.ssh/id_rsa*

# Check if key is in authorized_keys on server
ssh operation-dbus 'cat ~/.ssh/authorized_keys | grep "\$(cat ~/.ssh/id_rsa.pub)"'
\`\`\`

### Server Issues
\`\`\`bash
# Check if SSH service is running on server
ssh operation-dbus 'sudo systemctl status sshd'

# Check server SSH configuration
ssh operation-dbus 'sudo grep -E "^(PermitRootLogin|PasswordAuthentication)" /etc/ssh/sshd_config'
\`\`\`

## Development Workflow

1. **Connect**: Use RustRover's remote development
2. **Code**: Edit files directly on server
3. **Build**: Run builds remotely via RustRover
4. **Test**: Execute tests remotely
5. **Deploy**: Use deployment scripts when ready

## Quick Reference

**SSH Command**: \`ssh operation-dbus\`
**Project Path**: $PROJECT_PATH
**Build**: \`cargo build --release\`
**Test**: \`cargo test\`

## Files Created/Modified

- \`~/.ssh/config\` - Added operation-dbus configuration
- \`~/.ssh/id_rsa*\` - SSH key pair (if generated)
- \`~/.ssh/known_hosts_operation_dbus\` - Known hosts for this connection

---
**Setup completed on**: $(date)
**Remote server**: $REMOTE_HOST ($REMOTE_IP)
**Local user**: $(whoami)
EOF

    log_info "Instructions saved to: $instructions_file"
}

show_summary() {
    echo
    log_info "🎉 Setup Complete!"
    echo
    log_info "Summary:"
    echo "  ✅ SSH keys configured"
    echo "  ✅ SSH config updated"
    echo "  ✅ Public key installed on server"
    echo "  ✅ Connection tested successfully"
    echo "  ✅ Instructions created"
    echo
    log_info "Next steps:"
    echo "  1. Follow instructions in: $HOME/RUSTROVER-SETUP-INSTRUCTIONS.md"
    echo "  2. Configure RustRover with SSH settings above"
    echo "  3. Open the project: $PROJECT_PATH"
    echo
    log_info "Quick test: ssh operation-dbus 'ls $PROJECT_PATH'"
}

main() {
    echo "=========================================="
    echo "🛠️  RustRover SSH Setup for Operation D-Bus"
    echo "=========================================="
    echo
    log_info "This script configures your LOCAL machine for SSH access to Operation D-Bus"
    log_info "Remote server: $REMOTE_HOST ($REMOTE_IP)"
    echo

    check_ssh_keys
    create_ssh_config
    install_public_key
    test_ssh_connection
    create_rustrover_instructions
    show_summary

    echo
    echo "=========================================="
    echo "✅ Setup completed successfully!"
    echo "=========================================="
}

main "\$@"