#!/bin/bash
set -euo pipefail

# Generate and sign the binary update manifest for zecbox.io.
#
# Usage:
#   generate-manifest.sh \
#     --app-version 0.1.7 \
#     --binary zebrad --version 4.2.0 --platform aarch64-apple-darwin \
#     --url https://github.com/.../zebrad-aarch64-apple-darwin --sha256 abc123 --size 12345678 \
#     [--binary zaino --version 0.2.0 --platform aarch64-apple-darwin ...] \
#     --signing-key <hex-encoded-ed25519-private-key> \
#     --output site/public/updates/manifest.json
#
# Or pipe from environment:
#   MANIFEST_SIGNING_KEY=<hex> generate-manifest.sh ...
#
# If --output points to an existing manifest, new entries are merged (same name+platform replaces).

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

APP_VERSION=""
SIGNING_KEY="${MANIFEST_SIGNING_KEY:-}"
OUTPUT=""
BINARIES_JSON="[]"

# Parse args into binary entries
CURRENT_BINARY=""
CURRENT_VERSION=""
CURRENT_PLATFORM=""
CURRENT_URL=""
CURRENT_SHA256=""
CURRENT_SIZE=""

flush_binary() {
    if [ -n "$CURRENT_BINARY" ]; then
        ENTRY=$(cat <<ENTRY_EOF
{
  "name": "$CURRENT_BINARY",
  "version": "$CURRENT_VERSION",
  "platform": "$CURRENT_PLATFORM",
  "downloadUrl": "$CURRENT_URL",
  "sha256": "$CURRENT_SHA256",
  "sizeBytes": $CURRENT_SIZE
}
ENTRY_EOF
)
        # Append to array
        BINARIES_JSON=$(echo "$BINARIES_JSON" | python3 -c "
import sys, json
arr = json.load(sys.stdin)
entry = json.loads('''$ENTRY''')
# Replace existing entry with same name+platform, or append
arr = [e for e in arr if not (e['name'] == entry['name'] and e['platform'] == entry['platform'])]
arr.append(entry)
json.dump(arr, sys.stdout)
")
        CURRENT_BINARY=""
        CURRENT_VERSION=""
        CURRENT_PLATFORM=""
        CURRENT_URL=""
        CURRENT_SHA256=""
        CURRENT_SIZE=""
    fi
}

while [ $# -gt 0 ]; do
    case "$1" in
        --app-version) APP_VERSION="$2"; shift 2 ;;
        --signing-key) SIGNING_KEY="$2"; shift 2 ;;
        --output)      OUTPUT="$2"; shift 2 ;;
        --binary)
            flush_binary
            CURRENT_BINARY="$2"; shift 2 ;;
        --version)     CURRENT_VERSION="$2"; shift 2 ;;
        --platform)    CURRENT_PLATFORM="$2"; shift 2 ;;
        --url)         CURRENT_URL="$2"; shift 2 ;;
        --sha256)      CURRENT_SHA256="$2"; shift 2 ;;
        --size)        CURRENT_SIZE="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done
flush_binary

if [ -z "$APP_VERSION" ] || [ -z "$OUTPUT" ]; then
    echo "Usage: generate-manifest.sh --app-version <ver> --output <path> [--binary ...] [--signing-key <hex>]"
    exit 1
fi

# If existing manifest, merge new entries into it
if [ -f "$OUTPUT" ]; then
    EXISTING_BINARIES=$(python3 -c "
import json, sys
with open('$OUTPUT') as f:
    m = json.load(f)
json.dump(m.get('binaries', []), sys.stdout)
" 2>/dev/null || echo "[]")

    BINARIES_JSON=$(python3 -c "
import json, sys
existing = json.loads('''$EXISTING_BINARIES''')
new = json.loads('''$BINARIES_JSON''')
# New entries override existing by name+platform
keys = {(e['name'], e['platform']) for e in new}
merged = [e for e in existing if (e['name'], e['platform']) not in keys]
merged.extend(new)
json.dump(merged, sys.stdout)
")
fi

# Build unsigned manifest
UNSIGNED=$(python3 -c "
import json
manifest = {
    'appVersion': '$APP_VERSION',
    'binaries': json.loads('''$BINARIES_JSON''')
}
print(json.dumps(manifest, indent=2))
")

if [ -n "$SIGNING_KEY" ]; then
    # Write unsigned manifest, sign it with manifest-signer tool
    echo "$UNSIGNED" > "$OUTPUT"

    # Write key to temp file
    KEY_TMP=$(mktemp)
    echo "$SIGNING_KEY" > "$KEY_TMP"

    # Build and run signer
    cargo run -p manifest-signer --quiet -- sign --key "$KEY_TMP" "$OUTPUT"
    rm -f "$KEY_TMP"

    echo "Signed manifest written to $OUTPUT"
else
    echo "$UNSIGNED" > "$OUTPUT"
    echo "WARNING: No signing key provided — manifest is UNSIGNED"
    echo "Set MANIFEST_SIGNING_KEY or use --signing-key to sign"
fi

echo ""
echo "Manifest contents:"
cat "$OUTPUT"
