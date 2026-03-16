# Build the Windows release bundle for Simple Steel Calculator (Rust).
# Usage: powershell -ExecutionPolicy Bypass -File .\build_windows.ps1

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

# ── PATH setup ──────────────────────────────────────────────────────────
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

$repoRoot   = Split-Path -Parent $MyInvocation.MyCommand.Path
$distRoot   = Join-Path $repoRoot "dist\windows"
$bundleName = "SimpleSteelCalculator"
$bundleDir  = Join-Path $distRoot $bundleName
$assetsDir  = Join-Path $bundleDir "assets"

function Get-AppVersion {
    $metadataJson = cargo metadata --format-version 1 --no-deps 2>$null
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($metadataJson)) {
        throw "Unable to read Cargo metadata for release version."
    }

    $metadata = $metadataJson | ConvertFrom-Json
    $desktopPackage = $metadata.packages | Where-Object { $_.name -eq "steelcal-desktop" } | Select-Object -First 1
    if (-not $desktopPackage) {
        throw "Unable to locate steelcal-desktop package version in Cargo metadata."
    }

    return $desktopPackage.version
}

$appVersion = Get-AppVersion

# ── Cargo release build ────────────────────────────────────────────────
Write-Host "Building steelcal-desktop (release)..." -ForegroundColor Cyan
cargo build --release -p steelcal-desktop
if ($LASTEXITCODE -ne 0) { throw "cargo build --release -p steelcal-desktop failed (exit $LASTEXITCODE)" }

Write-Host "Building steelcal-cli (release)..." -ForegroundColor Cyan
cargo build --release -p steelcal-cli
if ($LASTEXITCODE -ne 0) { throw "cargo build --release -p steelcal-cli failed (exit $LASTEXITCODE)" }

# ── Stage binaries and assets ──────────────────────────────────────────
if (Test-Path $bundleDir) {
    Remove-Item -Recurse -Force $bundleDir
}
New-Item -ItemType Directory -Force -Path $bundleDir | Out-Null
New-Item -ItemType Directory -Force -Path $assetsDir | Out-Null

$desktopExe = Join-Path $repoRoot "target\release\$bundleName.exe"
$cliExe     = Join-Path $repoRoot "target\release\steelcal-cli.exe"
$overrideJson = Join-Path $repoRoot "assets\gauge_tables.override.json"

if (!(Test-Path $desktopExe)) { throw "Desktop exe not found: $desktopExe" }
if (!(Test-Path $cliExe))     { throw "CLI exe not found: $cliExe" }

Write-Host "Staging binaries to $bundleDir ..." -ForegroundColor Cyan
Copy-Item $desktopExe -Destination $bundleDir
Copy-Item $cliExe     -Destination $bundleDir

if (Test-Path $overrideJson) {
    Write-Host "Staging assets..." -ForegroundColor Cyan
    Copy-Item $overrideJson -Destination $assetsDir
}

# ── Optional: Inno Setup compile ───────────────────────────────────────
$issFile = Join-Path $repoRoot "compile\SimpleSteelCalculator.iss"
$iscc    = $null

# Check common Inno Setup locations
$pf86 = [Environment]::GetFolderPath('ProgramFilesX86')
$pf   = $env:ProgramFiles
$isccPaths = @(
    "$pf86\Inno Setup 6\ISCC.exe",
    "$pf\Inno Setup 6\ISCC.exe",
    "$pf86\Inno Setup 5\ISCC.exe",
    "$pf\Inno Setup 5\ISCC.exe"
)
foreach ($p in $isccPaths) {
    if (Test-Path $p) { $iscc = $p; break }
}

if ($iscc -and (Test-Path $issFile)) {
    Write-Host "Inno Setup found at $iscc - compiling installer..." -ForegroundColor Cyan
    & $iscc "/DMyAppVersion=$appVersion" "/DBuildOutputDir=$bundleDir" $issFile
    if ($LASTEXITCODE -ne 0) { throw "Inno Setup ISCC.exe failed (exit $LASTEXITCODE)" }
    Write-Host "Installer created." -ForegroundColor Green
} else {
    Write-Host "Inno Setup not found - skipping installer compilation." -ForegroundColor Yellow
}

# ── Summary ────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "Build complete." -ForegroundColor Green
Write-Host "Staging directory: $bundleDir"
Write-Host "Contents:"
Get-ChildItem -Recurse $bundleDir | ForEach-Object {
    Write-Host "  $($_.FullName.Replace($bundleDir, '.'))"
}
