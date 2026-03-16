# Release Packaging & Cleanup — Validation Assertions

---

## Area: Asset Resolution (ASSET)

### VAL-ASSET-001: Override gauge tables resolved relative to executable, not CWD

**Behavior:** When the desktop binary is launched from a directory other than the workspace root (e.g., from `C:\Program Files\SimpleSteelCalculator\`), it resolves `assets/gauge_tables.override.json` relative to the executable's location — not the shell's current working directory.

**Pass condition:** Place the built `SimpleSteelCalculator.exe` and its `assets/` folder in a temporary directory (e.g., `%TEMP%\steelcal_test\`). `cd` to a different directory (e.g., `C:\Users`). Launch the exe. It must load override gauge data without errors. No "file not found" warnings appear in the error log.

**Evidence:** Run the exe from a non-workspace CWD while an override file exists beside the exe's `assets/` dir. Confirm via UI that override-specific gauge entries are present, or inspect the `.error.log` file for absence of override-load warnings.

---

### VAL-ASSET-002: Graceful fallback to builtins when override file is absent

**Behavior:** When `assets/gauge_tables.override.json` does not exist next to the executable, the application starts normally using only the compiled-in builtin gauge tables. No error dialog is shown and no crash occurs.

**Pass condition:** Delete or rename `assets/gauge_tables.override.json` from the installation directory. Launch the exe. The application starts, all 7 builtin material tables are available (HR/HRPO/CR, GALV/JK/BOND, ALUMINIZED, ALUMINUM, HR FLOOR PLATE, HOT ROLLED PLATE, STAINLESS), and calculations produce correct results.

**Evidence:** Verify the table dropdown contains all 7 entries. Perform a sheet weight calculation (e.g., HR/HRPO/CR gauge 16, 48×96, qty 1 → expected ≈ 80.000 lb). No `.error.log` file is created.

---

### VAL-ASSET-003: Config path resolution uses platform data directory, not CWD

**Behavior:** `config_path()` returns a path under the platform-appropriate user data directory (`%APPDATA%\SimpleSteelCalculator\` on Windows), regardless of the current working directory at launch time.

**Pass condition:** Launch the exe from any directory. The user config (`steel_calc_config.json`) is read from/written to `%APPDATA%\SimpleSteelCalculator\`, not from the CWD or exe directory.

**Evidence:** Create a config file at `%APPDATA%\SimpleSteelCalculator\steel_calc_config.json` with `{"density_lb_ft3": 500.0}`. Launch the exe from a different directory. The density field in the Coil panel shows `500` (not the default `490.55`).

---

## Area: PE Metadata (PE)

### VAL-PE-001: Windows executable builds without repository-local logo assets

**Behavior:** The built `SimpleSteelCalculator.exe` does not require repository-local logo assets such as `logo.ico` or `Harbor_logo_hr.png` to compile successfully.

**Pass condition:** Build the desktop crate in release mode after removing repository-local logo assets. The build completes successfully and produces `SimpleSteelCalculator.exe`.

**Evidence:** `cargo build --release -p steelcal-desktop` succeeds in a clean checkout that does not include the removed logo files.

---

### VAL-PE-002: Properties dialog shows version, company, and copyright

**Behavior:** Right-clicking the exe → Properties → Details tab displays:
- **File description:** "Simple Steel Calculator" (or equivalent product name)
- **Product version:** Matches `APP_VERSION` from `steelcal-core` and the current workspace version in `Cargo.toml`
- **Company:** "Harbor Pipe & Steel Inc."
- **Copyright:** Contains "Harbor Pipe & Steel Inc."

**Pass condition:** Right-click the release exe → Properties → Details. All four metadata fields are populated with the expected values. No field shows "N/A" or is blank.

**Evidence:** Screenshot or manual inspection of the Properties → Details tab. Version must match the workspace version declared in `Cargo.toml`.

---

## Area: Build Infrastructure (BUILD)

### VAL-BUILD-001: build_windows.ps1 invokes `cargo build --release` and stages output

**Behavior:** The rewritten `build_windows.ps1` runs `cargo build --release` (not PyInstaller), stages the resulting binary and assets to `dist\windows\SimpleSteelCalculator\`, and verifies `SimpleSteelCalculator.exe` exists in the staging directory.

**Pass condition:** Run `powershell -ExecutionPolicy Bypass -File .\build_windows.ps1` from the workspace root. The script completes without errors. `dist\windows\SimpleSteelCalculator\SimpleSteelCalculator.exe` exists and is a valid PE binary. No references to PyInstaller, `.spec` files, or `python` remain in the script.

**Evidence:** Inspect script source for `cargo build --release`. Verify `dist\windows\SimpleSteelCalculator\SimpleSteelCalculator.exe` exists after a clean run. File size is reasonable (> 1 MB for a Slint GUI app).

---

### VAL-BUILD-002: build_windows.ps1 also builds steelcal-cli

**Behavior:** The build script produces both the desktop GUI binary (`SimpleSteelCalculator.exe`) and the CLI binary (`steelcal-cli.exe` or named equivalent) in the staging directory, so the installer bundles both.

**Pass condition:** After running `build_windows.ps1`, the staging directory `dist\windows\SimpleSteelCalculator\` contains both the desktop exe and a CLI exe. Running the CLI exe with `--help` produces valid usage output.

**Evidence:** List the staging directory contents. Both binaries are present. `steelcal-cli.exe --help` prints the CLI help text with subcommands (e.g., `sheet`, `coil`, `scrap`).

---

### VAL-BUILD-003: build_linux.sh invokes `cargo build --release`

**Behavior:** The rewritten `build_linux.sh` runs `cargo build --release` (not PyInstaller), and the resulting binary is placed under `dist/linux/`. No references to PyInstaller, `.spec` files, or `python`/`pip` remain in the script.

**Pass condition:** Run `bash build_linux.sh` on a Linux system with Rust toolchain installed. The script exits 0. A binary exists at `dist/linux/SimpleSteelCalculator` (or `dist/linux/steelcal-desktop`). No PyInstaller references remain in the script (`grep -c pyinstaller build_linux.sh` returns 0).

**Evidence:** Inspect script source for `cargo build --release`. Verify the output binary is an ELF executable (`file dist/linux/...` shows "ELF 64-bit").

---

### VAL-BUILD-004: build_and_test.ps1 runs fmt check, clippy, test, and release build

**Behavior:** A new `build_and_test.ps1` script executes four stages in order:
1. `cargo fmt --all -- --check` (formatting verification)
2. `cargo clippy --all-targets --all-features -- -D warnings` (lint check)
3. `cargo test` (unit tests)
4. `cargo build --release` (release build)

The script fails fast (stops at first failure).

**Pass condition:** Run `powershell -ExecutionPolicy Bypass -File .\build_and_test.ps1`. All four stages complete successfully (exit code 0 from each). Intentionally break formatting (add extra whitespace), re-run — the script should fail at stage 1 and not proceed to clippy/test/build.

**Evidence:** Script output shows all four stage invocations in order. Final exit code is 0 on a clean codebase. The `target\release\` directory contains the freshly built binaries.

---

### VAL-BUILD-005: Inno Setup script references correct Rust output directory

**Behavior:** The `compile\SimpleSteelCalculator.iss` Inno Setup script's `BuildOutputDir` preprocessor variable points to the Rust build output staging directory (`dist\windows\SimpleSteelCalculator` or equivalent), not the old PyInstaller output path. The `[Files]` section sources the exe and supporting files from this directory.

**Pass condition:** Open `SimpleSteelCalculator.iss` and verify `#define BuildOutputDir` resolves to the Rust staging path. Run the Inno Setup compiler after a successful `build_windows.ps1` run — it produces a valid `SimpleSteelCalculator-{version}-x64-Setup.exe` installer. Installing and launching from the installer works correctly.

**Evidence:** Inspect the `.iss` file's `BuildOutputDir` define. The value should match the staging directory used by `build_windows.ps1`. The `[Files]` section's `Source:` paths reference `{#BuildOutputDir}\SimpleSteelCalculator.exe`.

---

### VAL-BUILD-006: Desktop binary is named SimpleSteelCalculator via Cargo.toml

**Behavior:** The `steelcal-desktop` crate's `Cargo.toml` contains a `[[bin]]` section (or package rename) that produces `SimpleSteelCalculator.exe` on Windows (and `SimpleSteelCalculator` on Linux) rather than the default `steelcal-desktop.exe`.

**Pass condition:** Run `cargo build --release -p steelcal-desktop`. The output binary is `target\release\SimpleSteelCalculator.exe` (Windows) or `target/release/SimpleSteelCalculator` (Linux). No `steelcal-desktop.exe` is produced.

**Evidence:** `dir target\release\SimpleSteelCalculator.exe` succeeds. `dir target\release\steelcal-desktop.exe` fails (file not found).

---

## Area: Repository Cleanup (CLEAN)

### VAL-CLEAN-001: Public workspace excludes editor state and internal tooling data

**Behavior:** The tracked workspace excludes editor-specific state and internal tooling directories that are not required to build, test, or package the application.

**Pass condition:** The repository does not track `.idea/`, `.vscode/`, `.factory/`, `.rmv/`, or `*.code-workspace` files.

**Evidence:** `git ls-files` returns no matches for those paths.

---

### VAL-CLEAN-002: Historical Python backup sources are not tracked

**Behavior:** The public workspace does not track archived Python-era application files or PyInstaller spec files that are no longer part of the Rust build.

**Pass condition:** The repository does not track `bkup/`, `UlimateSteelCal_20250823_0054.py`, `SteelCalTotal_OptionH.py`, or `UlimateSteelCal_20250823_0054.spec`.

**Evidence:** `git ls-files | grep -E '(^bkup/|UlimateSteelCal_20250823_0054\.py|SteelCalTotal_OptionH\.py|UlimateSteelCal_20250823_0054\.spec)'` returns no matches.

---

### VAL-CLEAN-003: Build scripts contain no Python/PyInstaller references

**Behavior:** Both `build_windows.ps1` and `build_linux.sh` are free of any references to Python, PyInstaller, pip, `.py` files, `.spec` files, or virtual environments. They are pure Rust build scripts.

**Pass condition:** Search both build scripts for patterns: `python`, `PyInstaller`, `pip`, `.py`, `.spec`, `.venv`, `pyinstaller`. None are found.

**Evidence:** `grep -ciE "python|pyinstaller|pip|\.py|\.spec|\.venv" build_windows.ps1 build_linux.sh` returns 0 for both files.
