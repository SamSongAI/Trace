#Requires -Version 7.0
<#
.SYNOPSIS
    Build the Trace Windows MSI installer.

.DESCRIPTION
    Resolves the workspace version from Cargo.toml, compiles
    trace-app in release mode for the requested target triple, and
    invokes `wix build` with all paths relative to this script.

    This script runs on both a local dev box and the GitHub Actions
    windows-latest runner. It assumes:
      * Rust toolchain with the target already installed (`rustup target add`)
      * .NET 8 SDK on PATH
      * WiX v4 global tool installed (`dotnet tool install --global wix`)
      * WiX UI extension added once (`wix extension add -g WixToolset.UI.wixext`)

.PARAMETER Version
    Override the version string embedded in the MSI. When omitted,
    defaults to the workspace-level version in Cargo.toml.

.PARAMETER Arch
    Target architecture. One of `x64` (x86_64-pc-windows-msvc) or
    `arm64` (aarch64-pc-windows-msvc). The release workflow builds
    both; local dev boxes default to `x64` unless told otherwise.

.PARAMETER OutDir
    Destination directory for the MSI. Defaults to
    `<installer>/out/`. Created if absent.

.EXAMPLE
    pwsh installer/build-msi.ps1
    pwsh installer/build-msi.ps1 -Version 0.2.0-rc1 -Arch x64
#>
[CmdletBinding()]
param(
    [string]$Version = '',
    [ValidateSet('x64', 'arm64')]
    [string]$Arch = 'x64',
    [string]$OutDir = ''
)

$ErrorActionPreference = 'Stop'
$InformationPreference = 'Continue'

# $PSScriptRoot is only populated when the script is invoked as a file.
# Dot-sourcing (. ./build-msi.ps1) or pasting the body into an interactive
# session leaves it null, at which point every Join-Path below would silently
# build wrong paths (e.g. "\assets" resolved against the caller's cwd).
# Fail loud instead.
if (-not $PSScriptRoot) {
    throw "build-msi.ps1 must be invoked as a file (e.g. `pwsh installer/build-msi.ps1`), not dot-sourced."
}

$InstallerRoot = $PSScriptRoot
$WorkspaceRoot = Split-Path -Parent $InstallerRoot
$AssetsDir = Join-Path $InstallerRoot 'assets'
$WixDir = Join-Path $InstallerRoot 'wix'

if (-not $OutDir) {
    $OutDir = Join-Path $InstallerRoot 'out'
}

# --- resolve version from Cargo.toml if not overridden ---------------------
# The workspace Cargo.toml keeps a single [workspace.package] version that
# every crate inherits via `version.workspace = true`. Matching from the
# `[workspace.package]` table anchor avoids false-hit matches on any
# downstream [dependencies] entry that happens to carry its own version.
if (-not $Version) {
    $cargoToml = Get-Content (Join-Path $WorkspaceRoot 'Cargo.toml') -Raw
    if ($cargoToml -match '(?ms)\[workspace\.package\].*?^version\s*=\s*"([^"]+)"') {
        $Version = $Matches[1]
    } else {
        throw "could not resolve workspace version from Cargo.toml"
    }
    Write-Information "resolved workspace version: $Version"
}

# --- target triple ----------------------------------------------------------
# Keep these two arms in sync with the `[ValidateSet()]` on $Arch above.
# `wix build -arch <Arch>` accepts `x64` / `arm64` directly, so the WiX
# invocation below does not branch further.
$TargetTriple = switch ($Arch) {
    'x64'   { 'x86_64-pc-windows-msvc' }
    'arm64' { 'aarch64-pc-windows-msvc' }
    default { throw "unsupported arch: $Arch" }
}

# --- cargo build ------------------------------------------------------------
Write-Information "cargo build -p trace-app --release --target $TargetTriple"
Push-Location $WorkspaceRoot
try {
    cargo build -p trace-app --release --target $TargetTriple
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
} finally {
    Pop-Location
}

$BinDir = Join-Path $WorkspaceRoot "target\$TargetTriple\release"
$BinPath = Join-Path $BinDir 'trace-app.exe'
if (-not (Test-Path $BinPath)) {
    throw "expected $BinPath after cargo build"
}

# --- wix build --------------------------------------------------------------
# Ensure LICENSE.rtf and trace.ico are up to date before handing them to wix.
# These scripts are idempotent and produce ASCII / binary that diff-compare
# byte-for-byte on unchanged inputs, so they are safe to run every time.
#
# Use `python` rather than `python3`: on `windows-latest` GitHub runners
# actions/setup-python@v5 only registers `python`, not `python3`, so calling
# the latter errors with "command not found". On a local Windows dev machine
# Python installed from python.org also registers `python`. If a dev prefers
# `python3`, they can still run the asset scripts manually from mac/Linux
# shells — build-msi.ps1 is Windows-only (pwsh-only) by design.
Write-Information "regenerating LICENSE.rtf and trace.ico from sources"
& python (Join-Path $InstallerRoot 'scripts\build-ico.py')
if ($LASTEXITCODE -ne 0) { throw "build-ico.py failed ($LASTEXITCODE)" }
& python (Join-Path $InstallerRoot 'scripts\build-license-rtf.py')
if ($LASTEXITCODE -ne 0) { throw "build-license-rtf.py failed ($LASTEXITCODE)" }

$null = New-Item -ItemType Directory -Path $OutDir -Force
$MsiName = "Trace-$Version-$Arch.msi"
$MsiPath = Join-Path $OutDir $MsiName

Write-Information "wix build -> $MsiPath"
Push-Location $WixDir
try {
    wix build `
        -arch $Arch `
        -d "Version=$Version" `
        -d "BinDir=$BinDir" `
        -d "AssetsDir=$AssetsDir" `
        -ext WixToolset.UI.wixext `
        -o $MsiPath `
        Product.wxs
    if ($LASTEXITCODE -ne 0) { throw "wix build failed ($LASTEXITCODE)" }
} finally {
    Pop-Location
}

if (-not (Test-Path $MsiPath)) {
    throw "wix build claimed success but $MsiPath is missing"
}

$size = (Get-Item $MsiPath).Length
Write-Information ("produced {0} ({1:N0} bytes)" -f $MsiPath, $size)
# Emit the MSI path on stdout so CI scripts can capture it with
# `$msi = pwsh build-msi.ps1` or equivalent.
Write-Host $MsiPath
