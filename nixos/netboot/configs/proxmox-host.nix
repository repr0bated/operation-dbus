# Netboot Configuration: Proxmox Host
# Multi-socket Xeon system with NUMA optimization, GPU acceleration
#
# This configuration is booted over the network and includes:
# - operation-dbus with NUMA optimization
# - BTRFS subvolume support
# - ML vectorization with CUDA
# - LXC container management
# - Netmaker mesh networking
#
# Build netboot image:
#   nix-build '<nixpkgs/nixos>' -A config.system.build.netbootRamdisk \
#     -I nixos-config=./proxmox-host.nix

{ config, pkgs, lib, modulesPath, ... }:

{
  imports = [
    (modulesPath + "/installer/netboot/netboot-minimal.nix")
    ../../modules/operation-dbus.nix
  ];

  # Netboot-specific settings
  boot = {
    supportedFilesystems = [ "btrfs" "zfs" ]; # Support for container storage

    # Kernel modules for networking and containers
    kernelModules = [
      "veth"           # Virtual ethernet for containers
      "bridge"         # Network bridging
      "overlay"        # OverlayFS for containers
      "nf_nat"         # NAT for container networking
      "ip_tables"      # Firewall
      "xt_TPROXY"      # Transparent proxy
    ];

    # Kernel parameters
    kernelParams = [
      "boot.shell_on_fail"  # Drop to shell on boot failure
      "numa_balancing=enable"
      "transparent_hugepage=madvise"
      "console=ttyS0,115200"  # Serial console for remote management
      "console=tty0"
    ];

    # Network boot loader
    loader.grub.enable = false;
  };

  # Networking configuration
  networking = {
    hostName = "proxmox-netboot";  # Will be overridden by DHCP
    useDHCP = true;

    # Enable IPv4 forwarding for containers
    firewall.enable = true;
    firewall.allowedTCPPorts = [ 22 8006 ]; # SSH, Proxmox web UI

    # Bridge for LXC containers (created after boot)
    interfaces.vmbr0 = {
      ipv4.addresses = [];  # Dynamic
    };
  };

  # operation-dbus configuration
  services.operation-dbus = {
    enable = true;

    # Fetch state from netboot server
    stateFile = "/etc/operation-dbus/state.json";

    # NUMA optimization for multi-socket Xeon
    numa = {
      enable = true;
      node = 0;
      cpuList = "0-7";  # First 8 cores on socket 0
    };

    # BTRFS configuration
    btrfs = {
      enable = true;
      compressionLevel = 3;
      snapshotRetention = 24;
    };

    # ML vectorization with GPU
    ml = {
      enable = true;
      executionProvider = "cuda";
      gpuDeviceId = 0;
    };

    # Plugins
    plugins = with pkgs; [
      # Would reference actual plugin packages
      # operation-dbus-plugin-lxc
      # operation-dbus-plugin-netmaker
    ];
  };

  # Enable SSH for remote management
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";  # For initial setup
      PasswordAuthentication = false;
    };
  };

  # Root SSH key (replace with your own!)
  users.users.root.openssh.authorizedKeys.keys = [
    "ssh-ed25519 AAAAC3... your-key-here"
  ];

  # Essential system packages
  environment.systemPackages = with pkgs; [
    # BTRFS tools
    btrfs-progs
    compsize

    # NUMA tools
    numactl
    hwloc

    # Container tools
    lxc
    lxd

    # Networking
    bridge-utils
    ethtool
    tcpdump

    # Monitoring
    htop
    iotop
    nethogs

    # Text editors
    vim
    nano
  ];

  # Enable ZFS if needed
  boot.zfs.enabled = false;  # Set to true if using ZFS

  # Systemd network-online target
  systemd.targets.network-online.wantedBy = [ "multi-user.target" ];

  # Preload kernel modules
  boot.kernelModules = [ "vhost_net" ];

  # System state version
  system.stateVersion = "24.11";
}
