#!/bin/bash
# Simple build wrapper

set -e

echo "Building op-dbus..."
cargo build --release

echo ""
echo "✓ Build complete!"
echo ""
echo "Binary: target/release/op-dbus"
echo "Size:   $(du -h target/release/op-dbus | cut -f1)"
echo ""
echo "Next steps:"
echo "  sudo ./install.sh     - Install system-wide"
echo "  sudo ./test-safe.sh   - Run safe tests"
echo ""
