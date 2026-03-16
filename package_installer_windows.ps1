# Package Simple Steel Calculator into a Windows installer using Inno Setup.
# Usage:
#   powershell -ExecutionPolicy Bypass -File .\package_installer_windows.ps1
# Optional signing environment variables:
#   STEELCAL_SIGNTOOL  - path to signtool.exe (or a command resolvable via PATH)
#   STEELCAL_SIGN_ARGS - arguments inserted after "signtool.exe sign" and before the target file
#   STEELCAL_SIGN_CERT_SHA1 - certificate thumbprint used to auto-build signtool arguments
#   STEELCAL_SIGN_CERT_SUBJECT - certificate subject filter used when thumbprint is not supplied
#   STEELCAL_SIGN_TIMESTAMP_URL - RFC3161 timestamp URL (defaults to DigiCert)

[CmdletBinding()]
param(
  [switch]$SkipBuild,
  [string]$IsccPath = $env:STEELCAL_ISCC,
  [string]$SignToolPath = $env:STEELCAL_SIGNTOOL,
  [string]$SignArgs = $env:STEELCAL_SIGN_ARGS,
  [string]$SignCertThumbprint = $env:STEELCAL_SIGN_CERT_SHA1,
  [string]$SignCertSubject = $env:STEELCAL_SIGN_CERT_SUBJECT,
  [string]$TimestampUrl = $env:STEELCAL_SIGN_TIMESTAMP_URL
)

$ErrorActionPreference = "Stop"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

function Get-AppVersion {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoRoot
  )

  $metadataJson = cargo metadata --format-version 1 --no-deps 2>$null
  if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($metadataJson)) {
    throw "Unable to read Cargo metadata from $RepoRoot"
  }

  $metadata = $metadataJson | ConvertFrom-Json
  $desktopPackage = $metadata.packages | Where-Object { $_.name -eq "steelcal-desktop" } | Select-Object -First 1
  if (-not $desktopPackage) {
    throw "Unable to locate steelcal-desktop package version in Cargo metadata."
  }

  return $desktopPackage.version
}

function Find-Iscc {
  $candidates = @(
    "iscc.exe",
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe"
  )

  foreach ($candidate in $candidates) {
    try {
      $command = Get-Command $candidate -ErrorAction Stop
      return $command.Path
    } catch {
      if (Test-Path $candidate) {
        return (Resolve-Path $candidate).Path
      }
    }
  }

  return $null
}

function Find-SignTool {
  $candidates = New-Object System.Collections.Generic.List[string]
  $candidates.Add("signtool.exe")

  $kitsRoot = "C:\Program Files (x86)\Windows Kits\10\bin"
  if (Test-Path $kitsRoot) {
    Get-ChildItem $kitsRoot -Directory |
      Sort-Object Name -Descending |
      ForEach-Object {
        $x64Path = Join-Path $_.FullName "x64\signtool.exe"
        if (Test-Path $x64Path) {
          $candidates.Add($x64Path)
        }
      }
  }

  foreach ($candidate in $candidates) {
    try {
      $command = Get-Command $candidate -ErrorAction Stop
      return $command.Path
    } catch {
      if (Test-Path $candidate) {
        return (Resolve-Path $candidate).Path
      }
    }
  }

  return $null
}

function Resolve-CommandPath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Candidate
  )

  try {
    $command = Get-Command $Candidate -ErrorAction Stop
    return $command.Path
  } catch {
    if (Test-Path $Candidate) {
      return (Resolve-Path $Candidate).Path
    }
  }

  throw "Unable to locate command: $Candidate"
}

function Resolve-InnoCompilerPath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Candidate
  )

  $resolved = Resolve-CommandPath -Candidate $Candidate
  if ([System.IO.Path]::GetFileName($resolved).Equals("Compil32.exe", [System.StringComparison]::OrdinalIgnoreCase)) {
    $siblingIscc = Join-Path (Split-Path -Parent $resolved) "ISCC.exe"
    if (Test-Path $siblingIscc) {
      return (Resolve-Path $siblingIscc).Path
    }

    throw "Compil32.exe was provided, but sibling ISCC.exe was not found: $siblingIscc"
  }

  return $resolved
}

function Find-CodeSigningCertificate {
  param(
    [string]$Thumbprint,
    [string]$Subject
  )

  $stores = @("Cert:\CurrentUser\My", "Cert:\LocalMachine\My")
  $normalizedThumbprint = if ($Thumbprint) {
    ($Thumbprint -replace '\s', '').ToUpperInvariant()
  } else {
    $null
  }

  $matching = foreach ($store in $stores) {
    Get-ChildItem $store -CodeSigningCert -ErrorAction SilentlyContinue |
      Where-Object { $_.HasPrivateKey } |
      Where-Object {
        if ($normalizedThumbprint) {
          $_.Thumbprint.ToUpperInvariant() -eq $normalizedThumbprint
        } elseif ($Subject) {
          $_.Subject -like "*$Subject*"
        } else {
          $true
        }
      }
  }

  $certificates = @($matching)
  if ($certificates.Count -eq 0) {
    return $null
  }

  if (-not $normalizedThumbprint -and -not $Subject -and $certificates.Count -gt 1) {
    $subjects = $certificates | ForEach-Object { "$($_.Subject) [$($_.Thumbprint)]" }
    throw "Multiple code-signing certificates with private keys were found. Set STEELCAL_SIGN_CERT_SHA1 or STEELCAL_SIGN_CERT_SUBJECT. Matches: $($subjects -join '; ')"
  }

  if (($normalizedThumbprint -or $Subject) -and $certificates.Count -gt 1) {
    $certificates = $certificates |
      Sort-Object NotAfter -Descending |
      Select-Object -First 1
  }

  return @($certificates)[0]
}

function Get-DefaultTimestampUrl {
  param(
    [string]$ConfiguredUrl
  )

  if ([string]::IsNullOrWhiteSpace($ConfiguredUrl)) {
    return "http://timestamp.digicert.com"
  }

  return $ConfiguredUrl
}

function Get-SignArgs {
  param(
    [string]$ExplicitArgs,
    [System.Security.Cryptography.X509Certificates.X509Certificate2]$Certificate,
    [string]$ConfiguredTimestampUrl
  )

  if (-not [string]::IsNullOrWhiteSpace($ExplicitArgs)) {
    return $ExplicitArgs
  }

  if (-not $Certificate) {
    return $null
  }

  $parts = New-Object System.Collections.Generic.List[string]
  if ($Certificate.PSParentPath -like "*LocalMachine*") {
    $parts.Add("/sm")
  }

  $parts.Add("/sha1")
  $parts.Add($Certificate.Thumbprint)
  $parts.Add("/fd")
  $parts.Add("SHA256")
  $parts.Add("/tr")
  $parts.Add((Get-DefaultTimestampUrl -ConfiguredUrl $ConfiguredTimestampUrl))
  $parts.Add("/td")
  $parts.Add("SHA256")

  return ($parts -join " ")
}

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$buildScript = Join-Path $repoRoot "build_windows.ps1"
$issPath = Join-Path $repoRoot "compile\SimpleSteelCalculator.iss"
$bundleDir = Join-Path $repoRoot "dist\windows\SimpleSteelCalculator"
$outputDir = Join-Path $repoRoot "compile\Output"
$version = Get-AppVersion -RepoRoot $repoRoot

if (!(Test-Path $issPath)) {
  throw "Inno Setup script not found: $issPath"
}

if (!(Test-Path $buildScript)) {
  throw "Build script not found: $buildScript"
}

if (-not $SkipBuild) {
  Write-Host "Building application bundle..." -ForegroundColor Cyan
  & $buildScript
}

if (!(Test-Path $bundleDir)) {
  throw "Build output not found: $bundleDir"
}

$iscc = if ($IsccPath) { Resolve-InnoCompilerPath -Candidate $IsccPath } else { Find-Iscc }
if (-not $iscc) {
  throw "Inno Setup (ISCC.exe) not found. Install Inno Setup 6 and rerun packaging."
}

if (-not $SignToolPath) {
  $SignToolPath = Find-SignTool
}

$signCertificate = Find-CodeSigningCertificate -Thumbprint $SignCertThumbprint -Subject $SignCertSubject
$resolvedSignArgs = Get-SignArgs -ExplicitArgs $SignArgs -Certificate $signCertificate -ConfiguredTimestampUrl $TimestampUrl

if ($SignToolPath -and $resolvedSignArgs) {
  $resolvedSignToolPath = Resolve-CommandPath -Candidate $SignToolPath
  $env:STEELCAL_SIGNTOOL = $resolvedSignToolPath
  $env:STEELCAL_SIGN_ARGS = $resolvedSignArgs
  Write-Host "Installer signing is enabled via Inno Setup SignTool." -ForegroundColor Cyan
  Write-Host "Using SignTool: $resolvedSignToolPath" -ForegroundColor Cyan
  if ($signCertificate) {
    Write-Host "Using signing certificate: $($signCertificate.Subject) [$($signCertificate.Thumbprint)]" -ForegroundColor Cyan
  }
} else {
  Remove-Item Env:STEELCAL_SIGNTOOL -ErrorAction SilentlyContinue
  Remove-Item Env:STEELCAL_SIGN_ARGS -ErrorAction SilentlyContinue
  Write-Warning "Signing is disabled. Configure STEELCAL_SIGN_ARGS explicitly or supply a usable code-signing certificate via STEELCAL_SIGN_CERT_SHA1/STEELCAL_SIGN_CERT_SUBJECT."
}

Write-Host "Using ISCC: $iscc" -ForegroundColor Cyan
$isccArgs = @(
  "/DMyAppVersion=$version",
  "/DBuildOutputDir=$bundleDir"
)

if ($env:STEELCAL_SIGNTOOL -and $env:STEELCAL_SIGN_ARGS) {
  $isccArgs += ('/Ssteelcal=$q{0}$q sign $p' -f $env:STEELCAL_SIGNTOOL)
}

$isccArgs += $issPath
& $iscc @isccArgs

if ($LASTEXITCODE -ne 0) {
  throw "ISCC exited with code $LASTEXITCODE"
}

$setupName = "SimpleSteelCalculator-$version-x64-Setup.exe"
$setupPath = Join-Path $outputDir $setupName

Write-Host "Installer build complete." -ForegroundColor Green
if (Test-Path $setupPath) {
  Write-Host "Installer: $setupPath"
} else {
  Write-Host "Output directory: $outputDir"
}
