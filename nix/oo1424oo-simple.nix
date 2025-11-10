# oo1424oo Simple Socket Network Configuration
# ONE mesh bridge with privacy tunnel containers
# Connects to VPS via Netmaker for NAT traversal

{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./module.nix
  ];

  # Boot
  boot.loader.grub = {
    enable = true;
    device = "/dev/sda";
  };

  # Network
  networking = {
    hostName = "oo1424oo";
    firewall = {
      enable = true;
      allowedUDPPorts = [ 51820 51821 ];  # Gateway VPN, Netmaker
      allowedTCPPorts = [ 22 8006 ];  # SSH, Proxmox
      trustedInterfaces = [ "mesh" "nm-privacy" ];
    };
  };

  # Proxmox/LXC support
  virtualisation.lxc = {
    enable = true;
    lxcfs.enable = true;
  };

  # op-dbus with simple socket network
  services.op-dbus = {
    enable = true;
    mode = "full";

    stateConfig = {
      # Netmaker client - connects to VPS
      netmaker = {
        mode = "client";
        network = "privacy-mesh";
        interface = "nm-privacy";
        bridge = "mesh";  # ONE bridge (socket network)
        server = "https://vps-public-ip:8081";  # Replace with actual VPS IP
      };

      # WARP tunnel on host (privacy exit)
      wireguard = {
        warp_tunnel = {
          interface = "warp0";
          enabled = true;
          # PostUp adds warp0 to socket network
          post_up = "ovs-vsctl add-port mesh warp0";
          pre_down = "ovs-vsctl del-port mesh warp0";
        };

        # VPN server for authenticated clients
        vpn_server = {
          interface = "wg0";
          listen_port = 51820;
          ip_pool = "10.8.0.0/24";
          auto_provision = true;
          peers = [];
        };
      };

      # OpenFlow rules for socket network
      openflow = {
        bridges = {
          mesh = {
            flows = [
              # Privacy traffic from netmaker → Xray client
              "priority=100,in_port=nm-privacy,actions=output:veth102"

              # Xray client → WARP exit
              "priority=100,in_port=veth102,actions=output:warp0"

              # WARP return → Xray client
              "priority=100,in_port=warp0,actions=output:veth102"

              # Gateway → WARP (if VPN traffic needs routing)
              "priority=90,in_port=veth100,actions=output:warp0"

              # Response back to netmaker
              "priority=80,in_port=veth102,actions=output:nm-privacy"
              "priority=80,in_port=warp0,actions=output:nm-privacy"

              # Default: normal switching for socket network
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Privacy tunnel containers on mesh socket network
      lxc = {
        containers = [
          # Gateway - WireGuard VPN server
          {
            id = "100";
            veth = "veth100";
            bridge = "mesh";
            running = true;
            properties = {
              name = "gateway";
              network_type = "veth";
              ipv4_address = "10.0.0.100/24";
              gateway = "10.0.0.1";
              template = "ubuntu-22.04";
              startup = "order=1";
              memory = 512;
              swap = 512;
              features = {
                nesting = true;
              };
              wireguard = {
                enabled = true;
                interface = "wg0";
                listen_port = 51820;
                ip_pool = "10.8.0.0/24";
                dns = "1.1.1.1";
                auto_provision = true;
              };
            };
          }

          # Xray client - Privacy tunnel processor
          {
            id = "102";
            veth = "veth102";
            bridge = "mesh";
            running = true;
            properties = {
              name = "xray-client";
              network_type = "veth";
              ipv4_address = "10.0.0.102/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
              startup = "order=2";
              memory = 512;
              swap = 256;
              services = [
                {
                  name = "xray-client";
                  protocol = "tcp";
                  port = 8080;
                  description = "Receives traffic from VPS, exits via WARP";
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
          "wg-quick@wg0.service" = {
            enabled = true;
            active_state = "active";
          };
          "wg-quick@warp0.service" = {
            enabled = true;
            active_state = "active";
          };
          "netclient.service" = {
            enabled = true;
            active_state = "active";
          };
        };
      };

      # Required packages
      packagekit = {
        packages = {
          "lxc" = { ensure = "installed"; };
          "wireguard-tools" = { ensure = "installed"; };
          "netclient" = { ensure = "installed"; };
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
    tmux
    curl
    wget
    wireguard-tools
  ];

  system.stateVersion = "25.05";
}
