# Simple Steel Calculator - Linux Build, Install, and Release

SteelCal ships on Linux as a portable Rust bundle. The public GitHub release
publishes a `.tar.gz` archive, and the repo also includes local scripts for
building and installing from source.

## Prerequisites

On Debian or Ubuntu:

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libfontconfig1-dev
```

Install Rust with [rustup](https://rustup.rs/) if it is not already available.

## Build From Source

From the repository root:

```bash
bash build_linux.sh
```

This stages the release payload under `dist/linux/`:

- `dist/linux/SimpleSteelCalculator`
- `dist/linux/steelcal-cli`
- `dist/linux/assets/gauge_tables.override.json`

## Package A Release Archive

To create the same style of portable archive used by GitHub Releases:

```bash
bash package_unix_release.sh linux
```

Expected archive name:

- `dist/linux/SimpleSteelCalculator-<version>-linux-<arch>.tar.gz`

## Install

You can install either from a local source build or from an extracted release
archive. In both cases, run the installer script from the directory that
contains `install_linux.sh` and the packaged binaries:

```bash
bash install_linux.sh
```

System-wide install:

```bash
sudo bash install_linux.sh --system
```

This installs the app under `~/.local/opt/simple-steel-calculator` by default,
creates `~/.local/bin/simple-steel-calculator`, and installs a desktop entry so
the app appears in your launcher.

## Running

After install:

```bash
simple-steel-calculator
```

The portable bundle can also be run directly without installation:

```bash
./SimpleSteelCalculator
./steelcal-cli --help
```

## Uninstall

User-level uninstall:

```bash
rm -f ~/.local/bin/simple-steel-calculator
rm -rf ~/.local/opt/simple-steel-calculator
rm -f ~/.local/share/applications/simple-steel-calculator.desktop
update-desktop-database ~/.local/share/applications >/dev/null 2>&1 || true
```

System-wide uninstall:

```bash
sudo rm -f /usr/local/bin/simple-steel-calculator
sudo rm -rf /opt/simple-steel-calculator
sudo rm -f /usr/share/applications/simple-steel-calculator.desktop
sudo update-desktop-database >/dev/null 2>&1 || true
```

## Notes

- Writable configuration lives at `~/.SimpleSteelCalculator/steel_calc_config.json`.
- The bundled `assets/gauge_tables.override.json` file is optional override data.
- The Linux release asset is a portable archive, not an AppImage or distro-native package.
