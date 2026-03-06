#!/bin/bash
# deploy/deploy.sh
# Tight, Dinit-focused deployment for Operation D-Bus

set -e

# --- Configuration ---
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

# Components: "crate_name:binary_name:service_name"
SERVICES=(
    "op-web:op-web-server:op-web"
    "op-services:op-services:op-services"
    "op-chat:op-chat:op-chat"
)

# Detect Mode (System vs User)
if [ "$EUID" -ne 0 ]; then
    echo "Running in USER mode (installing to $PROJECT_ROOT/deploy/bin)"
    INSTALL_DIR="$PROJECT_ROOT/deploy/bin"
    SERVICE_DIR="$PROJECT_ROOT/deploy/services"
    SOCKET_PATH="$PROJECT_ROOT/deploy/dinit.socket"
    SUDO=""
    DINITCTL="dinitctl -p $SOCKET_PATH"
    
    mkdir -p "$INSTALL_DIR" "$SERVICE_DIR"
    
    # Start dinit if not running
    if [ ! -S "$SOCKET_PATH" ]; then
        echo "Starting local dinit instance..."
        # Create a boot service with correct type
        echo "type = internal" > "$SERVICE_DIR/boot"
        
        dinit --user -p "$SOCKET_PATH" -d "$SERVICE_DIR" -l "$PROJECT_ROOT/deploy/dinit.log" &
        sleep 5
    fi
else
    echo "Running in SYSTEM mode (installing to /usr/local/sbin)"
    INSTALL_DIR="/usr/local/sbin"
    SERVICE_DIR="/etc/dinit.d"
    SUDO="" # Already root
    DINITCTL="dinitctl"
    mkdir -p "$INSTALL_DIR" "$SERVICE_DIR"
fi

# --- Functions ---

log() { echo -e "\033[0;32m[DEPLOY]\033[0m $1"; }
warn() { echo -e "\033[1;33m[WARN]\033[0m $1"; }
error() { echo -e "\033[0;31m[ERROR]\033[0m $1"; exit 1; }
is_started() { $DINITCTL status "$1" 2>/dev/null | grep -q "State: STARTED"; }

build_and_install() {
    local crate=$1
    local binary=$2
    local service=$3

    log "Building $crate..."
    if ! cargo build --release -p "$crate"; then
        error "Build failed for $crate"
    fi

    log "Installing $binary..."
    local staged="$INSTALL_DIR/${binary}.new.$$"
    install -m 755 "target/release/$binary" "$staged"
    mv -f "$staged" "$INSTALL_DIR/$binary"
}

generate_service_file() {
    local binary=$1
    local service=$2
    local file="$SERVICE_DIR/$service"

    log "Generating dinit service for $service..."
    
    # Simple, robust dinit definition
    cat <<EOF > "$file"
type = process
command = $INSTALL_DIR/$binary
log-type = buffer
smooth-recovery = true
EOF

    # Local environment overrides
    if [ "$EUID" -ne 0 ]; then
        local DATA_DIR="$PROJECT_ROOT/deploy/data"
        mkdir -p "$DATA_DIR/cache"
        cat <<EOF >> "$file"
env = OP_DBUS_DATABASE_URL=sqlite://$DATA_DIR/state.db
env = OP_DBUS_CACHE_DIR=$DATA_DIR/cache
env = OP_DBUS_WEB_PORT=8081
env = OP_DBUS_SESSION_BUS=1
EOF
    fi

    # Add dependencies if needed
    if [ "$service" != "op-web" ]; then
        echo "depends-on = op-web" >> "$file"
    fi
}

deploy_service() {
    local crate=$1
    local binary=$2
    local service=$3
    local stopped_dependents=()

    # Build & Install
    build_and_install "$crate" "$binary" "$service"

    # Generate Service Config
    generate_service_file "$binary" "$service"

    # Ensure services start on boot in system mode
    if [ "$EUID" -eq 0 ]; then
        mkdir -p "$SERVICE_DIR/boot.d"
        ln -sf "../$service" "$SERVICE_DIR/boot.d/$service"
    fi

    # op-web cannot restart while dependents are active under dinit.
    # Stop dependents first and bring them back after op-web restarts.
    if [ "$service" = "op-web" ]; then
        for entry in "${SERVICES[@]}"; do
            IFS=':' read -r _ _ dep_service <<< "$entry"
            if [ "$dep_service" != "op-web" ] && is_started "$dep_service"; then
                log "Stopping dependent $dep_service..."
                $DINITCTL stop "$dep_service"
                stopped_dependents+=("$dep_service")
            fi
        done
    fi

    # Restart only when already started, otherwise start
    if is_started "$service"; then
        log "Restarting $service..."
        $DINITCTL restart "$service"
    else
        log "Starting $service..."
        $DINITCTL start "$service"
    fi

    if [ "$service" = "op-web" ] && [ "${#stopped_dependents[@]}" -gt 0 ]; then
        for dep in "${stopped_dependents[@]}"; do
            log "Starting dependent $dep..."
            $DINITCTL start "$dep"
        done
    fi
    
    log "✅ $service deployed"
}

# --- Main ---

# Check dependencies
command -v cargo >/dev/null || error "Cargo not found"
command -v dinitctl >/dev/null || warn "dinitctl not found"

# Deploy selected or all
TARGET=$1

for entry in "${SERVICES[@]}"; do
    IFS=':' read -r crate binary service <<< "$entry"
    
    if [ -z "$TARGET" ] || [ "$TARGET" == "all" ] || [ "$TARGET" == "$crate" ]; then
        deploy_service "$crate" "$binary" "$service"
    fi
done

log "Deployment Complete."
