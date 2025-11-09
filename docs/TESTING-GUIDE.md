# op-dbus Testing Guide

**Comprehensive testing strategy for all features**

## Testing Priority Order

As per user requirements:
1. **PRIORITY 1**: Containers and socket networking
2. **PRIORITY 2**: D-Bus server functions
3. **PRIORITY 3**: MCP chat console integration
4. **PRIORITY 4**: Full privacy chain end-to-end

## Prerequisites

### Required Environment

- **OS**: Debian/Ubuntu Linux with systemd
- **Permissions**: Root access (for OVS, LXC, D-Bus system bus)
- **Installed Packages**:
  ```bash
  sudo apt install -y \
    openvswitch-switch \
    lxc \
    dbus \
    busctl \
    jq \
    curl \
    net-tools \
    bridge-utils
  ```

### Build and Install op-dbus

```bash
# Build
./build.sh

# Install
sudo ./install.sh

# Verify installation
op-dbus --version
sudo systemctl status op-dbus
```

## PRIORITY 1: Containers and Socket Networking Tests

**Goal**: Verify containerless socket networking with OVS internal ports

### Test 1.1: Create OVS Bridge with Socket Ports

```bash
# Stop any existing op-dbus
sudo systemctl stop op-dbus

# Create test state file
sudo tee /etc/op-dbus/test-socket-networking.json > /dev/null <<'EOF'
{
  "version": 1,
  "plugins": {
    "openflow": {
      "bridges": [
        {
          "name": "ovsbr0",
          "datapath_type": "netdev",
          "socket_ports": [
            {"name": "internal_100", "container_id": "100", "ip": "10.0.0.100/24"},
            {"name": "internal_101", "container_id": "101", "ip": "10.0.0.101/24"},
            {"name": "internal_102", "container_id": "102", "ip": "10.0.0.102/24"}
          ],
          "flow_policies": [
            {
              "name": "allow-100-to-101",
              "selector": "container:100",
              "template": {
                "priority": 100,
                "match": {"dl_type": "0x0800"},
                "actions": [{"type": "output", "port": "internal_101"}]
              }
            }
          ]
        }
      ]
    }
  }
}
EOF

# Apply state
sudo op-dbus apply /etc/op-dbus/test-socket-networking.json

# Verify bridge created
sudo ovs-vsctl show | grep -A 10 "ovsbr0"

# Verify internal ports exist
sudo ovs-vsctl list-ports ovsbr0 | grep internal_

# Check port types
sudo ovs-vsctl list interface internal_100 | grep type
# Should show: type: internal

# Verify IP addresses assigned
ip addr show internal_100
ip addr show internal_101
ip addr show internal_102
```

**Expected Output**:
```
Bridge ovsbr0
    datapath_type: netdev
    Port internal_100
        Interface internal_100
            type: internal
    Port internal_101
        Interface internal_101
            type: internal
    Port internal_102
        Interface internal_102
            type: internal

internal_100
internal_101
internal_102

type                : internal

4: internal_100: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc noqueue state UNKNOWN group default qlen 1000
    inet 10.0.0.100/24 scope global internal_100
```

### Test 1.2: Test Socket Port Connectivity

```bash
# Ping from internal_100 to internal_101 (should work if flow policy allows)
sudo ip netns exec container-100 ping -c 3 10.0.0.101

# OR if not using network namespaces, test with socat
# Terminal 1 - Listen on internal_101
sudo socat TCP-LISTEN:8888,fork,bind=10.0.0.101 EXEC:cat

# Terminal 2 - Connect from internal_100
echo "test message" | socat - TCP:10.0.0.101:8888
```

**Expected Output**:
```
PING 10.0.0.101 (10.0.0.101) 56(84) bytes of data.
64 bytes from 10.0.0.101: icmp_seq=1 ttl=64 time=0.123 ms
64 bytes from 10.0.0.101: icmp_seq=2 ttl=64 time=0.089 ms
64 bytes from 10.0.0.101: icmp_seq=3 ttl=64 time=0.091 ms
```

### Test 1.3: Verify OpenFlow Rules

```bash
# List all flows on bridge
sudo ovs-ofctl dump-flows ovsbr0

# Check specific flow for container 100 → 101
sudo ovs-ofctl dump-flows ovsbr0 | grep "in_port=internal_100"

# Verify flow priority and actions
sudo ovs-ofctl dump-flows ovsbr0 -O OpenFlow13 --names
```

**Expected Output**:
```
priority=100,ip,in_port=internal_100 actions=output:internal_101
```

### Test 1.4: Create LXC Containers with Socket Networking

```bash
# Create test state with containers
sudo tee /etc/op-dbus/test-containers.json > /dev/null <<'EOF'
{
  "version": 1,
  "plugins": {
    "lxc": {
      "containers": [
        {
          "id": 100,
          "name": "test-container-100",
          "template": "debian-13",
          "autostart": true,
          "network": {
            "bridge": "ovsbr0",
            "veth": false,
            "socket_networking": true,
            "port_name": "internal_100",
            "ipv4": "10.0.0.100/24"
          }
        },
        {
          "id": 101,
          "name": "test-container-101",
          "template": "debian-13",
          "autostart": true,
          "network": {
            "bridge": "ovsbr0",
            "veth": false,
            "socket_networking": true,
            "port_name": "internal_101",
            "ipv4": "10.0.0.101/24"
          }
        }
      ]
    },
    "openflow": {
      "bridges": [
        {
          "name": "ovsbr0",
          "datapath_type": "netdev",
          "socket_ports": [
            {"name": "internal_100", "container_id": "100", "ip": "10.0.0.100/24"},
            {"name": "internal_101", "container_id": "101", "ip": "10.0.0.101/24"}
          ]
        }
      ]
    }
  }
}
EOF

# Apply state
sudo op-dbus apply /etc/op-dbus/test-containers.json

# Verify containers created
sudo pct list  # If Proxmox
# OR
sudo lxc-ls -f  # If traditional LXC

# Check container status
sudo pct status 100
sudo pct status 101

# Verify containers can see internal ports
sudo pct exec 100 -- ip addr show
sudo pct exec 101 -- ip addr show

# Test container-to-container connectivity via socket ports
sudo pct exec 100 -- ping -c 3 10.0.0.101
```

**Expected Output**:
```
VMID       Status     Lock         Name
100        running                 test-container-100
101        running                 test-container-101

status: running

4: internal_100: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500
    inet 10.0.0.100/24 scope global internal_100

PING 10.0.0.101 (10.0.0.101) 56(84) bytes of data.
64 bytes from 10.0.0.101: icmp_seq=1 ttl=64 time=0.098 ms
```

### Test 1.5: Query Container State

```bash
# Query current container state
sudo op-dbus query --plugin lxc | jq '.plugins.lxc.containers'

# Verify container matches desired state
sudo op-dbus diff /etc/op-dbus/test-containers.json --plugin lxc
```

**Expected Output**:
```json
{
  "containers": [
    {
      "id": 100,
      "name": "test-container-100",
      "status": "running",
      "network": {
        "socket_networking": true,
        "port_name": "internal_100",
        "ipv4": "10.0.0.100/24"
      }
    }
  ]
}

No drift detected
```

## PRIORITY 2: D-Bus Server Function Tests

**Goal**: Verify all D-Bus interfaces work correctly

### Test 2.1: State Manager D-Bus Service

```bash
# Start op-dbus daemon
sudo systemctl start op-dbus

# Wait for D-Bus service to register
sleep 3

# List D-Bus services (should see org.opdbus)
busctl list | grep opdbus

# Introspect StateManager interface
busctl introspect org.opdbus /org/opdbus/state

# Test query_state method
busctl call org.opdbus /org/opdbus/state \
  org.opdbus.StateManager query_state

# Test apply_state method
busctl call org.opdbus /org/opdbus/state \
  org.opdbus.StateManager apply_state s \
  '{"version":1,"plugins":{"systemd":{"units":{}}}}'
```

**Expected Output**:
```
org.opdbus                             1000 opdbus       :1.123      user@1000.service -       -

NAME                            TYPE      SIGNATURE RESULT/VALUE FLAGS
org.opdbus.StateManager         interface -         -            -
.apply_state                    method    s         s            -
.query_state                    method    -         s            -

s "{\"version\":1,\"plugins\":{...}}"

s "Applied successfully: 0"
```

### Test 2.2: Orchestrator D-Bus Service

```bash
# Start orchestrator (session bus - no sudo)
op-dbus orchestrator &

# Wait for registration
sleep 2

# List D-Bus services
busctl --user list | grep dbusmcp

# Introspect Orchestrator
busctl --user introspect org.dbusmcp /org/dbusmcp/orchestrator

# Test spawn_agent
busctl --user call org.dbusmcp /org/dbusmcp/orchestrator \
  org.dbusmcp.Orchestrator spawn_agent ss "file" ""

# Capture agent ID
AGENT_ID=$(busctl --user call org.dbusmcp /org/dbusmcp/orchestrator \
  org.dbusmcp.Orchestrator spawn_agent ss "file" "" \
  | awk '{print $2}' | tr -d '"')

echo "Spawned agent: $AGENT_ID"

# Test get_agent_status
busctl --user call org.dbusmcp /org/dbusmcp/orchestrator \
  org.dbusmcp.Orchestrator get_agent_status s "$AGENT_ID"

# Test list_agents
busctl --user call org.dbusmcp /org/dbusmcp/orchestrator \
  org.dbusmcp.Orchestrator list_agents
```

**Expected Output**:
```
org.dbusmcp                            1000 opdbus       :1.124      user@1000.service -       -

s "agent-uuid-12345-67890"

Spawned agent: agent-uuid-12345-67890

s "{\"agent_id\":\"agent-uuid-12345-67890\",\"agent_type\":\"file\",\"status\":\"running\"}"

s "{\"agents\":[{\"id\":\"agent-uuid-12345-67890\",\"type\":\"file\"}]}"
```

### Test 2.3: File Agent D-Bus Service

```bash
# Create test file
echo "test content from file agent" > /tmp/test-fileagent.txt

# Spawn file agent
AGENT_ID=$(busctl --user call org.dbusmcp /org/dbusmcp/orchestrator \
  org.dbusmcp.Orchestrator spawn_agent ss "file" "" \
  | awk '{print $2}' | tr -d '"')

# Test read operation
busctl --user call org.dbusmcp.agents "/org/dbusmcp/agents/$AGENT_ID" \
  org.dbusmcp.Agent.File execute s \
  '{"type":"file_operation","operation":"read","path":"/tmp/test-fileagent.txt"}'

# Test write operation
busctl --user call org.dbusmcp.agents "/org/dbusmcp/agents/$AGENT_ID" \
  org.dbusmcp.Agent.File execute s \
  '{"type":"file_operation","operation":"write","path":"/tmp/test-write.txt","content":"written by agent"}'

# Verify write
cat /tmp/test-write.txt

# Test list operation
busctl --user call org.dbusmcp.agents "/org/dbusmcp/agents/$AGENT_ID" \
  org.dbusmcp.Agent.File execute s \
  '{"type":"file_operation","operation":"list","path":"/tmp"}'

# Test exists operation
busctl --user call org.dbusmcp.agents "/org/dbusmcp/agents/$AGENT_ID" \
  org.dbusmcp.Agent.File execute s \
  '{"type":"file_operation","operation":"exists","path":"/tmp/test-fileagent.txt"}'
```

**Expected Output**:
```
s "{\"success\":true,\"operation\":\"read\",\"path\":\"/tmp/test-fileagent.txt\",\"data\":\"test content from file agent\"}"

written by agent

s "{\"success\":true,\"operation\":\"list\",\"path\":\"/tmp\",\"data\":\"[\\\"test-fileagent.txt\\\",\\\"test-write.txt\\\"]\"}"

s "{\"success\":true,\"operation\":\"exists\",\"path\":\"/tmp/test-fileagent.txt\",\"data\":\"true\"}"
```

### Test 2.4: Security Tests (File Agent)

```bash
# Test forbidden directory access
busctl --user call org.dbusmcp.agents "/org/dbusmcp/agents/$AGENT_ID" \
  org.dbusmcp.Agent.File execute s \
  '{"type":"file_operation","operation":"read","path":"/etc/shadow"}'
# Expected: Error - forbidden

# Test forbidden file access
busctl --user call org.dbusmcp.agents "/org/dbusmcp/agents/$AGENT_ID" \
  org.dbusmcp.Agent.File execute s \
  '{"type":"file_operation","operation":"read","path":"/root/.ssh/id_rsa"}'
# Expected: Error - forbidden

# Test path traversal attack
busctl --user call org.dbusmcp.agents "/org/dbusmcp/agents/$AGENT_ID" \
  org.dbusmcp.Agent.File execute s \
  '{"type":"file_operation","operation":"read","path":"/tmp/../../../../etc/shadow"}'
# Expected: Error - path validation failed
```

**Expected Output**:
```
s "{\"success\":false,\"operation\":\"read\",\"path\":\"/etc/shadow\",\"error\":\"Path is in forbidden directory\"}"

s "{\"success\":false,\"operation\":\"read\",\"path\":\"/root/.ssh/id_rsa\",\"error\":\"Path is forbidden\"}"

s "{\"success\":false,\"operation\":\"read\",\"path\":\"/tmp/../../../../etc/shadow\",\"error\":\"Path validation failed\"}"
```

## PRIORITY 3: MCP Chat Console Integration Tests

**Goal**: Verify MCP JSON-RPC bridge works correctly

### Test 3.1: MCP Introspection

```bash
# Start MCP bridge
op-dbus mcp-bridge --service org.opdbus > /tmp/mcp-introspection.json 2>&1 &
MCP_PID=$!

sleep 2

# Check introspection output
cat /tmp/mcp-introspection.json | jq '.tools[] | select(.name | contains("state"))'

# Kill MCP bridge
kill $MCP_PID
```

**Expected Output**:
```json
{
  "name": "state.apply",
  "description": "Apply state from JSON string",
  "inputSchema": {
    "type": "object",
    "properties": {
      "state_json": {"type": "string"}
    }
  }
}
{
  "name": "state.query",
  "description": "Query current state",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

### Test 3.2: MCP JSON-RPC Calls

```bash
# Start MCP bridge in background
mkfifo /tmp/mcp-input /tmp/mcp-output
op-dbus mcp-bridge < /tmp/mcp-input > /tmp/mcp-output 2>&1 &
MCP_PID=$!

sleep 2

# Send JSON-RPC request to query state
cat > /tmp/mcp-input <<'EOF'
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "state.query",
    "arguments": {}
  }
}
EOF

# Read response
timeout 5 cat /tmp/mcp-output

# Cleanup
kill $MCP_PID
rm /tmp/mcp-input /tmp/mcp-output
```

**Expected Output**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": "{\"version\":1,\"plugins\":{...}}"
  }
}
```

## PRIORITY 4: Full Privacy Chain End-to-End Tests

**Goal**: Verify complete privacy client deployment (Profile 2)

### Test 4.1: Deploy Privacy Client Containers

```bash
# Install privacy client profile
sudo ./install.sh --profile privacy-client --obfuscation-level 3

# Verify state file created
cat /etc/op-dbus/state.json | jq '.plugins.lxc.containers'

# Verify 3 containers created
sudo pct list | grep -E "100|101|102"

# Check container status
sudo pct status 100  # wireguard-gateway
sudo pct status 101  # warp-tunnel
sudo pct status 102  # xray-client

# Verify OVS bridge and ports
sudo ovs-vsctl show | grep -E "ovsbr0|internal_100|wg-warp|internal_102"

# Check OpenFlow flows (should have 18+ flows for level 3 obfuscation)
sudo ovs-ofctl dump-flows ovsbr0 | wc -l
# Should show ~18-25 flows
```

**Expected Output**:
```
VMID       Status     Lock         Name
100        running                 wireguard-gateway
101        running                 warp-tunnel
102        running                 xray-client

status: running

Bridge ovsbr0
    Port internal_100
    Port wg-warp
    Port internal_102

23
```

### Test 4.2: Test Privacy Chain Traffic Flow

```bash
# Test WireGuard container (100)
sudo pct exec 100 -- systemctl status wg-quick@wg0
sudo pct exec 100 -- wg show

# Test Warp container (101)
sudo pct exec 101 -- systemctl status wg-quick@wg-warp
sudo pct exec 101 -- ip link show wg-warp

# Test XRay container (102)
sudo pct exec 102 -- systemctl status xray
sudo pct exec 102 -- netstat -tlnp | grep 1080

# Test traffic flow: 100 → 101 → 102
sudo pct exec 100 -- tcpdump -i internal_100 -c 10 &
sudo pct exec 101 -- tcpdump -i wg-warp -c 10 &
sudo pct exec 102 -- tcpdump -i internal_102 -c 10 &

# Send test traffic
sudo pct exec 100 -- ping -c 5 10.0.0.101
```

**Expected Output**:
```
● wg-quick@wg0.service - WireGuard via wg-quick(8) for wg0
     Loaded: loaded
     Active: active (exited)

interface: wg0
  public key: ...
  private key: (hidden)
  listening port: 51820

● wg-quick@wg-warp.service - WireGuard via wg-quick(8) for wg-warp
     Active: active (exited)

wg-warp: <POINTOPOINT,NOARP,UP,LOWER_UP> mtu 1420 qdisc noqueue state UNKNOWN

● xray.service - XRay Client
     Active: active (running)

tcp        0      0 127.0.0.1:1080          0.0.0.0:*               LISTEN      123/xray
```

### Test 4.3: Verify Obfuscation Flows

```bash
# Check for obfuscation flow cookies
sudo ovs-ofctl dump-flows ovsbr0 | grep "cookie=0xDEAD"

# Count obfuscation flows
sudo ovs-ofctl dump-flows ovsbr0 | grep "cookie=0xDEAD" | wc -l
# Should be 11 (level 2) or 18 (level 3)

# Verify VLAN rotation
sudo ovs-ofctl dump-flows ovsbr0 | grep "mod_vlan_vid"

# Verify MAC randomization
sudo ovs-ofctl dump-flows ovsbr0 | grep "mod_dl_src"

# Verify TTL obfuscation
sudo ovs-ofctl dump-flows ovsbr0 | grep "mod_nw_ttl"
```

**Expected Output**:
```
18

cookie=0xDEAD0001, priority=150,ip actions=mod_vlan_vid:100,NORMAL
cookie=0xDEAD0002, priority=150,ip actions=mod_vlan_vid:200,NORMAL

cookie=0xDEAD0003, priority=140,ip actions=mod_dl_src:02:00:00:00:00:01,NORMAL

cookie=0xDEAD0004, priority=130,ip actions=mod_nw_ttl:64,NORMAL
```

### Test 4.4: End-to-End Privacy Test

```bash
# From client device, connect to WireGuard gateway
wg-quick up wg0

# Test IP is obfuscated
curl --interface wg0 https://ifconfig.me
# Should show VPS IP, not client IP

# Test DNS leak
curl --interface wg0 https://dnsleaktest.com/
# Should show VPS DNS, not client DNS

# Test traffic flows through all 3 hops
sudo pct exec 100 -- tcpdump -i wg0 -nn &          # Client → Gateway
sudo pct exec 101 -- tcpdump -i wg-warp -nn &      # Gateway → Warp
sudo pct exec 102 -- tcpdump -i internal_102 -nn &  # Warp → XRay → Internet

# Make request
curl --interface wg0 https://google.com

# Verify traffic appears in all 3 containers
```

## Test Results Documentation

Create a test results file after each test run:

```bash
sudo tee /var/log/op-dbus-test-results.log <<EOF
=== op-dbus Test Results ===
Date: $(date)
Version: $(op-dbus --version)

PRIORITY 1: Containers and Socket Networking
- Test 1.1: OVS Bridge Creation:             [ PASS / FAIL ]
- Test 1.2: Socket Port Connectivity:        [ PASS / FAIL ]
- Test 1.3: OpenFlow Rules:                  [ PASS / FAIL ]
- Test 1.4: Container Creation:              [ PASS / FAIL ]
- Test 1.5: Container State Query:           [ PASS / FAIL ]

PRIORITY 2: D-Bus Server Functions
- Test 2.1: StateManager D-Bus:              [ PASS / FAIL ]
- Test 2.2: Orchestrator D-Bus:              [ PASS / FAIL ]
- Test 2.3: File Agent D-Bus:                [ PASS / FAIL ]
- Test 2.4: Security Tests:                  [ PASS / FAIL ]

PRIORITY 3: MCP Chat Console
- Test 3.1: MCP Introspection:               [ PASS / FAIL ]
- Test 3.2: MCP JSON-RPC Calls:              [ PASS / FAIL ]

PRIORITY 4: Full Privacy Chain
- Test 4.1: Privacy Client Deployment:       [ PASS / FAIL ]
- Test 4.2: Traffic Flow:                    [ PASS / FAIL ]
- Test 4.3: Obfuscation Flows:               [ PASS / FAIL ]
- Test 4.4: End-to-End Privacy:              [ PASS / FAIL ]

Notes:
$(cat /var/log/op-dbus-test-notes.txt 2>/dev/null || echo "No additional notes")
EOF

cat /var/log/op-dbus-test-results.log
```

## Automated Testing Script

Create a comprehensive test runner:

```bash
sudo tee /usr/local/bin/test-opdbus.sh > /dev/null <<'TESTSCRIPT'
#!/bin/bash
# Automated op-dbus test suite

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=== op-dbus Automated Test Suite ==="
echo ""

# Test 1.1: OVS Bridge
echo -n "Test 1.1: OVS Bridge Creation... "
if sudo op-dbus apply /etc/op-dbus/test-socket-networking.json >/dev/null 2>&1; then
    if sudo ovs-vsctl show | grep -q "ovsbr0"; then
        echo -e "${GREEN}PASS${NC}"
    else
        echo -e "${RED}FAIL${NC}"
    fi
else
    echo -e "${RED}FAIL${NC}"
fi

# Test 2.1: D-Bus StateManager
echo -n "Test 2.1: StateManager D-Bus... "
if sudo busctl call org.opdbus /org/opdbus/state org.opdbus.StateManager query_state >/dev/null 2>&1; then
    echo -e "${GREEN}PASS${NC}"
else
    echo -e "${RED}FAIL${NC}"
fi

# Add more tests...

echo ""
echo "Test suite complete!"
TESTSCRIPT

chmod +x /usr/local/bin/test-opdbus.sh
```

## Continuous Testing

Set up continuous testing during development:

```bash
# Watch for changes and run tests
watch -n 60 'sudo /usr/local/bin/test-opdbus.sh'

# Or use systemd timer
sudo tee /etc/systemd/system/opdbus-test.timer > /dev/null <<EOF
[Unit]
Description=op-dbus test timer

[Timer]
OnCalendar=hourly
Persistent=true

[Install]
WantedBy=timers.target
EOF

sudo systemctl enable --now opdbus-test.timer
```

## Related Documentation

- **DBUS-SERVER-FUNCTIONS.md**: All D-Bus server methods
- **MCP-CHAT-CONSOLE.md**: Chat console integration
- **CONTAINER-PROFILES.md**: Container deployment profiles
- **CONTAINER-CLI.md**: CLI usage inside containers

---

**Version**: 1.0.0
**Last Updated**: 2025-11-08
**Branch**: claude/install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx
