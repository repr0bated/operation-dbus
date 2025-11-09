# Ready-to-deploy NixOS configuration for oo1424oo
# Full privacy router: Gateway + WARP + Xray containers
# Socket networking between containers via OpenFlow

{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./module.nix
  ];

  # Boot
  boot.loader.grub = {
    enable = true;
    device = "/dev/sda";  # Adjust based on your disk
  };

  # Network - Preserve existing configuration
  networking = {
    hostName = "oo1424oo";
    # Let existing network config handle physical interfaces
    # op-dbus will manage OVS bridges

    firewall = {
      enable = true;
      allowedTCPPorts = [ 22 8006 9573 9574 443 8443 ];  # SSH, Proxmox, MCP, Web, Proxies
      trustedInterfaces = [ "ovsbr0" "mesh" ];
    };
  };

  # Proxmox/LXC support
  virtualisation = {
    lxc = {
      enable = true;
      lxcfs.enable = true;
    };
  };

  # op-dbus with FULL privacy router
  services.op-dbus = {
    enable = true;
    mode = "full";  # Full Proxmox mode

    stateConfig = {
      # OVS Bridges
      net = {
        interfaces = [
          {
            name = "ovsbr0";
            type = "ovs-bridge";
            ports = [];  # Physical ports managed separately
            ipv4 = {
              enabled = true;
              dhcp = false;
              address = [ "10.0.0.1/24" ];
              gateway = null;
            };
          }
          {
            name = "mesh";
            type = "ovs-bridge";
            ports = [];
            ipv4 = {
              enabled = true;
              dhcp = false;
              address = [ "10.1.0.1/24" ];
              gateway = null;
            };
          }
        ];
      };

      # OpenFlow rules - Socket networking between containers
      openflow = {
        bridges = {
          ovsbr0 = {
            flows = [
              # WARP → Gateway (outbound traffic)
              "priority=100,in_port=veth101,actions=output:veth100"

              # Gateway → WARP (return traffic from internet)
              "priority=100,in_port=veth100,tcp,tp_dst=51820,actions=output:veth101"

              # Xray → WARP (proxy traffic routes through WARP)
              "priority=90,in_port=veth102,actions=output:veth101"

              # WARP → Xray (return traffic to proxy)
              "priority=90,in_port=veth101,tcp,tp_src=443,actions=output:veth102"
              "priority=90,in_port=veth101,tcp,tp_src=8443,actions=output:veth102"

              # Default: route to gateway
              "priority=10,actions=output:veth100"
            ];
          };
        };
      };

      # Privacy Router Containers
      lxc = {
        containers = [
          # Gateway container - NAT, routing, firewall
          {
            id = "100";
            veth = "veth100";
            bridge = "ovsbr0";
            running = true;
            properties = {
              name = "gateway";
              network_type = "veth";
              ipv4_address = "10.0.0.100/24";
              gateway = "10.0.0.1";
              template = "ubuntu-22.04";
              startup = "order=1";
              memory = 512;  # MB
              swap = 512;
              features = {
                nesting = true;
              };
            };
          }

          # WARP container - Cloudflare WARP via wgcf
          {
            id = "101";
            veth = "veth101";
            bridge = "ovsbr0";
            running = true;
            properties = {
              name = "warp";
              network_type = "veth";
              ipv4_address = "10.0.0.101/24";
              gateway = "10.0.0.100";
              template = "debian-12";
              startup = "order=2,up=30";  # Start after gateway, wait 30s
              memory = 1024;  # MB - WireGuard needs more memory
              swap = 512;
              features = {
                nesting = true;
              };
              # WARP-specific config
              wgcf = {
                enabled = true;
                interface = "wg0";
                ovs_attach = true;  # Attach wg0 to OVS bridge as port
              };
            };
          }

          # Xray container - V2Ray/Xray proxy server
          {
            id = "102";
            veth = "veth102";
            bridge = "ovsbr0";
            running = true;
            properties = {
              name = "xray";
              network_type = "veth";
              ipv4_address = "10.0.0.102/24";
              gateway = "10.0.0.101";  # Route through WARP
              template = "alpine-3.19";
              startup = "order=3,up=30";  # Start after WARP
              memory = 512;  # MB
              swap = 256;
              features = {
                nesting = false;
              };
              # Xray-specific config
              xray = {
                enabled = true;
                config_path = "/etc/xray/config.json";
                ports = {
                  vmess = 443;
                  vless = 8443;
                };
              };
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
        };
      };

      # Required packages
      packagekit = {
        packages = {
          "lxc" = { ensure = "installed"; };
          "lxc-templates" = { ensure = "installed"; };
          "bridge-utils" = { ensure = "installed"; };
          "iptables" = { ensure = "installed"; };
          "curl" = { ensure = "installed"; };
          "wget" = { ensure = "installed"; };
          "htop" = { ensure = "installed"; };
          "tmux" = { ensure = "installed"; };
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
    iotop
    ncdu
  ];

  system.stateVersion = "25.05";
}
