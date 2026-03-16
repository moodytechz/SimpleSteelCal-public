# SteelCal Release Process

## CI Baseline

Before cutting a release, `main` should be green on:

- `cargo build --workspace`
- `cargo test -p steelcal-core`
- `cargo test -p steelcal-cli`
- `cargo test -p steelcal-desktop`

## Local Packaging Commands

Build or package from the repository root:

```bash
bash build_linux.sh
bash build_macos.sh
bash package_unix_release.sh linux
bash package_unix_release.sh macos
```

On Windows:

```powershell
powershell -ExecutionPolicy Bypass -File .\build_windows.ps1
powershell -ExecutionPolicy Bypass -File .\package_installer_windows.ps1
```

Expected platform artifacts:

- Windows installer: `compile/Output/SimpleSteelCalculator-<version>-x64-Setup.exe`
- Windows portable zip: `dist/windows/SimpleSteelCalculator-<version>-portable.zip`
- Linux portable tarball: `dist/linux/SimpleSteelCalculator-<version>-linux-<arch>.tar.gz`
- macOS portable tarball: `dist/macos/SimpleSteelCalculator-<version>-macos-<arch>.tar.gz`

## Public GitHub Release Flow

The public repository publishes release assets when a tag matching `v*` is
pushed.

Workflow behavior:

1. Create or reuse the GitHub Release for the tag.
2. Build and upload the Windows installer and portable zip.
3. Build and upload the Linux portable archive.
4. Build and upload the macOS portable archive.

## Notes

- Linux archives include `install_linux.sh` so the extracted bundle can be installed locally.
- macOS releases are currently unsigned portable archives rather than notarized `.app` bundles or `.dmg` files.
- Asset names use the stripped tag version, for example `v0.1.3` becomes `0.1.3`.
