# Example NixOS configuration for a workstation running operation-dbus
# This demonstrates a minimal setup with CPU-only ML and PackageKit integration
#
# Usage:
#   1. Copy this to your NixOS configuration directory
#   2. Add to /etc/nixos/configuration.nix:
#      imports = [ ./workstation.nix ];
#   3. Run: sudo nixos-rebuild switch

{ config, pkgs, ... }:

{
  imports = [
    # Import the operation-dbus module
  ];

  # Minimal operation-dbus configuration for workstation
  services.operation-dbus = {
    enable = true;

    # State file location
    stateFile = "/etc/operation-dbus/workstation.json";

    # Disable NUMA (single-socket system)
    numa.enable = false;

    # BTRFS configuration (minimal)
    btrfs = {
      enable = true;
      compressionLevel = 3;
      snapshotRetention = 12; # Keep last 12 hours
    };

    # ML vectorization with CPU only
    ml = {
      enable = true;
      executionProvider = "cpu";
      numThreads = 4; # Adjust based on your CPU
    };

    # Auto-generated plugin with semantic mapping
    plugins = with pkgs; [
      operation-dbus-plugin-packagekit
    ];

    # Simple infrastructure state
    defaultState = {
      version = "1.0";

      plugins = {
        # PackageKit for package management
        packagekit = {
          packages = [
            "firefox"
            "vim"
            "git"
            "htop"
          ];

          repositories = {
            enable_contrib = true;
            enable_non_free = false;
          };
        };
      };
    };

    # Logging
    logLevel = "info";
  };

  # Example plugin: PackageKit (auto-generated with semantic mapping)
  nixpkgs.config.packageOverrides = pkgs: {
    operation-dbus-plugin-packagekit = pkgs.stdenv.mkDerivation {
      pname = "operation-dbus-plugin-packagekit";
      version = "1.0.0";

      src = pkgs.fetchFromGitHub {
        owner = "repr0bated";
        repo = "operation-dbus";
        rev = "main";
        sha256 = ""; # Add actual hash
      };

      installPhase = ''
        mkdir -p $out
        cp plugins/packagekit/plugin.toml $out/
        cp plugins/packagekit/semantic-mapping.toml $out/
        cp plugins/packagekit/introspection.xml $out/ || true
      '';

      meta = {
        description = "PackageKit auto-generated plugin with semantic mapping";
        license = pkgs.lib.licenses.mit;
      };
    };
  };
}
