#!/bin/bash
# Enable op-dbus service for reboot test

echo "Enabling op-dbus service for boot..."

# Enable the service (will start at next boot)
systemctl enable op-dbus

echo "✓ Service enabled"
echo ""
echo "Current status:"
systemctl is-enabled op-dbus || echo "Not enabled"
echo ""
echo "Service will start at next boot and apply network configuration."
echo ""
echo "After reboot, check:"
echo "  systemctl status op-dbus"
echo "  journalctl -u op-dbus"
echo "  # Check OVSDB via: echo '{\"method\":\"list_dbs\",\"params\":[],\"id\":0}' | socat - UNIX-CONNECT:/var/run/openvswitch/db.sock"
echo "  ip addr show ovsbr0"
echo ""
echo "Ready to reboot!"
