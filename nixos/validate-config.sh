#!/usr/bin/env bash
# NixOS Configuration Validation Script for op-dbus
# Validates configuration before applying to ensure it will build correctly

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "======================================"
echo "op-dbus NixOS Configuration Validator"
echo "======================================"
echo ""

# Check if running in NixOS directory
if [ ! -f "module.nix" ]; then
    echo -e "${RED}Error: module.nix not found. Run this script from the nixos/ directory.${NC}"
    exit 1
fi

# Test 1: Check Nix syntax
echo -e "${YELLOW}[1/7]${NC} Checking Nix syntax..."
if nix-instantiate --parse module.nix > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} module.nix syntax is valid"
else
    echo -e "${RED}✗${NC} module.nix has syntax errors"
    exit 1
fi

if [ -f "configuration.nix" ]; then
    if nix-instantiate --parse configuration.nix > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} configuration.nix syntax is valid"
    else
        echo -e "${RED}✗${NC} configuration.nix has syntax errors"
        exit 1
    fi
fi

# Test 2: Check if flake is valid
if [ -f "flake.nix" ]; then
    echo -e "${YELLOW}[2/7]${NC} Validating flake..."
    if nix flake check --no-build 2>&1 | grep -q "error:"; then
        echo -e "${RED}✗${NC} Flake validation failed"
        exit 1
    else
        echo -e "${GREEN}✓${NC} Flake is valid"
    fi
else
    echo -e "${YELLOW}[2/7]${NC} Skipping flake validation (no flake.nix)"
fi

# Test 3: Check Rust source exists
echo -e "${YELLOW}[3/7]${NC} Checking Rust source..."
if [ -f "../Cargo.toml" ] && [ -f "../Cargo.lock" ]; then
    echo -e "${GREEN}✓${NC} Rust source files found"
else
    echo -e "${RED}✗${NC} Missing Cargo.toml or Cargo.lock"
    exit 1
fi

# Test 4: Validate required dependencies
echo -e "${YELLOW}[4/7]${NC} Checking required packages..."
required_pkgs=("rustc" "cargo" "pkg-config")
missing_pkgs=()

for pkg in "${required_pkgs[@]}"; do
    if ! command -v "$pkg" &> /dev/null; then
        missing_pkgs+=("$pkg")
    fi
done

if [ ${#missing_pkgs[@]} -eq 0 ]; then
    echo -e "${GREEN}✓${NC} All required packages available"
else
    echo -e "${YELLOW}⚠${NC} Missing packages (optional for validation): ${missing_pkgs[*]}"
fi

# Test 5: Try to evaluate the module
echo -e "${YELLOW}[5/7]${NC} Evaluating module..."
cat > /tmp/test-op-dbus-config.nix <<'EOF'
{ config, lib, pkgs, ... }:
{
  imports = [ ./module.nix ];

  # Minimal configuration to test evaluation
  services.op-dbus = {
    enable = true;
    mode = "standalone";
  };

  # Required for evaluation
  system.stateVersion = "24.05";
  boot.loader.grub.device = "/dev/sda";
  fileSystems."/" = {
    device = "/dev/sda1";
    fsType = "ext4";
  };
}
EOF

if nix-instantiate --eval '<nixpkgs/nixos>' -A config.services.op-dbus.enable \
    -I nixos-config=/tmp/test-op-dbus-config.nix \
    -I nixpkgs=channel:nixos-24.05 &> /dev/null; then
    echo -e "${GREEN}✓${NC} Module evaluates successfully"
else
    echo -e "${RED}✗${NC} Module evaluation failed"
    echo "Try running manually to see errors:"
    echo "  nix-instantiate --eval '<nixpkgs/nixos>' -A config.services.op-dbus -I nixos-config=/tmp/test-op-dbus-config.nix"
    exit 1
fi

rm -f /tmp/test-op-dbus-config.nix

# Test 6: Check for common configuration errors
echo -e "${YELLOW}[6/7]${NC} Checking for common errors..."
errors=()

# Check if state directory paths are valid
if grep -q "ReadWritePaths.*\[" module.nix; then
    echo -e "${GREEN}✓${NC} State directory paths configured"
else
    errors+=("Missing ReadWritePaths configuration")
fi

# Check if D-Bus service name is defined
if grep -q 'BusName.*=.*"org.opdbus' module.nix; then
    echo -e "${GREEN}✓${NC} D-Bus service names configured"
else
    errors+=("Missing D-Bus service name configuration")
fi

# Check if capabilities are restricted
if grep -q "AmbientCapabilities" module.nix; then
    echo -e "${GREEN}✓${NC} Security capabilities configured"
else
    errors+=("Missing security capabilities")
fi

if [ ${#errors[@]} -gt 0 ]; then
    echo -e "${RED}Found configuration issues:${NC}"
    for error in "${errors[@]}"; do
        echo "  - $error"
    done
    exit 1
fi

# Test 7: Validate example configuration
if [ -f "configuration.nix" ]; then
    echo -e "${YELLOW}[7/7]${NC} Validating example configuration..."

    if nix-instantiate --parse configuration.nix > /dev/null 2>&1; then
        # Check for required options
        required_options=("services.op-dbus.enable" "system.stateVersion")
        for opt in "${required_options[@]}"; do
            if grep -q "$opt" configuration.nix; then
                echo -e "${GREEN}✓${NC} Found $opt"
            else
                echo -e "${YELLOW}⚠${NC} Missing $opt in example configuration"
            fi
        done
    fi
else
    echo -e "${YELLOW}[7/7]${NC} Skipping example configuration validation (not found)"
fi

echo ""
echo -e "${GREEN}======================================"
echo "✓ All validation checks passed!"
echo "======================================${NC}"
echo ""
echo "Next steps:"
echo "  1. Copy module.nix to your NixOS configuration"
echo "  2. Import it in configuration.nix"
echo "  3. Configure services.op-dbus options"
echo "  4. Run: sudo nixos-rebuild test"
echo "  5. If successful: sudo nixos-rebuild switch"
echo ""
echo "For more information, see: README.md"
