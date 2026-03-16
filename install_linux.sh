#!/usr/bin/env bash
# Install the Simple Steel Calculator on Linux (user-level by default)
# Usage: bash install_linux.sh [--system]
set -euo pipefail

APP_NAME="SimpleSteelCalculator"
BIN_NAME="simple-steel-calculator"
DIST_DIR="dist/linux"
MODE_FILE="$DIST_DIR/$APP_NAME"
MODE_DIR="$DIST_DIR/$APP_NAME/$APP_NAME"
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

mkdir -p "$(dirname "$TARGET_BIN")"

# Determine build type and source path
if [[ -f "$MODE_FILE" ]]; then
  SRC_APP="$MODE_FILE"
  LAUNCH_CMD="$TARGET_DIR/$APP_NAME"
elif [[ -d "$DIST_DIR/$APP_NAME" ]]; then
  SRC_APP_DIR="$DIST_DIR/$APP_NAME"
  LAUNCH_CMD="$TARGET_DIR/$APP_NAME/$APP_NAME"
else
  echo "Error: Build not found in $DIST_DIR. Run build_linux.sh first." >&2
  exit 1
fi

# Copy files
sudo_cmd() {
  if $SYSTEM_INSTALL; then
    sudo "$@"
  else
    "$@"
  fi
}

sudo_cmd mkdir -p "$TARGET_DIR"
if [[ -n ${SRC_APP:-} ]]; then
  sudo_cmd install -m 755 "$SRC_APP" "$TARGET_DIR/$APP_NAME"
else
  sudo_cmd rm -rf "$TARGET_DIR/$APP_NAME"
  sudo_cmd cp -a "$SRC_APP_DIR" "$TARGET_DIR/"
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
