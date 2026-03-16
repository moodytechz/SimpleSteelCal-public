# Simple Steel Calculator - Windows Packaging and Release

SteelCal ships on Windows as both an Inno Setup installer and a portable zip.
The public GitHub release workflow builds both artifacts from a `v*` tag.

## Prerequisites

- Windows 10 or 11
- Rust stable toolchain
- Visual Studio C++ build tools
- PowerShell

Optional for installer packaging:

- Inno Setup 6 with `ISCC.exe` available in `PATH`
- `STEELCAL_ISCC` if you want to point at a specific compiler install

Optional for signed builds:

- `signtool.exe`
- Authenticode signing credentials

## Build The App Bundle

Run from the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File .\build_windows.ps1
```

Output:

- `dist\windows\SimpleSteelCalculator\`
- `dist\windows\SimpleSteelCalculator\SimpleSteelCalculator.exe`
- `dist\windows\SimpleSteelCalculator\steelcal-cli.exe`

## Package The Installer

Run from the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File .\package_installer_windows.ps1
```

Expected installer name:

- `compile\Output\SimpleSteelCalculator-<version>-x64-Setup.exe`

## Public Release Assets

Tagged releases publish:

- `SimpleSteelCalculator-<version>-x64-Setup.exe`
- `SimpleSteelCalculator-<version>-portable.zip`

The portable zip is a ready-to-run extracted app folder. The installer provides
the standard Start Menu and `Program Files` install experience.

## Optional Signing

If you want signed installer output, set these environment variables before
packaging:

```powershell
$env:STEELCAL_ISCC = "C:\Path\To\ISCC.exe"
$env:STEELCAL_SIGNTOOL = "C:\Path\To\signtool.exe"
$env:STEELCAL_SIGN_ARGS = '/sha1 YOUR_CERT_THUMBPRINT /fd SHA256 /tr "http://timestamp.digicert.com" /td SHA256'
```

If signing variables are not set, the build still succeeds but the output is
unsigned.

## Runtime Layout

- Installed binaries: `%ProgramFiles%\Simple Steel Calculator`
- Writable config: `%APPDATA%\SimpleSteelCalculator\steel_calc_config.json`
- Exported history: `%APPDATA%\SimpleSteelCalculator\history.log`
