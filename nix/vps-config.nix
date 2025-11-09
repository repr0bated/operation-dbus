# Ready-to-deploy NixOS configuration for VPS
# VPS: 80.209.240.244
# Simple Xray proxy server only

{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./module.nix
  ];

  # Boot
  boot.loader.grub = {
    enable = true;
    device = "/dev/vda";  # Change to /dev/sda if needed
  };

  # Network - Static IP
  networking = {
    hostName = "ghostbridge-vps";
    useDHCP = false;
    interfaces.eth0.ipv4.addresses = [{
      address = "80.209.240.244";
      prefixLength = 25;
    }];
    defaultGateway = "80.209.240.129";
    nameservers = [ "8.8.8.8" "1.1.1.1" ];

    firewall = {
      enable = true;
      allowedTCPPorts = [ 22 443 8443 ];
      trustedInterfaces = [ "ovsbr0" ];
    };
  };

  # LXC/Containers
  virtualisation.lxc = {
    enable = true;
    lxcfs.enable = true;
  };

  # op-dbus with single Xray container
  services.op-dbus = {
    enable = true;
    mode = "standalone";  # Standalone mode - no full router

    stateConfig = {
      # Simple OVS Bridge
      net = {
        interfaces = [{
          name = "ovsbr0";
          type = "ovs-bridge";
          ports = [];
          ipv4 = {
            enabled = true;
            dhcp = false;
            address = [ "10.0.0.1/24" ];
            gateway = null;
          };
        }];
      };

      # Container: Xray only
      lxc = {
        containers = [
          {
            id = "102";
            veth = "veth102";
            bridge = "ovsbr0";
            running = true;
            properties = {
              name = "xray";
              ipv4_address = "10.0.0.102/24";
            };
          }
        ];
      };

      # Packages
      packagekit = {
        packages = {
          "lxc" = { ensure = "installed"; };
          "bridge-utils" = { ensure = "installed"; };
          "curl" = { ensure = "installed"; };
          "htop" = { ensure = "installed"; };
        };
      };
    };
  };

  # SSH
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "prohibit-password";
      PasswordAuthentication = false;
    };
  };

  # Add your SSH key
  users.users.root.openssh.authorizedKeys.keys = [
    "ssh-rsa AAAAB3... your-key-here"  # Replace with your actual key
  ];

  # Essential packages
  environment.systemPackages = with pkgs; [
    vim
    git
    htop
    tmux
    curl
    wget
  ];

  system.stateVersion = "25.05";
}
