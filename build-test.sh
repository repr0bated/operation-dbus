#!/bin/bash
# Quick build test script for op-dbus

set -e

echo "Building op-dbus..."
cd /git/op-dbus

# Try to build
cargo build --release 2>&1 | tee build.log

if [ $? -eq 0 ]; then
    echo "✓ Build successful!"
    echo "Binary: /git/op-dbus/target/release/op-dbus"
else
    echo "✗ Build failed. Check build.log for errors."
    exit 1
fi
