# Contributing

Thanks for taking the time to contribute to SteelCal.

## Getting started

- Fork the repository and create a feature branch from `main`.
- Keep changes focused. Small pull requests are easier to review and test.
- If your change affects behavior, include or update tests when practical.

## Development setup

Prerequisites:

- Rust stable
- On Windows, MSVC build tools for the desktop app

Common commands:

```bash
cargo build --workspace
cargo test --workspace
```

Windows packaging helpers:

```powershell
powershell -ExecutionPolicy Bypass -File .\build_and_test.ps1
powershell -ExecutionPolicy Bypass -File .\build_windows.ps1
```

Linux packaging helpers:

```bash
bash build_linux.sh
bash install_linux.sh
```

## Style and quality

- Run `cargo fmt --all` before opening a pull request.
- Run `cargo clippy --all-targets --all-features -- -D warnings` when possible.
- Prefer focused commits with clear commit messages.
- Update documentation when commands, packaging, or user-facing behavior changes.

## Pull requests

Please include:

- a short summary of the problem being solved
- the approach you took
- any testing you ran
- screenshots if the desktop UI changed

## Issues

- Use the bug report template for defects and regressions.
- Use the feature request template for ideas and enhancements.

## Licensing and branding

- Code contributions are made under the repository license in `LICENSE`.
- Do not submit third-party assets, logos, or branding unless you have the
  right to share them.
- Branding assets in this repository remain subject to `NOTICE`.
