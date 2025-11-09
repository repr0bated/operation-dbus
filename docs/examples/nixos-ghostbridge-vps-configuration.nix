# GhostBridge VPS - NixOS Configuration
# Privacy Router Server (Profile 3: privacy-vps)

{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    # Uncomment when op-dbus is in nixpkgs or add as flake input
    # ./op-dbus/nix/module.nix
  ];

  # System Information
  system.stateVersion = "24.05"; # Don't change after install
  networking.hostName = "ghostbridge-vps";

  # Boot Configuration
  boot.loader.grub = {
    enable = true;
    device = "/dev/vda"; # Change to your disk (vda, sda, nvme0n1)
    # For UEFI:
    # efiSupport = true;
    # device = "nodev";
  };

  # Networking
  networking = {
    useDHCP = false;
    interfaces.eth0 = {
      useDHCP = true;
      # Or static:
      # ipv4.addresses = [{
      #   address = "80.209.240.244";
      #   prefixLength = 24;
      # }];
    };

    # Firewall
    firewall = {
      enable = true;
      allowedTCPPorts = [
        22    # SSH
        443   # XRay (appears as HTTPS)
        9574  # op-dbus web UI
      ];
      allowedUDPPorts = [
        51820 # WireGuard (if using Netmaker)
      ];
    };
  };

  # Enable OpenVSwitch (required for GhostBridge)
  virtualisation.vswitch = {
    enable = true;
    package = pkgs.openvswitch;
  };

  # Enable LXC/LXD for containers
  virtualisation.lxc = {
    enable = true;
    lxcfs.enable = true;
  };

  # System packages
  environment.systemPackages = with pkgs; [
    # Networking
    openvswitch
    wireguard-tools

    # Container tools
    lxc

    # Development/debugging
    vim
    git
    curl
    wget
    htop

    # Build tools (for op-dbus)
    cargo
    rustc
    pkg-config
    openssl
    dbus
  ];

  # SSH Configuration
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "prohibit-password";
      PasswordAuthentication = false;
    };
    # Add your SSH key here:
    # users.users.root.openssh.authorizedKeys.keys = [
    #   "ssh-ed25519 AAAA... your-key-here"
    # ];
  };

  # D-Bus (required for op-dbus)
  services.dbus.enable = true;

  # Time & Locale
  time.timeZone = "UTC";
  i18n.defaultLocale = "en_US.UTF-8";

  # User configuration
  users.users.root = {
    openssh.authorizedKeys.keys = [
      # Add your SSH key from the private key you shared earlier
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFJbn4KUHoDm+YL2EubNIw3L96vfsVwmvDJ2oGD+4B+c root@ghostbridge"
    ];
  };

  # op-dbus systemd service (manual until module is ready)
  systemd.services.op-dbus = {
    description = "op-dbus declarative system state manager";
    wantedBy = [ "multi-user.target" ];
    after = [ "network.target" "openvswitch.service" ];

    serviceConfig = {
      Type = "simple";
      ExecStart = "${pkgs.writeShellScript "op-dbus-start" ''
        # This will run op-dbus from /usr/local/bin once installed
        exec /usr/local/bin/op-dbus run
      ''}";
      Restart = "on-failure";
      RestartSec = "5s";
    };
  };

  # op-dbus web UI service
  systemd.services.op-dbus-webui = {
    description = "op-dbus Web UI";
    wantedBy = [ "multi-user.target" ];
    after = [ "network.target" ];

    serviceConfig = {
      Type = "simple";
      ExecStart = "/usr/local/bin/op-dbus serve --bind 0.0.0.0 --port 9574";
      Restart = "on-failure";
      RestartSec = "5s";
    };
  };

  # Netmaker client (if using mesh networking)
  # systemd.services.netclient = {
  #   description = "Netmaker VPN Client";
  #   wantedBy = [ "multi-user.target" ];
  #   after = [ "network.target" ];
  #   serviceConfig = {
  #     Type = "simple";
  #     ExecStart = "/usr/local/bin/netclient daemon";
  #     Restart = "always";
  #   };
  # };

}
