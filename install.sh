#!/bin/bash
# op-dbus installer/upgrader (FULL by default)
# - Always cargo build --release
# - Installs /usr/local/bin/op-dbus
# - Writes /etc/op-dbus/state.json with ONLY: ovsbr0, meshbr (IP-less by default)
# - --atomic: uplink->ovsbr0 handover is done DECLARATIVELY by op-dbus apply
# - Proxmox/LXC global defaults: no NICs by default (socket-first)
# - Container Netmaker first-boot injector (LXC start-host hook)
# - Host order: diff -> apply (create bridges) -> [--atomic: attach uplink, write IP, re-apply] -> host netmaker join -> enslave nm-* to meshbr -> enable op-dbus
set -euo pipefail

LOG_FILE="/var/log/op-dbus-install.log"
mkdir -p "$(dirname "$LOG_FILE")" /etc/op-dbus /usr/local/bin
touch "$LOG_FILE"
exec > >(tee -a "$LOG_FILE") 2>&1

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'

# -------- Flags --------
NO_PROXMOX=false
AGENT_ONLY=false
ATOMIC=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-proxmox) NO_PROXMOX=true; shift ;;
    --agent-only) AGENT_ONLY=true; NO_PROXMOX=true; shift ;;
    --atomic) ATOMIC=true; shift ;;
    --help|-h)
      cat <<'HLP'
Usage: ./install.sh [--atomic] [--no-proxmox] [--agent-only]
Default: Full install (Proxmox/LXC features enabled when detected)
--atomic  Attach uplink to ovsbr0, write IP/gateway into state.json, then op-dbus apply to set L3.
HLP
      exit 0;;
    *) echo -e "${YELLOW}Ignoring arg:${NC} $1"; shift ;;
  esac
done

# -------- Root check --------
[ "$EUID" -eq 0 ] || { echo -e "${RED}Run as root${NC}"; exit 1; }

# -------- Introspection --------
SCRIPT_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HAS_SYSTEMD=false; SYSTEMD_DIR=""
if command -v systemctl >/dev/null 2>&1; then HAS_SYSTEMD=true; SYSTEMD_DIR="/etc/systemd/system"; fi

INSTALL_DIR=""
for d in /usr/local/bin /usr/bin; do [ -d "$d" ] && [ -w "$d" ] && INSTALL_DIR="$d" && break; done
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

CONFIG_DIR="${CONFIG_DIR:-/etc/op-dbus}"
STATE_FILE="$CONFIG_DIR/state.json"

OVSDB_SOCK=""
for s in /var/run/openvswitch/db.sock /run/openvswitch/db.sock; do [ -S "$s" ] && OVSDB_SOCK="$s" && break; done
OVSDB_SOCK="${OVSDB_SOCK:-/var/run/openvswitch/db.sock}"

need=()
for bin in jq socat ip awk sed grep cut tee cargo; do command -v "$bin" >/dev/null 2>&1 || need+=("$bin"); done
if [ "${#need[@]}" -gt 0 ]; then
  echo -e "${YELLOW}Missing tools:${NC} ${need[*]}"
  [[ " ${need[*]} " =~ " cargo " ]] && { echo -e "${RED}Cargo is required.${NC}"; exit 1; }
fi

IS_PROXMOX=false
if command -v pct >/dev/null 2>&1 && [ -d /etc/pve ]; then IS_PROXMOX=true; fi

if [ -d /usr/share/lxc/config/common.conf.d ]; then
  LXC_COMMON_DIR="/usr/share/lxc/config/common.conf.d"
elif [ -d /etc/lxc ]; then
  mkdir -p /etc/lxc/common.conf.d; LXC_COMMON_DIR="/etc/lxc/common.conf.d"
else
  mkdir -p /usr/share/lxc/config/common.conf.d; LXC_COMMON_DIR="/usr/share/lxc/config/common.conf.d"
fi

# Fixed bridge names
MAIN_BRIDGE="ovsbr0"
MESH_BRIDGE="meshbr"

echo "Detected:"
echo "  systemd:      $HAS_SYSTEMD"
echo "  install dir:  $INSTALL_DIR"
echo "  config dir:   $CONFIG_DIR"
echo "  OVS socket:   $OVSDB_SOCK"
echo "  proxmox:      $IS_PROXMOX"
echo "  lxc dir:      $LXC_COMMON_DIR"
echo "  bridges:      MAIN=$MAIN_BRIDGE  MESH=$MESH_BRIDGE"
echo "  atomic L3:    $ATOMIC"
echo ""

# -------- Stop service if running --------
if $HAS_SYSTEMD && systemctl is-active --quiet op-dbus 2>/dev/null; then
  echo "Stopping op-dbus service..."; systemctl stop op-dbus; echo -e "${GREEN}✓${NC} Stopped"
fi

# -------- Always build (upgrade-safe) --------
echo "Building (cargo build --release)..."
( cd "$SCRIPT_DIR" && cargo build --release )
BINARY_PATH="$SCRIPT_DIR/target/release/op-dbus"
[ -f "$BINARY_PATH" ] || { echo -e "${RED}Build failed: binary missing${NC}"; exit 1; }
echo -e "${GREEN}✓${NC} Built: $BINARY_PATH"

# -------- Install binary --------
echo "Installing binary to $INSTALL_DIR..."
install -m 0755 "$BINARY_PATH" "$INSTALL_DIR/op-dbus"
echo -e "${GREEN}✓${NC} Installed: $INSTALL_DIR/op-dbus"

# -------- state.json with ONLY ovsbr0 + meshbr (IP-less) --------
mkdir -p "$CONFIG_DIR"
cat > "$STATE_FILE" <<EOFJSON
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": [
        { "name": "$MAIN_BRIDGE", "type": "ovs-bridge", "ports": [], "ipv4": { "enabled": false } },
        { "name": "$MESH_BRIDGE", "type": "ovs-bridge", "ports": [], "ipv4": { "enabled": false } }
      ]
    },
    "systemd": {
      "units": {
        "openvswitch-switch.service": { "active_state": "active", "enabled": true }
      }
    }
  }
}
EOFJSON
echo -e "${GREEN}✓${NC} Wrote $STATE_FILE (ONLY: $MAIN_BRIDGE, $MESH_BRIDGE, IP-less)"

# =========================
# OVSDB helpers (JSON-RPC)
# =========================
ovsdb_rpc() { echo "{\"method\":\"$1\",\"params\":$2,\"id\":0}" | socat - UNIX-CONNECT:"$OVSDB_SOCK" 2>/dev/null | head -1; }
ovsdb_list_ports() {
  local br="$1" r u ps p pr
  r=$(ovsdb_rpc "transact" "[\"Open_vSwitch\",[{\"op\":\"select\",\"table\":\"Bridge\",\"where\":[[\"name\",\"==\",\"$br\"]],\"columns\":[\"_uuid\",\"ports\"]}]]")
  u=$(echo "$r" | jq -r '.result[0].rows[0]._uuid[1]' 2>/dev/null)
  [ -z "$u" ] || [ "$u" = "null" ] && return 1
  ps=$(echo "$r" | jq -r '.result[0].rows[0].ports[1][]?[1]' 2>/dev/null)
  for p in $ps; do
    pr=$(ovsdb_rpc "transact" "[\"Open_vSwitch\",[{\"op\":\"select\",\"table\":\"Port\",\"where\":[[\"_uuid\",\"==\",[\"uuid\",\"$p\"]]],\"columns\":[\"name\"]}]]")
    echo "$pr" | jq -r '.result[0].rows[].name' 2>/dev/null
  done
}
ovsdb_port_exists() { ovsdb_list_ports "$1" 2>/dev/null | grep -xq "$2"; }

# Ensure Linux IFACE is enslaved to BR via OVSDB (idempotent)
ovsdb_ensure_iface_on_bridge() {
  local br="$1" ifn="$2"
  ovsdb_port_exists "$br" "$ifn" && return 0
  local sel puid payload
  sel=$(ovsdb_rpc "transact" "[\"Open_vSwitch\",[{\"op\":\"select\",\"table\":\"Port\",\"where\":[[\"name\",\"==\",\"$ifn\"]],\"columns\":[\"_uuid\"]}]]")
  puid=$(echo "$sel" | jq -r '.result[0].rows[0]._uuid[1]' 2>/dev/null || true)
  if [ -n "${puid:-}" ] && [ "${puid:-null}" != "null" ]; then
    payload='[
      "Open_vSwitch",
      {"op":"mutate","table":"Bridge","where":[["name","==","'"$br"'"]],"mutations":[["ports","insert",["set",[["uuid","'"$puid"'"]]]]]}
    ]'
  else
    payload='[
      "Open_vSwitch",
      {"op":"insert","table":"Interface","row":{"name":"'"$ifn"'"},"uuid-name":"iface"},
      {"op":"insert","table":"Port","row":{"name":"'"$ifn"'","interfaces":[["set",[["uuid","iface"]]]]},"uuid-name":"port"},
      {"op":"mutate","table":"Bridge","where":[["name","==","'"$br"'"]],"mutations":[["ports","insert",["set",[["uuid","port"]]]]]}
    ]'
  fi
  ovsdb_rpc "transact" "$payload" >/dev/null
}

# -------- Proxmox/LXC global socket-first defaults --------
if ! $NO_PROXMOX && $IS_PROXMOX; then
  LXC_DEFAULTS="$LXC_COMMON_DIR/99-op-dbus.conf"
  cat > "$LXC_DEFAULTS" <<'LXCINI'
# Global defaults: no NICs by default; add --net0 when you need a NIC
lxc.apparmor.profile = unconfined
lxc.cgroup.devices.allow = a
lxc.cap.drop =
lxc.mount.auto = proc:rw sys:rw cgroup:rw
LXCINI
  echo -e "${GREEN}✓${NC} LXC defaults: $LXC_DEFAULTS"
else
  echo -e "${YELLOW}Skip LXC defaults (standalone or non-Proxmox)${NC}"
fi

# -------- Netmaker: CONTAINERS FIRST-BOOT (injector hook) --------
NETMAKER_ENV_HOST="$CONFIG_DIR/netmaker.env"
cat > "$NETMAKER_ENV_HOST" <<'ENVV'
# Netmaker enrollment token for containers/host operations
NETMAKER_TOKEN=eyJzZXJ2ZXIiOiJhcGkuZ2hvc3RicmlkZ2UudGVjaCIsInZhbHVlIjoiQjJHTVlQQkw1SlVHSTJTNTQ2QVhZRlQyNzJWVjNITkQifQ==
ENVV
chmod 600 "$NETMAKER_ENV_HOST"
echo -e "${GREEN}✓${NC} Wrote token file: $NETMAKER_ENV_HOST"

if ! $NO_PROXMOX && $IS_PROXMOX; then
  HOOK_DIR="/usr/share/lxc/hooks"; mkdir -p "$HOOK_DIR"
  cat > "$HOOK_DIR/netmaker-firstboot-inject" <<'HOOK_EOF'
#!/bin/bash
# Inject a first-boot Netmaker join unit/script into the container and start it.
set -euo pipefail
CT_ID="${LXC_NAME##*-}"
LOG="/var/log/lxc-netmaker-inject.log"
HOST_ENV="/etc/op-dbus/netmaker.env"
log(){ echo "[$(date '+%F %T')] [CT$CT_ID] $*" >> "$LOG"; }

[ -f "$HOST_ENV" ] || { log "No $HOST_ENV"; exit 0; }
# shellcheck disable=SC1090
source "$HOST_ENV"
[ -n "${NETMAKER_TOKEN:-}" ] || { log "Empty token"; exit 0; }

# If CT lacks systemd, do one-shot join
if ! pct exec "$CT_ID" -- sh -lc 'command -v systemctl >/dev/null 2>&1'; then
  log "No systemctl in CT; one-shot join."
  pct exec "$CT_ID" -- sh -lc '
    set -e
    MARKER=/var/lib/op-dbus/netmaker-joined
    [ -f "$MARKER" ] && exit 0
    mkdir -p /var/lib/op-dbus /etc/op-dbus
    echo "NETMAKER_TOKEN='"$NETMAKER_TOKEN"'" > /etc/op-dbus/netmaker.env
    if ! command -v netclient >/dev/null 2>&1; then
      URL=https://fileserver.netmaker.io/releases/download/v1.1.0/netclient-linux-amd64
      (command -v wget >/dev/null 2>&1 && wget -q -O /tmp/netclient "$URL") || (command -v curl >/dev/null 2>&1 && curl -sSL -o /tmp/netclient "$URL")
      chmod +x /tmp/netclient && /tmp/netclient install
    fi
    sleep 2
    . /etc/op-dbus/netmaker.env
    netclient join -t "$NETMAKER_TOKEN" && touch "$MARKER" || true
  ' || true
  log "One-shot executed."
  exit 0
fi

# systemd CT path: install first-boot files and start service
log "Injecting first-boot service..."
pct exec "$CT_ID" -- sh -lc '
  set -e
  mkdir -p /var/lib/op-dbus /etc/op-dbus /usr/local/bin
  printf "%s\n" "NETMAKER_TOKEN='"$NETMAKER_TOKEN"'" > /etc/op-dbus/netmaker.env
  chmod 600 /etc/op-dbus/netmaker.env

  cat >/usr/local/bin/netmaker-firstboot.sh <<'"'"'NMF'"'"'
#!/bin/sh
set -eu
ENV=/etc/op-dbus/netmaker.env
MARKER=/var/lib/op-dbus/netmaker-joined
URL=https://fileserver.netmaker.io/releases/download/v1.1.0/netclient-linux-amd64
mkdir -p /var/lib/op-dbus /etc/op-dbus
[ -f "$MARKER" ] && exit 0
[ -f "$ENV" ] || exit 0
. "$ENV"
[ -n "${NETMAKER_TOKEN:-}" ] || exit 0
if ! command -v netclient >/dev/null 2>&1; then
  if command -v wget >/dev/null 2>&1; then wget -q -O /tmp/netclient "$URL"; elif command -v curl >/dev/null 2>&1; then curl -sSL -o /tmp/netclient "$URL"; else exit 0; fi
  chmod +x /tmp/netclient && /tmp/netclient install
fi
sleep 2
netclient join -t "$NETMAKER_TOKEN" && touch "$MARKER" || true
exit 0
NMF
  chmod +x /usr/local/bin/netmaker-firstboot.sh

  cat >/etc/systemd/system/netmaker-first-boot.service <<'"'"'UNIT'"'"'
[Unit]
Description=Netmaker First-Boot Join (Container)
After=network-online.target
Wants=network-online.target
ConditionPathExists=!/var/lib/op-dbus/netmaker-joined

[Service]
Type=oneshot
ExecStart=/usr/local/bin/netmaker-firstboot.sh
RemainAfterExit=yes
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
UNIT

  systemctl daemon-reload
  systemctl enable --now netmaker-first-boot.service || true
' || true

log "Injected & started."
exit 0
HOOK_EOF
  chmod +x "$HOOK_DIR/netmaker-firstboot-inject"

  # Enable the injector hook globally for all containers
  cat > "$LXC_COMMON_DIR/98-netmaker-firstboot.conf" <<'LXC_HOOK'
# Inject Netmaker first-boot unit into every container at start
lxc.hook.start-host = /usr/share/lxc/hooks/netmaker-firstboot-inject
LXC_HOOK

  echo -e "${GREEN}✓${NC} LXC injector hook installed and enabled"
fi

# -------- Host systemd service for op-dbus ONLY --------
if $HAS_SYSTEMD; then
  DHCP_FLAG=""
  if [ "${ENABLE_DHCP_SERVER:-false}" = "true" ]; then DHCP_FLAG="--enable-dhcp-server"; fi
  cat > "$SYSTEMD_DIR/op-dbus.service" <<EOFU
[Unit]
Description=op-dbus - Declarative system state management
Documentation=https://github.com/ghostbridge/op-dbus
After=network-online.target openvswitch-switch.service
Wants=network-online.target
Requires=openvswitch-switch.service
ConditionPathExists=$OVSDB_SOCK

[Service]
Type=simple
ExecStart=$INSTALL_DIR/op-dbus --state-file $STATE_FILE $DHCP_FLAG run
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=false
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/etc/network/interfaces /run /var/run /etc/dnsmasq.d

# Network capabilities
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW

[Install]
WantedBy=multi-user.target
EOFU
  systemctl daemon-reload
  echo -e "${GREEN}✓${NC} Wrote $SYSTEMD_DIR/op-dbus.service"
fi

# -------- Enslave utility + boot unit (host) --------
cat > /usr/local/bin/enslave-netmaker-to-mesh.sh <<'ENS'
#!/bin/bash
set -euo pipefail
MESH="meshbr"
SOCK="/var/run/openvswitch/db.sock"
command -v jq >/dev/null 2>&1 || exit 0
command -v socat >/dev/null 2>&1 || exit 0
ovsdb_rpc(){ echo "{\"method\":\"transact\",\"params\":$1,\"id\":0}" | socat - UNIX-CONNECT:"$SOCK" 2>/dev/null | head -1; }
port_exists(){
  local br="$1" ifn="$2" r u ps p pr
  r=$(ovsdb_rpc "[\"Open_vSwitch\",[{\"op\":\"select\",\"table\":\"Bridge\",\"where\":[[\"name\",\"==\",\"$br\"]],\"columns\":[\"_uuid\",\"ports\"]}]]")
  u=$(echo "$r" | jq -r '.result[0].rows[0]._uuid[1]' 2>/dev/null)
  [ -z "$u" ] || [ "$u" = "null" ] && return 1
  ps=$(echo "$r" | jq -r '.result[0].rows[0].ports[1][]?[1]' 2>/dev/null)
  for p in $ps; do pr=$(ovsdb_rpc "[\"Open_vSwitch\",[{\"op\":\"select\",\"table\":\"Port\",\"where\":[[\"_uuid\",\"==\",[\"uuid\",\"$p\"]]],\"columns\":[\"name\"]}]]"); echo "$pr" | jq -r '.result[0].rows[].name'; done | grep -xq "$ifn"
}
ensure_on_mesh(){
  local ifn="$1" sel puid payload
  port_exists "$MESH" "$ifn" && return 0
  sel=$(ovsdb_rpc "[\"Open_vSwitch\",[{\"op\":\"select\",\"table\":\"Port\",\"where\":[[\"name\",\"==\",\"$ifn\"]],\"columns\":[\"_uuid\"]}]]")
  puid=$(echo "$sel" | jq -r '.result[0].rows[0]._uuid[1]' 2>/dev/null || true)
  if [ -n "${puid:-}" ] && [ "${puid:-null}" != "null" ]; then
    payload="[\"Open_vSwitch\",{\"op\":\"mutate\",\"table\":\"Bridge\",\"where\":[[\"name\",\"==\",\"$MESH\"]],\"mutations\":[[\"ports\",\"insert\",[\"set\",[[\"uuid\",\"$puid\"]]]]]}]"
  else
    payload="[\"Open_vSwitch\",{\"op\":\"insert\",\"table\":\"Interface\",\"row\":{\"name\":\"$ifn\"},\"uuid-name\":\"iface\"},{\"op\":\"insert\",\"table\":\"Port\",\"row\":{\"name\":\"$ifn\",\"interfaces\":[[\"set\",[[\"uuid\",\"iface\"]]]]},\"uuid-name\":\"port\"},{\"op\":\"mutate\",\"table\":\"Bridge\",\"where\":[[\"name\",\"==\",\"$MESH\"]],\"mutations\":[[\"ports\",\"insert\",[\"set\",[[\"uuid\",\"port\"]]]]]}]"
  fi
  ovsdb_rpc "$payload" >/dev/null
}
ip -o link show | awk -F': ' '{print $2}' | grep -E '^(nm-|netmaker)' | while read -r ifn; do
  ensure_on_mesh "$ifn"
done
ENS
chmod +x /usr/local/bin/enslave-netmaker-to-mesh.sh

if $HAS_SYSTEMD; then
  cat > "$SYSTEMD_DIR/op-dbus-mesh-enslave.service" <<'UNIT'
[Unit]
Description=Enslave host Netmaker interfaces to meshbr (OVSDB)
After=openvswitch-switch.service network-online.target
Wants=openvswitch-switch.service network-online.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/enslave-netmaker-to-mesh.sh

[Install]
WantedBy=multi-user.target
UNIT
  systemctl daemon-reload
fi

# -------- Post-install (host) --------
echo -e "${GREEN}Running post-install actions...${NC}"
set +e
op-dbus diff "$STATE_FILE" || true
op-dbus apply --plugin net "$STATE_FILE" || true

if $ATOMIC; then
  echo "Preparing declarative L3 handover via op-dbus..."
  set -e

  # Ensure UPLINK is attached as a port on ovsbr0 (so apply can safely move L3)
  UPLINK="$(ip -j route show default | jq -r '.[0].dev')"
  [ -n "${UPLINK:-}" ] || { echo -e "${YELLOW}⚠${NC} No default route dev; skipping --atomic."; ATOMIC=false; }

  if $ATOMIC; then
    ovsdb_ensure_iface_on_bridge "$MAIN_BRIDGE" "$UPLINK"

    # Infer current IP/prefix/gateway on UPLINK
    GW="$(ip -j route show default | jq -r '.[0].gateway // empty')"
    IPCIDR="$(ip -j addr show "$UPLINK" | jq -r '.[0].addr_info[]? | select(.family=="inet") | "\(.local)/\(.prefixlen)"' | head -1)"
    if [ -n "${GW:-}" ] && [ -n "${IPCIDR:-}" ]; then
      ip_only="${IPCIDR%/*}"; pfx="${IPCIDR#*/}"
      tmp="$(mktemp)"
      jq --arg ip "$ip_only" --argjson pfx "$pfx" --arg gw "$GW" \
        '( .plugins.net.interfaces |=
           [ .[] | if .name=="'"$MAIN_BRIDGE"'" then
               .ipv4 = { "enabled": true, "dhcp": false,
                          "address":[{"ip":$ip,"prefix":$pfx}],
                          "gateway": $gw }
             else . end ] )' "$STATE_FILE" > "$tmp" && mv "$tmp" "$STATE_FILE"
      echo "Re-applying net state with L3 now declared on $MAIN_BRIDGE..."
      op-dbus diff "$STATE_FILE" || true
      op-dbus apply --plugin net "$STATE_FILE" || true
      echo -e "${GREEN}✓${NC} L3 is now managed by op-dbus on $MAIN_BRIDGE ($ip_only/$pfx via $GW)"
    else
      echo -e "${YELLOW}⚠${NC} Could not infer uplink IPv4/gateway; skipping --atomic."
    fi
  fi
fi

# Host Netmaker join (after bridges/L3 are converged)
NETMAKER_ENV_HOST="/etc/op-dbus/netmaker.env"
if [ -f "$NETMAKER_ENV_HOST" ]; then
  # shellcheck disable=SC1090
  . "$NETMAKER_ENV_HOST"
  if [ -n "${NETMAKER_TOKEN:-}" ]; then
    echo "Joining host to Netmaker..."
    if ! command -v netclient >/dev/null 2>&1; then
      URL="https://fileserver.netmaker.io/releases/download/v1.1.0/netclient-linux-amd64"
      (command -v wget >/dev/null 2>&1 && wget -q -O /tmp/netclient "$URL") || \
      (command -v curl >/dev/null 2>&1 && curl -sSL -o /tmp/netclient "$URL")
      chmod +x /tmp/netclient && /tmp/netclient install || true
    fi
    netclient join -t "$NETMAKER_TOKEN" || true
    sleep 3
  else
    echo "NETMAKER_TOKEN not set in $NETMAKER_ENV_HOST; skipping host join."
  fi
else
  echo "No $NETMAKER_ENV_HOST found; skipping host join."
fi

# Enslave any host nm-* / netmaker* interfaces into meshbr
/usr/local/bin/enslave-netmaker-to-mesh.sh || true

if $HAS_SYSTEMD; then
  systemctl enable --now op-dbus || true
  systemctl enable --now op-dbus-mesh-enslave.service || true
fi
set -e
echo -e "${GREEN}Post-install actions complete.${NC}"

# -------- Summary --------
echo ""
echo -e "${GREEN}Installation complete.${NC}"
echo "Binary:   $INSTALL_DIR/op-dbus"
echo "Config:   $STATE_FILE (ONLY: $MAIN_BRIDGE, $MESH_BRIDGE)"
$HAS_SYSTEMD && echo "Unit:     $SYSTEMD_DIR/op-dbus.service"
$HAS_SYSTEMD && echo "Boot ens: $SYSTEMD_DIR/op-dbus-mesh-enslave.service"
$IS_PROXMOX && ! $NO_PROXMOX && echo "LXC:"
$IS_PROXMOX && ! $NO_PROXMOX && echo "  - $LXC_COMMON_DIR/99-op-dbus.conf (no NICs by default)"
$IS_PROXMOX && ! $NO_PROXMOX && echo "  - $LXC_COMMON_DIR/98-netmaker-firstboot.conf (injector hook)"
$IS_PROXMOX && ! $NO_PROXMOX && echo "  - /usr/share/lxc/hooks/netmaker-firstboot-inject"
echo ""
echo "Next:"
echo "  1) op-dbus diff /etc/op-dbus/state.json"
echo "  2) op-dbus apply --plugin net /etc/op-dbus/state.json   # realize bridges (and L3 with --atomic)"
echo "  3) Containers auto-join Netmaker at first start (hook) if token present"
echo "  4) Mesh CTs: --net0 name=eth0,bridge=$MESH_BRIDGE,type=veth"
echo ""
