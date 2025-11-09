# Nix package derivation for op-dbus
{ lib
, rustPlatform
, pkg-config
, openssl
, openvswitch
, systemd
, dbus
, claude-cli
, makeWrapper
}:

rustPlatform.buildRustPackage rec {
  pname = "op-dbus";
  version = "0.1.0";

  src = ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    makeWrapper
  ];

  buildInputs = [
    openssl
    openvswitch
    systemd
    dbus
  ];

  propagatedBuildInputs = [
    claude-cli
  ];

  # Build with default features (web UI)
  # To build with MCP: buildFeatures = [ "mcp" ];
  # To build with ML: buildFeatures = [ "ml" ];
  # To build all features: buildFeatures = [ "mcp" "ml" "web" ];
  buildFeatures = [ "web" ];

  # Tests require system access (OVS, D-Bus)
  doCheck = false;

  postInstall = ''
    # Install nix folder for module and package definitions
    mkdir -p $out/share/op-dbus
    cp -r ${../nix} $out/share/op-dbus/nix
  '';

  meta = with lib; {
    description = "Declarative system state management via native protocols";
    homepage = "https://github.com/ghostbridge/op-dbus";
    license = licenses.mit;
    maintainers = with maintainers; [ ];
    platforms = platforms.linux;
  };
}
