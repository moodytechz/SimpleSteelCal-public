# Run the full build-and-test pipeline for Simple Steel Calculator.
# Stages: fmt check -> clippy -> test -> release build.  Fail-fast on first error.
# Usage: powershell -ExecutionPolicy Bypass -File .\build_and_test.ps1

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

# ── PATH setup ──────────────────────────────────────────────────────────
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

# ── Step 1: Format check ──────────────────────────────────────────────
Write-Host ""
Write-Host "=== cargo fmt --check ===" -ForegroundColor Cyan
cargo fmt --check
if ($LASTEXITCODE -ne 0) {
    Write-Host "FAILED: cargo fmt --check (exit code $LASTEXITCODE)" -ForegroundColor Red
    exit $LASTEXITCODE
}
Write-Host "PASSED: cargo fmt --check" -ForegroundColor Green

# ── Step 2: Clippy lint ───────────────────────────────────────────────
Write-Host ""
Write-Host "=== cargo clippy --workspace -- -D warnings ===" -ForegroundColor Cyan
cargo clippy --workspace -- -D warnings
if ($LASTEXITCODE -ne 0) {
    Write-Host "FAILED: cargo clippy (exit code $LASTEXITCODE)" -ForegroundColor Red
    exit $LASTEXITCODE
}
Write-Host "PASSED: cargo clippy --workspace -- -D warnings" -ForegroundColor Green

# ── Step 3: Tests ─────────────────────────────────────────────────────
Write-Host ""
Write-Host "=== cargo test --workspace ===" -ForegroundColor Cyan
cargo test --workspace
if ($LASTEXITCODE -ne 0) {
    Write-Host "FAILED: cargo test (exit code $LASTEXITCODE)" -ForegroundColor Red
    exit $LASTEXITCODE
}
Write-Host "PASSED: cargo test --workspace" -ForegroundColor Green

# ── Step 4: Release build ────────────────────────────────────────────
Write-Host ""
Write-Host "=== cargo build --release --workspace ===" -ForegroundColor Cyan
cargo build --release --workspace
if ($LASTEXITCODE -ne 0) {
    Write-Host "FAILED: cargo build --release (exit code $LASTEXITCODE)" -ForegroundColor Red
    exit $LASTEXITCODE
}
Write-Host "PASSED: cargo build --release --workspace" -ForegroundColor Green

# ── Summary ───────────────────────────────────────────────────────────
Write-Host ""
Write-Host "All steps passed." -ForegroundColor Green
