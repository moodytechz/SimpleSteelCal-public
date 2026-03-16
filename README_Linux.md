# Simple Steel Calculator - Linux Build and Install

This project includes scripts to build and install a Linux package using PyInstaller.

## Prerequisites

On Debian/Ubuntu (or similar):

```bash
sudo apt update
sudo apt install -y python3 python3-venv python3-tk python3-pip build-essential
python3 -m pip install --upgrade pip
python3 -m pip install pyinstaller pandas openpyxl
```

Notes:
- The app uses Tkinter; ensure `python3-tk` is installed.
- If you want XLSX override tables, `pandas` and `openpyxl` are required.

## Build

From the project root:

```bash
bash build_linux.sh          # one-file build (default)
# or
bash build_linux.sh onedir   # one-dir build
```

The output will be placed under `dist/linux`.

Expected release artifacts:

- `dist/linux/SimpleSteelCalculator`
- `dist/linux/steelcal-cli`
- `dist/linux/assets/gauge_tables.override.json`

## Install

User-level install (no sudo required):

```bash
bash install_linux.sh
```

System-wide install (requires sudo):

```bash
bash install_linux.sh --system
```

This will:
- Copy the built app to `~/.local/opt/simple-steel-calculator` (or `/opt/simple-steel-calculator` for system install).
- Create a launcher symlink at `~/.local/bin/simple-steel-calculator` (or `/usr/local/bin/simple-steel-calculator`).
- Install a desktop entry so you can launch it from the system menu.

## Running

After install, run:

```bash
simple-steel-calculator
```

or use your Desktop Environment’s application menu.

## Uninstall

User-level uninstall:

```bash
rm -f ~/.local/bin/simple-steel-calculator
rm -rf ~/.local/opt/simple-steel-calculator
rm -f ~/.local/share/applications/simple-steel-calculator.desktop
rm -f ~/.local/share/icons/simple-steel-calculator.png
update-desktop-database ~/.local/share/applications || true
```

System-wide uninstall (requires sudo):

```bash
sudo rm -f /usr/local/bin/simple-steel-calculator
sudo rm -rf /opt/simple-steel-calculator
sudo rm -f /usr/share/applications/simple-steel-calculator.desktop
sudo rm -f /usr/share/icons/hicolor/256x256/apps/simple-steel-calculator.png
sudo update-desktop-database || true
```

## Notes
- Linux uses the in-app Tk splash fallback rather than the Windows bootloader splash.
- Writable configuration is stored under `~/.SimpleSteelCalculator/steel_calc_config.json`. On first launch, a legacy sidecar `steel_calc_config.json` next to the binary is imported into that folder once.
- Data files included in the build: `Harbor_logo_hr.png`, `Harbor_splash.png`, `logo.ico`, `lbs_ft_table.xlsx`, and `steel_calc_config.json`.
- To override gauge tables, place `lbs_ft_table.xlsx` next to the installed binary.
- If you see a blank window on Wayland, try launching with `XDG_SESSION_TYPE=x11`. Most DEs work out-of-the-box.

## Runtime quick reference

- Main buttons: `Calculate Sheet & Quote`, `Calculate Scrap/Pickup`, `Calculate Coil`, and `Clear`.
- Help menu commands: `User Guide`, `View History`, `Edit Configuration…`, and `About`.
- Sheet calculations support three input modes: gauge/size, direct `lb/ft²`, or manual thickness.
- Coil inputs are `Coil Width`, `Coil Thickness`, `Coil ID`, and `Coil Weight`. The app computes `Coil Footage`, `PIW`, and `Coil OD`; it does not accept PIW, OD, or footage as coil inputs.
- Enable `Show Summary Popups` to open a copyable summary dialog after calculations.
- `View History` supports search, type filtering, `Recall`, and `Export`.
- History is kept in memory for the current session until you export it to `~/.SimpleSteelCalculator/history.log`.
- The active configuration file is `~/.SimpleSteelCalculator/steel_calc_config.json`.

## Examples
- Sheet example: Material HR/HRPO/CR, Gauge 16, Width 48 in, Length 96 in, Qty 10. Click "Calculate Sheet & Quote".
- Coil example: Coil Width 48 in, Coil Thickness 0.060 in, Coil ID 20 in, Coil Weight 2000 lb. Click "Calculate Coil" to compute footage, PIW, and OD.
- Costing example: Mode per lb, Price $0.60, Markup 15%, Tax 6%, Fees $25. Click "Calculate Sheet & Quote".
- Sample config to change defaults:
  {
    "default_table": "GALV/JK/BOND",
    "default_gauge": "20"
  }
- Sample config to adjust UI scaling:
  {
    "ui_scaling": 1.2,
    "ui_font_size": 13
  }
