# Template for creating operation-dbus plugin packages in Nix
#
# Copy this file and customize for your plugin:
#   cp template.nix my-plugin.nix
#   Edit my-plugin.nix with your plugin details
#
# Then reference in configuration.nix:
#   services.operation-dbus.plugins = [
#     (pkgs.callPackage ./nixos/plugins/my-plugin.nix {})
#   ];

{ stdenv
, lib
, fetchFromGitHub
, writeText
}:

stdenv.mkDerivation rec {
  pname = "operation-dbus-plugin-example";
  version = "1.0.0";

  # Source options:

  # Option 1: Fetch from GitHub
  src = fetchFromGitHub {
    owner = "your-username";
    repo = "operation-dbus-plugins";
    rev = "v${version}";
    sha256 = lib.fakeSha256; # Replace with actual hash after first build
  };

  # Option 2: Local directory
  # src = /path/to/your/plugin/directory;

  # Option 3: Inline plugin.toml
  # src = ./.;

  # Build phases
  dontBuild = true; # Plugins are configuration files, not binaries

  installPhase = ''
    mkdir -p $out

    # Install plugin metadata
    cp plugin.toml $out/

    # For auto-generated plugins: install semantic mapping
    ${lib.optionalString (builtins.pathExists ./semantic-mapping.toml) ''
      cp semantic-mapping.toml $out/
    ''}

    # For auto-generated plugins: install D-Bus introspection
    ${lib.optionalString (builtins.pathExists ./introspection.xml) ''
      cp introspection.xml $out/
    ''}

    # Install example configurations
    ${lib.optionalString (builtins.pathExists ./examples) ''
      cp -r examples $out/
    ''}

    # Install README
    ${lib.optionalString (builtins.pathExists ./README.md) ''
      cp README.md $out/
    ''}
  '';

  # Metadata
  meta = with lib; {
    description = "Example plugin for operation-dbus";
    homepage = "https://github.com/your-username/operation-dbus-plugins";
    license = licenses.mit;
    maintainers = [ "Your Name <your.email@example.com>" ];
    platforms = platforms.linux;
  };
}
