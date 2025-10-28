# Systemd Service for D-Bus MCP Orchestrator

This directory contains systemd service files to automatically start the D-Bus MCP orchestrator.

## Installation

### User Service (Recommended)

Install as a user service (runs when you log in):

```bash
# Create user systemd directory
mkdir -p ~/.config/systemd/user/

# Copy service file
cp systemd/dbus-orchestrator.service ~/.config/systemd/user/

# Reload systemd
systemctl --user daemon-reload

# Enable to start on login
systemctl --user enable dbus-orchestrator.service

# Start now
systemctl --user start dbus-orchestrator.service

# Check status
systemctl --user status dbus-orchestrator.service
```

### System Service (Optional)

Install as a system service (runs at boot for all users):

```bash
# Copy to system location
sudo cp systemd/dbus-orchestrator.service /etc/systemd/system/

# Edit the service file to set the correct user
sudo systemctl edit dbus-orchestrator.service

# Add this content:
# [Service]
# User=YOUR_USERNAME
# Group=YOUR_USERNAME

# Reload systemd
sudo systemctl daemon-reload

# Enable to start at boot
sudo systemctl enable dbus-orchestrator.service

# Start now
sudo systemctl start dbus-orchestrator.service

# Check status
sudo systemctl status dbus-orchestrator.service
```

## Management Commands

### User Service

```bash
# Start
systemctl --user start dbus-orchestrator.service

# Stop
systemctl --user stop dbus-orchestrator.service

# Restart
systemctl --user restart dbus-orchestrator.service

# Status
systemctl --user status dbus-orchestrator.service

# Logs
journalctl --user -u dbus-orchestrator.service -f

# Enable (start on login)
systemctl --user enable dbus-orchestrator.service

# Disable
systemctl --user disable dbus-orchestrator.service
```

### System Service

Replace `--user` with nothing and add `sudo`:

```bash
sudo systemctl start dbus-orchestrator.service
sudo journalctl -u dbus-orchestrator.service -f
```

## Updating the Binary Path

If you move the project or use a release build, update the service file:

```bash
# Edit the service
systemctl --user edit --full dbus-orchestrator.service

# Change ExecStart to:
ExecStart=/path/to/dbus-mcp-server/target/release/dbus-orchestrator

# Reload and restart
systemctl --user daemon-reload
systemctl --user restart dbus-orchestrator.service
```

## Troubleshooting

### Service won't start

Check logs:
```bash
journalctl --user -u dbus-orchestrator.service -n 50
```

### D-Bus connection failed

Ensure D-Bus session bus is running:
```bash
echo $DBUS_SESSION_BUS_ADDRESS
```

### Permission denied

Make sure the binary is executable:
```bash
chmod +x /git/wayfire-mcp-server/target/debug/dbus-orchestrator
```

## Testing Without Systemd

To test manually:
```bash
/git/wayfire-mcp-server/target/debug/dbus-orchestrator
```

Should output:
```
Starting Wayfire D-Bus Orchestrator...
Orchestrator ready on D-Bus: org.dbusmcp.Orchestrator
Listening for agent spawn requests...
```
