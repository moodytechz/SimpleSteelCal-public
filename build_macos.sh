#!/usr/bin/env bash
# Build Simple Steel Calculator for macOS (Rust).
# Usage: bash build_macos.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
DIST_DIR="$REPO_ROOT/dist/macos"
BUNDLE_NAME="SimpleSteelCalculator"
ASSETS_DIR="$DIST_DIR/assets"

echo "Building steelcal-desktop (release)..."
cargo build --release -p steelcal-desktop

echo "Building steelcal-cli (release)..."
cargo build --release -p steelcal-cli

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"
mkdir -p "$ASSETS_DIR"

DESKTOP_BIN="$REPO_ROOT/target/release/$BUNDLE_NAME"
CLI_BIN="$REPO_ROOT/target/release/steelcal-cli"
OVERRIDE_JSON="$REPO_ROOT/assets/gauge_tables.override.json"

if [ ! -f "$DESKTOP_BIN" ]; then
    echo "ERROR: Desktop binary not found: $DESKTOP_BIN" >&2
    exit 1
fi
if [ ! -f "$CLI_BIN" ]; then
    echo "ERROR: CLI binary not found: $CLI_BIN" >&2
    exit 1
fi

echo "Staging binaries to $DIST_DIR ..."
cp "$DESKTOP_BIN" "$DIST_DIR/"
cp "$CLI_BIN" "$DIST_DIR/"

if [ -f "$OVERRIDE_JSON" ]; then
    echo "Staging assets..."
    cp "$OVERRIDE_JSON" "$ASSETS_DIR/"
fi

echo ""
echo "Build complete."
echo "Staging directory: $DIST_DIR"
echo "Contents:"
find "$DIST_DIR" -type f | sort | while read -r f; do
    echo "  ${f#"$DIST_DIR"/}"
done
