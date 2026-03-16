# Simple Steel Calculator - Windows Packaging

This repository now targets a standard Windows customer install flow:

- PyInstaller `onedir` bundle
- Inno Setup x64 installer
- Machine-wide install under `Program Files`
- Per-user writable state under `%APPDATA%\SimpleSteelCalculator`

## Prerequisites

- Windows 10/11
- Python 3.x installed and in `PATH`
- PowerShell
- Build dependencies:

```powershell
python -m pip install --upgrade pip
pip install pyinstaller pandas openpyxl
```

Optional for installer packaging:

- Inno Setup 6 with `ISCC.exe` available in `PATH`, or installed in the default location
- A commercial Inno Setup license if your deployment requires it
- Optional override: set `STEELCAL_ISCC` if the licensed compiler install is not the one found by default

Optional for signed builds:

- `signtool.exe`
- A usable Authenticode signing configuration

## Build The App Bundle

Run from the project root:

```powershell
powershell -ExecutionPolicy Bypass -File .\build_windows.ps1
```

Output:

- `dist\windows\SimpleSteelCalculator\`
- `dist\windows\SimpleSteelCalculator\SimpleSteelCalculator.exe`

The build is an installer-owned application folder. Customers should not edit files in that installed directory.

## Package The Installer

Run from the project root:

```powershell
powershell -ExecutionPolicy Bypass -File .\package_installer_windows.ps1
```

This script:

- builds the latest PyInstaller bundle
- locates `ISCC.exe`
- passes the application version into Inno Setup
- emits a versioned installer in `compile\Output`

Expected installer name:

- `compile\Output\SimpleSteelCalculator-<version>-x64-Setup.exe`

## Optional Signing

If you want signed installer output, set these environment variables before packaging:

```powershell
$env:STEELCAL_ISCC = "C:\Path\To\Licensed\ISCC.exe"
$env:STEELCAL_SIGNTOOL = "C:\Program Files (x86)\Windows Kits\10\App Certification Kit\signtool.exe"
$env:STEELCAL_SIGN_ARGS = '/sha1 YOUR_CERT_THUMBPRINT /fd SHA256 /tr "http://timestamp.digicert.com" /td SHA256'
```

`STEELCAL_ISCC` may point to either `ISCC.exe` or `Compil32.exe`; the packaging script will normalize `Compil32.exe` to the sibling `ISCC.exe`.

Then run:

```powershell
powershell -ExecutionPolicy Bypass -File .\package_installer_windows.ps1
```

When signing is enabled, the Inno Setup script signs:

- the packaged application EXE before it is embedded
- the setup EXE
- the uninstaller

If signing variables are not set, the scripts still build successfully, but the output is unsigned and may trigger SmartScreen or publisher warnings.

## Runtime Layout

- Installed binaries: `%ProgramFiles%\Simple Steel Calculator`
- Writable config: `%APPDATA%\SimpleSteelCalculator\steel_calc_config.json`
- Exported history: `%APPDATA%\SimpleSteelCalculator\history.log`

On first launch, the app can import a legacy sidecar `steel_calc_config.json` from the install directory into `%APPDATA%`. After that, runtime writes stay out of `Program Files`.

## Validation Checklist

- Install on Windows 10 x64 and Windows 11 x64
- Launch from Start Menu and optional desktop shortcut
- Verify splash screen, icons, and bundled workbook load
- Confirm config/history are created in `%APPDATA%\SimpleSteelCalculator`
- Upgrade over an older install and verify `%APPDATA%` data is preserved
- Uninstall and verify binaries and shortcuts are removed
- If signing is enabled, verify signatures on both the app EXE and setup EXE
- Smoke-test silent install/uninstall for IT-managed scenarios:

```powershell
.\compile\Output\SimpleSteelCalculator-<version>-x64-Setup.exe /VERYSILENT /NORESTART
```

## Notes

- MSIX is intentionally not the default packaging format for this repo.
- WiX/MSI can be added later if a customer IT team requires MSI-native deployment.
