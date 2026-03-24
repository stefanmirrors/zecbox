#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BINARIES_DIR="$PROJECT_DIR/src-tauri/binaries"
TARGET_TRIPLE="aarch64-apple-darwin"

mkdir -p "$BINARIES_DIR"

# Fetch real binaries or build mocks
if [ "${REAL_BINARIES:-0}" = "1" ]; then
    echo "Fetching real upstream binaries..."
    "$SCRIPT_DIR/fetch-binaries.sh"
else
    for binary in zebrad arti zaino; do
        BINARY_PATH="$BINARIES_DIR/${binary}-${TARGET_TRIPLE}"
        if [ ! -f "$BINARY_PATH" ]; then
            echo "Building mock-${binary}..."
            cargo build -p "mock-${binary}" --release --manifest-path "$PROJECT_DIR/Cargo.toml"
            cp "$PROJECT_DIR/target/release/mock-${binary}" "$BINARY_PATH"
            chmod +x "$BINARY_PATH"
        else
            echo "Using existing ${binary} binary at ${BINARY_PATH}"
        fi
    done
fi

# Build firewall helper binary
HELPER_BINARY="$BINARIES_DIR/zecbox-firewall-helper-${TARGET_TRIPLE}"
echo "Building firewall-helper..."
cargo build -p firewall-helper --release --manifest-path "$PROJECT_DIR/Cargo.toml"
cp "$PROJECT_DIR/target/release/zecbox-firewall-helper" "$HELPER_BINARY"
chmod +x "$HELPER_BINARY"

# If APPLE_SIGNING_IDENTITY is set, codesign sidecar binaries
if [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
    echo "Signing sidecar binaries..."
    for binary in zebrad arti zaino; do
        BINARY_PATH="$BINARIES_DIR/${binary}-${TARGET_TRIPLE}"
        codesign --force --options runtime --sign "$APPLE_SIGNING_IDENTITY" "$BINARY_PATH"
    done
    echo "Signing firewall helper..."
    codesign --force --options runtime --sign "$APPLE_SIGNING_IDENTITY" "$HELPER_BINARY"
fi

# Install frontend dependencies if needed
if [ ! -d "$PROJECT_DIR/node_modules" ]; then
    echo "Installing frontend dependencies..."
    cd "$PROJECT_DIR" && npm install
fi

# Build the Tauri app
echo "Building ZecBox..."
cd "$PROJECT_DIR"
npx tauri build

echo ""
echo "Build complete."
DMG_PATH=$(find "$PROJECT_DIR/target/release/bundle/dmg" -name "*.dmg" 2>/dev/null | head -1)
if [ -n "$DMG_PATH" ]; then
    echo "DMG: $DMG_PATH"
    ls -lh "$DMG_PATH"
else
    echo "Warning: No DMG found in bundle output"
fi
