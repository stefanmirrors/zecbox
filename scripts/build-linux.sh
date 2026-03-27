#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BINARIES_DIR="$PROJECT_DIR/src-tauri/binaries"
TARGET_TRIPLE="${TARGET_TRIPLE:-x86_64-unknown-linux-gnu}"

mkdir -p "$BINARIES_DIR"

# Build mock sidecar binaries
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

# Build firewall helper binary
HELPER_BINARY="$BINARIES_DIR/zecbox-firewall-helper-${TARGET_TRIPLE}"
echo "Building firewall-helper..."
cargo build -p firewall-helper --release --manifest-path "$PROJECT_DIR/Cargo.toml"
cp "$PROJECT_DIR/target/release/zecbox-firewall-helper" "$HELPER_BINARY"
chmod +x "$HELPER_BINARY"

# Install frontend dependencies if needed
if [ ! -d "$PROJECT_DIR/node_modules" ]; then
    echo "Installing frontend dependencies..."
    cd "$PROJECT_DIR" && npm install
fi

# Build the Tauri app
echo "Building ZecBox for Linux..."
cd "$PROJECT_DIR"
npx tauri build

echo ""
echo "Build complete."

APPIMAGE_PATH=$(find "$PROJECT_DIR/target/release/bundle/appimage" -name "*.AppImage" 2>/dev/null | head -1)
if [ -n "$APPIMAGE_PATH" ]; then
    echo "AppImage: $APPIMAGE_PATH"
    ls -lh "$APPIMAGE_PATH"
fi

DEB_PATH=$(find "$PROJECT_DIR/target/release/bundle/deb" -name "*.deb" 2>/dev/null | head -1)
if [ -n "$DEB_PATH" ]; then
    echo "Deb: $DEB_PATH"
    ls -lh "$DEB_PATH"
fi
