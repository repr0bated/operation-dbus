#!/bin/sh
set -eu

ROOT="${ROOT:-/}"
SCRIPT_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH='' cd -- "$SCRIPT_DIR/../.." && pwd)"

echo "Installing dinit op-dbus service files..."

install -d "$ROOT/etc/dinit.d" "$ROOT/etc/dinit.d/boot.d" "$ROOT/etc/op-dbus" "$ROOT/usr/local/bin"
install -m 0644 "$SCRIPT_DIR/op-dbus" "$ROOT/etc/dinit.d/op-dbus"
install -m 0755 "$SCRIPT_DIR/op-dbus-dinit.sh" "$ROOT/usr/local/bin/op-dbus-dinit.sh"
install -m 0755 "$SCRIPT_DIR/op-mcp-proxy-select3" "$ROOT/usr/local/bin/op-mcp-proxy-select3"

if [ ! -f "$ROOT/etc/op-dbus/environment" ]; then
  install -m 0644 "$SCRIPT_DIR/environment.op-dbus.template" "$ROOT/etc/op-dbus/environment"
  echo "Wrote new environment template to $ROOT/etc/op-dbus/environment"
else
  echo "Keeping existing $ROOT/etc/op-dbus/environment"
fi

ln -sfn ../op-dbus "$ROOT/etc/dinit.d/boot.d/op-dbus"

if command -v dinitctl >/dev/null 2>&1 && [ "$ROOT" = "/" ]; then
  dinitctl restart op-dbus || dinitctl start op-dbus || true
fi

echo "Done."
echo "If needed, copy your op-dbus binaries:"
echo "  install -m 0755 \"$REPO_ROOT/target/release/op-dbus\" /usr/local/bin/op-dbus"
echo "  install -m 0755 \"$REPO_ROOT/target/release/op-mcp-proxy\" /usr/local/bin/op-mcp-proxy"
