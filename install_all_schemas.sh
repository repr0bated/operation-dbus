#!/bin/bash
# install_all_schemas.sh - Batch install all production container schema bundles

set +e  # Don't exit on errors, process all schemas

SEARCH_DIR="${1:-/home/jeremy}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🔍 Searching for schema bundles in: $SEARCH_DIR"
echo ""

# Find all Production_Container_Spec_*.zip files
BUNDLES=($(find "$SEARCH_DIR" -maxdepth 1 -name "Production_Container_Spec_*.zip" -type f 2>/dev/null))

if [ ${#BUNDLES[@]} -eq 0 ]; then
    echo "❌ No schema bundles found matching: Production_Container_Spec_*.zip"
    echo ""
    echo "Expected filename format:"
    echo "  Production_Container_Spec_<Domain>_<YYYYMMDD-HHMMSS>.zip"
    echo ""
    echo "Example:"
    echo "  Production_Container_Spec_Smart_Aquarium_20251031-062835.zip"
    exit 1
fi

echo "📦 Found ${#BUNDLES[@]} schema bundle(s)"
echo ""

SUCCESS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0
FAILED_BUNDLES=()

for bundle in "${BUNDLES[@]}"; do
    BASENAME=$(basename "$bundle")

    # Extract domain for duplicate checking
    DOMAIN=$(echo "$BASENAME" | sed -E 's/Production_Container_Spec_([^_]+)_.*/\1/' | tr '[:upper:]' '[:lower:]' | tr '_' '-')

    # Check if already installed
    if [ -f "/git/operation-dbus/schemas/$DOMAIN/production.schema.json" ]; then
        echo "⏭️  Skipping $BASENAME (already installed)"
        ((SKIP_COUNT++))
        continue
    fi

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Processing: $BASENAME"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    if "$SCRIPT_DIR/install_schema.sh" "$bundle"; then
        echo "✅ Successfully installed: $BASENAME"
        ((SUCCESS_COUNT++))
    else
        echo "❌ Failed to install: $BASENAME"
        ((FAIL_COUNT++))
        FAILED_BUNDLES+=("$BASENAME")
    fi

    echo ""
done

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Batch Installation Summary"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "   ✅ Successful: $SUCCESS_COUNT"
echo "   ❌ Failed: $FAIL_COUNT"
echo "   ⏭️  Skipped: $SKIP_COUNT"
echo "   📦 Total: ${#BUNDLES[@]}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ $FAIL_COUNT -gt 0 ]; then
    echo ""
    echo "❌ Failed bundles:"
    for failed in "${FAILED_BUNDLES[@]}"; do
        echo "   • $failed"
    done
    echo ""
    echo "💡 Check the error messages above for details"
fi

if [ $SUCCESS_COUNT -gt 0 ]; then
    echo ""
    echo "📋 Installed schema domains:"
    ls -1 /git/operation-dbus/schemas/ | while read domain; do
        if [ -f "/git/operation-dbus/schemas/$domain/production.schema.json" ]; then
            EXAMPLE_COUNT=$(find "/git/operation-dbus/schemas/$domain/examples" -name "*.json" 2>/dev/null | wc -l)
            TEST_COUNT=$(find "/git/operation-dbus/schemas/$domain/tests" -name "*.json" 2>/dev/null | wc -l)
            echo "   • $domain ($EXAMPLE_COUNT examples, $TEST_COUNT tests)"
        fi
    done
fi

echo ""

if [ $FAIL_COUNT -eq 0 ] && [ $SUCCESS_COUNT -gt 0 ]; then
    echo "🎉 All schema bundles installed successfully!"
    exit 0
elif [ $SUCCESS_COUNT -gt 0 ]; then
    echo "⚠️  Some schema bundles installed with errors"
    exit 1
else
    echo "❌ No schema bundles were installed"
    exit 1
fi
