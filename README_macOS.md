# Simple Steel Calculator - macOS Build and Release

SteelCal currently ships on macOS as a portable `.tar.gz` archive built from
the native Rust binaries. The public release workflow uploads this archive for
each tagged release.

## Prerequisites

- macOS
- Xcode command line tools
- Rust stable toolchain

Install the command line tools if needed:

```bash
xcode-select --install
```

Install Rust with [rustup](https://rustup.rs/) if it is not already available.

## Build From Source

From the repository root:

```bash
bash build_macos.sh
```

This stages the release payload under `dist/macos/`:

- `dist/macos/SimpleSteelCalculator`
- `dist/macos/steelcal-cli`
- `dist/macos/assets/gauge_tables.override.json`

## Package A Release Archive

To create the same portable archive uploaded by GitHub Releases:

```bash
bash package_unix_release.sh macos
```

Expected archive name:

- `dist/macos/SimpleSteelCalculator-<version>-macos-<arch>.tar.gz`

## Running

Extract the archive and launch the binaries directly:

```bash
./SimpleSteelCalculator
./steelcal-cli --help
```

## Notes

- The macOS release is currently an unsigned portable archive, not a notarized `.app` bundle or `.dmg`.
- Because the archive is unsigned, Gatekeeper may require using Finder's "Open" flow or removing quarantine attributes after download.
- Writable configuration lives at `~/.SimpleSteelCalculator/steel_calc_config.json`.
