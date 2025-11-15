# Development shell for op-dbus
# Usage: nix-shell (or use flake.nix with 'nix develop')

{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust toolchain
    cargo
    rustc
    rustfmt
    rust-analyzer
    clippy

    # Build dependencies
    pkg-config
    dbus
    systemd
    openssl

    # Optional runtime dependencies
    openvswitch

    # Development tools
    jq           # JSON manipulation
    git          # Version control
    cargo-watch  # Auto-rebuild on changes

    # Debugging tools
    gdb
    valgrind
  ];

  shellHook = ''
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║  op-dbus Development Environment (Traditional Nix)            ║"
    echo "╚════════════════════════════════════════════════════════════════╝"
    echo ""
    echo "📦 Available tools:"
    echo "  • cargo build --release    - Build op-dbus"
    echo "  • cargo test               - Run tests"
    echo "  • cargo watch -x check     - Auto-check on file changes"
    echo "  • cargo run -- query       - Query system state"
    echo "  • cargo run -- doctor      - System diagnostics"
    echo ""
    echo "🔧 Development dependencies:"
    echo "  ✓ Rust $(rustc --version | cut -d' ' -f2)"
    echo "  ✓ D-Bus development libraries"
    echo "  ✓ systemd development libraries"
    echo "  ✓ OpenVSwitch (for net plugin testing)"
    echo ""
    echo "💡 Quick start:"
    echo "  1. cargo build --release"
    echo "  2. sudo ./target/release/op-dbus doctor"
    echo "  3. sudo ./target/release/op-dbus query"
    echo ""
    echo "📚 Documentation:"
    echo "  • README.md - Overview and quick start"
    echo "  • INSTALL.md - Installation guide"
    echo "  • docs/ - Additional documentation"
    echo ""
  '';

  # Environment variables
  RUST_BACKTRACE = "1";
  RUST_LOG = "op_dbus=debug";

  # Ensure pkg-config can find libraries
  PKG_CONFIG_PATH = "${pkgs.lib.makeLibraryPath [
    pkgs.dbus
    pkgs.systemd
    pkgs.openssl
  ]}";
}
