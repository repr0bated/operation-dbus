# START HERE - Fresh NixOS Installation

**You just wiped your drive and installed NixOS. Here's what to do RIGHT NOW.**

## Step 1: Get Git (NixOS doesn't have it by default)

```bash
# Install git temporarily (doesn't persist after reboot on live system)
nix-shell -p git

# You now have git available in this shell
```

## Step 2: Get the Latest Code

```bash
# Go to the operation-dbus folder you copied
cd /path/to/operation-dbus

# Pull the latest changes (this gets all the files I created)
git pull origin claude/this-is-no-011CUr6iUZZC7YKhhLSY2MK6

# Verify you have the files
ls -la nixos/netboot/
# Should show: build-installer.sh, build-for-netboot-xyz.sh, configs/, etc.
```

## Step 3: You Have Two Options

### Option A: Install NixOS to Disk (Recommended)

If you haven't installed NixOS to disk yet (just booted from USB):

```bash
# Read this guide
cat NETBOOT-QUICKSTART.md

# Or if you have local disk and want to install now:
# 1. Check your disk name
lsblk

# 2. Partition and format (BE CAREFUL - this erases the disk!)
sudo parted /dev/sda --script -- \
  mklabel gpt \
  mkpart ESP fat32 1MiB 512MiB \
  set 1 esp on \
  mkpart primary 512MiB 100%

sudo mkfs.fat -F 32 -n BOOT /dev/sda1
sudo mkfs.btrfs -f -L nixos /dev/sda2

# 3. Create BTRFS subvolumes
sudo mount /dev/sda2 /mnt
sudo btrfs subvolume create /mnt/@root
sudo btrfs subvolume create /mnt/@cache
sudo btrfs subvolume create /mnt/@timing
sudo btrfs subvolume create /mnt/@vectors
sudo btrfs subvolume create /mnt/@state
sudo umount /mnt

# 4. Mount everything
sudo mount -o subvol=@root,compress=zstd:1,noatime /dev/sda2 /mnt
sudo mkdir -p /mnt/boot
sudo mount /dev/sda1 /mnt/boot
sudo mkdir -p /mnt/var/lib/op-dbus/{cache,timing,vectors,state}
sudo mount -o subvol=@cache,compress=zstd:3,noatime /dev/sda2 /mnt/var/lib/op-dbus/cache
sudo mount -o subvol=@timing,compress=zstd:3,noatime /dev/sda2 /mnt/var/lib/op-dbus/timing
sudo mount -o subvol=@vectors,compress=zstd:3,noatime /dev/sda2 /mnt/var/lib/op-dbus/vectors
sudo mount -o subvol=@state,compress=zstd:1,noatime /dev/sda2 /mnt/var/lib/op-dbus/state

# 5. Generate config
sudo nixos-generate-config --root /mnt

# 6. Copy operation-dbus to the new system
sudo mkdir -p /mnt/etc/nixos/operation-dbus
sudo cp -r ./* /mnt/etc/nixos/operation-dbus/

# 7. Create configuration
sudo nano /mnt/etc/nixos/configuration.nix
```

Paste this configuration:

```nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./operation-dbus/nixos/modules/operation-dbus.nix
  ];

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  networking.hostName = "opdbus-laptop";
  networking.networkmanager.enable = true;

  time.timeZone = "America/New_York";  # Change to your timezone

  users.users.yourname = {  # CHANGE THIS
    isNormalUser = true;
    extraGroups = [ "wheel" "networkmanager" ];
    # Set password with: passwd yourname
  };

  services.operation-dbus = {
    enable = true;
    numa.enable = false;  # Laptop usually single-socket
    btrfs.enable = true;
    ml = {
      enable = true;
      executionProvider = "cpu";
      numThreads = 4;
    };
    defaultState = {
      version = "1.0";
      plugins = {};
    };
  };

  services.openssh.enable = true;

  environment.systemPackages = with pkgs; [
    vim
    git
    firefox
    htop
  ];

  system.stateVersion = "24.11";
}
```

Then install:

```bash
# Install NixOS
sudo nixos-install

# Set root password when prompted
# Reboot
sudo reboot
```

### Option B: Just Build the Project (Testing)

If you just want to build and test operation-dbus:

```bash
# Install Rust and build tools
nix-shell -p rustc cargo gcc pkg-config openssl dbus systemd

# Build the project
cargo build --release

# The binary is at: target/release/op-dbus
```

## Common NixOS Commands You Need

```bash
# Install packages temporarily (gone after reboot on live system)
nix-shell -p git vim htop

# Install packages permanently (only works if NixOS is installed to disk)
# Edit /etc/nixos/configuration.nix and add to environment.systemPackages
# Then run:
sudo nixos-rebuild switch

# Check what's installed
nix-env -q

# Search for packages
nix-env -qaP | grep packagename
```

## What Files Do You Have Now?

After `git pull`, you should have:

```
operation-dbus/
├── NETBOOT-QUICKSTART.md           ← Quick reference
├── NETBOOT-TO-DISK-INSTALL.md      ← Detailed netboot guide
├── NIXOS-SETUP-GUIDE.md             ← NixOS module guide
├── START-HERE.md                    ← THIS FILE
│
├── nixos/
│   ├── modules/
│   │   └── operation-dbus.nix       ← NixOS module (import this)
│   ├── netboot/
│   │   ├── build-installer.sh       ← Build installer image
│   │   ├── build-for-netboot-xyz.sh ← Build netboot images
│   │   └── configs/
│   │       ├── installer.nix        ← Installer config
│   │       ├── proxmox-host.nix     ← Example config
│   │       └── workstation.nix      ← Example config
│   └── examples/
│       ├── proxmox-host.nix         ← Full example
│       └── workstation.nix          ← Minimal example
│
├── src/                             ← Rust source code
├── Cargo.toml                       ← Rust project file
└── README.md                        ← Project overview
```

## If You're Confused - Read This

**What is operation-dbus?**
It's a Rust program that manages infrastructure (LXC containers, VMs, etc.) via D-Bus.

**What is NixOS?**
A Linux distribution where the entire system is configured in one file.

**What should I do?**
1. If you want to USE operation-dbus: Install NixOS to disk (Option A)
2. If you want to DEVELOP operation-dbus: Just build it (Option B)

## Quick Validation

```bash
# Check you're on NixOS
nixos-version
# Should show: 24.11.something

# Check you have the files
ls nixos/netboot/build-installer.sh
# Should exist

# Check git status
git status
# Should show: On branch claude/this-is-no-011CUr6iUZZC7YKhhLSY2MK6
```

## I Just Want It Working!

**Fastest path to working system:**

```bash
# 1. Get git
nix-shell -p git

# 2. Pull latest
cd /path/to/operation-dbus
git pull origin claude/this-is-no-011CUr6iUZZC7YKhhLSY2MK6

# 3. Read the quickstart
cat NETBOOT-QUICKSTART.md | less

# 4. If you have NixOS installed, enable operation-dbus
sudo nano /etc/nixos/configuration.nix
# Add the imports and services.operation-dbus section shown above
sudo nixos-rebuild switch
```

## Still Stuck?

Tell me:
1. Are you on a live USB or installed NixOS?
2. What does `ls nixos/` show?
3. What does `git status` show?
4. What specific error are you getting?

I'll give you exact commands to fix it.
