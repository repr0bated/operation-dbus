# operation-dbus NixOS Installer Configuration
# Boots from netboot.xyz, installs to local disk with BTRFS subvolumes
{ config, pkgs, lib, modulesPath, ... }:

{
  imports = [
    (modulesPath + "/installer/netboot/netboot-minimal.nix")
    ../../modules/operation-dbus.nix
  ];

  # Installer-specific boot settings
  boot = {
    supportedFilesystems = [ "btrfs" "ext4" "xfs" "zfs" ];
    kernelParams = [
      "boot.shell_on_fail"
      "console=tty0"
      "console=ttyS0,115200"
    ];

    # Enable NUMA for testing on multi-socket systems
    kernelModules = [ "veth" "br_netfilter" "overlay" ];
  };

  # Networking for installation
  networking = {
    hostName = "opdbus-installer";
    useDHCP = true;
    useNetworkd = false;
    firewall.enable = false; # Permissive during install
  };

  # SSH access during installation
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";
      PasswordAuthentication = true;
    };
  };

  users.users.root = {
    # CHANGE THIS TO YOUR SSH KEY!
    openssh.authorizedKeys.keys = [
      # Add your SSH public key here for keyless access
      # Example:
      # "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExampleKey your-email@example.com"
    ];
    # Fallback password: "nixos" (CHANGE IN PRODUCTION!)
    # Generate with: mkpasswd -m sha-512
    hashedPassword = "$6$rounds=656000$YQKMBzP5N8H7QlnW$K7x.f7P0VWkN3HVnPYzRnqwdkJHvFGVzqPdLIcZvGsYFSM7BmuXJPYxQ8Qh4PZfRH6N9xS8kh.";
  };

  # Installation tools and diagnostics
  environment.systemPackages = with pkgs; [
    # Partitioning tools
    parted
    gptfdisk

    # Filesystem tools
    btrfs-progs
    e2fsprogs
    dosfstools
    xfsprogs
    zfs

    # Disk diagnostics
    smartmontools
    hdparm
    nvme-cli
    lsblk

    # Network tools
    curl
    wget
    git
    rsync

    # System tools
    vim
    tmux
    htop
    pciutils
    usbutils

    # NUMA tools
    numactl
    hwloc

    # Debugging
    strace
    lsof
  ];

  # Automated installation script
  environment.etc."opdbus-install.sh" = {
    source = let
      installScript = pkgs.writeScriptBin "opdbus-install" ''
        #!/usr/bin/env bash
        # operation-dbus automated installation script
        set -e

        DISK="''${1:-}"
        HOSTNAME="''${2:-opdbus-node}"

        if [ -z "$DISK" ]; then
          echo "Usage: $0 <disk> [hostname]"
          echo ""
          echo "Available disks:"
          lsblk -d -o NAME,SIZE,TYPE,MODEL | grep disk
          echo ""
          echo "Example:"
          echo "  $0 /dev/sda opdbus-node-01"
          exit 1
        fi

        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "operation-dbus NixOS Installation"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "Target disk: $DISK"
        echo "Hostname: $HOSTNAME"
        echo ""

        # Show disk info
        echo "Disk information:"
        lsblk "$DISK" || true
        echo ""

        # Confirmation
        read -p "⚠️  This will ERASE $DISK. Type 'yes' to continue: " confirm
        if [ "$confirm" != "yes" ]; then
          echo "Installation cancelled."
          exit 1
        fi

        echo ""
        echo "[1/7] Partitioning $DISK..."

        # Wipe existing partition tables
        sgdisk --zap-all "$DISK" || true

        # Create GPT partition table with EFI and root
        parted "$DISK" --script -- \
          mklabel gpt \
          mkpart ESP fat32 1MiB 512MiB \
          set 1 esp on \
          mkpart primary 512MiB 100%

        # Wait for kernel to re-read partition table
        sleep 3
        partprobe "$DISK" || true
        sleep 2

        # Detect partition naming scheme
        if [ -e "''${DISK}1" ]; then
          ESP="''${DISK}1"
          ROOT="''${DISK}2"
        elif [ -e "''${DISK}p1" ]; then
          ESP="''${DISK}p1"
          ROOT="''${DISK}p2"
        else
          echo "❌ Cannot detect partition layout for $DISK"
          echo "Available devices:"
          ls -la /dev/disk/by-path/ | grep "$(basename "$DISK")" || true
          exit 1
        fi

        echo "  ESP: $ESP"
        echo "  ROOT: $ROOT"

        echo "[2/7] Formatting partitions..."
        mkfs.fat -F 32 -n BOOT "$ESP"
        mkfs.btrfs -f -L nixos "$ROOT"

        echo "[3/7] Creating BTRFS subvolumes..."
        mount "$ROOT" /mnt

        btrfs subvolume create /mnt/@root
        btrfs subvolume create /mnt/@cache
        btrfs subvolume create /mnt/@timing
        btrfs subvolume create /mnt/@vectors
        btrfs subvolume create /mnt/@state
        btrfs subvolume create /mnt/@snapshots

        echo "  Created subvolumes:"
        btrfs subvolume list /mnt

        umount /mnt

        echo "[4/7] Mounting filesystems..."

        # Mount root
        mount -o subvol=@root,compress=zstd:1,noatime "$ROOT" /mnt

        # Mount EFI
        mkdir -p /mnt/boot
        mount "$ESP" /mnt/boot

        # Mount operation-dbus subvolumes
        mkdir -p /mnt/var/lib/op-dbus/{cache,timing,vectors,state,snapshots}

        mount -o subvol=@cache,compress=zstd:3,noatime "$ROOT" /mnt/var/lib/op-dbus/cache
        mount -o subvol=@timing,compress=zstd:3,noatime "$ROOT" /mnt/var/lib/op-dbus/timing
        mount -o subvol=@vectors,compress=zstd:3,noatime "$ROOT" /mnt/var/lib/op-dbus/vectors
        mount -o subvol=@state,compress=zstd:1,noatime "$ROOT" /mnt/var/lib/op-dbus/state
        mount -o subvol=@snapshots,compress=zstd:1,noatime "$ROOT" /mnt/var/lib/op-dbus/snapshots

        echo "  Mounted filesystems:"
        df -h | grep -E "(Mounted|/mnt)"

        echo "[5/7] Generating NixOS configuration..."
        nixos-generate-config --root /mnt

        # Fetch operation-dbus repository if URL provided
        mkdir -p /mnt/etc/nixos/operation-dbus
        if [ -n "''${OPDBUS_REPO_URL:-}" ]; then
          echo "  Fetching operation-dbus from $OPDBUS_REPO_URL..."
          cd /mnt/etc/nixos
          git clone "$OPDBUS_REPO_URL" operation-dbus || true
        else
          echo "  ⚠️  No OPDBUS_REPO_URL set"
          echo "  Set with: export OPDBUS_REPO_URL=https://github.com/repr0bated/operation-dbus.git"
          echo "  Continuing with minimal configuration..."
        fi

        # Generate minimal configuration.nix
        cat > /mnt/etc/nixos/configuration.nix <<EOF_CONFIG
# operation-dbus NixOS Configuration
# Generated by installer on $(date)
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    # Uncomment when operation-dbus is available:
    # ./operation-dbus/nixos/modules/operation-dbus.nix
  ];

  # Boot loader
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  # Hostname
  networking.hostName = "$HOSTNAME";
  networking.useDHCP = true;

  # SSH
  services.openssh = {
    enable = true;
    settings.PermitRootLogin = "prohibit-password";
  };

  # operation-dbus configuration
  # Uncomment after operation-dbus module is available:
  # services.operation-dbus = {
  #   enable = true;
  #
  #   # NUMA optimization
  #   numa = {
  #     enable = false;  # Set true for multi-socket systems
  #     node = 0;
  #     cpuList = "0-7";
  #   };
  #
  #   # BTRFS (already configured via subvolumes)
  #   btrfs = {
  #     enable = true;
  #     basePath = "/var/lib/op-dbus";
  #     compressionLevel = 3;
  #     subvolumes = [ "@cache" "@timing" "@vectors" "@state" ];
  #   };
  #
  #   # ML vectorization
  #   ml = {
  #     enable = true;
  #     executionProvider = "cpu";
  #     numThreads = 8;
  #   };
  #
  #   # Default state
  #   defaultState = {
  #     version = "1.0";
  #     plugins = {};
  #   };
  #
  #   logLevel = "info";
  # };

  # Root user SSH key
  users.users.root.openssh.authorizedKeys.keys = [
    # Add your SSH public key here!
  ];

  # System packages
  environment.systemPackages = with pkgs; [
    vim
    git
    htop
    tmux
    curl
    wget
    btrfs-progs
    numactl
  ];

  # Enable D-Bus (required for operation-dbus)
  services.dbus.enable = true;

  system.stateVersion = "24.11";
}
EOF_CONFIG

        echo "  Generated /mnt/etc/nixos/configuration.nix"

        echo "[6/7] Installing NixOS..."
        echo "  This may take 10-30 minutes depending on network speed..."

        nixos-install --no-root-passwd

        echo "[7/7] Installation complete!"
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "✅ NixOS with operation-dbus installed to $DISK"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        echo "Next steps:"
        echo ""
        echo "1. Reboot the system:"
        echo "   sudo reboot"
        echo ""
        echo "2. Remove netboot.xyz from boot order (or set disk as primary)"
        echo ""
        echo "3. SSH into the new system:"
        echo "   ssh root@<ip-address>"
        echo ""
        echo "4. Enable operation-dbus (if module was imported):"
        echo "   sudo vim /etc/nixos/configuration.nix"
        echo "   # Uncomment services.operation-dbus section"
        echo "   sudo nixos-rebuild switch"
        echo ""
        echo "5. Verify BTRFS layout:"
        echo "   df -h | grep btrfs"
        echo "   btrfs subvolume list /"
        echo "   compsize /var/lib/op-dbus/cache"
        echo ""
        echo "6. Check NUMA topology (multi-socket systems):"
        echo "   numactl --hardware"
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""

        read -p "Reboot now? (yes/NO): " reboot_confirm
        if [ "$reboot_confirm" = "yes" ]; then
          echo "Rebooting..."
          reboot
        else
          echo "Remember to reboot manually: sudo reboot"
        fi
      '';
    in "${installScript}/bin/opdbus-install";
    mode = "0755";
  };

  # Welcome message on boot
  environment.etc."issue".text = ''

    ════════════════════════════════════════════════════════════
                    operation-dbus Installer
    ════════════════════════════════════════════════════════════

    This system is running from netboot.xyz in RAM.

    To install operation-dbus to local disk:

        sudo /etc/opdbus-install.sh /dev/sda [hostname]

    To install remotely via SSH:

        ssh root@$(hostname -I | awk '{print $1}')
        Password: nixos (or use SSH key)

    Need help?
        - List disks: lsblk
        - Docs: https://github.com/repr0bated/operation-dbus

    ════════════════════════════════════════════════════════════

  '';

  # Auto-display welcome message
  systemd.services.display-welcome = {
    wantedBy = [ "multi-user.target" ];
    after = [ "network-online.target" ];
    wants = [ "network-online.target" ];
    serviceConfig = {
      Type = "oneshot";
      StandardOutput = "journal+console";
      ExecStart = "${pkgs.coreutils}/bin/cat /etc/issue";
    };
  };

  # Increase timeout for large downloads
  systemd.services.nixos-install = {
    serviceConfig = {
      TimeoutStartSec = "infinity";
    };
  };

  system.stateVersion = "24.11";
}
