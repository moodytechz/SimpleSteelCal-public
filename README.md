# SteelCal

Steel industry calculator for sheet weight/cost, coil metrics, and scrap calculations. Migrated from Python to Rust for improved performance and type safety.

Version 0.1.2 · Copyright (c) Harbor Pipe & Steel Inc.

## Features

- **Sheet weight** — calculate per-sheet and total weight from dimensions and gauge/PSF/thickness
- **Coil metrics** — linear footage, PIW (lb/in), and outer diameter from coil dimensions and weight
- **Scrap calculations** — scrap weight, total cost, price per lb, and pickup determination
- **Pricing** — per-lb, per-ft², or per-sheet modes with markup, tax, setup fee, and minimum order
- **Batch automation** — run homogeneous batch jobs from JSON for sheet/coil/scrap and from CSV for sheet
- **Gauge tables** — built-in tables (HR/HRPO/CR, GALV/JK/BOND, STAINLESS, HOT ROLLED PLATE, etc.) plus user-defined overrides
- **Config migration** — versioned config schema with automatic rewrite of older configs to the current format
- **JSON output** — structured JSON output for integration with other tools
- **Desktop GUI** — native Windows application built with Slint (tabbed Sheet/Coil/Scrap panels)

## Latest Changes

This repository was updated with the following project-level changes:

- batch CLI support for:
  - JSON sheet jobs
  - JSON `job_type: "coil"` jobs
  - JSON `job_type: "scrap"` jobs
  - CSV sheet jobs
- batch CSV export via `--output-file`, while keeping JSON on stdout
- raw-cell preservation for CSV parse failures in exported error rows
- config schema versioning with automatic migration/rewrite for older config files
- safer handling of newer-than-supported config files by leaving them untouched and running on safe defaults
- desktop codebase refactor splitting startup, initialization, handler logic, and callback wiring into focused modules
- expanded CI smoke coverage for:
  - single-run CLI
  - sheet JSON batch
  - sheet CSV batch
  - partial-success CSV batch
  - batch export
  - coil JSON batch
  - scrap JSON batch

## Workspace Structure

```
crates/
  steelcal-core/     Core calculation library (sheet, coil, scrap, gauges, config, history)
  steelcal-cli/      Command-line interface
  steelcal-desktop/  Native desktop GUI (Slint)
```

## Prerequisites

- Rust stable toolchain (edition 2021)
- On Windows: MSVC build tools for the desktop app

Install Rust via [rustup](https://rustup.rs/) if you don't have it.

## Build

Build the entire workspace:

```
cargo build --workspace
```

Build in release mode:

```
cargo build --workspace --release
```

## License

Code in this repository is available under the MIT License. See `LICENSE`.

Branding assets such as logos and splash screens remain subject to the notice
in `NOTICE`.

## Contributing

Contributions are welcome. See `CONTRIBUTING.md` for setup, testing, and pull
request guidance.

## CLI Usage

The CLI binary is `steelcal-cli`.

### Sheet weight calculation

Calculate weight for a 48×96 inch sheet at gauge 16 (default table HR/HRPO/CR):

```
cargo run -p steelcal-cli -- --width 48 --length 96 --gauge 16
```

With a specific gauge table:

```
cargo run -p steelcal-cli -- --width 48 --length 96 --gauge 14 --table STAINLESS
```

Using PSF directly instead of gauge lookup:

```
cargo run -p steelcal-cli -- --width 48 --length 96 --psf 2.5
```

Using raw thickness (inches):

```
cargo run -p steelcal-cli -- --width 48 --length 96 --thickness 0.0598
```

Multiple sheets with quantity:

```
cargo run -p steelcal-cli -- --width 48 --length 96 --gauge 16 --qty 10
```

### Pricing

Add pricing to a sheet calculation (per-lb with markup and tax):

```
cargo run -p steelcal-cli -- --width 48 --length 96 --gauge 16 --qty 5 \
  --price-mode per-lb --price 0.45 --markup 10 --tax 8.25
```

Per-sheet pricing with setup fee and minimum order:

```
cargo run -p steelcal-cli -- --width 48 --length 96 --gauge 16 --qty 2 \
  --price-mode per-sheet --price 50.00 --setup-fee 25.00 --min-order 100.00
```

### Coil metrics

Calculate linear footage and PIW from coil weight and dimensions:

```
cargo run -p steelcal-cli -- --coil-weight 10000 --coil-width 48 --coil-thickness 0.0598 --coil-id 20
```

### Scrap calculation

```
cargo run -p steelcal-cli -- --scrap-actual 10000 --scrap-ending 9500 \
  --scrap-base-cost 0.35 --scrap-processing-cost 0.05
```

### Discovery commands

List all available gauge tables:

```
cargo run -p steelcal-cli -- --list-tables
```

List gauge keys and PSF values for a specific table:

```
cargo run -p steelcal-cli -- --list-gauges HR/HRPO/CR
```

### JSON output

Append `--json` to any calculation to get structured JSON output:

```
cargo run -p steelcal-cli -- --width 48 --length 96 --gauge 16 --json
```

### Batch input

Batch mode supports homogeneous JSON files for `sheet`, `coil`, and `scrap`
jobs. CSV input remains sheet-only. Use `--input-file` with `--json` to
process multiple jobs in one invocation.

JSON input:

```
cargo run -p steelcal-cli -- --input-file fixtures/batch/sheet-batch-happy-path.json --json
```

JSON `job_type` examples:

```json
{ "job_type": "coil", "jobs": [ { "coil_width": 48, "coil_thickness": 0.06, "coil_id": 20, "coil_weight": 2000 } ] }
```

```json
{ "job_type": "scrap", "jobs": [ { "actual_weight": 5000, "ending_weight": 4800, "base_cost": 0.35, "processing_cost": 0.05 } ] }
```

CSV input:

```
cargo run -p steelcal-cli -- --input-file fixtures/batch/sheet-batch-happy-path.csv --json
```

CSV headers:

- required: `width`, `length`
- optional: `qty`, `gauge`, `table`, `psf`, `thickness`, `density`

The batch response is structured as:

- `results[]` for successful rows
- `errors[]` for row-level failures
- `row_index` to identify which input row failed

CSV-specific behavior:

- missing required headers such as `width` or `length` fail the whole command
- invalid row values such as `qty=abc` become row-level `errors[]` entries
- valid rows in the same file still complete normally
- CSV batch input is currently limited to sheet jobs

Batch mode is intentionally separate from the single-row flags. It does not
combine `--input-file` with direct per-run sheet, coil, or scrap arguments.

### Batch CSV export

Use `--output-file` with batch mode to keep JSON on stdout and also write a
combined CSV export file:

```
cargo run -p steelcal-cli -- --input-file fixtures/batch/sheet-batch-invalid-row.csv --json \
  --output-file /tmp/steelcal-export.csv
```

Export columns:

- echoed inputs: `row_index`, `width`, `length`, `qty`, `gauge`, `table`, `psf`, `thickness`, `density`
- results: `each_lb`, `total_lb`, `psf_result`, `area_ft2_each`, `area_ft2_total`, `used_key`
- failures: `error_message`

Behavior:

- success rows populate result columns and leave `error_message` blank
- failed rows leave result columns blank and populate `error_message`
- if a CSV row fails during parsing, echoed input columns preserve the original raw cell values when available
- JSON output is still printed to stdout when `--output-file` is used

## Desktop App

Build and run the native desktop application:

```
cargo run -p steelcal-desktop
```

Or build a release binary:

```
cargo build -p steelcal-desktop --release
```

The release binary is named `SimpleSteelCalculator` (`SimpleSteelCalculator.exe` on Windows). On Windows, the binary includes PE metadata such as version information and company name embedded via `winresource` in `build.rs`.

The desktop app provides a tabbed interface with Sheet, Coil, and Scrap panels. The Sheet panel supports gauge/PSF/thickness input modes and includes a pricing section. Gauge table and gauge key dropdowns are populated from the built-in tables and any overrides.

### Menu Bar

| Menu  | Item                | Action                                     |
|-------|---------------------|--------------------------------------------|
| File  | Exit                | Quit the application                       |
| Help  | User Guide          | Open the built-in help dialog              |
| Help  | View History        | Open the session history dialog            |
| Help  | About               | Show version and copyright info            |
| Tools | Edit Configuration  | Open the JSON config editor dialog         |

### Dialogs

- **About** — displays version, copyright, and developer credit.
- **User Guide** — scrollable reference covering all panels, pricing, history, config, and shortcuts.
- **Config Editor** — inline JSON editor with Validate, Save, Revert, Restore Defaults, and Open Config Location buttons.
- **History** — lists all calculations made in the current session. Supports type filtering (All/Sheet/Coil/Scrap), text search, entry preview, Recall (repopulates panel inputs), and Export to text file.

### Keyboard Shortcuts

| Shortcut       | Action                          |
|----------------|---------------------------------|
| F1             | Open User Guide                 |
| Ctrl+H         | Open View History               |
| Ctrl+E         | Open Edit Configuration         |
| Ctrl+S         | Calculate Sheet & Quote         |
| Ctrl+Enter     | Calculate Sheet & Quote         |
| Ctrl+Shift+C   | Calculate Coil                  |
| Ctrl+L         | Copy sheet total to Scrap Actual Weight |
| Ctrl+R         | Copy sheet total to Scrap Ending Weight |
| Escape         | Close any open dialog           |

Action shortcuts (Ctrl+S, Ctrl+Enter, Ctrl+Shift+C, Ctrl+L, Ctrl+R) are suppressed while a dialog is open.

### Additional UI Features

- **Tooltips** — all input fields display descriptive tooltip text on hover.
- **Clear buttons** — each panel (Sheet, Coil, Scrap) has a Clear button that resets inputs and results to defaults.
- **Copy-to-Scrap helpers** — the Scrap panel includes "Copy to Actual" and "Copy to Ending" buttons that transfer the latest sheet total weight into the corresponding scrap field.

## Testing

Run all tests across the workspace:

```
cargo test --workspace
```

The test suite includes 170+ tests covering sheet weight calculations, coil metrics, scrap logic, gauge table lookups, config loading, input validation, and CLI integration.

Run tests with the `selftest` feature (built-in self-tests in the core library):

```
cargo test --workspace --features selftest
```

Pull requests should also pass the workspace build and a CLI smoke run:

```
cargo build --workspace
cargo run -p steelcal-cli -- --width 48 --length 96 --gauge 16 --json
cargo run -p steelcal-cli -- --input-file fixtures/batch/sheet-batch-invalid-row.csv --json --output-file /tmp/steelcal-export.csv
```

Current CI smoke coverage also includes:

- JSON batch sheet input
- CSV batch sheet input
- CSV partial-success handling
- CSV export generation
- JSON batch coil input
- JSON batch scrap input

## Build & Release

### build_and_test.ps1

Runs the full CI pipeline locally on Windows: format check → Clippy lint → workspace tests → release build.

```
powershell -ExecutionPolicy Bypass -File .\build_and_test.ps1
```

### build_windows.ps1

Builds release binaries and stages a distributable bundle under `dist/windows/SimpleSteelCalculator/` containing the desktop executable (`SimpleSteelCalculator.exe`), the CLI (`steelcal-cli.exe`), and assets.

```
powershell -ExecutionPolicy Bypass -File .\build_windows.ps1
```

### build_linux.sh

Builds release binaries and stages them under `dist/linux/` with the same layout.

```
bash build_linux.sh
```

### build_macos.sh

Builds release binaries and stages them under `dist/macos/` for portable macOS packaging.

```
bash build_macos.sh
```

### Tagged GitHub Releases

Pushing a `v*` tag in the public repository publishes platform assets to a single GitHub Release:

- Windows installer: `SimpleSteelCalculator-<version>-x64-Setup.exe`
- Windows portable zip: `SimpleSteelCalculator-<version>-portable.zip`
- Linux portable archive: `SimpleSteelCalculator-<version>-linux-<arch>.tar.gz`
- macOS portable archive: `SimpleSteelCalculator-<version>-macos-<arch>.tar.gz`

The Linux and macOS archives contain the GUI binary, the CLI binary, bundled assets, license files, and platform-specific release notes. The macOS archive is a portable unsigned bundle rather than a notarized `.app` or `.dmg`.

### Inno Setup Installer (Windows)

An Inno Setup script at `compile/SimpleSteelCalculator.iss` generates a Windows installer from the staged bundle. It is driven by `package_installer_windows.ps1` and supports optional code-signing via environment variables.

## Configuration

SteelCal looks for a config file at:

- **Windows:** `%APPDATA%\SimpleSteelCalculator\steel_calc_config.json`
- **Linux/macOS:** `~/.SimpleSteelCalculator/steel_calc_config.json`

Example config:

```json
{
  "config_version": 1,
  "default_table": "HR/HRPO/CR",
  "default_gauge": "16",
  "steel_density": 490.0
}
```

| Key | Default | Description |
|-----|---------|-------------|
| `config_version` | `1` | Schema version used for config migration and rewrite behavior |
| `default_table` | `HR/HRPO/CR` | Gauge table used when `--table` is not specified |
| `default_gauge` | `16` | Gauge key used when no gauge/psf/thickness is provided |
| `steel_density` | `490.0` | Steel density in lb/ft³ for thickness-to-PSF conversion |

Config migration behavior:

- missing or older `config_version` values are migrated and rewritten automatically
- current-version files load normally and may be rewritten if normalization cleans them up
- newer-than-supported config files are preserved untouched, and the app runs on safe defaults instead of attempting a destructive downgrade

## Override Gauge Tables

Place a file at `assets/gauge_tables.override.json` to add or replace gauge tables. The format maps table names to objects of gauge-key → PSF pairs:

```json
{
  "ALUMINUM": {
    "12": 4.452,
    "14": 3.202,
    "16": 2.577,
    "18": 2.077
  },
  "CUSTOM TABLE": {
    "10": 5.5,
    "12": 4.0
  }
}
```

Override entries are merged with the built-in tables. If a table name already exists, the override entries replace it.
