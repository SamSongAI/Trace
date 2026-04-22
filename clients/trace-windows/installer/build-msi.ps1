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

function Invoke-TrustedSign {
    <#
    .SYNOPSIS
        Sign a file with Azure Trusted Signing, or skip silently when
        credentials are not configured.

    .DESCRIPTION
        Phase 15 wires Azure Trusted Signing via the `signtool sign /dlib`
        path. All seven ATS knobs are sourced from environment variables
        so that local dev boxes and PR builds (which never see the
        secrets) naturally fall through to unsigned output. The CI
        release workflow (trace-windows-release.yml) wires the secrets in
        as job-level `env:` entries.

        Callers sign the application executable **before** WiX consumes
        it (so the embedded binary on the installed machine is signed)
        and sign the final MSI **after** WiX produces it (so Windows
        SmartScreen / UAC show a verified publisher at install time).
        MSI signatures are separate from inner-payload signatures — both
        calls are required.

        The function throws on signtool non-zero exit when signing was
        attempted; it returns silently when signing is skipped. Temp
        metadata JSON is cleaned up in a `finally` block even on throw.

    .PARAMETER Path
        Absolute path to the file to sign (.exe or .msi).

    .OUTPUTS
        None.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    # Required env list: the three Azure service-principal knobs, the
    # three Trusted Signing account knobs, and the path to the
    # CodeSigning.Dlib.dll (installed via NuGet in CI, documented in
    # installer/README.md for local sign-test). Missing any one means
    # we cannot perform a signing call at all, so skip cleanly.
    $requiredEnv = @(
        'AZURE_TENANT_ID',
        'AZURE_CLIENT_ID',
        'AZURE_CLIENT_SECRET',
        'AZURE_TS_ENDPOINT',
        'AZURE_TS_ACCOUNT_NAME',
        'AZURE_TS_PROFILE_NAME',
        'TRACE_ATS_DLIB'
    )
    $missing = $requiredEnv | Where-Object {
        -not [Environment]::GetEnvironmentVariable($_)
    }
    if ($missing) {
        Write-Information "skipping signing for $Path (missing env: $($missing -join ', '))"
        return
    }

    if (-not (Test-Path $Path)) {
        throw "Invoke-TrustedSign: target does not exist: $Path"
    }

    # The CodeSigning dlib reads the three Trusted Signing knobs from a
    # JSON metadata file (not from CLI args). Write a per-call temp file
    # so concurrent signings on the same box do not clobber each other.
    $metadata = [ordered]@{
        Endpoint               = $env:AZURE_TS_ENDPOINT
        CodeSigningAccountName = $env:AZURE_TS_ACCOUNT_NAME
        CertificateProfileName = $env:AZURE_TS_PROFILE_NAME
    }
    $metadataJson = $metadata | ConvertTo-Json -Depth 3
    $metadataPath = Join-Path ([System.IO.Path]::GetTempPath()) "trace-trusted-signing-$([guid]::NewGuid()).json"
    Set-Content -Path $metadataPath -Value $metadataJson -Encoding utf8

    try {
        Write-Information "signing $Path via Azure Trusted Signing"
        # /dlib -> CodeSigning client DLL (resolved via NuGet restore
        # into $env:TRACE_ATS_DLIB). /dmdf -> metadata file above.
        # /tr   -> Microsoft's RFC 3161 timestamp server that pairs with
        #          ATS; /td / /fd pin hash to SHA-256 (ATS minimum).
        # /v + /debug keep the output verbose so failure diagnosis is
        # tractable from the CI log without another rerun.
        signtool sign `
            /v `
            /debug `
            /fd SHA256 `
            /tr 'http://timestamp.acs.microsoft.com' `
            /td SHA256 `
            /dlib $env:TRACE_ATS_DLIB `
            /dmdf $metadataPath `
            $Path
        if ($LASTEXITCODE -ne 0) {
            throw "signtool exited $LASTEXITCODE while signing $Path"
        }
    } finally {
        Remove-Item -Path $metadataPath -Force -ErrorAction SilentlyContinue
    }
}

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

# --- sign app executable (pre-WiX) -----------------------------------------
# Sign trace-app.exe before WiX embeds it. This way the binary that lands
# in Program Files\Trace\ on the user's disk carries a valid signature —
# not just the MSI wrapper. When the ATS env vars are absent (every local
# build, every PR build), Invoke-TrustedSign prints "skipping signing"
# and returns without error.
Invoke-TrustedSign -Path $BinPath

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

# --- sign MSI (post-WiX) ----------------------------------------------------
# Sign the installer package itself so Windows SmartScreen and UAC show a
# verified publisher at install time. MSI signatures are independent of
# the exe signature above — Windows Installer wraps the payload but does
# not propagate any signature, so both invocations are mandatory for a
# fully-signed distribution.
Invoke-TrustedSign -Path $MsiPath

$size = (Get-Item $MsiPath).Length
Write-Information ("produced {0} ({1:N0} bytes)" -f $MsiPath, $size)
# Emit the MSI path on stdout so CI scripts can capture it with
# `$msi = pwsh build-msi.ps1` or equivalent.
Write-Host $MsiPath
