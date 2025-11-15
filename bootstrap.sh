#!/usr/bin/env bash
# Bootstrap script for fresh NixOS installation
# This gets you from "just installed NixOS" to "working operation-dbus"

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "operation-dbus Bootstrap Script"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check if we're on NixOS
if ! command -v nixos-version &> /dev/null; then
    echo "❌ This doesn't look like NixOS!"
    echo "   Run 'nixos-version' to check"
    exit 1
fi

echo "✓ NixOS detected: $(nixos-version)"
echo ""

# Check if operation-dbus folder exists
if [ ! -f "Cargo.toml" ] || [ ! -d "src" ]; then
    echo "❌ This doesn't look like the operation-dbus folder"
    echo "   Run this script from the operation-dbus directory:"
    echo "   cd /path/to/operation-dbus"
    echo "   ./bootstrap.sh"
    exit 1
fi

echo "✓ Found operation-dbus project"
echo ""

# Pull latest changes if we have git
if command -v git &> /dev/null; then
    echo "📥 Pulling latest changes..."
    git pull origin claude/this-is-no-011CUr6iUZZC7YKhhLSY2MK6 || echo "⚠️  Git pull failed (maybe you're offline?)"
    echo ""
else
    echo "⚠️  git not found - installing temporarily..."
    nix-shell -p git --run "git pull origin claude/this-is-no-011CUr6iUZZC7YKhhLSY2MK6" || echo "⚠️  Git pull failed"
    echo ""
fi

# Check what we have
echo "📋 Checking files..."

MISSING=()

if [ ! -f "nixos/modules/operation-dbus.nix" ]; then
    MISSING+=("nixos/modules/operation-dbus.nix")
fi

if [ ! -f "nixos/netboot/build-installer.sh" ]; then
    MISSING+=("nixos/netboot/build-installer.sh")
fi

if [ ! -f "NETBOOT-QUICKSTART.md" ]; then
    MISSING+=("NETBOOT-QUICKSTART.md")
fi

if [ ${#MISSING[@]} -gt 0 ]; then
    echo "❌ Missing files:"
    for file in "${MISSING[@]}"; do
        echo "   - $file"
    done
    echo ""
    echo "This means:"
    echo "1. You might be on the wrong git branch"
    echo "2. You might have an old copy of the repository"
    echo ""
    echo "Current branch:"
    if command -v git &> /dev/null; then
        git branch --show-current
    else
        echo "(git not available)"
    fi
    echo ""
    echo "Try:"
    echo "  nix-shell -p git"
    echo "  git checkout claude/this-is-no-011CUr6iUZZC7YKhhLSY2MK6"
    echo "  git pull"
    exit 1
fi

echo "✓ All required files present"
echo ""

# Ask what they want to do
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "What do you want to do?"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "1. Install NixOS to disk with operation-dbus"
echo "2. Just build operation-dbus (development/testing)"
echo "3. Show me the documentation"
echo "4. Exit (I'll do it manually)"
echo ""
read -p "Choice [1-4]: " choice

case $choice in
    1)
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "NixOS Installation"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        echo "⚠️  WARNING: This will ERASE your disk!"
        echo ""
        echo "Available disks:"
        lsblk -d -o NAME,SIZE,TYPE,MODEL | grep disk
        echo ""
        read -p "Which disk? (e.g., sda, nvme0n1): " disk_name

        if [ -z "$disk_name" ]; then
            echo "❌ No disk specified"
            exit 1
        fi

        DISK="/dev/$disk_name"

        if [ ! -b "$DISK" ]; then
            echo "❌ $DISK is not a block device"
            exit 1
        fi

        echo ""
        echo "Will install to: $DISK"
        read -p "Type 'yes' to continue: " confirm

        if [ "$confirm" != "yes" ]; then
            echo "Cancelled"
            exit 0
        fi

        echo ""
        echo "Read START-HERE.md for manual installation steps"
        echo "Or use the automated installer from NETBOOT-TO-DISK-INSTALL.md"
        echo ""
        echo "Quick version:"
        echo "  cat START-HERE.md | less"
        ;;

    2)
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "Building operation-dbus"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        echo "Entering nix-shell with Rust and dependencies..."
        echo ""

        nix-shell -p rustc cargo gcc pkg-config openssl dbus.dev systemd.dev --run "
            echo '🔨 Building...'
            cargo build --release
            echo ''
            echo '✅ Build complete!'
            echo ''
            echo 'Binary location: target/release/op-dbus'
            echo ''
            echo 'Try running:'
            echo '  ./target/release/op-dbus --help'
        "
        ;;

    3)
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "Documentation"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        echo "Available guides:"
        echo ""
        echo "  START-HERE.md              - Start here (you are here!)"
        echo "  NETBOOT-QUICKSTART.md      - Quick reference for netboot"
        echo "  NETBOOT-TO-DISK-INSTALL.md - Detailed installation guide"
        echo "  NIXOS-SETUP-GUIDE.md        - NixOS module configuration"
        echo "  README.md                   - Project overview"
        echo ""
        echo "To read:"
        echo "  cat START-HERE.md | less"
        echo "  cat NETBOOT-QUICKSTART.md | less"
        echo ""
        ;;

    4)
        echo ""
        echo "No problem! Here are the key files:"
        echo ""
        echo "  START-HERE.md              - Read this first"
        echo "  NETBOOT-QUICKSTART.md      - Quick command reference"
        echo "  nixos/modules/operation-dbus.nix - NixOS module"
        echo ""
        echo "Good luck!"
        ;;

    *)
        echo "Invalid choice"
        exit 1
        ;;
esac

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Bootstrap complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
