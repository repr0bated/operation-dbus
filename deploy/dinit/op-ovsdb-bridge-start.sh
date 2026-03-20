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

call_dbus() {
  busctl --system --timeout="$BUSCTL_TIMEOUT_SECS" call "$@"
}

wait_for_kernel_link() {
  iface="$1"
  i=0
  while [ "$i" -lt 20 ]; do
    if ip link show "$iface" >/dev/null 2>&1; then
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

BRIDGE_EXISTS="$(call_dbus "$DBUS_DEST" "$OVS_PATH" "$OVS_IFACE" BridgeExists s "$BRIDGE" 2>/dev/null || true)"
if ! echo "$BRIDGE_EXISTS" | grep -q "true"; then
  echo "op-ovsdb-bridge: missing required bridge $BRIDGE (register via OVSDB D-Bus tool before boot)" >&2
  exit 1
fi

echo "op-ovsdb-bridge: validating uplink $UPLINK attached to $BRIDGE"
PORTS="$(call_dbus "$DBUS_DEST" "$OVS_PATH" "$OVS_IFACE" ListPorts s "$BRIDGE" 2>/dev/null || true)"
if echo "$PORTS" | grep -F "\"$UPLINK\"" >/dev/null 2>&1; then
  echo "op-ovsdb-bridge: uplink $UPLINK is present"
else
  echo "op-ovsdb-bridge: missing required uplink port $UPLINK on $BRIDGE (register via OVSDB D-Bus tool before boot)" >&2
  exit 1
fi

if wait_for_kernel_link "$BRIDGE"; then
  echo "op-ovsdb-bridge: kernel link $BRIDGE is present"
else
  echo "op-ovsdb-bridge: kernel link $BRIDGE did not appear after OVS restore" >&2
  exit 1
fi

# MirrorV1 does not always expose Introspectable reliably, so probe by method call.
if call_dbus "$MIRROR_DEST" "$MIRROR_PATH" "$MIRROR_IFACE" GetStats >/dev/null 2>&1; then
  call_dbus "$MIRROR_DEST" "$MIRROR_PATH" "$MIRROR_IFACE" Reconcile >/dev/null 2>&1 || true
elif call_dbus "$DBUS_DEST" "/org/opdbus" "$MIRROR_IFACE" GetStats >/dev/null 2>&1; then
  # Legacy fallback for older deployments where MirrorV1 lived on org.opdbus.
  call_dbus "$DBUS_DEST" "/org/opdbus" "$MIRROR_IFACE" Reconcile >/dev/null 2>&1 || true
else
  echo "op-ovsdb-bridge: mirror interface $MIRROR_IFACE unavailable on $MIRROR_DEST$MIRROR_PATH, skipping reconcile"
fi

echo "op-ovsdb-bridge: reconciliation complete"
