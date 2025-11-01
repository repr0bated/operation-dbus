#!/bin/bash
# validate_container_spec.sh - Validate a container spec against its domain schema

if [ $# -lt 2 ]; then
    echo "Usage: $0 <domain> <container-spec.json>"
    echo ""
    echo "Example: $0 telehealth examples/my-container.json"
    echo ""
    echo "Available domains:"
    ls -1 /git/operation-dbus/schemas/ 2>/dev/null | while read d; do
        if [ -d "/git/operation-dbus/schemas/$d" ]; then
            echo "  • $d"
        fi
    done
    exit 1
fi

DOMAIN="$1"
SPEC_FILE="$2"
SCHEMA_FILE="/git/operation-dbus/schemas/$DOMAIN/schema.json"

if [ ! -f "$SCHEMA_FILE" ]; then
    echo "❌ Schema not found for domain: $DOMAIN"
    echo "   Expected: $SCHEMA_FILE"
    exit 1
fi

if [ ! -f "$SPEC_FILE" ]; then
    echo "❌ Container spec not found: $SPEC_FILE"
    exit 1
fi

echo "🔍 Validating container spec against $DOMAIN schema..."
echo ""
echo "Schema: $SCHEMA_FILE"
echo "Spec: $SPEC_FILE"
echo ""

# Check if ajv-cli is available
if ! command -v ajv >/dev/null 2>&1; then
    echo "⚠️  ajv-cli not found. Install with: npm install -g ajv-cli"
    echo ""
    echo "Falling back to basic JSON validation..."

    if python3 -m json.tool "$SPEC_FILE" >/dev/null 2>&1; then
        echo "✅ Spec is valid JSON"
        echo ""
        echo "ℹ️  Install ajv-cli for full schema validation"
        exit 0
    else
        echo "❌ Spec is not valid JSON!"
        python3 -m json.tool "$SPEC_FILE"
        exit 1
    fi
fi

# Validate with ajv
if ajv validate -s "$SCHEMA_FILE" -d "$SPEC_FILE" 2>&1; then
    echo ""
    echo "✅ Container spec is valid!"
    exit 0
else
    echo ""
    echo "❌ Container spec validation failed"
    exit 1
fi
