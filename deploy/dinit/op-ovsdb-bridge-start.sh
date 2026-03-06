#!/bin/sh
set -eu

if [ -f /etc/op-dbus/environment ]; then
  # shellcheck disable=SC1091
  . /etc/op-dbus/environment
fi

DBUS_DEST="${OP_DBUS_DEST:-org.opdbus}"
OVS_PATH="${OP_DBUS_OVS_PATH:-/org/opdbus/ovsdb}"
OVS_IFACE="${OP_DBUS_OVS_IFACE:-org.opdbus.OvsdbV1}"
MIRROR_DEST="${OP_DBUS_MIRROR_DEST:-org.opdbus.v1}"
MIRROR_PATH="${OP_DBUS_MIRROR_PATH:-/org/opdbus/v1}"
MIRROR_IFACE="${OP_DBUS_MIRROR_IFACE:-org.opdbus.MirrorV1}"
RTNET_PATH="${OP_DBUS_RTNET_PATH:-/org/opdbus/rtnetlink}"
RTNET_IFACE="${OP_DBUS_RTNET_IFACE:-org.opdbus.RtnetlinkV1}"
BRIDGE="${PRIVACY_BRIDGE_NAME:-ovsbr0}"
UPLINK="${PRIVACY_UPLINK_PORT:-ens3}"
BUSCTL_TIMEOUT_SECS="${OP_DBUS_BUSCTL_TIMEOUT_SECS:-3}"

wait_for_opdbus() {
  i=0
  while [ "$i" -lt 60 ]; do
    if busctl --system status "$DBUS_DEST" >/dev/null 2>&1; then
      return 0
    fi
    i=$((i + 1))
    sleep 1
  done
  return 1
}

if ! wait_for_opdbus; then
  echo "op-ovsdb-bridge: D-Bus service $DBUS_DEST unavailable after timeout" >&2
  exit 1
fi

BRIDGE_EXISTS="$(busctl --system call "$DBUS_DEST" "$OVS_PATH" "$OVS_IFACE" BridgeExists s "$BRIDGE" 2>/dev/null || true)"
if ! echo "$BRIDGE_EXISTS" | grep -q "true"; then
  echo "op-ovsdb-bridge: bridge $BRIDGE is missing (bootstrap required)" >&2
  echo "op-ovsdb-bridge: create bridge once outside dinit, then reboot/restart service" >&2
  exit 1
fi
echo "op-ovsdb-bridge: bridge $BRIDGE present; applying boot-time bring-up"

if ip link show "$UPLINK" >/dev/null 2>&1; then
  PORTS="$(busctl --system call "$DBUS_DEST" "$OVS_PATH" "$OVS_IFACE" ListPorts s "$BRIDGE" 2>/dev/null)"
  if ! echo "$PORTS" | grep -F "\"$UPLINK\"" >/dev/null 2>&1; then
    busctl --system call "$DBUS_DEST" "$OVS_PATH" "$OVS_IFACE" AddPort ss "$BRIDGE" "$UPLINK" >/dev/null
    echo "op-ovsdb-bridge: added uplink $UPLINK to $BRIDGE"
  else
    echo "op-ovsdb-bridge: uplink $UPLINK already attached"
  fi
else
  echo "op-ovsdb-bridge: uplink $UPLINK not present, skipping AddPort"
fi

if busctl --system introspect "$DBUS_DEST" "$RTNET_PATH" 2>/dev/null | grep -q "$RTNET_IFACE"; then
  busctl --system call "$DBUS_DEST" "$RTNET_PATH" "$RTNET_IFACE" LinkUp s "$BRIDGE" >/dev/null 2>&1 || true
  echo "op-ovsdb-bridge: requested LinkUp for bridge $BRIDGE via rtnetlink D-Bus"

  if ip link show "$UPLINK" >/dev/null 2>&1; then
    busctl --system call "$DBUS_DEST" "$RTNET_PATH" "$RTNET_IFACE" LinkUp s "$UPLINK" >/dev/null 2>&1 || true
    echo "op-ovsdb-bridge: requested LinkUp for uplink $UPLINK via rtnetlink D-Bus"
  fi
else
  echo "op-ovsdb-bridge: rtnetlink interface $RTNET_IFACE unavailable, skipping LinkUp"
fi

# MirrorV1 does not always expose Introspectable reliably, so probe by method call.
if busctl --system --timeout="$BUSCTL_TIMEOUT_SECS" call "$MIRROR_DEST" "$MIRROR_PATH" "$MIRROR_IFACE" GetStats >/dev/null 2>&1; then
  busctl --system --timeout="$BUSCTL_TIMEOUT_SECS" call "$MIRROR_DEST" "$MIRROR_PATH" "$MIRROR_IFACE" Reconcile >/dev/null 2>&1 || true
elif busctl --system --timeout="$BUSCTL_TIMEOUT_SECS" call "$DBUS_DEST" "/org/opdbus" "$MIRROR_IFACE" GetStats >/dev/null 2>&1; then
  # Legacy fallback for older deployments where MirrorV1 lived on org.opdbus.
  busctl --system --timeout="$BUSCTL_TIMEOUT_SECS" call "$DBUS_DEST" "/org/opdbus" "$MIRROR_IFACE" Reconcile >/dev/null 2>&1 || true
else
  echo "op-ovsdb-bridge: mirror interface $MIRROR_IFACE unavailable on $MIRROR_DEST$MIRROR_PATH, skipping reconcile"
fi

echo "op-ovsdb-bridge: reconciliation complete"
