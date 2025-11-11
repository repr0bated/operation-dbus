# VPS with OVS Bridge - CORRECT NixOS METHOD
{ config, pkgs, ... }:

{
  imports = [ ./hardware-configuration.nix ];

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  # === Enable OpenVSwitch ===
  virtualisation.vswitch.enable = true;

  # === Manual OVS Bridge Configuration ===
  # networking.vswitches has bugs, so we create bridge manually via systemd

  systemd.services.ovs-setup = {
    description = "Create OVS Bridge ovsbr0";
    after = [ "ovsdb.service" "ovs-vswitchd.service" ];
    requires = [ "ovsdb.service" "ovs-vswitchd.service" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
    };
    script = ''
      # Wait for OVS to be ready
      ${pkgs.coreutils}/bin/sleep 2

      # Create bridge if it doesn't exist
      ${pkgs.openvswitch}/bin/ovs-vsctl --may-exist add-br ovsbr0

      # Disable STP
      ${pkgs.openvswitch}/bin/ovs-vsctl set Bridge ovsbr0 stp_enable=false

      # Add physical interface as port
      ${pkgs.openvswitch}/bin/ovs-vsctl --may-exist add-port ovsbr0 ens1

      # Set bridge to UP
      ${pkgs.iproute2}/bin/ip link set ovsbr0 up
      ${pkgs.iproute2}/bin/ip link set ens1 up

      echo "OVS bridge ovsbr0 configured"
    '';
    preStop = ''
      ${pkgs.openvswitch}/bin/ovs-vsctl --if-exists del-port ovsbr0 ens1 || true
      ${pkgs.openvswitch}/bin/ovs-vsctl --if-exists del-br ovsbr0 || true
    '';
  };

  # === Network Configuration ===
  networking = {
    hostName = "vps-ovs";
    useDHCP = false;

    # Physical interface - no IP, managed by OVS
    interfaces.ens1 = {
      useDHCP = false;
      ipv4.addresses = [ ];
    };

    # OVS bridge interface - this is where we put the IP
    interfaces.ovsbr0 = {
      ipv4.addresses = [{
        address = "80.209.240.244";
        prefixLength = 25;
      }];
      useDHCP = false;
    };

    defaultGateway = "80.209.230.129";
    nameservers = [ "8.8.8.8" "8.8.4.4" ];

    firewall = {
      enable = true;
      allowedTCPPorts = [ 22 80 443 ];
    };
  };

  # === SSH ===
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";
      PasswordAuthentication = true;
    };
  };

  # === Development Tools ===
  environment.systemPackages = with pkgs; [
    vim
    git
    htop
    curl
    wget
    tmux
    nodejs_20
    gcc
    gnumake
    pkg-config
    python3
    jq
    ripgrep
    openvswitch
    iproute2
  ];

  # === User ===
  users.users.root.initialPassword = "O52131o44";

  system.stateVersion = "25.05";
}
