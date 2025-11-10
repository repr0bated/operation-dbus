{ config, pkgs, ... }:

{
  imports = [
    <nixpkgs/nixos/modules/installer/netboot/netboot-minimal.nix>
    ./op-dbus-modules/module.nix
    ./opdbus-configuration.nix
  ];
}
