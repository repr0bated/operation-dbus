# RustRover SSH Setup for Operation D-Bus

## ✅ Setup Complete!

SSH access to `/git/operation-dbus` has been configured successfully. Here's how to connect RustRover:

## SSH Configuration Details

- **Host**: `proxmox.ghostbridge.tech` or `80.209.240.244`
- **User**: `jeremy`
- **Port**: `22`
- **SSH Key**: `~/.ssh/id_rsa`
- **Project Path**: `/git/operation-dbus`

## RustRover Configuration Steps

### Method 1: Direct SSH Configuration in RustRover

1. **Open RustRover**
2. **Go to Settings**: `File` → `Settings` → `Tools` → `SSH Configurations`
3. **Add New Configuration**:
   - Click the `+` button
   - **Host**: `proxmox.ghostbridge.tech`
   - **Port**: `22`
   - **User name**: `jeremy`
   - **Authentication type**: `Key pair (OpenSSH or PuTTY)`
   - **Private key file**: Select `~/.ssh/id_rsa` (or the full path to your private key)
   - **Passphrase**: Leave empty (unless your key has one)

4. **Test Connection**: Click `Test Connection` to verify

### Method 2: Using SSH Config File

If you prefer using the SSH config file that was created:

1. **Add to your SSH config** (optional):
   ```bash
   cat ~/.ssh/config_rustrover >> ~/.ssh/config
   ```

2. **In RustRover**:
   - **Host**: `operation-dbus`
   - **Port**: `22`
   - **User name**: `jeremy`
   - **Authentication type**: `Key pair (OpenSSH or PuTTY)`
   - **Private key file**: `~/.ssh/id_rsa`

## Opening the Project in RustRover

### Option A: Remote Development
1. In RustRover: `File` → `Open`
2. Select `Remote Development`
3. Choose `SSH` as connection type
4. Select your SSH configuration from dropdown
5. Set **Project path** to: `/git/operation-dbus`
6. Click `Download and Open`

### Option B: Direct Open
1. In RustRover: `File` → `Open`
2. Enter the SSH URL: `ssh://jeremy@proxmox.ghostbridge.tech/git/operation-dbus`
3. Click `OK`

## Testing the Setup

### Manual SSH Test
```bash
# Test connection
ssh operation-dbus

# Test project access
ssh operation-dbus 'ls -la /git/operation-dbus | head -5'
```

### RustRover Test
1. Connect using the configuration above
2. Try opening a file from `/git/operation-dbus/src/main.rs`
3. Test building: Right-click on `Cargo.toml` → `Run Cargo Build`

## Troubleshooting

### Connection Issues
```bash
# Check SSH service
sudo systemctl status sshd

# Test manual connection
ssh -v jeremy@proxmox.ghostbridge.tech

# Check SSH keys
ls -la ~/.ssh/
ssh-keygen -l -f ~/.ssh/id_rsa
```

### Permission Issues
```bash
# Check directory permissions
ls -la /git/operation-dbus | head -3

# Check ownership
stat -c '%U:%G %a %n' /git/operation-dbus
```

### Firewall Issues
```bash
# Check if SSH port is accessible
telnet proxmox.ghostbridge.tech 22
```

## Security Notes

- ✅ SSH key authentication (no passwords)
- ✅ Connections are encrypted
- ✅ Known hosts verification enabled
- ✅ Strict host key checking disabled for convenience

## Development Workflow

1. **Connect**: Use RustRover's remote development features
2. **Code**: Edit files directly on the server
3. **Build**: Run `cargo build --release` remotely
4. **Test**: Execute `cargo test` remotely
5. **Deploy**: Use deployment scripts when ready

## Files Created

- `~/.ssh/config_rustrover` - SSH configuration for Operation D-Bus
- `~/.ssh/known_hosts_rustrover` - Known hosts file for RustRover connections

## Quick Reference

**SSH Command**: `ssh operation-dbus`
**Project Path**: `/git/operation-dbus`
**Build Command**: `cargo build --release`
**Test Command**: `cargo test`

---

**Status**: ✅ SSH access configured and tested successfully!

You can now connect RustRover to the Operation D-Bus project via SSH.