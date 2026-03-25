#!/bin/bash
set -euo pipefail

# Fetch real upstream sidecar binaries for bundling.
# Builds from source via cargo install when pre-built binaries are not available.
# Set TARGET_TRIPLE env var to cross-compile (e.g. x86_64-apple-darwin).

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BINARIES_DIR="$PROJECT_DIR/src-tauri/binaries"
TARGET_TRIPLE="${TARGET_TRIPLE:-aarch64-apple-darwin}"

ZEBRAD_VERSION="${ZEBRAD_VERSION:-4.2.0}"
ZAINO_VERSION="${ZAINO_VERSION:-0.2.0-rc.6}"

# Detect if we need to cross-compile
HOST_TRIPLE=$(rustc -vV | grep host | awk '{print $2}')
if [ "$TARGET_TRIPLE" != "$HOST_TRIPLE" ]; then
    echo "Cross-compiling for $TARGET_TRIPLE (host is $HOST_TRIPLE)"
    CROSS_ARGS="--target $TARGET_TRIPLE"
    CROSS_DIR="$TARGET_TRIPLE/release"
else
    CROSS_ARGS=""
    CROSS_DIR="release"
fi

mkdir -p "$BINARIES_DIR"

# --- zebrad ---
ZEBRAD_PATH="$BINARIES_DIR/zebrad-${TARGET_TRIPLE}"
if [ ! -f "$ZEBRAD_PATH" ]; then
    echo "Installing zebrad ${ZEBRAD_VERSION} from crates.io..."
    # cargo install doesn't support --target well, so we clone and build
    ZEBRAD_TMP="$PROJECT_DIR/target/zebrad-build"
    if [ ! -d "$ZEBRAD_TMP" ]; then
        git clone --depth 1 --branch "v${ZEBRAD_VERSION}" https://github.com/ZcashFoundation/zebra.git "$ZEBRAD_TMP"
    fi
    cargo build --release --manifest-path "$ZEBRAD_TMP/Cargo.toml" -p zebrad $CROSS_ARGS
    cp "$ZEBRAD_TMP/target/$CROSS_DIR/zebrad" "$ZEBRAD_PATH"
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
    cargo build --release --manifest-path "$ZAINO_TMP/Cargo.toml" -p zainod $CROSS_ARGS
    if [ -f "$ZAINO_TMP/target/$CROSS_DIR/zainod" ]; then
        cp "$ZAINO_TMP/target/$CROSS_DIR/zainod" "$ZAINO_PATH"
    else
        echo "Warning: Could not find zaino binary, using mock"
        cargo build -p mock-zaino --release --manifest-path "$PROJECT_DIR/Cargo.toml" $CROSS_ARGS
        cp "$PROJECT_DIR/target/$CROSS_DIR/mock-zaino" "$ZAINO_PATH"
    fi
    chmod +x "$ZAINO_PATH"
    echo "zaino installed at $ZAINO_PATH"
else
    echo "Using existing zaino at $ZAINO_PATH"
fi

# --- arti ---
# Arti does not provide pre-built binaries and is complex to build.
# For v0.1.x, use the mock binary. Shield Mode UI will function but
# will not route through real Tor until a real arti binary is provided.
ARTI_PATH="$BINARIES_DIR/arti-${TARGET_TRIPLE}"
if [ ! -f "$ARTI_PATH" ]; then
    echo "Building mock-arti (real arti planned for future release)..."
    cargo build -p mock-arti --release --manifest-path "$PROJECT_DIR/Cargo.toml" $CROSS_ARGS
    cp "$PROJECT_DIR/target/$CROSS_DIR/mock-arti" "$ARTI_PATH"
    chmod +x "$ARTI_PATH"
    echo "mock-arti installed at $ARTI_PATH"
else
    echo "Using existing arti at $ARTI_PATH"
fi

echo ""
echo "Sidecar binaries ready:"
ls -lh "$BINARIES_DIR"/*-"${TARGET_TRIPLE}" 2>/dev/null || echo "No binaries found"
