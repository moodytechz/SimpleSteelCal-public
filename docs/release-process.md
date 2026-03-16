# SteelCal Release Process

## CI Baseline

Every pull request should pass:

- `cargo build --workspace`
- `cargo test -p steelcal-core`
- `cargo test -p steelcal-cli`
- `cargo run -p steelcal-cli -- --width 48 --length 96 --gauge 16 --json`

## Linux Packaging

Build the Linux release bundle from the repository root:

```bash
bash build_linux.sh
```

Expected staged artifacts:

- `dist/linux/SimpleSteelCalculator`
- `dist/linux/steelcal-cli`
- `dist/linux/assets/gauge_tables.override.json`

## Local Verification

After packaging, smoke-run the packaged binaries:

```bash
./dist/linux/steelcal-cli --width 48 --length 96 --gauge 16 --json
timeout 10s ./dist/linux/SimpleSteelCalculator
```

Expected results:

- The packaged CLI prints valid JSON to stdout.
- The GUI binary stays alive until timeout when a display is available, or emits a clear startup error instead of exiting silently.
