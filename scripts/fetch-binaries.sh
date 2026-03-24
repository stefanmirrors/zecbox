#!/bin/bash
set -euo pipefail

# Fetch real upstream sidecar binaries for bundling.
# Builds from source via cargo install when pre-built binaries are not available.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BINARIES_DIR="$PROJECT_DIR/src-tauri/binaries"
TARGET_TRIPLE="aarch64-apple-darwin"

ZEBRAD_VERSION="${ZEBRAD_VERSION:-4.2.0}"
ZAINO_VERSION="${ZAINO_VERSION:-0.2.0-rc.6}"

mkdir -p "$BINARIES_DIR"

# --- zebrad ---
ZEBRAD_PATH="$BINARIES_DIR/zebrad-${TARGET_TRIPLE}"
if [ ! -f "$ZEBRAD_PATH" ]; then
    echo "Installing zebrad ${ZEBRAD_VERSION} from crates.io..."
    cargo install zebrad --version "$ZEBRAD_VERSION" --locked --root "$PROJECT_DIR/target/sidecar-install"
    cp "$PROJECT_DIR/target/sidecar-install/bin/zebrad" "$ZEBRAD_PATH"
    chmod +x "$ZEBRAD_PATH"
    echo "zebrad installed at $ZEBRAD_PATH"
else
    echo "Using existing zebrad at $ZEBRAD_PATH"
fi

# --- zaino ---
ZAINO_PATH="$BINARIES_DIR/zaino-${TARGET_TRIPLE}"
if [ ! -f "$ZAINO_PATH" ]; then
    echo "Building zaino from source (zingolabs/zaino@${ZAINO_VERSION})..."
    ZAINO_TMP="$PROJECT_DIR/target/zaino-build"
    if [ ! -d "$ZAINO_TMP" ]; then
        git clone --depth 1 --branch "$ZAINO_VERSION" https://github.com/zingolabs/zaino.git "$ZAINO_TMP"
    fi
    cargo build --release --manifest-path "$ZAINO_TMP/Cargo.toml" -p zaino
    # Find the built binary (could be zaino or zaino-serve depending on version)
    if [ -f "$ZAINO_TMP/target/release/zaino" ]; then
        cp "$ZAINO_TMP/target/release/zaino" "$ZAINO_PATH"
    elif [ -f "$ZAINO_TMP/target/release/zaino-serve" ]; then
        cp "$ZAINO_TMP/target/release/zaino-serve" "$ZAINO_PATH"
    else
        echo "Warning: Could not find zaino binary, using mock"
        cargo build -p mock-zaino --release --manifest-path "$PROJECT_DIR/Cargo.toml"
        cp "$PROJECT_DIR/target/release/mock-zaino" "$ZAINO_PATH"
    fi
    chmod +x "$ZAINO_PATH"
    echo "zaino installed at $ZAINO_PATH"
else
    echo "Using existing zaino at $ZAINO_PATH"
fi

# --- arti ---
# Arti does not provide pre-built binaries and is complex to build.
# For v0.1.0, use the mock binary. Shield Mode UI will function but
# will not route through real Tor until a real arti binary is provided.
ARTI_PATH="$BINARIES_DIR/arti-${TARGET_TRIPLE}"
if [ ! -f "$ARTI_PATH" ]; then
    echo "Building mock-arti (real arti planned for future release)..."
    cargo build -p mock-arti --release --manifest-path "$PROJECT_DIR/Cargo.toml"
    cp "$PROJECT_DIR/target/release/mock-arti" "$ARTI_PATH"
    chmod +x "$ARTI_PATH"
    echo "mock-arti installed at $ARTI_PATH"
else
    echo "Using existing arti at $ARTI_PATH"
fi

echo ""
echo "Sidecar binaries ready:"
ls -lh "$BINARIES_DIR"/*-"${TARGET_TRIPLE}" 2>/dev/null || echo "No binaries found"
