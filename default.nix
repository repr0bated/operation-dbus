# Traditional Nix expression for op-dbus
# For use without flakes: nix-build or nix-env -i -f default.nix

{ pkgs ? import <nixpkgs> { } }:

pkgs.rustPlatform.buildRustPackage rec {
  pname = "op-dbus";
  version = "0.1.0";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    dbus
    systemd
    openssl
  ];

  # Tests may require D-Bus and system access
  doCheck = false;

  meta = with pkgs.lib; {
    description = "Portable declarative system state management via native protocols";
    longDescription = ''
      op-dbus is a portable tool for declarative infrastructure management
      that automatically adapts to your system's capabilities.

      Features:
      - Dynamic plugin discovery (auto-detects OVS, Proxmox, D-Bus services)
      - Blockchain audit trail (SHA-256 cryptographic footprints)
      - Native protocols (OVSDB, Netlink, D-Bus)
      - Auto-generates plugins for any D-Bus service
      - Works on any Linux with systemd

      Plugins are automatically discovered at runtime:
      - systemd: Always available
      - net: Requires OpenVSwitch
      - lxc: Requires Proxmox VE
      - Auto-generated: Any D-Bus service
    '';
    homepage = "https://github.com/repr0bated/operation-dbus";
    license = licenses.mit;
    maintainers = [ ];
    platforms = platforms.linux;
  };
}
