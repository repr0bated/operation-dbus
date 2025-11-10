# NixOS Configuration Generated from op-dbus Introspection
# Server: root@80.209.240.244
# Date: 2025-11-09

{ config, pkgs, lib, ... }:

{
  imports = [ ];

  # Enable experimental features for flakes
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  # op-dbus service configuration
  services.op-dbus = {
    enable = true;
    mode = "agent";  # Agent mode - no OVS required
    
    # Use the introspected state configuration
    stateConfig = {
      # DNS Resolver Configuration (from introspection)
      dnsresolver = {
        version = 1;
        items = [
          {
            id = "resolvconf";
            mode = "observe-only";
            servers = [ "8.8.8.8" "8.8.4.4" ];
            options = [ "edns0" ];
          }
        ];
      };

      # Session Management (from introspection)
      sess = {
        sessions = [
          {
            session_id = "1";
            user = "nixos";
            uid = "1000";
            seat = "seat0";
            tty = "931";
          }
          # Additional sessions will be managed dynamically
        ];
      };

      # Login1 D-Bus Integration
      login1 = {
        sessions = [
          {
            id = "1";
            user = "nixos";
            uid = 1000;
            seat = "seat0";
            path = "/org/freedesktop/login1/session/_31";
          }
        ];
      };

      # Network Configuration (empty from introspection, but can be added)
      net = {
        interfaces = [];
      };

      # LXC Containers (empty, not using containers)
      lxc = {
        containers = [];
      };

      # Systemd Units Management
      systemd = {};

      # PCI Device Declaration (empty from introspection)
      pcidecl = {
        version = 1;
        items = [];
      };
    };

    # Enable blockchain audit logging
    enableBlockchain = true;

    # Enable caching
    enableCache = true;

    # Data directory
    dataDir = "/var/lib/op-dbus";
  };

  # System-wide packages
  environment.systemPackages = with pkgs; [
    vim
    git
    htop
    curl
  ];

  # Enable OpenSSH for remote access
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";
      PasswordAuthentication = true;
    };
  };

  # Networking configuration
  networking = {
    hostName = "nixos-opdbus";
    firewall.enable = true;
    firewall.allowedTCPPorts = [ 22 ];
  };

  # Boot loader configuration (adjust as needed)
  boot.loader.grub.enable = true;
  boot.loader.grub.device = "/dev/sda";

  # System state version
  system.stateVersion = "25.05";
}
