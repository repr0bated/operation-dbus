#!/bin/bash
# install-dependencies.sh - Install system prerequisites (imperative bootstrap)
# These are generic technologies, not unique to op-dbus

set -euo pipefail

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  op-dbus Dependency Installer"
echo "  Installing generic prerequisites..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check root
if [ "$EUID" -ne 0 ]; then
    echo "❌ This script must be run as root"
    echo "   Run: sudo $0"
    exit 1
fi

# Detect platform
detect_platform() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$ID
        OS_VERSION=$VERSION_ID
    elif [ -f /etc/debian_version ]; then
        OS="debian"
        OS_VERSION=$(cat /etc/debian_version)
    else
        OS="unknown"
        OS_VERSION="unknown"
    fi

    echo "📋 Detected platform: $OS $OS_VERSION"
}

# Install dependencies based on platform
install_debian_ubuntu() {
    echo ""
    echo "━━━ Installing Debian/Ubuntu packages ━━━"

    # Update package list
    echo "🔄 Updating package lists..."
    apt-get update -qq

    # Core dependencies
    PACKAGES=(
        openvswitch-switch   # CRITICAL: OVS for network management
        build-essential      # Build tools
        pkg-config           # Build configuration
        libssl-dev           # SSL/TLS development files
        ca-certificates      # SSL certificates
        curl                 # HTTP client
        git                  # Version control
        jq                   # JSON processor (for scripts)
    )

    # Optional dependencies
    OPTIONAL_PACKAGES=(
        btrfs-progs          # BTRFS tools for cache storage
        numactl              # NUMA control utilities
    )

    echo ""
    echo "📦 Installing core packages..."
    for pkg in "${PACKAGES[@]}"; do
        if dpkg -l "$pkg" 2>/dev/null | grep -q "^ii"; then
            echo "  ✅ $pkg (already installed)"
        else
            echo "  ⏳ Installing $pkg..."
            apt-get install -y -qq "$pkg"
            echo "  ✅ $pkg installed"
        fi
    done

    echo ""
    echo "📦 Installing optional packages..."
    for pkg in "${OPTIONAL_PACKAGES[@]}"; do
        if dpkg -l "$pkg" 2>/dev/null | grep -q "^ii"; then
            echo "  ✅ $pkg (already installed)"
        else
            echo "  ⏳ Installing $pkg..."
            if apt-get install -y -qq "$pkg" 2>/dev/null; then
                echo "  ✅ $pkg installed"
            else
                echo "  ⚠️  $pkg installation failed (optional, continuing)"
            fi
        fi
    done
}

install_rhel_centos() {
    echo ""
    echo "━━━ Installing RHEL/CentOS packages ━━━"

    PACKAGES=(
        openvswitch
        gcc
        make
        pkg-config
        openssl-devel
        ca-certificates
        curl
        git
        jq
    )

    echo "📦 Installing packages..."
    for pkg in "${PACKAGES[@]}"; do
        if rpm -q "$pkg" &>/dev/null; then
            echo "  ✅ $pkg (already installed)"
        else
            echo "  ⏳ Installing $pkg..."
            yum install -y -q "$pkg"
            echo "  ✅ $pkg installed"
        fi
    done
}

# Check Rust installation
check_rust() {
    echo ""
    echo "━━━ Checking Rust installation ━━━"

    if command -v cargo &> /dev/null; then
        RUST_VERSION=$(rustc --version 2>/dev/null || echo "unknown")
        echo "✅ Rust is installed: $RUST_VERSION"
        return 0
    else
        echo "⚠️  Rust/Cargo not found"
        echo ""
        echo "op-dbus is written in Rust and requires cargo to build."
        echo ""
        read -rp "Install Rust via rustup? [Y/n]: " INSTALL_RUST

        if [[ ! "$INSTALL_RUST" =~ ^[Nn]$ ]]; then
            echo "⏳ Installing Rust via rustup..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

            # Source cargo env
            if [ -f "$HOME/.cargo/env" ]; then
                source "$HOME/.cargo/env"
            fi

            if command -v cargo &> /dev/null; then
                echo "✅ Rust installed successfully"
                rustc --version
            else
                echo "❌ Rust installation failed"
                echo "   Please install manually: https://rustup.rs"
                return 1
            fi
        else
            echo "⚠️  Skipping Rust installation"
            echo "   You'll need to install it manually to build op-dbus"
            return 1
        fi
    fi
}

# Verify OVS installation
verify_ovs() {
    echo ""
    echo "━━━ Verifying OpenVSwitch ━━━"

    # Check if ovs-vsctl exists
    if ! command -v ovs-vsctl &> /dev/null; then
        echo "❌ ovs-vsctl command not found"
        echo "   OpenVSwitch installation may have failed"
        return 1
    fi

    # Start OVS services
    echo "🔧 Starting OVS services..."
    systemctl start openvswitch-switch 2>/dev/null || systemctl start openvswitch 2>/dev/null || true
    sleep 2

    # Check if OVS is responding
    if ovs-vsctl show &> /dev/null; then
        echo "✅ OpenVSwitch is working"
        return 0
    else
        echo "⚠️  OVS not responding, restarting..."
        systemctl restart openvswitch-switch 2>/dev/null || systemctl restart openvswitch 2>/dev/null
        sleep 3

        if ovs-vsctl show &> /dev/null; then
            echo "✅ OpenVSwitch is now working"
            return 0
        else
            echo "❌ OpenVSwitch is not responding"
            echo "   Check: systemctl status openvswitch-switch"
            return 1
        fi
    fi
}

# Optional: Install Netmaker client
install_netmaker() {
    echo ""
    echo "━━━ Netmaker Installation (Optional) ━━━"
    echo "Netmaker provides mesh networking for containers."
    echo ""

    if command -v netclient &> /dev/null; then
        echo "✅ netclient already installed"
        netclient --version
        return 0
    fi

    read -rp "Install Netmaker netclient? [y/N]: " INSTALL_NETCLIENT

    if [[ "$INSTALL_NETCLIENT" =~ ^[Yy]$ ]]; then
        echo "⏳ Installing netclient..."

        # Add Netmaker repository
        curl -sL https://apt.netmaker.org/gpg.key | apt-key add - 2>/dev/null || true
        curl -sL https://apt.netmaker.org/debian.deb.txt | tee /etc/apt/sources.list.d/netmaker.list >/dev/null
        apt-get update -qq

        if apt-get install -y netclient 2>/dev/null; then
            echo "✅ netclient installed"
        else
            echo "⚠️  netclient installation failed (optional, continuing)"
        fi
    else
        echo "⏹️  Skipping netclient installation"
    fi
}

# Check if running in Proxmox
check_proxmox() {
    if command -v pct &> /dev/null; then
        echo "✅ Proxmox detected (pct command available)"
        return 0
    else
        echo "ℹ️  Proxmox not detected (no pct command)"
        return 1
    fi
}

# Main installation flow
main() {
    detect_platform

    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Install packages based on platform
    case "$OS" in
        debian|ubuntu)
            install_debian_ubuntu
            ;;
        rhel|centos|fedora)
            install_rhel_centos
            ;;
        *)
            echo "❌ Unsupported platform: $OS"
            echo "   Please install dependencies manually:"
            echo "   - openvswitch-switch"
            echo "   - build-essential, pkg-config, libssl-dev"
            echo "   - Rust/Cargo (https://rustup.rs)"
            exit 1
            ;;
    esac

    # Verify installations
    verify_ovs
    check_rust
    check_proxmox

    # Optional components
    install_netmaker

    # Final summary
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  ✅ Dependency Installation Complete"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "Next steps:"
    echo "  1. Build op-dbus:     ./build.sh"
    echo "  2. Install op-dbus:   sudo ./install.sh"
    echo ""
}

# Run main
main "$@"
