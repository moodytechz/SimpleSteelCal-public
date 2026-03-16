#!/usr/bin/env bash
# Package a portable Linux or macOS release archive.
# Usage: bash package_unix_release.sh <linux|macos> [version]
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
    echo "Usage: bash package_unix_release.sh <linux|macos> [version]" >&2
    exit 1
fi

PLATFORM="$1"
VERSION="${2:-}"
REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"

read_workspace_version() {
    awk '
        /^\[workspace.package\]/ { in_section = 1; next }
        /^\[/ { in_section = 0 }
        in_section && /^version[[:space:]]*=/ {
            gsub(/"/, "", $3)
            print $3
            exit
        }
    ' "$REPO_ROOT/Cargo.toml"
}

case "$PLATFORM" in
    linux)
        BUILD_SCRIPT="$REPO_ROOT/build_linux.sh"
        STAGE_DIR="$REPO_ROOT/dist/linux"
        README_FILE="$REPO_ROOT/README_Linux.md"
        EXTRA_FILES=("$REPO_ROOT/install_linux.sh")
        ;;
    macos)
        BUILD_SCRIPT="$REPO_ROOT/build_macos.sh"
        STAGE_DIR="$REPO_ROOT/dist/macos"
        README_FILE="$REPO_ROOT/README_macOS.md"
        EXTRA_FILES=()
        ;;
    *)
        echo "Unsupported platform: $PLATFORM" >&2
        exit 1
        ;;
esac

if [[ -z "$VERSION" ]]; then
    VERSION="$(read_workspace_version)"
fi

if [[ -z "$VERSION" ]]; then
    echo "Unable to determine release version from Cargo.toml" >&2
    exit 1
fi

bash "$BUILD_SCRIPT"

ARCH="$(uname -m)"
BUNDLE_NAME="SimpleSteelCalculator-${VERSION}-${PLATFORM}-${ARCH}"
BUNDLE_DIR="$STAGE_DIR/$BUNDLE_NAME"
ARCHIVE_PATH="$STAGE_DIR/$BUNDLE_NAME.tar.gz"

rm -rf "$BUNDLE_DIR" "$ARCHIVE_PATH"
mkdir -p "$BUNDLE_DIR"

cp "$STAGE_DIR/SimpleSteelCalculator" "$BUNDLE_DIR/"
cp "$STAGE_DIR/steelcal-cli" "$BUNDLE_DIR/"

if [[ -d "$STAGE_DIR/assets" ]]; then
    cp -R "$STAGE_DIR/assets" "$BUNDLE_DIR/"
fi

cp "$README_FILE" "$BUNDLE_DIR/"
cp "$REPO_ROOT/LICENSE" "$BUNDLE_DIR/"
cp "$REPO_ROOT/NOTICE" "$BUNDLE_DIR/"

if [[ ${#EXTRA_FILES[@]} -gt 0 ]]; then
    for file in "${EXTRA_FILES[@]}"; do
        cp "$file" "$BUNDLE_DIR/"
    done
fi

tar -C "$STAGE_DIR" -czf "$ARCHIVE_PATH" "$BUNDLE_NAME"

echo ""
echo "Release archive created:"
echo "  $ARCHIVE_PATH"
