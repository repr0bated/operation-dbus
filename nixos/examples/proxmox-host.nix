# Example NixOS configuration for a Proxmox host running operation-dbus
# This demonstrates the full feature set including NUMA, BTRFS, ML, and plugins
#
# Usage:
#   1. Copy this to your NixOS configuration directory
#   2. Add to /etc/nixos/configuration.nix:
#      imports = [ ./proxmox-host.nix ];
#   3. Run: sudo nixos-rebuild switch

{ config, pkgs, ... }:

{
  imports = [
    # Import the operation-dbus module from the flake
    # If using flakes, this is automatically available
    # If not using flakes, import directly:
    # (import /path/to/operation-dbus/nixos/modules/operation-dbus.nix)
  ];

  # operation-dbus configuration
  services.operation-dbus = {
    enable = true;

    # State file location
    stateFile = "/etc/operation-dbus/proxmox.json";

    # NUMA optimization for multi-socket Xeon systems
    numa = {
      enable = true;
      node = 0;          # Pin to first NUMA node
      cpuList = "0-7";   # Use first 8 cores (sharing L3 cache)
    };

    # BTRFS configuration
    btrfs = {
      enable = true;
      basePath = "/var/lib/op-dbus";
      compressionLevel = 3;  # Balanced compression for embeddings
      snapshotRetention = 24; # Keep last 24 hourly snapshots
      subvolumes = [
        "@cache"    # ML embedding cache
        "@timing"   # Blockchain timing data
        "@vectors"  # Blockchain vector embeddings
        "@state"    # Blockchain state snapshots
      ];
    };

    # ML vectorization with GPU acceleration
    ml = {
      enable = true;
      executionProvider = "cuda";  # Use NVIDIA GPU
      gpuDeviceId = 0;
      # For CPU-only: executionProvider = "cpu"; numThreads = 8;
    };

    # Plugins (will be defined below)
    plugins = with pkgs; [
      operation-dbus-plugin-lxc
      operation-dbus-plugin-netmaker
    ];

    # Declarative infrastructure state
    defaultState = {
      version = "1.0";

      plugins = {
        # LXC container management
        lxc = {
          containers = [
            {
              id = "100";
              hostname = "web-server-01";
              template = "debian-13-standard";
              golden_image = "debian-nginx";
              memory = 2048;
              cores = 2;
              rootfs_size = 20;
              network = {
                bridge = "vmbr0";
                ip = "10.0.0.100/24";
                gateway = "10.0.0.1";
              };
            }
            {
              id = "101";
              hostname = "db-server-01";
              template = "debian-13-standard";
              golden_image = "postgres-15";
              memory = 4096;
              cores = 4;
              rootfs_size = 50;
              network = {
                bridge = "vmbr0";
                ip = "10.0.0.101/24";
                gateway = "10.0.0.1";
              };
            }
            {
              id = "102";
              hostname = "redis-cache-01";
              template = "debian-13-standard";
              golden_image = "redis-7";
              memory = 1024;
              cores = 1;
              rootfs_size = 10;
              network = {
                bridge = "vmbr0";
                ip = "10.0.0.102/24";
                gateway = "10.0.0.1";
              };
            }
          ];
        };

        # Netmaker WireGuard mesh networking
        netmaker = {
          networks = [
            {
              name = "prod-mesh";
              endpoint = "https://api.netmaker.local";
              listen_port = 51820;
              mtu = 1420;
              post_up = "iptables -A FORWARD -i nm-prod-mesh -j ACCEPT";
              post_down = "iptables -D FORWARD -i nm-prod-mesh -j ACCEPT";
            }
          ];
        };
      };
    };

    # Logging level
    logLevel = "info";

    # Extra environment variables
    extraEnvironment = {
      # Custom settings
      OPDBUS_CACHE_WARMUP = "true";
      OPDBUS_BENCHMARK_MODE = "false";
    };
  };

  # Additional system configuration for Proxmox
  boot = {
    # Enable NUMA balancing kernel parameter
    kernelParams = [
      "numa_balancing=enable"
      "transparent_hugepage=madvise"
    ];

    # Load kernel modules for containers and networking
    kernelModules = [
      "veth"
      "ip_tables"
      "ip6_tables"
      "nf_nat"
      "overlay"
      "br_netfilter"
    ];
  };

  # Networking for Proxmox
  networking = {
    # Enable IPv4 forwarding for containers
    firewall.enable = true;
    firewall.allowedTCPPorts = [ 8006 ]; # Proxmox web UI
    nat.enable = true;

    # Bridge for LXC containers
    bridges = {
      vmbr0 = {
        interfaces = [ "eth0" ];
      };
    };
  };

  # Enable LXC/LXD if needed
  virtualisation.lxc.enable = true;
  virtualisation.lxd.enable = false; # Using Proxmox LXC instead

  # System packages
  environment.systemPackages = with pkgs; [
    # BTRFS tools
    btrfs-progs
    compsize  # Check compression ratio

    # NUMA tools
    numactl
    hwloc
    lstopo

    # Performance monitoring
    htop
    iotop
    nethogs

    # Debugging
    strace
    ltrace
    gdb
  ];

  # Systemd resource limits
  systemd.services.operation-dbus.serviceConfig = {
    # Memory limits
    MemoryMax = "8G";
    MemoryHigh = "6G";

    # CPU limits (allow full usage on assigned cores)
    CPUQuota = "800%"; # 8 cores * 100%

    # I/O priority
    IOSchedulingClass = "best-effort";
    IOSchedulingPriority = 2;
  };

  # Example plugin packages (would be defined separately)
  nixpkgs.config.packageOverrides = pkgs: {
    operation-dbus-plugin-lxc = pkgs.stdenv.mkDerivation {
      pname = "operation-dbus-plugin-lxc";
      version = "1.0.0";

      src = pkgs.fetchFromGitHub {
        owner = "repr0bated";
        repo = "operation-dbus";
        rev = "main"; # Update to specific tag
        sha256 = ""; # Add actual hash
      };

      installPhase = ''
        mkdir -p $out
        cp plugins/lxc/plugin.toml $out/
        cp -r plugins/lxc/examples $out/ || true
      '';

      meta = {
        description = "Proxmox LXC container management plugin";
        license = pkgs.lib.licenses.mit;
      };
    };

    operation-dbus-plugin-netmaker = pkgs.stdenv.mkDerivation {
      pname = "operation-dbus-plugin-netmaker";
      version = "1.0.0";

      src = pkgs.fetchFromGitHub {
        owner = "repr0bated";
        repo = "operation-dbus";
        rev = "main";
        sha256 = "";
      };

      installPhase = ''
        mkdir -p $out
        cp plugins/netmaker/plugin.toml $out/
        cp -r plugins/netmaker/examples $out/ || true
      '';

      meta = {
        description = "Netmaker WireGuard mesh network plugin";
        license = pkgs.lib.licenses.mit;
      };
    };
  };
}
