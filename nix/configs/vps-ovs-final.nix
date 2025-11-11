{ config, pkgs, ... }:

{
  imports = [ ./hardware-configuration.nix ];

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  # Enable OpenVSwitch
  virtualisation.vswitch.enable = true;

  # Physical interface - no IP (managed by OVS)
  networking.interfaces.ens1.useDHCP = false;
  networking.interfaces.ens1.ipv4.addresses = [];

  # Create OVS bridge manually (networking.vswitches has bugs)
  systemd.services.ovs-bridge = {
    description = "OVS Bridge Setup";
    after = [ "ovsdb.service" "ovs-vswitchd.service" ];
    requires = [ "ovsdb.service" "ovs-vswitchd.service" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
    };
    script = ''
      ${pkgs.coreutils}/bin/sleep 2
      ${pkgs.openvswitch}/bin/ovs-vsctl --may-exist add-br ovsbr0
      ${pkgs.openvswitch}/bin/ovs-vsctl set Bridge ovsbr0 stp_enable=false
      ${pkgs.openvswitch}/bin/ovs-vsctl --may-exist add-port ovsbr0 ens1
      ${pkgs.iproute2}/bin/ip link set ovsbr0 up
      ${pkgs.iproute2}/bin/ip link set ens1 up
    '';
  };

  # Bridge interface IP
  networking.interfaces.ovsbr0 = {
    ipv4.addresses = [{
      address = "80.209.240.244";
      prefixLength = 25;
    }];
    useDHCP = false;
  };

  # Network settings
  networking.defaultGateway = "80.209.230.129";
  networking.nameservers = [ "8.8.8.8" "8.8.4.4" ];
  networking.useDHCP = false;
  networking.hostName = "vps";
  networking.firewall.enable = false;

  # SSH
  services.openssh.enable = true;
  services.openssh.settings.PermitRootLogin = "yes";
  services.openssh.settings.PasswordAuthentication = true;

  # Packages
  environment.systemPackages = with pkgs; [
    vim git curl wget nodejs_20 openvswitch iproute2
  ];

  users.users.root.initialPassword = "O52131o44";

  system.stateVersion = "25.05";
}
