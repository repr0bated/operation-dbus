# Example NixOS Configuration for op-dbus
# This configuration demonstrates a complete op-dbus setup with:
# - D-Bus state management
# - Open vSwitch networking
# - MCP introspection and agents
# - Package management via PackageKit

{ config, pkgs, lib, ... }:

{
  imports = [
    ./module.nix
  ];

  # op-dbus configuration
  services.op-dbus = {
    enable = true;
    mode = "standalone"; # Options: full, standalone, agent-only

    # MCP (Model Context Protocol) Integration
    mcp = {
      enable = true;
      introspection = true;
      hybridScanner = true;

      agents = {
        systemd = true;
        packagekit = true;
        network = true;
        file = true;
      };
    };

    # Network Configuration
    network = {
      interfaces = [
        {
          name = "ovsbr0";
          type = "ovs-bridge";
          ports = [ "eth0" ];
          ipv4 = {
            enabled = true;
            dhcp = false;
            addresses = [
              {
                ip = "192.168.1.100";
                prefix = 24;
              }
            ];
            gateway = "192.168.1.1";
          };
        }
      ];

      ovs = {
        enable = true;
        bridges = [ "ovsbr0" "mesh" ];
      };
    };

    # Systemd Units Management
    systemd = {
      units = {
        "sshd.service" = {
          active_state = "active";
          enabled = true;
        };
        "op-dbus.service" = {
          active_state = "active";
          enabled = true;
        };
      };
    };

    # Package Management
    packages = {
      enable = true;
      installed = [
        "nginx"
        "postgresql"
        "git"
        "htop"
        "tmux"
      ];
      removed = [
        # Packages to explicitly remove
      ];
      autoUpdate = false;
    };
  };

  # Additional system configuration
  system = {
    stateVersion = "24.05"; # NixOS version
  };

  # Basic networking (besides OVS)
  networking = {
    hostName = "op-dbus-node";
    firewall = {
      enable = true;
      allowedTCPPorts = [ 22 80 443 6653 ]; # SSH, HTTP, HTTPS, OpenFlow
    };
  };

  # Enable SSH
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "no";
      PasswordAuthentication = false;
    };
  };

  # Users configuration
  users.users.admin = {
    isNormalUser = true;
    extraGroups = [ "wheel" "networkmanager" "docker" ];
    openssh.authorizedKeys.keys = [
      # Add SSH public keys here
    ];
  };

  # System packages
  environment.systemPackages = with pkgs; [
    vim
    git
    curl
    wget
    htop
    tmux
    busctl # D-Bus introspection tool
  ];

  # Enable D-Bus
  services.dbus.enable = true;

  # Time zone
  time.timeZone = "UTC";

  # Locale
  i18n.defaultLocale = "en_US.UTF-8";

  # Nix configuration
  nix = {
    settings = {
      experimental-features = [ "nix-command" "flakes" ];
      auto-optimise-store = true;
    };

    # Automatic garbage collection
    gc = {
      automatic = true;
      dates = "weekly";
      options = "--delete-older-than 30d";
    };
  };

  # Enable automatic system upgrades (optional)
  system.autoUpgrade = {
    enable = false; # Set to true for automatic updates
    allowReboot = false;
  };
}
