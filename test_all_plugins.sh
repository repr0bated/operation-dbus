#!/bin/bash
# test_all_plugins.sh - Quick test of all installed plugins

# Don't exit on errors - we want to test all plugins
set +e

BINARY="./target/release/op-dbus"

if [ ! -f "$BINARY" ]; then
    echo "❌ Binary not found: $BINARY"
    echo "Run: cargo build --release"
    exit 1
fi

echo "🧪 Testing all installed plugins..."
echo ""

# Get list of ChatGPT-generated plugins (built-in plugins use different query methods)
PLUGINS=("sess" "dnsresolver" "pcidecl")

SUCCESS_COUNT=0
FAIL_COUNT=0
FAILED_PLUGINS=()

for plugin in "${PLUGINS[@]}"; do
    echo -n "Testing $plugin... "

    if OUTPUT=$($BINARY query -p "$plugin" 2>&1); then
        # Check if output contains valid JSON
        if echo "$OUTPUT" | grep -q '{'; then
            echo "✅ OK"
            ((SUCCESS_COUNT++))
        else
            echo "⚠️  No JSON output"
            ((FAIL_COUNT++))
            FAILED_PLUGINS+=("$plugin (no JSON)")
        fi
    else
        echo "❌ Failed"
        ((FAIL_COUNT++))
        FAILED_PLUGINS+=("$plugin (error)")
    fi
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Test Results:"
echo "   ✅ Passed: $SUCCESS_COUNT"
echo "   ❌ Failed: $FAIL_COUNT"

if [ $FAIL_COUNT -gt 0 ]; then
    echo ""
    echo "Failed plugins:"
    for failed in "${FAILED_PLUGINS[@]}"; do
        echo "   • $failed"
    done
    exit 1
fi

echo ""
echo "🎉 All plugins working correctly!"
