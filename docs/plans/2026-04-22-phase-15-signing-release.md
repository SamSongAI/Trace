# Phase 15 — 代码签名 + Release 发布流水线 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 把 Phase 14 产出的 x64 MSI 扩展为双架构（x64 + arm64）、签名（Azure Trusted Signing）、tag-触发的 GitHub Release 自动化产物集（MSI + 便携 ZIP）。

**Architecture:** 以 Phase 14 的 `build-msi.ps1` 为中心扩展，新增 `-Arch arm64` 支路、Azure Trusted Signing 签名封装（环境变量驱动，未配置则跳过）、便携 ZIP 打包；CI 侧新开一份只在 `push tags v*.*.*` 触发的 release workflow，与 Phase 14 的 every-push CI 完全分离；无真实 ATS 凭据时，所有签名代码路径都优雅降级成 unsigned 构建，保证开发期可验证。

**Tech Stack:** PowerShell 7+、WiX v4、Azure Trusted Signing（signtool + Azure.CodeSigning.Dlib）、GitHub Actions（tag trigger + matrix + softprops/action-gh-release）、.NET 8（驱动 signtool）。

---

## 0. 前置决策（已定）

- **架构矩阵**：x64（`x86_64-pc-windows-msvc`）+ arm64（`aarch64-pc-windows-msvc`）。arm64 暂不在 every-push CI，只在 release workflow 跑。
- **签名对象**：`trace-app.exe`（pre-WiX）+ `Trace-<ver>-<arch>.msi`（post-WiX）。ATS 会同时把时间戳注入，不需要额外配置。
- **便携 ZIP 内容**：仅 `trace-app.exe`（已签名）+ 仓库根 `LICENSE`。字体、资源全走 `include_bytes!` 进二进制，无外挂资产。
- **环境变量接口**（secrets）：`AZURE_TENANT_ID` / `AZURE_CLIENT_ID` / `AZURE_CLIENT_SECRET` / `AZURE_TS_ENDPOINT` / `AZURE_TS_ACCOUNT_NAME` / `AZURE_TS_PROFILE_NAME`。全部存在则签名；任一缺失则跳过（本地构建友好）。
- **Release 命名**：`Trace v<version>` 作为 Release 名称；`v<version>` 作为 tag；产物文件名 `Trace-<version>-<arch>.msi` / `Trace-<version>-<arch>-portable.zip`。
- **不做**：CHANGELOG 自动生成（手工维护足够；避免引入 conventional-commits 强约束）、arm64 自动 smoke-install 验证（需要 arm64 runner，下一迭代）、MSIX 平行打包（与 MSI 形态冲突，Phase 15 不处理）。

---

## 1. 仓库相对路径约定

所有路径**相对于仓库根**（`/Users/apple/Desktop/ContextOS/Projects/Trace`），写脚本/工作流时注意：

- 仓库根 CI 工作流在 `.github/workflows/`
- Windows 客户端代码在 `clients/trace-windows/`
- 安装器相关在 `clients/trace-windows/installer/`
- Phase 15 计划 `docs/plans/2026-04-22-phase-15-signing-release.md`（本文件）

---

## Task 15.1: arm64 目标三元组接入 build-msi.ps1

**目标**：让 `build-msi.ps1` 在传 `-Arch arm64` 时，正确解析 `aarch64-pc-windows-msvc` 三元组、向 WiX 传 `-arch arm64`、产出 `Trace-<ver>-arm64.msi`。

**Files:**
- Modify: `clients/trace-windows/installer/build-msi.ps1`（`[ValidateSet(...)]`、target-triple switch、WiX arch 标志）

**Step 1: 检查现状**

Run:
```bash
grep -n "ValidateSet\|TargetTriple\|-arch" clients/trace-windows/installer/build-msi.ps1
```

Expected: 看到 `[ValidateSet('x64')]`、`'x64' { 'x86_64-pc-windows-msvc' }`、`wix build -arch $Arch`。

**Step 2: 扩展 `[ValidateSet()]`**

编辑 `clients/trace-windows/installer/build-msi.ps1` 参数块：

```powershell
[ValidateSet('x64', 'arm64')]
[string]$Arch = 'x64',
```

**Step 3: 扩展 target triple switch**

```powershell
$TargetTriple = switch ($Arch) {
    'x64'   { 'x86_64-pc-windows-msvc' }
    'arm64' { 'aarch64-pc-windows-msvc' }
    default { throw "unsupported arch: $Arch" }
}
```

**Step 4: 更新文件头注释**

把 help block 里 `PARAMETER Arch` 的说明改为：

```
Target architecture. One of `x64` (x86_64-pc-windows-msvc) or
`arm64` (aarch64-pc-windows-msvc). The CI release workflow builds
both; local dev boxes default to `x64` unless told otherwise.
```

删掉之前 "arm64 will be wired in Phase 15" 的 TODO。

**Step 5: 语法自测（mac 也能跑）**

Run:
```bash
pwsh -NoProfile -Command "[System.Management.Automation.Language.Parser]::ParseFile('clients/trace-windows/installer/build-msi.ps1', [ref]\$null, [ref]\$errs); if (\$errs) { \$errs; exit 1 } else { 'parse ok' }"
```

Expected: `parse ok`

**Step 6: 参数校验自测**

Run:
```bash
pwsh -NoProfile -Command "& { param([ValidateSet('x64','arm64')][string]\$a) Write-Host \"arm64 accepted: \$a\" } -a arm64"
```

Expected: `arm64 accepted: arm64`

**Step 7: 提交**

```bash
git add clients/trace-windows/installer/build-msi.ps1
git commit -m "feat(installer): accept -Arch arm64 for aarch64-pc-windows-msvc builds"
```

---

## Task 15.2: Azure Trusted Signing 签名封装

**目标**：在 `build-msi.ps1` 里新增 `Invoke-TrustedSign` 函数；当 6 个 ATS 环境变量都齐全时，依次对 `trace-app.exe`（pre-WiX）和最终 MSI（post-WiX）调用 `signtool sign /dlib` + Azure.CodeSigning.Dlib；任一 env 变量缺失则打印 "skipping signing" 并继续（本地/PR 场景）。

**Files:**
- Modify: `clients/trace-windows/installer/build-msi.ps1`（新增函数 + 调用点 + 新 `-Sign` 参数）

**Step 1: 函数契约**

在 build-msi.ps1 的 `param(...)` 块下方、`$ErrorActionPreference = 'Stop'` 上方，新增一个只处理"读 env + 调 signtool + 报错"的函数。它**不关心**要签谁、什么时候签——那由调用方决定。

函数签名与行为：

```powershell
function Invoke-TrustedSign {
    <#
    .SYNOPSIS
        Sign a file with Azure Trusted Signing, or skip silently if
        credentials are not configured.

    .DESCRIPTION
        Phase 15 wires Azure Trusted Signing via the `signtool sign /dlib`
        path. All six ATS knobs are sourced from environment variables so
        that local dev boxes and PR builds (which never see the secrets)
        naturally fall through to unsigned output. CI wires the secrets in
        via the release workflow.

        Signs exactly one file per invocation. The caller decides ordering
        (sign the exe before WiX consumes it, then sign the resulting MSI).

    .PARAMETER Path
        Absolute path to the file to sign (`.exe` or `.msi`).

    .OUTPUTS
        None. Throws on signtool non-zero exit when signing was attempted.
        Returns silently when signing is skipped.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory=$true)]
        [string]$Path
    )

    $requiredEnv = @(
        'AZURE_TENANT_ID',
        'AZURE_CLIENT_ID',
        'AZURE_CLIENT_SECRET',
        'AZURE_TS_ENDPOINT',
        'AZURE_TS_ACCOUNT_NAME',
        'AZURE_TS_PROFILE_NAME'
    )
    $missing = $requiredEnv | Where-Object { -not [Environment]::GetEnvironmentVariable($_) }
    if ($missing) {
        Write-Information "skipping signing for $Path (missing env: $($missing -join ', '))"
        return
    }

    if (-not (Test-Path $Path)) {
        throw "Invoke-TrustedSign: target does not exist: $Path"
    }

    # Write the ATS dlib metadata JSON next to the target file. signtool
    # reads the six knobs from this JSON, not from command-line args.
    $metadata = [ordered]@{
        Endpoint            = $env:AZURE_TS_ENDPOINT
        CodeSigningAccountName = $env:AZURE_TS_ACCOUNT_NAME
        CertificateProfileName = $env:AZURE_TS_PROFILE_NAME
    }
    $metadataJson = $metadata | ConvertTo-Json -Depth 3
    $metadataPath = Join-Path ([System.IO.Path]::GetTempPath()) "trace-trusted-signing-$([guid]::NewGuid()).json"
    Set-Content -Path $metadataPath -Value $metadataJson -Encoding utf8

    try {
        Write-Information "signing $Path via Azure Trusted Signing"
        # /dlib points at the CodeSigning client DLL (installed via the
        # Microsoft.Trusted.Signing.Client NuGet package, resolved by the
        # release workflow). /dmdf is the metadata we just wrote. /tr is
        # the ATS-provided RFC 3161 timestamp server. /td / /fd pin the
        # hash algorithm to SHA-256 (the ATS minimum).
        signtool sign `
            /v `
            /debug `
            /fd SHA256 `
            /tr "http://timestamp.acs.microsoft.com" `
            /td SHA256 `
            /dlib "$env:TRACE_ATS_DLIB" `
            /dmdf $metadataPath `
            $Path
        if ($LASTEXITCODE -ne 0) {
            throw "signtool exited $LASTEXITCODE while signing $Path"
        }
    } finally {
        Remove-Item -Path $metadataPath -Force -ErrorAction SilentlyContinue
    }
}
```

说明：
- `TRACE_ATS_DLIB` 指向 `Azure.CodeSigning.Dlib.dll` 的绝对路径。CI 里会由 NuGet restore 得到此路径并 `echo "TRACE_ATS_DLIB=..."  >> $GITHUB_ENV`。本地手动测签时用户手动 export。
- 如果 `TRACE_ATS_DLIB` 也未设置但其它 6 个都设了，`signtool` 会直接报错——这是想要的行为（配置错误不能被静默掩盖）。但为保险起见，下一步把这个也加入 `$requiredEnv`。

**Step 2: 把 TRACE_ATS_DLIB 加入 required 列表**

```powershell
$requiredEnv = @(
    'AZURE_TENANT_ID',
    'AZURE_CLIENT_ID',
    'AZURE_CLIENT_SECRET',
    'AZURE_TS_ENDPOINT',
    'AZURE_TS_ACCOUNT_NAME',
    'AZURE_TS_PROFILE_NAME',
    'TRACE_ATS_DLIB'
)
```

**Step 3: 在 cargo build 后签名 .exe**

在 `build-msi.ps1` 里 cargo build 结束、即 `$BinPath` 检查通过、`wix build` 之前，插入：

```powershell
# Sign the application executable before WiX embeds it, so the installed
# binary on disk is signed (users running trace-app.exe directly from
# Program Files also see a valid signature).
Invoke-TrustedSign -Path $BinPath
```

位置：在当前 `Write-Information "regenerating LICENSE.rtf and trace.ico from sources"` 之前。

**Step 4: 在 wix build 后签名 .msi**

紧接 `if (-not (Test-Path $MsiPath)) { throw ... }` 之后、`Write-Information ("produced ...")` 之前：

```powershell
# Sign the MSI so Windows SmartScreen / UAC show a verified publisher.
# MSI signatures are separate from inner-payload (.exe) signatures, so
# both Invoke-TrustedSign calls are required.
Invoke-TrustedSign -Path $MsiPath
```

**Step 5: 语法自测**

Run:
```bash
pwsh -NoProfile -Command "[System.Management.Automation.Language.Parser]::ParseFile('clients/trace-windows/installer/build-msi.ps1', [ref]\$null, [ref]\$errs); if (\$errs) { \$errs; exit 1 } else { 'parse ok' }"
```

Expected: `parse ok`

**Step 6: 跳过分支自测（mac 也能跑）**

在 mac 上以清白环境调用 Invoke-TrustedSign，验证 "skipping signing" 分支。写一个临时 test snippet：

```bash
pwsh -NoProfile -Command @'
. ./clients/trace-windows/installer/build-msi.ps1 -Arch x64 -Version 'test' 2>&1 | Select-String 'skipping signing|parse ok' | Out-Null
'@
```

这里 dot-source 会被 build-msi.ps1 的 `$PSScriptRoot` 防护 throw 挡住——所以换个策略：直接把函数单独复制到一个 temp 脚本里跑。

简化版（用真脚本抽函数）：

```bash
pwsh -NoProfile -Command @'
# Reuse only the Invoke-TrustedSign function; fake env is clean so it
# should print the skip message and return cleanly.
$scriptContent = Get-Content clients/trace-windows/installer/build-msi.ps1 -Raw
$ast = [System.Management.Automation.Language.Parser]::ParseInput($scriptContent, [ref]$null, [ref]$null)
$funcAst = $ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq "Invoke-TrustedSign" }, $true)[0]
if (-not $funcAst) { throw "Invoke-TrustedSign not found" }
Invoke-Expression $funcAst.Extent.Text
"touch" | Out-File -FilePath /tmp/fake-trace.exe
$InformationPreference = "Continue"
Invoke-TrustedSign -Path /tmp/fake-trace.exe
'@
```

Expected: 控制台看到 `skipping signing for /tmp/fake-trace.exe (missing env: AZURE_TENANT_ID, AZURE_CLIENT_ID, AZURE_CLIENT_SECRET, AZURE_TS_ENDPOINT, AZURE_TS_ACCOUNT_NAME, AZURE_TS_PROFILE_NAME, TRACE_ATS_DLIB)`，且命令 exit 0。

**Step 7: 路径不存在分支自测**

```bash
pwsh -NoProfile -Command @'
$scriptContent = Get-Content clients/trace-windows/installer/build-msi.ps1 -Raw
$ast = [System.Management.Automation.Language.Parser]::ParseInput($scriptContent, [ref]$null, [ref]$null)
$funcAst = $ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq "Invoke-TrustedSign" }, $true)[0]
Invoke-Expression $funcAst.Extent.Text
# Seed all 7 envs so we pass the skip check but target file is missing
$env:AZURE_TENANT_ID='a'; $env:AZURE_CLIENT_ID='b'; $env:AZURE_CLIENT_SECRET='c'
$env:AZURE_TS_ENDPOINT='d'; $env:AZURE_TS_ACCOUNT_NAME='e'; $env:AZURE_TS_PROFILE_NAME='f'
$env:TRACE_ATS_DLIB='g'
try {
    Invoke-TrustedSign -Path /no/such/file.exe
    throw "expected throw, got silent success"
} catch {
    if ($_.Exception.Message -match "target does not exist") { "missing-path branch ok" } else { throw }
}
'@
```

Expected: `missing-path branch ok`

**Step 8: 提交**

```bash
git add clients/trace-windows/installer/build-msi.ps1
git commit -m "feat(installer): wire Azure Trusted Signing into build-msi.ps1"
```

---

## Task 15.3: 便携 ZIP 打包

**目标**：在 `build-msi.ps1` 的 WiX+签名流程收尾之后，额外产出 `Trace-<version>-<arch>-portable.zip`，内容为 `trace-app.exe`（已签名，如果启用了签名）+ 仓库根 `LICENSE`。

**Files:**
- Modify: `clients/trace-windows/installer/build-msi.ps1`（新增 portable zip 步骤 + stdout 协议调整）

**Step 1: 在脚本尾部加入 ZIP 打包块**

位置：在 `Write-Information ("produced {0} ({1:N0} bytes)" -f $MsiPath, $size)` 之后、`Write-Host $MsiPath` 之前。

```powershell
# --- portable ZIP ----------------------------------------------------------
# Ships the (possibly signed) exe alongside LICENSE so a user can unzip
# into e.g. %USERPROFILE%\Apps\Trace\ and run trace-app.exe without going
# through Windows Installer. No font/icon side-cars: everything bundles
# into the exe via `include_bytes!` at the Rust crate level.
$ZipName = "Trace-$Version-$Arch-portable.zip"
$ZipPath = Join-Path $OutDir $ZipName
$LicensePath = Join-Path $WorkspaceRoot '..\..\LICENSE'
$LicensePath = [System.IO.Path]::GetFullPath($LicensePath)
if (-not (Test-Path $LicensePath)) {
    throw "LICENSE not found at $LicensePath"
}

# Stage files under a temp dir first so the archive root is clean (exe
# and LICENSE at the top level, no directory wrapper).
$Staging = Join-Path ([System.IO.Path]::GetTempPath()) "trace-portable-$([guid]::NewGuid())"
$null = New-Item -ItemType Directory -Path $Staging -Force
try {
    Copy-Item -Path $BinPath -Destination (Join-Path $Staging 'trace-app.exe')
    Copy-Item -Path $LicensePath -Destination (Join-Path $Staging 'LICENSE')

    if (Test-Path $ZipPath) { Remove-Item -Path $ZipPath -Force }
    Compress-Archive -Path (Join-Path $Staging '*') -DestinationPath $ZipPath
} finally {
    Remove-Item -Path $Staging -Recurse -Force -ErrorAction SilentlyContinue
}

if (-not (Test-Path $ZipPath)) {
    throw "portable zip assembly claimed success but $ZipPath is missing"
}
$zipSize = (Get-Item $ZipPath).Length
Write-Information ("produced {0} ({1:N0} bytes)" -f $ZipPath, $zipSize)
```

**Step 2: stdout 协议更新**

当前结尾：`Write-Host $MsiPath`。扩展为输出两行（MSI + ZIP），CI 可以用 `Select-String '\.msi$'` / `'\.zip$'` 分别取。

```powershell
# Emit artifact paths on stdout. CI captures these via the following
# convention:
#   $lines = pwsh build-msi.ps1 -Arch x64
#   $msi   = $lines | Where-Object { $_ -like '*.msi' }
#   $zip   = $lines | Where-Object { $_ -like '*.zip' }
Write-Host $MsiPath
Write-Host $ZipPath
```

**Step 3: 检查 LICENSE 路径推理**

`$WorkspaceRoot` 在 build-msi.ps1 里是 `clients/trace-windows/`。仓库根 LICENSE 路径为 `$WorkspaceRoot/../../LICENSE`。验证：

Run:
```bash
ls -la LICENSE clients/trace-windows/../../LICENSE
```

Expected: 两条路径指向同一文件。

**Step 4: 语法自测**

```bash
pwsh -NoProfile -Command "[System.Management.Automation.Language.Parser]::ParseFile('clients/trace-windows/installer/build-msi.ps1', [ref]\$null, [ref]\$errs); if (\$errs) { \$errs; exit 1 } else { 'parse ok' }"
```

Expected: `parse ok`

**Step 5: 端到端 ZIP 打包自测（mac 能跑）**

用临时 fixture 验证 `Compress-Archive` 在跨平台 pwsh 下行为正确：

```bash
pwsh -NoProfile -Command @'
$tmp = New-TemporaryFile
Remove-Item $tmp
New-Item -ItemType Directory -Path $tmp.FullName | Out-Null
"fake exe" | Out-File -FilePath (Join-Path $tmp.FullName 'trace-app.exe') -Encoding ascii
"MIT blah blah" | Out-File -FilePath (Join-Path $tmp.FullName 'LICENSE') -Encoding ascii
$zipPath = Join-Path ([System.IO.Path]::GetTempPath()) 'test-portable.zip'
if (Test-Path $zipPath) { Remove-Item $zipPath }
Compress-Archive -Path (Join-Path $tmp.FullName '*') -DestinationPath $zipPath
$entries = (Get-Item $zipPath).FullName | ForEach-Object {
    [System.IO.Compression.ZipFile]::OpenRead($_).Entries.FullName -join ','
}
if ($entries -notmatch 'trace-app\.exe' -or $entries -notmatch 'LICENSE') {
    throw "expected trace-app.exe and LICENSE, got: $entries"
}
"zip contents ok: $entries"
Remove-Item $zipPath -Force
Remove-Item $tmp.FullName -Recurse -Force
'@
```

Expected: `zip contents ok: LICENSE,trace-app.exe`（顺序可能反，但两个都在）。

**Step 6: 提交**

```bash
git add clients/trace-windows/installer/build-msi.ps1
git commit -m "feat(installer): produce portable zip alongside MSI"
```

---

## Task 15.4: Release workflow（tag 触发 + 矩阵 + GitHub Release）

**目标**：新增 `.github/workflows/trace-windows-release.yml`，在 `push tags 'v*.*.*'` 时触发；矩阵 `[x64, arm64]` 各跑一次 `build-msi.ps1`；统一 6 个 ATS secret + 一条 NuGet restore 得到 `TRACE_ATS_DLIB`；把 4 个产物（2 MSI + 2 ZIP）上传到对应 GitHub Release。不改 Phase 14 的 `trace-windows-build.yml`（保持 PR / push-to-main 路径干净）。

**Files:**
- Create: `.github/workflows/trace-windows-release.yml`

**Step 1: 工作流骨架**

完整文件内容（逐字段加注释）：

```yaml
name: trace-windows-release

# Triggered only by tags shaped like v1.2.3 / v0.1.0 / v1.2.3-rc1.
# Pushes to main and PRs are handled by trace-windows-build.yml.
on:
  push:
    tags:
      - 'v*.*.*'

# Release job uploads signed artifacts to the matching GitHub Release.
# Requires write permission on `contents`.
permissions:
  contents: write

defaults:
  run:
    working-directory: clients/trace-windows

jobs:
  build-release:
    name: build ${{ matrix.arch }} MSI + portable
    runs-on: windows-latest
    strategy:
      # Keep going even if one arch blows up; we'd rather publish a
      # partial release than nothing. The final publish step gracefully
      # handles missing arch artifacts by globbing the output directory.
      fail-fast: false
      matrix:
        arch: [x64, arm64]
        include:
          - arch: x64
            triple: x86_64-pc-windows-msvc
          - arch: arm64
            triple: aarch64-pc-windows-msvc
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Rust (stable + ${{ matrix.triple }})
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.triple }}

      - name: Cache Cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: clients/trace-windows -> target
          # Separate cache buckets per arch so x64 / arm64 caches don't
          # thrash each other.
          key: release-${{ matrix.arch }}

      - name: Setup .NET 8 SDK
        uses: actions/setup-dotnet@v4
        with:
          dotnet-version: '8.0.x'

      - name: Install WiX v4 global tool
        run: dotnet tool install --global wix --version 4.0.5

      - name: Add WiX UI extension
        run: wix extension add -g WixToolset.UI.wixext/4.0.5

      - name: Setup Python (for asset regeneration)
        uses: actions/setup-python@v5
        with:
          python-version: '3.11'

      - name: Install Pillow
        run: python -m pip install Pillow==10.*

      # Azure Trusted Signing requires the CodeSigning dlib (a C# DLL)
      # which ships as a NuGet package. `dotnet nuget install` drops it
      # in a predictable path we can echo into GITHUB_ENV for
      # build-msi.ps1 / signtool to find.
      - name: Install Azure Trusted Signing client
        shell: pwsh
        run: |
          $nugetDir = "${{ runner.temp }}/ats-nuget"
          New-Item -ItemType Directory -Path $nugetDir -Force | Out-Null
          nuget install Microsoft.Trusted.Signing.Client `
            -Version 1.0.53 `
            -OutputDirectory $nugetDir `
            -ExcludeVersion
          $dlib = Join-Path $nugetDir `
            "Microsoft.Trusted.Signing.Client/bin/x64/Azure.CodeSigning.Dlib.dll"
          if (-not (Test-Path $dlib)) {
            throw "expected dlib at $dlib after NuGet install"
          }
          "TRACE_ATS_DLIB=$dlib" | Out-File -FilePath $env:GITHUB_ENV -Append

      - name: Build signed MSI + portable ZIP
        shell: pwsh
        env:
          AZURE_TENANT_ID:         ${{ secrets.AZURE_TENANT_ID }}
          AZURE_CLIENT_ID:         ${{ secrets.AZURE_CLIENT_ID }}
          AZURE_CLIENT_SECRET:     ${{ secrets.AZURE_CLIENT_SECRET }}
          AZURE_TS_ENDPOINT:       ${{ secrets.AZURE_TS_ENDPOINT }}
          AZURE_TS_ACCOUNT_NAME:   ${{ secrets.AZURE_TS_ACCOUNT_NAME }}
          AZURE_TS_PROFILE_NAME:   ${{ secrets.AZURE_TS_PROFILE_NAME }}
          # TRACE_ATS_DLIB was exported by the previous step via GITHUB_ENV.
        run: ./installer/build-msi.ps1 -Arch ${{ matrix.arch }}

      - name: Upload matrix artifact
        # Each matrix leg uploads its pair into the same artifact name
        # via unique suffixes; the publish job downloads the lot.
        uses: actions/upload-artifact@v4
        with:
          name: trace-release-${{ matrix.arch }}
          path: |
            clients/trace-windows/installer/out/Trace-*-${{ matrix.arch }}.msi
            clients/trace-windows/installer/out/Trace-*-${{ matrix.arch }}-portable.zip
          if-no-files-found: error
          retention-days: 14

  publish:
    name: publish GitHub Release
    runs-on: ubuntu-latest
    needs: build-release
    # Run only if at least one matrix leg succeeded. `fail-fast: false`
    # on the matrix means a partial set is possible; the publish step
    # globs whatever landed and attaches all of it.
    if: always() && needs.build-release.result != 'cancelled'
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Download all release artifacts
        uses: actions/download-artifact@v4
        with:
          pattern: trace-release-*
          merge-multiple: true
          path: ./release-artifacts

      - name: Derive release version from tag
        id: ver
        run: |
          echo "version=${GITHUB_REF_NAME#v}" >> "$GITHUB_OUTPUT"

      - name: Publish release
        uses: softprops/action-gh-release@v2
        with:
          name: "Trace v${{ steps.ver.outputs.version }}"
          tag_name: ${{ github.ref_name }}
          # Pre-release auto-detect: any tag with a `-` in it (e.g.
          # v0.1.0-rc1) is treated as prerelease.
          prerelease: ${{ contains(github.ref_name, '-') }}
          files: ./release-artifacts/*
          generate_release_notes: true
          fail_on_unmatched_files: true
```

**Step 2: YAML 语法验证**

Run:
```bash
python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/trace-windows-release.yml')); print('yaml ok')"
```

Expected: `yaml ok`

**Step 3: actionlint 验证（可选，若已装）**

Run:
```bash
which actionlint && actionlint .github/workflows/trace-windows-release.yml || echo "actionlint not installed, skipping"
```

Expected: 没有报错，或者 `actionlint not installed, skipping`。

**Step 4: 与 Phase 14 workflow 路径不重叠验证**

Run:
```bash
grep -l "trace-windows-release\|trace-windows-build" .github/workflows/
```

Expected: 两个文件都列出，内容独立。

**Step 5: 提交**

```bash
git add .github/workflows/trace-windows-release.yml
git commit -m "ci(trace-windows): add tag-triggered signed release pipeline"
```

---

## Task 15.5: 发布与签名文档

**目标**：在 `installer/README.md` 里新增一节描述（1）Azure Trusted Signing 一次性配置，（2）打 tag → 产物 → Release 的全流程，（3）签名验证手工步骤。并在 manual QA checklist 里追加"签名产物"小节。

**Files:**
- Modify: `clients/trace-windows/installer/README.md`

**Step 1: 在 `## CI 产物` 之后新增 `## Release 发布流水线` 小节**

在文件末尾、`## CI 产物` 段落之后追加（保留 `## CI 产物` 原文，因为它描述的是 Phase 14 的 every-push 行为，仍然有效）：

```markdown
## Release 发布流水线（Phase 15）

`trace-windows-release.yml` 负责对齐 tag 的正式发布：`push tags
v*.*.*` 会并行构建 x64 与 arm64 签名 MSI + 便携 ZIP，并自动发布到
GitHub Releases。

### 一次性准备：Azure Trusted Signing

1. Azure 订阅下创建一个 **Trusted Signing Account**（资源类型
   `Microsoft.CodeSigning/codeSigningAccounts`）。
2. 在账户下完成 **Identity Validation**（个人作者走 `publicTrust`，
   审核周期几分钟到几小时）。
3. 创建一个 **Certificate Profile**（记下名字，例如
   `TraceAuthorCert`）。
4. 创建一个 **Service Principal** 并授权其 `Trusted Signing Certificate
   Profile Signer` 角色；记下 tenant id / client id / client secret。
5. 在仓库 Settings → Secrets → Actions 添加 6 个 secret：
   - `AZURE_TENANT_ID`
   - `AZURE_CLIENT_ID`
   - `AZURE_CLIENT_SECRET`
   - `AZURE_TS_ENDPOINT`（例如 `https://eus.codesigning.azure.net/`）
   - `AZURE_TS_ACCOUNT_NAME`
   - `AZURE_TS_PROFILE_NAME`

**任一 secret 缺失时，release workflow 仍然会构建，但产物不会签名**
——`build-msi.ps1` 的 `Invoke-TrustedSign` 函数在检测到环境变量缺失
时会打印 `skipping signing` 并继续。本地开发机不配置这些变量即可。

### 打 tag 发布

```bash
# 1. 确认 Cargo.toml 的 workspace version 已 bump 到目标版本
grep '^version' clients/trace-windows/Cargo.toml | head -1

# 2. 本地打 tag
git tag v0.2.0
git push origin v0.2.0

# 3. 观察 Actions 页面，等待 trace-windows-release workflow 结束
#    （x64 + arm64 两条腿独立，一条失败另一条仍会发布）

# 4. 打开 Releases 页面核对：
#    Trace-0.2.0-x64.msi
#    Trace-0.2.0-x64-portable.zip
#    Trace-0.2.0-arm64.msi
#    Trace-0.2.0-arm64-portable.zip
```

预发布 tag（带 `-`，如 `v0.2.0-rc1`）会自动标记为 Pre-release。

### 验证签名

在 Windows 机器上：

```powershell
# signtool 在 "Windows Kits\10\bin\<ver>\x64\signtool.exe" 下
signtool verify /pa /all Trace-0.2.0-x64.msi
signtool verify /pa /all trace-app.exe   # 从 MSI 解出或从 ZIP 解出
```

输出应该包含 `Successfully verified` 和时间戳信息（来自
`timestamp.acs.microsoft.com`）。

或者在 Explorer 里右键 MSI → 属性 → 数字签名，看到 "Sam Song"（通过
Microsoft Identity Verification CA 颁发的证书）。
```

**Step 2: 在 manual QA checklist 末尾追加"签名产物"小节**

紧接在 `### 降级（反例）` 之后、`## CI 产物` 之前插入：

```markdown
### 签名产物（仅 Release workflow 产物）

- [ ] `signtool verify /pa /all Trace-<ver>-x64.msi` 返回 Successfully
      verified
- [ ] MSI 属性对话框 → 数字签名 → 看到 Sam Song 签名 + ACS 时间戳
- [ ] UAC 弹窗显示 "已验证的发布者：Sam Song"（不是"未知发布者"）
- [ ] SmartScreen 不再弹"Windows 已保护你的电脑"警告（可能需要少量
      下载量后 Microsoft reputation 才完全建立，新证书前几次安装可能
      仍会被拦，但签名有效性已建立）
- [ ] 解开 portable.zip 后 `trace-app.exe` 属性对话框也带签名
- [ ] arm64 MSI 在 Windows on ARM 机器 / VM 里安装成功（可选，视硬
      件条件；至少在 x64 机器上 `msiexec /a Trace-<ver>-arm64.msi /qn
      TARGETDIR=C:\tmp\arm64-probe` 展开无报错）
```

**Step 3: 在文件顶部目录结构图里补充 release workflow 引用**

在 `## 目录结构` 的 Markdown 树下方已有 `installer/` 子树描述；紧跟
其后加一小句（不修改原树图本身）：

```markdown
Release 发布通过仓库根的 `.github/workflows/trace-windows-release.yml`
驱动，见下方 [Release 发布流水线](#release-发布流水线phase-15) 一节。
```

**Step 4: 提交**

```bash
git add clients/trace-windows/installer/README.md
git commit -m "docs(installer): document Azure Trusted Signing + release flow"
```

---

## Task 15.6: Phase 15 holistic final review

**目标**：以全量 diff（Phase 15 新增的 5 次提交合起来）为基础，跑一次 spec+quality 综合 review，抓跨任务问题（CI ↔ 本地脚本的环境变量协议、命名一致性、文档覆盖率）。

**Files:**
- 无直接修改，由 reviewer 结果决定是否回填修复 commit

**Step 1: 汇总 Phase 15 提交列表**

Run:
```bash
git log --oneline b67d416..HEAD -- clients/trace-windows/installer/ .github/workflows/trace-windows-release.yml docs/plans/2026-04-22-phase-15-signing-release.md
```

Expected: 看到本 Phase 的提交（plan、15.1、15.2、15.3、15.4、15.5）。

**Step 2: 对照本计划 §0 前置决策逐条 check**

- [ ] 架构矩阵 x64+arm64 在 build-msi.ps1 的 `[ValidateSet()]` 与 CI matrix 都落实
- [ ] 6+1 签名环境变量在 build-msi.ps1 与 release.yml env 块一致
- [ ] 便携 ZIP 只含 trace-app.exe + LICENSE
- [ ] Release 产物命名 `Trace-<ver>-<arch>.msi` / `Trace-<ver>-<arch>-portable.zip` 前后一致
- [ ] CHANGELOG 自动生成、arm64 smoke install、MSIX 均明确未做

**Step 3: 签名跳过分支手动重走**

Run:
```bash
pwsh -NoProfile -Command @'
$scriptContent = Get-Content clients/trace-windows/installer/build-msi.ps1 -Raw
$ast = [System.Management.Automation.Language.Parser]::ParseInput($scriptContent, [ref]$null, [ref]$null)
$funcAst = $ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq "Invoke-TrustedSign" }, $true)[0]
Invoke-Expression $funcAst.Extent.Text
"x" | Out-File /tmp/phase15-review.exe
$InformationPreference = "Continue"
Invoke-TrustedSign -Path /tmp/phase15-review.exe
Remove-Item /tmp/phase15-review.exe
'@
```

Expected: 输出含 `skipping signing`，exit 0。

**Step 4: 触发全量 YAML / PowerShell parse**

Run:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/trace-windows-release.yml')); yaml.safe_load(open('.github/workflows/trace-windows-build.yml'))" && \
pwsh -NoProfile -Command "[System.Management.Automation.Language.Parser]::ParseFile('$(pwd)/clients/trace-windows/installer/build-msi.ps1', [ref]\$null, [ref]\$errs); if (\$errs) { \$errs | ForEach-Object { Write-Error \$_ }; exit 1 } else { Write-Host 'parse ok' }"
```

Expected: 两个 yaml 无异常 + `parse ok`。

**Step 5: 派发 `superpowers:code-reviewer` 做最终审阅**

用自检拉起 reviewer subagent，要点：
- spec 对齐 §0 前置决策
- 签名逻辑在 7 个 env 齐、缺失任一、目标文件不存在三种分支的行为
- ZIP 内容 = trace-app.exe + LICENSE，无多余
- release workflow 的 secrets 与 build-msi.ps1 的 env 变量名完全一致
- 文档（installer/README.md）对齐实际脚本/工作流行为

**Step 6: 根据 reviewer 反馈决定**

- reviewer 返回 ✅：Phase 15 关闭，todo 列表 Phase 15 打完标
- reviewer 返回 ❌：按反馈回填 fix commit，循环再 review

---

## 附录 A：全部涉及文件速查

| 文件 | 动作 | 任务 |
|------|------|------|
| `clients/trace-windows/installer/build-msi.ps1` | modify | 15.1, 15.2, 15.3 |
| `.github/workflows/trace-windows-release.yml` | create | 15.4 |
| `clients/trace-windows/installer/README.md` | modify | 15.5 |
| `docs/plans/2026-04-22-phase-15-signing-release.md` | create（本文件） | 计划 |

**不改**：
- `clients/trace-windows/installer/wix/Product.wxs`（WiX v4 的 `wix build -arch` 直接接受 `x64` / `arm64`；无条件分支需要）
- `.github/workflows/trace-windows-build.yml`（Phase 14 的 every-push 工作流独立存在）
- `clients/trace-windows/installer/scripts/*.py`（资产生成脚本与架构无关）
- `clients/trace-windows/installer/assets/*`（已生成的 ICO / RTF 与架构无关）

---

## 附录 B：失败场景手册

| 症状 | 可能原因 | 处置 |
|------|---------|------|
| release workflow 在 `Install Azure Trusted Signing client` 步骤失败 | `Microsoft.Trusted.Signing.Client` NuGet 版本号被 Microsoft 下架 | 升级 `-Version` 参数到最新稳定版，重新发布 tag |
| `signtool sign /dlib` 报错 `AccessDenied` | Service Principal 角色分配丢失 | 重新在 Trusted Signing Account 的 IAM 里给 SPN 分配 `Trusted Signing Certificate Profile Signer` |
| SmartScreen 仍弹"未知发布者" | 证书 reputation 尚未建立 | 不是 bug；下载几百次后 Microsoft reputation 会自动建立 |
| arm64 cargo build 失败 | 某 native dep 无 arm64 预编译产物 | 单腿失败 release 仍会发布 x64；debug 时在本地 `rustup target add aarch64-pc-windows-msvc && cargo build --target aarch64-pc-windows-msvc -p trace-app --release` 复现 |
| ZIP 缺 LICENSE | `$WorkspaceRoot/../../LICENSE` 路径假设错位 | 检查仓库根是否还有 LICENSE；如果项目根重构过，更新 `$LicensePath` 派生规则 |

---

*End of plan*
