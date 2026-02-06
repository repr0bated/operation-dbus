#!/bin/bash
# deploy/deploy.sh
# Smart incremental deployment - User Mode Compatible

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Load agent integration
if [[ -f "$SCRIPT_DIR/lib/agent-integration.sh" ]]; then
    source "$SCRIPT_DIR/lib/agent-integration.sh"
fi

# Configuration & Mode Detection
if [ "${FORCE_USER_MODE:-false}" == "true" ] || ([ "$EUID" -ne 0 ] && ! sudo -n true 2>/dev/null); then
    echo -e "\033[1;33m[WARN]\033[0m Sudo not available, switching to USER MODE."
    USER_MODE=true
    SUDO=""
    SYSTEMCTL_CMD="systemctl --user"
    INSTALL_DIR="$PROJECT_ROOT/target/release"
    SYSTEMD_DIR="$HOME/.config/systemd/user"
    mkdir -p "$SYSTEMD_DIR"
else
    USER_MODE=false
    SUDO="sudo"
    SYSTEMCTL_CMD="sudo systemctl"
    INSTALL_DIR="/usr/local/sbin"
    SYSTEMD_DIR="/etc/systemd/system"
fi

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'
BLUE='\033[0;34m'

# Flags
BUILD_WEB=false
BUILD_MCP=false
BUILD_AGENTS=false

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

check_root() {
    if [[ "$USER_MODE" == "false" ]] && [[ $EUID -ne 0 ]]; then
        log_warn "This script requires sudo privileges for system installation."
        sudo -v
    fi
}

deploy_component() {
    local package=$1
    local binary=$2
    local service=$3
    
    log_info "🚀 Deploying $package..."
    
    # Pre-build checks (Agent)
    if type agent_recall &>/dev/null; then
        agent_recall "deploy:$package:error" 5 2>/dev/null || true
        agent_store "pre-build" "$package" "{\"package\":\"$package\"}" "deployment" "pre-build" "$package" 2>/dev/null || true
    fi

    # SPECIAL: Build UI for op-web
    if [[ "$package" == "op-web" ]]; then
        local ui_changed=false
        if git diff --name-only HEAD~1 | grep -q "crates/op-web-ui"; then ui_changed=true; fi
        if [[ ! -d "$PROJECT_ROOT/crates/op-web-ui/dist" ]]; then ui_changed=true; fi
        
        if [[ "$ui_changed" == "true" ]]; then
            log_info "Building Web Frontend (crates/op-web-ui)..."
            if command -v trunk >/dev/null; then
                pushd "$PROJECT_ROOT/crates/op-web-ui" >/dev/null
                if trunk build --release; then
                    log_info "UI Build Successful"
                    popd >/dev/null
                    ln -sf "$PROJECT_ROOT/crates/op-web-ui/dist" "$PROJECT_ROOT/static"
                else
                    log_error "UI Build Failed"
                    popd >/dev/null
                    return 1
                fi
            else
                log_warn "Trunk not found, skipping UI build."
            fi
        else
            log_info "UI Unchanged, skipping build."
            # Ensure link exists even if skipped
            if [[ -d "$PROJECT_ROOT/crates/op-web-ui/dist" ]]; then
                ln -sf "$PROJECT_ROOT/crates/op-web-ui/dist" "$PROJECT_ROOT/static"
            fi
        fi
    fi
    
    # 1. Build

    log_info "Building $package..."
    mkdir -p target/release/deps
    local build_log="/tmp/deploy-build-${package}.log"
    # Ensure log file is writable (may be running as sudo)
    touch "$build_log" 2>/dev/null || true
    local build_start=$(date +%s)
    
    # Run cargo build - output goes directly to console for visibility
    # Only tee to log file if we have permission
    if command -v cargo &> /dev/null; then
        if cargo build --release -p "$package" 2>&1; then
            log_info "Cargo Build successful."
            if type agent_store &>/dev/null; then
                 agent_store "post-build" "$package" "{\"status\":\"success\"}" "deployment" "post-build" "$package" "success" 2>/dev/null || true
            fi
        else
            log_warn "Build failed for $package. Checking for existing binary..."
        if [[ -f "target/release/$binary" ]]; then
            log_info "Found existing binary at target/release/$binary. Proceeding..."
        else
            log_error "Build failed and no existing binary found for $binary. Cannot deploy."
            return 1
        fi
        fi
    else
        log_warn "Cargo not found. Skipping build step."
        if [[ -f "target/release/$binary" ]]; then
            log_info "Found existing binary at target/release/$binary. Proceeding with deployment..."
        else
            log_error "Cargo missing and no existing binary found for $binary. Cannot deploy."
            return 1
        fi
    fi
    
    # 2. Stop service
    log_info "Stopping $service..."
    $SYSTEMCTL_CMD stop "$service" || true
    
    # 3. Install binary
    local bin_path="target/release/$binary"
    if [[ -f "$bin_path" ]]; then
        if [[ "$USER_MODE" == "true" ]]; then
             log_info "User Mode: Using binary at $bin_path"
        else
             log_info "Installing $binary to $INSTALL_DIR..."
             $SUDO cp -f "$bin_path" "$INSTALL_DIR/$binary"
             $SUDO chown root:root "$INSTALL_DIR/$binary"
             $SUDO chmod 755 "$INSTALL_DIR/$binary"
        fi
    else
        log_error "Binary not found at $bin_path"
        return 1
    fi


    
    # 3b. System Dirs (System only)
    if [[ "$USER_MODE" == "false" ]]; then
        $SUDO mkdir -p /etc/op-dbus /opt/op-dbus /var/lib/op-dbus /var/log/op-dbus
        $SUDO chown root:root /etc/op-dbus /opt/op-dbus /var/lib/op-dbus /var/log/op-dbus
        if [[ -f ".env" ]]; then
             $SUDO cp ".env" "/etc/op-dbus/environment"
        fi
    fi
    
    # 4. Service File
    local repo_service="deploy/systemd/$service"
    local installed_service="$SYSTEMD_DIR/$service"
    
    if [[ -f "$repo_service" ]]; then
        local temp_svc="/tmp/${service}.install"
        cp "$repo_service" "$temp_svc"
        
        if [[ "$USER_MODE" == "true" ]]; then
             # Transform for User Mode
             sed -i "s|/usr/local/sbin/|${PROJECT_ROOT}/target/release/|g" "$temp_svc"
             sed -i "s|EnvironmentFile=-/etc/op-dbus/environment|EnvironmentFile=${HOME}/.gemini/systemd_env|g" "$temp_svc"
             sed -i "s|EnvironmentFile=-/etc/op-dbus/secrets.env||g" "$temp_svc"
             sed -i "s|/var/lib/op-dbus|${PROJECT_ROOT}|g" "$temp_svc"
             sed -i "s|/var/log/op-dbus|${PROJECT_ROOT}/logs|g" "$temp_svc"
             sed -i "s|ProtectHome=read-only|ProtectHome=false|g" "$temp_svc"
        else
             # Ensure defaults (if repo file was dev)
             sed -i "s|${PROJECT_ROOT}|/opt/op-dbus|g" "$temp_svc"
        fi
        
        if [[ ! -f "$installed_service" ]] || ! cmp -s "$temp_svc" "$installed_service"; then
             log_info "Updating service file..."
             $SUDO cp "$temp_svc" "$installed_service"
             $SYSTEMCTL_CMD daemon-reload
        fi
        rm -f "$temp_svc"
    fi
    
    # 5. Start
    log_info "Starting $service..."
    $SYSTEMCTL_CMD start "$service"
    
    if $SYSTEMCTL_CMD is-active --quiet "$service"; then
        log_info "✅ $service is running."
    else
         log_error "❌ $service failed to start."
         if [[ "$USER_MODE" == "true" ]]; then
             journalctl --user -u "$service" -n 20 --no-pager 2>/dev/null || true
         else
             journalctl -u "$service" -n 20 2>/dev/null || true
         fi
    fi
}

# Load Services Manifest
if [[ -f "$SCRIPT_DIR/services.conf" ]]; then
    source "$SCRIPT_DIR/services.conf"
else
    log_warn "No services.conf found, using defaults."
    REGISTERED_SERVICES=(
        "op-web:op-web-server:op-web.service"
        "op-mcp:op-mcp-server:op-mcp.service"
        "op-agents:op-agent-manager:op-agents.service"
    )
fi

# Track what needs building
declare -A TOOLS_TO_BUILD

detect_changes() {
    log_info "🔍 Detecting changes..."
    local changes_detected=false
    
    # Check each registered service
    for entry in "${REGISTERED_SERVICES[@]}"; do
        IFS=':' read -r crate binary service <<< "$entry"
        
        # Check core or specific crate changes (support root level or crates/ dir)
        if git diff --name-only HEAD~1 | grep -qE "(^${crate}/|crates/${crate}|crates/op-core|crates/op-tools)"; then
            TOOLS_TO_BUILD[$crate]=true
            changes_detected=true
        fi
    done
    
    if [[ "$changes_detected" == "false" ]]; then
         log_warn "No specific changes detected, building ALL."
         for entry in "${REGISTERED_SERVICES[@]}"; do
            IFS=':' read -r crate binary service <<< "$entry"
            TOOLS_TO_BUILD[$crate]=true
         done
    fi
}

# Main Execution logic
check_root

# Parse args or detect changes
if [[ -n "$1" ]]; then
    if [[ "$1" == "all" ]] || [[ "$1" == "--all" ]]; then
        log_info "Building ALL services (--all flag detected)"
        for entry in "${REGISTERED_SERVICES[@]}"; do
            IFS=':' read -r crate binary service <<< "$entry"
            TOOLS_TO_BUILD[$crate]=true
        done
    else
        # Allow deploying specific crate name
        for entry in "${REGISTERED_SERVICES[@]}"; do
            IFS=':' read -r crate binary service <<< "$entry"
            if [[ "$1" == "$crate" ]]; then
                 TOOLS_TO_BUILD[$crate]=true
            fi
        done
    fi
else
    detect_changes
fi

# Execute Deployments
# Use a separate array to track what we've validated/deployed to avoid duplicates if 
# multiple binaries come from same crate (though deploy_component builds per binary logic usually)
# Actually deploy_component takes component/binary/service.
# If a crate produces multiple binaries, our config handles that as separate lines.

for entry in "${REGISTERED_SERVICES[@]}"; do
    IFS=':' read -r crate binary service <<< "$entry"
    
    if [[ "${TOOLS_TO_BUILD[$crate]}" == "true" ]]; then
        deploy_component "$crate" "$binary" "$service"
    fi
done
