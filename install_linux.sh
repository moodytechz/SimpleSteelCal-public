#!/usr/bin/env bash
# Install the Simple Steel Calculator on Linux (user-level by default)
# Usage: bash install_linux.sh [--system]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_NAME="SimpleSteelCalculator"
BIN_NAME="simple-steel-calculator"
DIST_DIR="$SCRIPT_DIR/dist/linux"
DESKTOP_FILE="$HOME/.local/share/applications/simple-steel-calculator.desktop"
TARGET_DIR="$HOME/.local/opt/simple-steel-calculator"
TARGET_BIN="$HOME/.local/bin/$BIN_NAME"
SYSTEM_INSTALL=false

if [[ ${1:-} == "--system" ]]; then
  SYSTEM_INSTALL=true
  TARGET_DIR="/opt/simple-steel-calculator"
  TARGET_BIN="/usr/local/bin/$BIN_NAME"
  DESKTOP_FILE="/usr/share/applications/simple-steel-calculator.desktop"
fi

# Determine source layout from either the repo staging directory or an extracted release bundle.
if [[ -f "$DIST_DIR/$APP_NAME" || -d "$DIST_DIR/$APP_NAME" ]]; then
  SOURCE_ROOT="$DIST_DIR"
elif [[ -f "$SCRIPT_DIR/$APP_NAME" || -d "$SCRIPT_DIR/$APP_NAME" ]]; then
  SOURCE_ROOT="$SCRIPT_DIR"
else
  echo "Error: Build not found. Run build_linux.sh first or extract a release archive." >&2
  exit 1
fi

if [[ -f "$SOURCE_ROOT/$APP_NAME" ]]; then
  SRC_APP="$SOURCE_ROOT/$APP_NAME"
  LAUNCH_CMD="$TARGET_DIR/$APP_NAME"
elif [[ -d "$SOURCE_ROOT/$APP_NAME" ]]; then
  SRC_APP_DIR="$SOURCE_ROOT/$APP_NAME"
  LAUNCH_CMD="$TARGET_DIR/$APP_NAME/$APP_NAME"
else
  echo "Error: App payload not found under $SOURCE_ROOT." >&2
  exit 1
fi

SRC_CLI="$SOURCE_ROOT/steelcal-cli"
SRC_ASSETS_DIR="$SOURCE_ROOT/assets"

# Copy files
sudo_cmd() {
  if $SYSTEM_INSTALL; then
    sudo "$@"
  else
    "$@"
  fi
}

sudo_cmd mkdir -p "$(dirname "$TARGET_BIN")"
sudo_cmd mkdir -p "$TARGET_DIR"
sudo_cmd mkdir -p "$(dirname "$DESKTOP_FILE")"
if [[ -n ${SRC_APP:-} ]]; then
  sudo_cmd install -m 755 "$SRC_APP" "$TARGET_DIR/$APP_NAME"
else
  sudo_cmd rm -rf "$TARGET_DIR/$APP_NAME"
  sudo_cmd cp -a "$SRC_APP_DIR" "$TARGET_DIR/"
fi

if [[ -f "$SRC_CLI" ]]; then
  sudo_cmd install -m 755 "$SRC_CLI" "$TARGET_DIR/steelcal-cli"
fi

if [[ -d "$SRC_ASSETS_DIR" ]]; then
  sudo_cmd rm -rf "$TARGET_DIR/assets"
  sudo_cmd cp -a "$SRC_ASSETS_DIR" "$TARGET_DIR/"
fi

# Symlink launcher
if $SYSTEM_INSTALL; then
  sudo_cmd ln -sf "$LAUNCH_CMD" "$TARGET_BIN"
else
  ln -sf "$LAUNCH_CMD" "$TARGET_BIN"
fi

# Create .desktop entry
TMP_DESKTOP=$(mktemp)
cat > "$TMP_DESKTOP" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=Simple Steel Calculator
Comment=Sheet, coil, and costing calculator for steel
Exec=$TARGET_BIN
Terminal=false
Categories=Office;Engineering;
Keywords=steel;calculator;coil;sheet;scrap;weight;quote;
StartupNotify=false
EOF

if $SYSTEM_INSTALL; then
  sudo_cmd install -m 644 "$TMP_DESKTOP" "$DESKTOP_FILE"
  sudo_cmd update-desktop-database >/dev/null 2>&1 || true
else
  install -m 644 "$TMP_DESKTOP" "$DESKTOP_FILE"
  update-desktop-database ~/.local/share/applications >/dev/null 2>&1 || true
fi
rm -f "$TMP_DESKTOP"

echo "Installed. Launch from your applications menu or run: $BIN_NAME"
