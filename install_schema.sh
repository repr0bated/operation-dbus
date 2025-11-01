#!/bin/bash
# install_schema.sh - Install and validate a production container specification schema bundle

set -e

if [ $# -lt 1 ]; then
    echo "Usage: $0 <schema-bundle.zip>"
    echo ""
    echo "Example: $0 Production_Container_Spec_Smart_Aquarium_*.zip"
    exit 1
fi

BUNDLE_ZIP="$1"
SCHEMA_DIR="/git/operation-dbus/schemas"

if [ ! -f "$BUNDLE_ZIP" ]; then
    echo "❌ Bundle not found: $BUNDLE_ZIP"
    exit 1
fi

echo "📦 Installing schema bundle: $(basename "$BUNDLE_ZIP")"
echo ""

# Extract domain from filename
# Expected format: Production_Container_Spec_<Domain>_<timestamp>.zip
BASENAME=$(basename "$BUNDLE_ZIP" .zip)
DOMAIN=$(echo "$BASENAME" | sed -E 's/Production_Container_Spec_([^_]+)_.*/\1/' | tr '[:upper:]' '[:lower:]' | tr '_' '-')

if [ -z "$DOMAIN" ] || [ "$DOMAIN" = "$BASENAME" ]; then
    echo "⚠️  Could not extract domain from filename, using 'unknown'"
    DOMAIN="unknown"
fi

echo "🏢 Domain: $DOMAIN"

# Create temp extraction directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Extract bundle
echo "📂 Extracting bundle..."
unzip -q "$BUNDLE_ZIP" -d "$TEMP_DIR"

# Detect structure
SCHEMA_FILE=""
BASE_SCHEMA_FILE=""
MAPPING_FILE=""
EXAMPLE_DIR=""
TEST_DIR=""
README_FILE=""

if [ -f "$TEMP_DIR/schema/production.container.schema.json" ]; then
    SCHEMA_FILE="$TEMP_DIR/schema/production.container.schema.json"
fi

if [ -f "$TEMP_DIR/LXC-CONFIGURATION-SCHEMA.json" ]; then
    BASE_SCHEMA_FILE="$TEMP_DIR/LXC-CONFIGURATION-SCHEMA.json"
fi

if [ -f "$TEMP_DIR/mapping/legacy_to_production.csv" ]; then
    MAPPING_FILE="$TEMP_DIR/mapping/legacy_to_production.csv"
fi

if [ -d "$TEMP_DIR/examples" ]; then
    EXAMPLE_DIR="$TEMP_DIR/examples"
fi

if [ -d "$TEMP_DIR/tests" ]; then
    TEST_DIR="$TEMP_DIR/tests"
fi

if [ -f "$TEMP_DIR/docs/README.md" ]; then
    README_FILE="$TEMP_DIR/docs/README.md"
fi

if [ -z "$SCHEMA_FILE" ]; then
    echo "❌ No production schema file found in bundle"
    exit 1
fi

# Create domain directory
TARGET_DIR="$SCHEMA_DIR/$DOMAIN"
mkdir -p "$TARGET_DIR"

# Install production schema
echo "📋 Installing production overlay schema..."
cp "$SCHEMA_FILE" "$TARGET_DIR/production.schema.json"

# Install base LXC schema if present
if [ -n "$BASE_SCHEMA_FILE" ]; then
    echo "📋 Installing base LXC schema..."
    cp "$BASE_SCHEMA_FILE" "$TARGET_DIR/lxc-base.schema.json"
fi

# Install mapping if present
if [ -n "$MAPPING_FILE" ]; then
    echo "🗺️  Installing legacy migration mapping..."
    cp "$MAPPING_FILE" "$TARGET_DIR/legacy-mapping.csv"
fi

# Install examples
if [ -n "$EXAMPLE_DIR" ]; then
    echo "📝 Installing examples..."
    mkdir -p "$TARGET_DIR/examples"
    cp -r "$EXAMPLE_DIR"/* "$TARGET_DIR/examples/" 2>/dev/null || true
    EXAMPLE_COUNT=$(find "$TARGET_DIR/examples" -name "*.json" 2>/dev/null | wc -l)
    echo "   Found $EXAMPLE_COUNT example files"
fi

# Install tests
if [ -n "$TEST_DIR" ]; then
    echo "🧪 Installing test cases..."
    mkdir -p "$TARGET_DIR/tests"
    cp -r "$TEST_DIR"/* "$TARGET_DIR/tests/" 2>/dev/null || true
    TEST_COUNT=$(find "$TARGET_DIR/tests" -name "*.json" 2>/dev/null | wc -l)
    echo "   Found $TEST_COUNT test files"
fi

# Install README if present
if [ -n "$README_FILE" ]; then
    echo "📖 Installing documentation..."
    cp "$README_FILE" "$TARGET_DIR/README.md"
fi

echo ""
echo "✅ Schema bundle installed to: $TARGET_DIR"
echo ""

# Validate schema is valid JSON
echo "🔍 Validating production schema JSON..."
if ! python3 -m json.tool "$TARGET_DIR/production.schema.json" >/dev/null 2>&1; then
    echo "❌ Production schema is not valid JSON!"
    exit 1
fi
echo "✅ Production schema JSON is valid"

if [ -f "$TARGET_DIR/lxc-base.schema.json" ]; then
    echo "🔍 Validating base LXC schema JSON..."
    if ! python3 -m json.tool "$TARGET_DIR/lxc-base.schema.json" >/dev/null 2>&1; then
        echo "⚠️  Base LXC schema is not valid JSON"
    else
        echo "✅ Base LXC schema JSON is valid"
    fi
fi

# Validate examples against schema (if ajv-cli is available)
if command -v ajv >/dev/null 2>&1 && [ -d "$TARGET_DIR/examples" ]; then
    echo ""
    echo "🧪 Validating examples against production schema..."

    VALID_COUNT=0
    INVALID_COUNT=0

    for example in "$TARGET_DIR/examples"/*.json; do
        if [ -f "$example" ]; then
            BASENAME=$(basename "$example")
            if ajv validate -s "$TARGET_DIR/production.schema.json" -d "$example" >/dev/null 2>&1; then
                echo "  ✅ $BASENAME"
                ((VALID_COUNT++))
            else
                echo "  ❌ $BASENAME"
                ((INVALID_COUNT++))
            fi
        fi
    done

    echo ""
    echo "📊 Example Validation: ✅ $VALID_COUNT valid, ❌ $INVALID_COUNT invalid"
fi

# Test that invalid cases correctly fail validation
if command -v ajv >/dev/null 2>&1 && [ -d "$TARGET_DIR/tests" ]; then
    echo ""
    echo "🧪 Testing invalid cases (should fail validation)..."

    CORRECT_FAIL=0
    INCORRECT_PASS=0

    for invalid in "$TARGET_DIR/tests"/invalid*.json; do
        if [ -f "$invalid" ]; then
            BASENAME=$(basename "$invalid")
            if ajv validate -s "$TARGET_DIR/production.schema.json" -d "$invalid" >/dev/null 2>&1; then
                echo "  ⚠️  $BASENAME (should have failed!)"
                ((INCORRECT_PASS++))
            else
                echo "  ✅ $BASENAME (correctly rejected)"
                ((CORRECT_FAIL++))
            fi
        fi
    done

    if [ $INCORRECT_PASS -gt 0 ]; then
        echo ""
        echo "⚠️  Warning: $INCORRECT_PASS invalid cases incorrectly passed validation!"
    else
        echo ""
        echo "✅ All invalid test cases correctly rejected"
    fi
fi

# Show README excerpt if present
if [ -f "$TARGET_DIR/README.md" ]; then
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "📖 Schema Documentation"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    head -5 "$TARGET_DIR/README.md"
    echo "..."
    echo ""
    echo "Full documentation: $TARGET_DIR/README.md"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📦 Schema Bundle Summary"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "   Domain: $DOMAIN"
echo "   Production Schema: $([ -f "$TARGET_DIR/production.schema.json" ] && echo "✅" || echo "❌")"
echo "   Base LXC Schema: $([ -f "$TARGET_DIR/lxc-base.schema.json" ] && echo "✅" || echo "❌")"
echo "   Legacy Mapping: $([ -f "$TARGET_DIR/legacy-mapping.csv" ] && echo "✅" || echo "❌")"
echo "   Examples: $(find "$TARGET_DIR/examples" -name "*.json" 2>/dev/null | wc -l) files"
echo "   Tests: $(find "$TARGET_DIR/tests" -name "*.json" 2>/dev/null | wc -l) files"
echo "   Documentation: $([ -f "$TARGET_DIR/README.md" ] && echo "✅" || echo "❌")"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "🎉 Schema installation complete!"
echo ""

if ! command -v ajv >/dev/null 2>&1; then
    echo "💡 Tip: Install ajv-cli for schema validation: npm install -g ajv-cli"
fi
