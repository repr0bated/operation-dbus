#!/bin/bash
# verify-installation.sh - Comprehensive installation verification

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
PASSED=0
FAILED=0
WARNINGS=0

# Configuration paths
BINARY_PATH="/usr/local/bin/op-dbus"
CONFIG_DIR="/etc/op-dbus"
DATA_DIR="/var/lib/op-dbus"
BLOCKCHAIN_DIR="${DATA_DIR}/blockchain"
CACHE_DIR="${DATA_DIR}/@cache"
RUNTIME_DIR="/run/op-dbus"
STATE_FILE="${CONFIG_DIR}/state.json"
SERVICE_FILE="/etc/systemd/system/op-dbus.service"

# Check functions
check_pass() {
    echo -e "${GREEN}✅ PASS${NC}: $1"
    ((PASSED++))
}

check_fail() {
    echo -e "${RED}❌ FAIL${NC}: $1"
    ((FAILED++))
}

check_warn() {
    echo -e "${YELLOW}⚠️  WARN${NC}: $1"
    ((WARNINGS++))
}

check_info() {
    echo -e "${BLUE}ℹ️  INFO${NC}: $1"
}

section_header() {
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  $1"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
}

# 1. Binary Checks
check_binary() {
    section_header "1. Binary Installation"

    # Check if op-dbus exists
    if [ -f "$BINARY_PATH" ]; then
        check_pass "op-dbus binary exists: $BINARY_PATH"
    else
        check_fail "op-dbus binary not found: $BINARY_PATH"
        return
    fi

    # Check if executable
    if [ -x "$BINARY_PATH" ]; then
        check_pass "Binary is executable"
    else
        check_fail "Binary is not executable"
    fi

    # Check version
    if VERSION=$("$BINARY_PATH" --version 2>&1); then
        check_pass "Binary runs: $VERSION"
    else
        check_fail "Binary fails to run"
    fi

    # Check for MCP binaries (optional)
    MCP_BINARIES=("dbus-mcp" "dbus-orchestrator" "dbus-mcp-web" "mcp-chat")
    MCP_FOUND=0

    for bin in "${MCP_BINARIES[@]}"; do
        if [ -f "/usr/local/bin/$bin" ]; then
            ((MCP_FOUND++))
        fi
    done

    if [ $MCP_FOUND -gt 0 ]; then
        check_info "Found $MCP_FOUND MCP binaries"
    else
        check_info "No MCP binaries found (optional feature)"
    fi
}

# 2. Directory Structure
check_directories() {
    section_header "2. Directory Structure"

    # Check config directory
    if [ -d "$CONFIG_DIR" ]; then
        check_pass "Config directory exists: $CONFIG_DIR"
    else
        check_fail "Config directory missing: $CONFIG_DIR"
    fi

    # Check state file
    if [ -f "$STATE_FILE" ]; then
        check_pass "State file exists: $STATE_FILE"

        # Validate JSON
        if command -v jq &> /dev/null; then
            if jq empty "$STATE_FILE" 2>/dev/null; then
                check_pass "State file is valid JSON"
            else
                check_fail "State file has JSON syntax errors"
            fi
        else
            check_warn "Cannot validate JSON (jq not installed)"
        fi
    else
        check_fail "State file missing: $STATE_FILE"
    fi

    # Check data directory
    if [ -d "$DATA_DIR" ]; then
        check_pass "Data directory exists: $DATA_DIR"
    else
        check_fail "Data directory missing: $DATA_DIR"
    fi

    # Check blockchain directories
    local blockchain_subdirs=("timing" "vectors" "snapshots")
    for subdir in "${blockchain_subdirs[@]}"; do
        if [ -d "${BLOCKCHAIN_DIR}/${subdir}" ]; then
            check_pass "Blockchain subdir exists: ${subdir}"
        else
            check_warn "Blockchain subdir missing: ${subdir}"
        fi
    done

    # Check cache directory
    if [ -d "$CACHE_DIR" ]; then
        check_pass "Cache directory exists: $CACHE_DIR"

        # Check if it's a BTRFS subvolume
        if df -T /var/lib 2>/dev/null | grep -q btrfs; then
            if btrfs subvolume show "$CACHE_DIR" &>/dev/null; then
                check_pass "Cache is BTRFS subvolume"
            else
                check_warn "Cache exists but is not a BTRFS subvolume"
            fi
        else
            check_warn "Cache exists but filesystem is not BTRFS"
        fi
    else
        check_warn "Cache directory missing: $CACHE_DIR"
    fi

    # Check runtime directory (may not exist if service not running)
    if [ -d "$RUNTIME_DIR" ]; then
        check_pass "Runtime directory exists: $RUNTIME_DIR"
    else
        check_info "Runtime directory not found (created when service starts)"
    fi
}

# 3. System Dependencies
check_dependencies() {
    section_header "3. System Dependencies"

    # OpenVSwitch
    if command -v ovs-vsctl &> /dev/null; then
        check_pass "ovs-vsctl command found"
    else
        check_fail "ovs-vsctl not found"
    fi

    # OVSDB server
    if systemctl is-active --quiet ovsdb-server 2>/dev/null || systemctl is-active --quiet openvswitch 2>/dev/null; then
        check_pass "OVSDB server is running"
    else
        check_fail "OVSDB server is not running"
    fi

    # OVS vswitchd
    if systemctl is-active --quiet ovs-vswitchd 2>/dev/null || systemctl is-active --quiet openvswitch 2>/dev/null; then
        check_pass "OVS vswitchd is running"
    else
        check_fail "OVS vswitchd is not running"
    fi

    # OVSDB socket
    if [ -S "/var/run/openvswitch/db.sock" ]; then
        check_pass "OVSDB socket exists: /var/run/openvswitch/db.sock"

        # Test OVSDB connectivity
        if ovs-vsctl show &> /dev/null; then
            check_pass "OVSDB is accessible"
        else
            check_fail "OVSDB socket exists but not responding"
        fi
    else
        check_fail "OVSDB socket not found"
    fi

    # D-Bus
    if [ -S "/var/run/dbus/system_bus_socket" ]; then
        check_pass "D-Bus system socket exists"
    else
        check_fail "D-Bus system socket not found"
    fi

    # Optional: netclient
    if command -v netclient &> /dev/null; then
        check_info "Netclient installed (optional)"
    fi

    # Optional: Proxmox
    if command -v pct &> /dev/null; then
        check_info "Proxmox pct command available"
    fi
}

# 4. Systemd Service
check_service() {
    section_header "4. Systemd Service"

    # Check if service file exists
    if [ -f "$SERVICE_FILE" ]; then
        check_pass "Service file exists: $SERVICE_FILE"
    else
        check_fail "Service file missing: $SERVICE_FILE"
        return
    fi

    # Check if service is loaded
    if systemctl list-unit-files op-dbus.service &> /dev/null; then
        check_pass "Service is loaded in systemd"
    else
        check_fail "Service not loaded in systemd"
    fi

    # Check if enabled
    if systemctl is-enabled --quiet op-dbus.service 2>/dev/null; then
        check_pass "Service is enabled (will start at boot)"
    else
        check_warn "Service is not enabled"
    fi

    # Check if active
    if systemctl is-active --quiet op-dbus.service 2>/dev/null; then
        check_pass "Service is running"
    else
        check_info "Service is not running (this is OK if not started yet)"
    fi

    # Check service status details
    if systemctl status op-dbus.service &> /dev/null; then
        check_info "Service status: $(systemctl is-active op-dbus.service 2>/dev/null || echo 'inactive')"
    fi
}

# 5. OVS Bridges
check_ovs_bridges() {
    section_header "5. OVS Bridges"

    if ! command -v ovs-vsctl &> /dev/null; then
        check_warn "Cannot check bridges (ovs-vsctl not found)"
        return
    fi

    # List all bridges
    if BRIDGES=$(ovs-vsctl list-br 2>/dev/null); then
        if [ -z "$BRIDGES" ]; then
            check_info "No OVS bridges found (may not be created yet)"
        else
            check_pass "Found OVS bridges:"
            echo "$BRIDGES" | while read -r bridge; do
                echo "       - $bridge"

                # Check if bridge exists in kernel
                if ip link show "$bridge" &> /dev/null; then
                    check_pass "  Bridge $bridge visible in kernel"
                else
                    check_warn "  Bridge $bridge not visible in kernel"
                fi

                # Check controller
                if CONTROLLER=$(ovs-vsctl get-controller "$bridge" 2>/dev/null); then
                    if [ -n "$CONTROLLER" ]; then
                        check_info "  Controller: $CONTROLLER"
                    fi
                fi
            done
        fi
    else
        check_fail "Failed to list OVS bridges"
    fi
}

# 6. D-Bus Access
check_dbus() {
    section_header "6. D-Bus Access"

    # Check if we can access system bus
    if command -v busctl &> /dev/null; then
        if busctl status org.freedesktop.systemd1 &> /dev/null; then
            check_pass "Can access D-Bus system bus"
        else
            check_fail "Cannot access D-Bus system bus"
        fi

        # Check for org.opdbus service (if running)
        if busctl status org.opdbus &> /dev/null; then
            check_info "org.opdbus service is available on D-Bus"
        else
            check_info "org.opdbus not on D-Bus (service may not be running)"
        fi
    else
        check_warn "busctl not available, skipping D-Bus checks"
    fi
}

# 7. Blockchain
check_blockchain() {
    section_header "7. Blockchain Storage"

    if [ -d "$BLOCKCHAIN_DIR" ]; then
        # Count footprints
        if [ -d "${BLOCKCHAIN_DIR}/timing" ]; then
            FOOTPRINT_COUNT=$(find "${BLOCKCHAIN_DIR}/timing" -name "*.json" 2>/dev/null | wc -l)
            if [ "$FOOTPRINT_COUNT" -gt 0 ]; then
                check_info "Found $FOOTPRINT_COUNT blockchain footprints"
            else
                check_info "No blockchain footprints yet (will be created on first apply)"
            fi
        fi

        # Check snapshots
        if [ -d "${BLOCKCHAIN_DIR}/snapshots" ]; then
            SNAPSHOT_COUNT=$(find "${BLOCKCHAIN_DIR}/snapshots" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)
            if [ "$SNAPSHOT_COUNT" -gt 0 ]; then
                check_info "Found $SNAPSHOT_COUNT BTRFS snapshots"
            else
                check_info "No snapshots yet"
            fi
        fi
    fi
}

# 8. Command Tests
check_commands() {
    section_header "8. Command Tests"

    # Test --version
    if "$BINARY_PATH" --version &> /dev/null; then
        check_pass "op-dbus --version works"
    else
        check_fail "op-dbus --version fails"
    fi

    # Test doctor command (doesn't require root)
    if timeout 5s "$BINARY_PATH" doctor &> /dev/null; then
        check_pass "op-dbus doctor works"
    else
        check_warn "op-dbus doctor failed or timed out"
    fi

    # Test query (requires root or appropriate permissions)
    if [ "$EUID" -eq 0 ]; then
        if timeout 10s "$BINARY_PATH" query &> /dev/null; then
            check_pass "op-dbus query works"
        else
            check_warn "op-dbus query failed or timed out (may need running services)"
        fi

        # Test introspect
        if timeout 10s "$BINARY_PATH" introspect --pretty &> /dev/null; then
            check_pass "op-dbus introspect works"
        else
            check_warn "op-dbus introspect failed or timed out"
        fi
    else
        check_info "Skipping root-required command tests (not running as root)"
    fi
}

# 9. Network Connectivity
check_network() {
    section_header "9. Network Connectivity"

    # Check OVSDB socket accessibility
    if [ -S "/var/run/openvswitch/db.sock" ]; then
        if timeout 2s ovs-vsctl show &> /dev/null; then
            check_pass "OVSDB socket is accessible"
        else
            check_fail "OVSDB socket exists but not responding"
        fi
    fi

    # Check D-Bus socket accessibility
    if [ -S "/var/run/dbus/system_bus_socket" ]; then
        check_pass "D-Bus socket is accessible"
    else
        check_fail "D-Bus socket not accessible"
    fi

    # TODO: Check OpenFlow controller connectivity
    # if netcat or telnet available, test tcp:127.0.0.1:6653
}

# 10. Netmaker (Optional)
check_netmaker() {
    section_header "10. Netmaker (Optional)"

    if [ -f "/etc/op-dbus/netmaker.env" ]; then
        check_info "Netmaker configuration file exists"

        # Check if token is set
        if grep -q "NETMAKER_TOKEN=" "/etc/op-dbus/netmaker.env" 2>/dev/null; then
            check_info "Netmaker token is configured"
        fi
    else
        check_info "Netmaker not configured (optional feature)"
    fi

    if command -v netclient &> /dev/null; then
        # Check if joined to network
        if netclient list &> /dev/null; then
            check_info "Netclient is installed and accessible"
        fi
    fi
}

# Summary
show_summary() {
    section_header "VERIFICATION SUMMARY"

    local total=$((PASSED + FAILED + WARNINGS))

    echo "Results:"
    echo -e "  ${GREEN}Passed:   $PASSED${NC}"
    echo -e "  ${RED}Failed:   $FAILED${NC}"
    echo -e "  ${YELLOW}Warnings: $WARNINGS${NC}"
    echo "  Total:    $total"
    echo ""

    if [ $FAILED -eq 0 ]; then
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo -e "${GREEN}  ✅ INSTALLATION VERIFIED SUCCESSFULLY${NC}"
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo ""
        if [ $WARNINGS -gt 0 ]; then
            echo "Note: There are $WARNINGS warnings, but no critical failures."
        fi
        return 0
    else
        echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo -e "${RED}  ❌ INSTALLATION HAS ISSUES${NC}"
        echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo ""
        echo "Please address the failed checks above."
        return 1
    fi
}

# Main
main() {
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  op-dbus Installation Verification"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    check_binary
    check_directories
    check_dependencies
    check_service
    check_ovs_bridges
    check_dbus
    check_blockchain
    check_commands
    check_network
    check_netmaker
    show_summary
}

main "$@"
