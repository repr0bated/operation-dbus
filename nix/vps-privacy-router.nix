# VPS Privacy Router Configuration
# Lightweight: Xray ingress + WARP exit + Netmaker server
# Solves NAT problem - public IP for direct ingress

{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./module.nix
  ];

  # Boot
  boot.loader.grub = {
    enable = true;
    device = "/dev/vda";  # DigitalOcean virtual disk
  };

  # Network
  networking = {
    hostName = "vps-privacy-router";
    firewall = {
      enable = true;
      allowedTCPPorts = [
        22        # SSH
        443       # Xray VLESS
        8443      # Xray VMess
        8081      # Netmaker API
      ];
      allowedUDPPorts = [
        51821     # Netmaker WireGuard
      ];
      trustedInterfaces = [ "socket" "nm-server" "warp0" ];
    };
  };

  # op-dbus privacy router
  services.op-dbus = {
    enable = true;
    mode = "standalone";

    stateConfig = {
      # Netmaker server - provides L3 routing to oo1424oo
      netmaker = {
        mode = "server";
        network = "privacy-mesh";
        interface = "nm-server";
        bridge = "socket";  # ONE bridge
        listen_port = 51821;
        api_endpoint = "https://<vps-public-ip>:8081";  # Replace with actual IP
      };

      # WARP tunnel on VPS (privacy exit)
      wireguard = {
        warp_tunnel = {
          interface = "warp0";
          enabled = true;
          # PostUp adds warp0 to socket network
          post_up = "ovs-vsctl add-port socket warp0";
          pre_down = "ovs-vsctl del-port socket warp0";
        };
      };

      # OpenFlow rules for privacy router
      openflow = {
        bridges = {
          socket = {
            flows = [
              # Ingress to Xray server
              "priority=100,tcp,tp_dst=443,actions=output:veth102"
              "priority=100,tcp,tp_dst=8443,actions=output:veth102"

              # Xray → WARP exit
              "priority=100,in_port=veth102,actions=output:warp0"

              # WARP return → Xray
              "priority=100,in_port=warp0,actions=output:veth102"

              # Netmaker traffic (to/from oo1424oo)
              "priority=80,in_port=nm-server,actions=normal"

              # Default: normal switching
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Privacy router container (lightweight)
      lxc = {
        containers = [
          {
            id = "102";
            veth = "veth102";
            bridge = "socket";
            running = true;
            properties = {
              name = "xray-server";
              network_type = "veth";
              ipv4_address = "10.2.0.102/24";
              gateway = "10.2.0.1";
              template = "alpine-3.19";
              memory = 512;
              swap = 256;
              services = [
                {
                  name = "xray-vless";
                  protocol = "tcp";
                  port = 443;
                  exposed = true;
                  description = "Privacy tunnel ingress, exits via WARP";
                }
                {
                  name = "xray-vmess";
                  protocol = "tcp";
                  port = 8443;
                  exposed = true;
                }
              ];
            };
          }
        ];
      };

      # System services
      systemd = {
        units = {
          "openvswitch.service" = {
            enabled = true;
            active_state = "active";
          };
          "wg-quick@warp0.service" = {
            enabled = true;
            active_state = "active";
          };
          "netmaker.service" = {
            enabled = true;
            active_state = "active";
          };
        };
      };

      # Required packages
      packagekit = {
        packages = {
          "wireguard-tools" = { ensure = "installed"; };
          "netmaker" = { ensure = "installed"; };
          "curl" = { ensure = "installed"; };
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

  # Essential packages
  environment.systemPackages = with pkgs; [
    vim
    git
    htop
    curl
    wget
    wireguard-tools
    netmaker
  ];

  system.stateVersion = "25.05";
}
