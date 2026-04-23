# Phase 16 — WiX Burn EXE Bundle Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 把 Phase 15 产出的 MSI 包进一个 WiX Burn Bundle，产出主流 `.exe` 安装包形态 `Trace-Setup-<ver>-<arch>.exe`，降低普通 Windows 用户"这文件不是 exe，是不是病毒"的心智门槛。MSI 与便携 ZIP 保留为次级产物，不拆链路。

**Architecture:** 在 `installer/wix/` 下新增 `Bundle.wxs`（WiX Burn 声明），用 `WixStandardBootstrapperApplication`（Bal 扩展提供的零代码 BA）串起单条 `MsiPackage` Chain。`build-msi.ps1` 在 MSI + 其签名完成之后多跑一步 `wix build Bundle.wxs` 产出 `Setup.exe`，按 WiX Burn 标准流程 `detach → sign engine → reattach → sign Setup.exe` 对引擎与外壳各签一次。整个 Phase 15 的 6+1 ATS 环境变量协议、跳过分支、版本推导都原样复用；`build-msi.ps1` 的 stdout 从两行（MSI + ZIP）扩到三行（MSI + ZIP + Setup.exe）。release workflow 多加一条 Bal 扩展安装步骤，artifact glob 多收一个 Setup.exe 文件模式。

**Tech Stack:** WiX v4 Burn + `WixToolset.Bal.wixext` 4.0.5、PowerShell 7+、Azure Trusted Signing（复用 Phase 15 封装好的 `Invoke-TrustedSign`）、GitHub Actions（tag trigger + matrix，复用 Phase 15 结构）。

---

## 0. 前置决策（已定）

- **主产物形态切到 `.exe`**：`Trace-Setup-<ver>-<arch>.exe`（Burn bootstrapper）成为 Release 的第一顺位产物；MSI 继续产出，作为"给 IT 部门用 msiexec 批量部署 / 给熟悉 Windows Installer 的人手工安装"的次级形态；便携 ZIP 不动。主流开源 Windows 项目（VS Code、OBS、Joplin、Notepad++、RustDesk、Logseq、Alacritty 7/9）首选 exe，少数（Flameshot、Spacedrive）只出 msi——对目标用户群（普通中文桌面用户），exe 更可识别。
- **Bundle UpgradeCode 独立**：`B98D8477-7730-4BC5-B177-8E00DC5C7DD0`（已生成，距 MSI 的 `4F0FC3A3-C718-4DD4-BB01-0351E9960E8C` 完全隔离）。Burn Bundle 与内部 MSI 是两个独立的 Windows Installer 身份——Bundle 负责 "ARP 条目 + 外壳 UX + 升级链"，MSI 负责 "物理文件安装"。两者 UpgradeCode 同值会让 Windows Installer 分不清是升级哪一个。
- **ARP 只露 Bundle 不露 MSI**：Chain 里 `<MsiPackage Visible="no">`。用户"控制面板 → 程序与功能"只看到一条 Trace 条目（版本号 = Bundle 版本号）。卸载 Bundle 会连带卸载内部 MSI。
- **BA 选 WixStandardBootstrapperApplication**：`HyperlinkLicense` 变体（RTF EULA 页 + 安装进度 + 完成页，零 C# / 零 WPF），最少代码量。UI 文本（标题、按钮、EULA）走 WiX 的 `WixStandardBootstrapperApplication` 默认本地化 + 我们在 Bundle.wxs 里传入的 `LicenseFile`、`LogoFile` override。
- **签名链三段**：`trace-app.exe`（pre-WiX-MSI）→ `Trace-<ver>-<arch>.msi`（post-WiX-MSI）→ Burn engine.exe（detach 后）→ `Trace-Setup-<ver>-<arch>.exe`（reattach 后）。前两段是 Phase 15 已经在跑的；Phase 16 新增后两段。Burn engine 必须先签才能 reattach，否则"外壳签名 + 内部未签引擎"在 SmartScreen 眼里是半签名、仍弹"未知发布者"。
- **ATS 环境变量完全沿用 Phase 15**：7 个 env（6 个 Azure + 1 个 `TRACE_ATS_DLIB`）都齐 → 全链路签名；任一缺失 → `Invoke-TrustedSign` 对 4 次调用全部走 "skipping signing" 分支，Setup.exe 仍然能产出（未签名，本地验证用）。
- **版本号同源**：Bundle `Version="$(Version)"` 与 MSI `Version="$(Version)"` 读同一个 `$Version`（来自 Cargo.toml `[workspace.package]`）。保持 Bundle ARP 显示的版本号与 MSI 文件名、Rust crate 版本完全一致。
- **不做**：独立的 Burn ManagedBootstrapperApplication（WPF/C#，维护成本高；`WixStandardBootstrapperApplication` 足够）、.NET Framework 依赖检测 / 安装（Trace 是纯 Rust，无 .NET 运行时依赖）、MSIX（与 Bundle 定位冲突）、Bundle 跨版本 per-user / per-machine 切换（保持 perMachine，沿用 MSI 策略）。

---

## 1. 仓库相对路径约定

所有路径**相对于仓库根**（`/Users/apple/Desktop/ContextOS/Projects/Trace`）：

- Phase 15 MSI 声明：`clients/trace-windows/installer/wix/Product.wxs`（不改）
- Phase 16 Bundle 声明：`clients/trace-windows/installer/wix/Bundle.wxs`（新增）
- 构建脚本：`clients/trace-windows/installer/build-msi.ps1`（扩）
- Release workflow：`.github/workflows/trace-windows-release.yml`（扩）
- 安装器 README：`clients/trace-windows/installer/README.md`（改）
- 本计划：`docs/plans/2026-04-23-phase-16-exe-bundle.md`（本文件）

---

## Task 16.1: 创建 Bundle.wxs（WiX Burn Bundle 声明）

**目标**：写一份最小可编的 `Bundle.wxs`，用 `WixStandardBootstrapperApplication/HyperlinkLicense` 作为 BA，Chain 只含一个 `MsiPackage`（指向同 build 产出的 `Trace-<ver>-<arch>.msi`）。UpgradeCode 硬编码为 Phase 16 新生成的 GUID。

**Files:**
- Create: `clients/trace-windows/installer/wix/Bundle.wxs`

**Step 1: 写 Bundle.wxs**

完整文件内容（逐段注释）：

```xml
<?xml version="1.0" encoding="utf-8"?>
<!--
  Trace Windows client Bundle (Phase 16) — WiX v4 Burn bootstrapper.

  Build (delegated by build-msi.ps1, not run by hand):
    wix build -arch x64 ^
      -d Version=0.1.0 ^
      -d AssetsDir=..\assets ^
      -d MsiPath=..\out\Trace-0.1.0-x64.msi ^
      -ext WixToolset.Bal.wixext ^
      -o ..\out\Trace-Setup-0.1.0-x64.exe Bundle.wxs

  ===========================================================================
  UpgradeCode is FIXED for the lifetime of this Bundle, and DISTINCT from
  the MSI's UpgradeCode (4F0FC3A3-C718-4DD4-BB01-0351E9960E8C). Burn treats
  a Bundle as its own installer identity — share UpgradeCodes and Windows
  Installer can't tell whether a given upgrade applies to the MSI or the
  Bundle, leading to orphaned ARP entries and broken major-upgrade chains.

  Generated 2026-04-23 — DO NOT EDIT the GUID below.
  ===========================================================================
-->
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs"
     xmlns:bal="http://wixtoolset.org/schemas/v4/wxs/bal">
  <Bundle
      Name="Trace"
      Manufacturer="Sam Song"
      Version="$(Version)"
      UpgradeCode="B98D8477-7730-4BC5-B177-8E00DC5C7DD0"
      IconSourceFile="$(AssetsDir)\trace.ico"
      AboutUrl="https://github.com/SamSong1997/Trace"
      HelpUrl="https://github.com/SamSong1997/Trace">

    <!--
      BootstrapperApplication: WixStandardBootstrapperApplication is the
      zero-code BA ship-built with Bal. HyperlinkLicense variant gives us
      a Welcome page with inline EULA link, plus a progress page and an
      exit page — no WPF, no C#, just XML config.

      LicenseFile is rendered via the RTF we already generate for the MSI
      EULA, so the bundle EULA and MSI EULA are one document.
    -->
    <BootstrapperApplication>
      <bal:WixStandardBootstrapperApplication
          Theme="hyperlinkLicense"
          LicenseFile="$(AssetsDir)\LICENSE.rtf"
          LogoFile="$(AssetsDir)\trace.ico"
          SuppressOptionsUI="yes"
          SuppressRepair="yes" />
    </BootstrapperApplication>

    <!--
      Chain: the MSI is the one and only package. Visible="no" keeps it
      from registering its own ARP entry, so the user sees exactly one
      "Trace" row in Programs and Features (the Bundle's own entry,
      versioned the same as the MSI since both read $(Version)).

      Cache="keep" so Windows Installer cache holds onto the MSI after
      install — this is what makes the Bundle's uninstall path work
      without re-downloading. DisplayInternalUI="no" because the Bundle
      is the sole UI driver; we don't want the MSI's WixUI_InstallDir
      popping up inside the Burn progress page.

      Vital="yes" means if the MSI install fails, the Bundle fails too
      (default, but stated for clarity).
    -->
    <Chain>
      <MsiPackage
          Id="TraceMsi"
          SourceFile="$(MsiPath)"
          Visible="no"
          Cache="keep"
          Compressed="yes"
          DisplayInternalUI="no"
          Vital="yes" />
    </Chain>
  </Bundle>
</Wix>
```

设计要点：

- `bal:WixStandardBootstrapperApplication` 的 `Theme` 有 `hyperlinkLicense` / `rtfLicense` / `hyperlinkSidebarLicense` 几种。`hyperlinkLicense` 最简单——单页欢迎 + 许可证超链接，不额外占对话框宽度。
- `SuppressOptionsUI="yes"`：我们不提供"更改安装目录"按钮（普通用户无需；想改目录的走 `msiexec /i Trace-*.msi TARGETDIR=...` 路径）。
- `SuppressRepair="yes"`：匹配 Phase 14 MSI 的 `ARPNOREPAIR=1` 策略，保持 Bundle 与 MSI 行为一致。
- `Compressed="yes"`：把 MSI 直接打进 Setup.exe（单文件分发）；设 `no` 的话 Setup.exe 会在运行时找同目录外挂 MSI，UX 差。
- `Cache="keep"`：Burn 默认会把 MSI 缓存进 `%ProgramData%\Package Cache\`，用来支持卸载。显式声明避免未来 WiX 改默认值。
- `MsiPath` 作为 `-d` 变量由 `build-msi.ps1` 传入（即 `installer/out/Trace-<ver>-<arch>.msi` 的绝对路径），而不是硬编码路径，避免和 Product.wxs 一样被 `$(BinDir)` 风格调用参数。

**Step 2: WiX XML schema 离线校验（mac 也能跑）**

WiX 4 的 schema 发布在 `http://wixtoolset.org/schemas/v4/wxs`。本地 xmllint 无法联网解析 schema，但能做基本 well-formed 校验：

Run:
```bash
xmllint --noout clients/trace-windows/installer/wix/Bundle.wxs && echo "xml ok"
```

Expected: `xml ok`

**Step 3: GUID 唯一性复核**

Run:
```bash
grep -r "B98D8477-7730-4BC5-B177-8E00DC5C7DD0\|4F0FC3A3-C718-4DD4-BB01-0351E9960E8C" clients/trace-windows/installer/wix/
```

Expected:
- `Product.wxs` 里只有 MSI 的 GUID；
- `Bundle.wxs` 里只有 Bundle 的 GUID；
- 两者不交叉。

**Step 4: Bundle.wxs 与 Product.wxs 的版本变量一致性**

Run:
```bash
grep -n "Version=\"\$(Version)\"" clients/trace-windows/installer/wix/*.wxs
```

Expected: `Product.wxs` 与 `Bundle.wxs` 各有一行，写法完全一致。

**Step 5: 提交**

```bash
git add clients/trace-windows/installer/wix/Bundle.wxs
git commit -m "feat(installer): add WiX Burn bundle (Trace-Setup-<ver>-<arch>.exe)"
```

---

## Task 16.2: build-msi.ps1 扩展 Bundle 构建 + 引擎签名三段式

**目标**：在 MSI 已签名之后、Compress-Archive 之前，新增 Bundle 构建块。流程：`wix build Bundle.wxs` → `wix burn detach` 抽出未签引擎 → `Invoke-TrustedSign` 签引擎 → `wix burn reattach` 把签名引擎合回 Setup.exe → `Invoke-TrustedSign` 签外壳 Setup.exe。stdout 多打一行 Setup.exe 路径。

**Files:**
- Modify: `clients/trace-windows/installer/build-msi.ps1`

**Step 1: 定位插入点**

在当前脚本里找 `# --- portable zip ----------------------------------------------------------`（第 265 行附近），Bundle 构建块插在它**之前**；portable ZIP 产物不依赖 Bundle，但时间顺序上 Bundle 先完成方便上游 CI 把 Setup.exe 也塞进同目录。

**Step 2: 加入 Bundle 构建 + 签名三段式**

在 `Invoke-TrustedSign -Path $MsiPath` 后一行（即"MSI 签完"之后）、portable zip 段落之前，插入：

```powershell
# --- wix build bundle (Trace-Setup-<ver>-<arch>.exe) -----------------------
# WiX Burn Bundle wraps the MSI into a standalone Setup.exe. Users who
# double-click Trace-Setup-*.exe get the same experience as clicking an
# MSI (WixStandardBootstrapperApplication/HyperlinkLicense renders the
# same EULA -> progress -> done flow), except the file extension says
# ".exe" — which matters because ".msi is also a real installer" is
# non-obvious to non-technical Windows users.
#
# The Bundle references the already-built MSI as an absolute path via
# the `MsiPath` preprocessor variable. Keeping this indirection (not
# hard-coding the path in Bundle.wxs) lets us change $MsiName layout
# in the future without touching WiX sources.
$SetupName = "Trace-Setup-$Version-$Arch.exe"
$SetupPath = Join-Path $OutDir $SetupName

Write-Information "wix build -> $SetupPath"
Push-Location $WixDir
try {
    wix build `
        -arch $Arch `
        -d "Version=$Version" `
        -d "AssetsDir=$AssetsDir" `
        -d "MsiPath=$MsiPath" `
        -ext WixToolset.Bal.wixext `
        -o $SetupPath `
        Bundle.wxs
    if ($LASTEXITCODE -ne 0) { throw "wix build Bundle.wxs failed ($LASTEXITCODE)" }
} finally {
    Pop-Location
}

if (-not (Test-Path $SetupPath)) {
    throw "wix build claimed success but $SetupPath is missing"
}

# --- sign Burn engine + Setup.exe (post-Bundle) ----------------------------
# WiX Burn bootstrappers are two parts: an outer Setup.exe shell and an
# inner "engine" PE that does the real orchestration. signtool can sign
# the outer shell directly, but the inner engine needs the Burn-standard
# detach -> sign -> reattach dance or else SmartScreen / UAC will see
# half-signed output.
#
# `wix burn detach` extracts the unsigned engine.exe next to Setup.exe,
# we sign that with the same ATS function used for the app exe and MSI,
# then `wix burn reattach` splices the signed engine back into Setup.exe.
# Finally signtool runs once more to sign the outer shell.
#
# If ATS env vars are missing (local build, PR), Invoke-TrustedSign skips
# both calls — the engine/shell stay unsigned but the triple still
# produces a functional (unsigned) Setup.exe.
$EnginePath = Join-Path $OutDir "engine-$Version-$Arch.exe"
if (Test-Path $EnginePath) { Remove-Item -Path $EnginePath -Force }

Write-Information "wix burn detach -> $EnginePath"
wix burn detach $SetupPath -engine $EnginePath
if ($LASTEXITCODE -ne 0) { throw "wix burn detach failed ($LASTEXITCODE)" }
if (-not (Test-Path $EnginePath)) {
    throw "wix burn detach claimed success but $EnginePath is missing"
}

Invoke-TrustedSign -Path $EnginePath

Write-Information "wix burn reattach $SetupPath"
wix burn reattach $SetupPath -engine $EnginePath -o $SetupPath
if ($LASTEXITCODE -ne 0) { throw "wix burn reattach failed ($LASTEXITCODE)" }

# The detached engine is no longer needed once reattach succeeds; delete
# it so it doesn't accidentally get uploaded as a release artifact.
Remove-Item -Path $EnginePath -Force -ErrorAction SilentlyContinue

Invoke-TrustedSign -Path $SetupPath

$setupSize = (Get-Item $SetupPath).Length
Write-Information ("produced {0} ({1:N0} bytes)" -f $SetupPath, $setupSize)
```

**Step 3: 扩展尾部 stdout 协议到三行**

脚本末尾当前是两行 `Write-Host`（MSI + ZIP）。改为三行（MSI + ZIP + Setup.exe），并把注释里的捕获约定同步更新：

```powershell
# Emit artifact paths on stdout, one per line, so CI can capture them:
#   $lines = pwsh build-msi.ps1 -Arch x64
#   $msi   = $lines | Where-Object { $_ -like '*.msi' }
#   $zip   = $lines | Where-Object { $_ -like '*-portable.zip' }
#   $setup = $lines | Where-Object { $_ -like 'Trace-Setup-*.exe' }
# Keep MSI first for back-compat with callers that only grabs $lines[0].
Write-Host $MsiPath
Write-Host $ZipPath
Write-Host $SetupPath
```

**Step 4: 语法自测**

Run:
```bash
pwsh -NoProfile -Command "[System.Management.Automation.Language.Parser]::ParseFile('clients/trace-windows/installer/build-msi.ps1', [ref]\$null, [ref]\$errs); if (\$errs) { \$errs; exit 1 } else { 'parse ok' }"
```

Expected: `parse ok`

**Step 5: dry-run 场景自测（mac 能跑的那部分）**

mac 上 `wix build` / `wix burn` 都跑不动（WiX 是 .NET 工具，但依赖 Windows 文件格式），所以不做端到端。只验：

- `$SetupName` 格式正确（用 pwsh 字符串插值验证）
- `$EnginePath` 清理逻辑（存在 → Remove-Item → 不存在也 OK）

Run:
```bash
pwsh -NoProfile -Command @'
$Version = "0.1.0"
$Arch = "x64"
$OutDir = "/tmp/trace-setup-dry"
New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
$SetupName = "Trace-Setup-$Version-$Arch.exe"
$SetupPath = Join-Path $OutDir $SetupName
$EnginePath = Join-Path $OutDir "engine-$Version-$Arch.exe"
"fake-setup"  | Out-File $SetupPath  -Encoding ascii
"fake-engine" | Out-File $EnginePath -Encoding ascii
if (Test-Path $EnginePath) { Remove-Item -Path $EnginePath -Force }
if (Test-Path $EnginePath) { throw "engine cleanup failed" }
if (-not (Test-Path $SetupPath)) { throw "setup unexpectedly gone" }
Remove-Item $OutDir -Recurse -Force
"dry-run ok: names produce as Trace-Setup-0.1.0-x64.exe / engine-0.1.0-x64.exe"
'@
```

Expected: `dry-run ok: names produce as Trace-Setup-0.1.0-x64.exe / engine-0.1.0-x64.exe`

**Step 6: 签名跳过分支复验（Phase 15 测过一次，这次 4 个签名点合一起再跑）**

Run:
```bash
pwsh -NoProfile -Command @'
$scriptContent = Get-Content clients/trace-windows/installer/build-msi.ps1 -Raw
$ast = [System.Management.Automation.Language.Parser]::ParseInput($scriptContent, [ref]$null, [ref]$null)
$funcAst = $ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq "Invoke-TrustedSign" }, $true)[0]
if (-not $funcAst) { throw "Invoke-TrustedSign not found" }
Invoke-Expression $funcAst.Extent.Text
$InformationPreference = "Continue"
"fake-exe"    | Out-File /tmp/p16-app.exe
"fake-msi"    | Out-File /tmp/p16.msi
"fake-engine" | Out-File /tmp/p16-engine.exe
"fake-setup"  | Out-File /tmp/p16-setup.exe
Invoke-TrustedSign -Path /tmp/p16-app.exe
Invoke-TrustedSign -Path /tmp/p16.msi
Invoke-TrustedSign -Path /tmp/p16-engine.exe
Invoke-TrustedSign -Path /tmp/p16-setup.exe
Remove-Item /tmp/p16-app.exe, /tmp/p16.msi, /tmp/p16-engine.exe, /tmp/p16-setup.exe
'@
```

Expected: 看到 4 条 `skipping signing for ...` 输出，exit 0。

**Step 7: 提交**

```bash
git add clients/trace-windows/installer/build-msi.ps1
git commit -m "feat(installer): build signed Trace-Setup-<ver>-<arch>.exe via WiX Burn"
```

---

## Task 16.3: release workflow 加 Bal 扩展 + 扩展 artifact glob

**目标**：`trace-windows-release.yml` 需要多跑一条 `wix extension add -g WixToolset.Bal.wixext/4.0.5`（Bundle.wxs 引用了 Bal 命名空间，没有这个扩展 `wix build` 直接报 `Unknown element bal:WixStandardBootstrapperApplication`）；upload-artifact 的 glob 要多收 Setup.exe。

**Files:**
- Modify: `.github/workflows/trace-windows-release.yml`

**Step 1: 在 `Add WiX UI extension` 步骤之后新增 Bal 扩展**

当前工作流第 67-68 行：

```yaml
      - name: Add WiX UI extension
        run: wix extension add -g WixToolset.UI.wixext/4.0.5
```

紧接其后插入：

```yaml
      - name: Add WiX Bal extension
        # Bundle.wxs references bal:WixStandardBootstrapperApplication, so
        # the WixToolset.Bal.wixext package must be globally added to the
        # `wix` tool before build-msi.ps1 invokes `wix build Bundle.wxs`.
        run: wix extension add -g WixToolset.Bal.wixext/4.0.5
```

**Step 2: 扩展 upload-artifact 的 path glob**

当前工作流第 116-118 行：

```yaml
          path: |
            clients/trace-windows/installer/out/Trace-*-${{ matrix.arch }}.msi
            clients/trace-windows/installer/out/Trace-*-${{ matrix.arch }}-portable.zip
```

扩为三行（顺序调整：把 Setup.exe 放第一，对齐主产物定位）：

```yaml
          path: |
            clients/trace-windows/installer/out/Trace-Setup-*-${{ matrix.arch }}.exe
            clients/trace-windows/installer/out/Trace-*-${{ matrix.arch }}.msi
            clients/trace-windows/installer/out/Trace-*-${{ matrix.arch }}-portable.zip
```

注意 Setup.exe 的 glob 是 `Trace-Setup-*-<arch>.exe`，刻意区分开 `trace-app.exe`（裸 exe 不会出现在 `installer/out/`，但防御性地把 glob 收紧）。

**Step 3: YAML 语法验证**

Run:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/trace-windows-release.yml')); print('yaml ok')"
```

Expected: `yaml ok`

**Step 4: 确认没污染 Phase 14 的 every-push 工作流**

Phase 15 新增的 release workflow 与 Phase 14 的 `trace-windows-build.yml`（per-push / PR 触发）完全分离，Phase 16 继续守住这条界线。

Run:
```bash
grep -l "Bal.wixext\|Trace-Setup" .github/workflows/
```

Expected: **仅** `.github/workflows/trace-windows-release.yml`，不能有 `.github/workflows/trace-windows-build.yml`。

**Step 5: 提交**

```bash
git add .github/workflows/trace-windows-release.yml
git commit -m "ci(trace-windows): add WiX Bal extension + ship Trace-Setup-*.exe artifact"
```

---

## Task 16.4: installer/README.md 改为 Setup.exe 主流程

**目标**：README 里把"首次安装"的验收步骤从"双击 MSI"改为"双击 Setup.exe"；MSI 继续在 README 里被提到，但定位下调到"给 IT 管理员的次级产物"。签名产物小节新增 Setup.exe 的 `signtool verify` 条目。打 tag 发布小节的产物列表多列 Setup.exe 一行。

**Files:**
- Modify: `clients/trace-windows/installer/README.md`

**Step 1: 在 `本地构建` 段落收尾处补 Setup.exe 产物说明**

当前第 43-44 行：

```markdown
pwsh installer\build-msi.ps1
# 产出：installer\out\Trace-<version>-x64.msi
```

扩为：

```markdown
pwsh installer\build-msi.ps1
# 产出（三个）：
#   installer\out\Trace-Setup-<version>-x64.exe  ← 主产物（双击即装）
#   installer\out\Trace-<version>-x64.msi        ← 供 msiexec / 批量部署
#   installer\out\Trace-<version>-x64-portable.zip
```

**Step 2: 在前置清单里加 WiX Bal 扩展**

当前第 32-33 行：

```markdown
3. WiX v4 global tool：`dotnet tool install --global wix --version 4.0.5`
4. WiX UI 扩展：`wix extension add -g WixToolset.UI.wixext/4.0.5`
```

在 4 之后插入（重新编号）：

```markdown
3. WiX v4 global tool：`dotnet tool install --global wix --version 4.0.5`
4. WiX UI 扩展：`wix extension add -g WixToolset.UI.wixext/4.0.5`
5. WiX Bal 扩展：`wix extension add -g WixToolset.Bal.wixext/4.0.5`
```

原 5、6 号变 6、7 号。

**Step 3: `首次安装` 验收列表改为双击 Setup.exe**

当前第 89-94 行：

```markdown
- [ ] 双击 `Trace-0.1.0-x64.msi` → 弹出 WixUI_InstallDir 欢迎页
- [ ] 下一步 → EULA 页显示 MIT 许可证全文（英文）
- [ ] 勾选"我接受"→ 下一步可用
- [ ] 安装路径默认 `C:\Program Files\Trace\` → 下一步
- [ ] 点击"安装"→ UAC 提示 → 批准
- [ ] 安装完成页显示 → 点击"完成"
```

改为（体现 WixStandardBootstrapperApplication 的 HyperlinkLicense 交互）：

```markdown
- [ ] 双击 `Trace-Setup-0.1.0-x64.exe` → 弹出 Trace 欢迎页（WiX Burn
      标准 UI，显示 "Trace" 标题 + 图标 + License 超链接）
- [ ] 点 "License" 超链接 → 弹出的 RTF 里是 MIT 许可证全文（英文）
- [ ] 点 "Install" → UAC 提示 → 批准
- [ ] 安装进度条走完 → 完成页 → 点 "Close"
```

同一小节继续往下（第 97-102 行）加一条 ARP 相关的新条目：

在 `- [ ] 发布者 "Sam Song"` 这条**之前**插入：

```markdown
  - [ ] 条目名 "Trace"（只一条；内部 MSI 被 Bundle 的 Visible="no" 藏掉）
```

**Step 4: `签名产物` 小节加 Setup.exe 校验条目**

当前第 137-148 行是签名产物 checklist。在 `signtool verify /pa /all Trace-<ver>-x64.msi` 那行**之前**插入：

```markdown
- [ ] `signtool verify /pa /all Trace-Setup-<ver>-x64.exe` 返回 Successfully
      verified（外壳 + 内部 Burn engine 都带签名）
```

**Step 5: `打 tag 发布` 产物列表补 Setup.exe**

当前第 195-201 行：

```markdown
# 4. 打开 Releases 页面核对：
#    Trace-0.2.0-x64.msi
#    Trace-0.2.0-x64-portable.zip
#    Trace-0.2.0-arm64.msi
#    Trace-0.2.0-arm64-portable.zip
```

扩为：

```markdown
# 4. 打开 Releases 页面核对（6 个文件：x64/arm64 各 3）：
#    Trace-Setup-0.2.0-x64.exe       ← 主产物
#    Trace-0.2.0-x64.msi
#    Trace-0.2.0-x64-portable.zip
#    Trace-Setup-0.2.0-arm64.exe     ← 主产物
#    Trace-0.2.0-arm64.msi
#    Trace-0.2.0-arm64-portable.zip
```

**Step 6: `验证签名` 示例命令补 Setup.exe**

当前第 209-213 行：

```powershell
# signtool 在 "Windows Kits\10\bin\<ver>\x64\signtool.exe" 下
signtool verify /pa /all Trace-0.2.0-x64.msi
signtool verify /pa /all trace-app.exe   # 从 MSI 解出或从 ZIP 解出
```

前面加一行：

```powershell
# signtool 在 "Windows Kits\10\bin\<ver>\x64\signtool.exe" 下
signtool verify /pa /all Trace-Setup-0.2.0-x64.exe
signtool verify /pa /all Trace-0.2.0-x64.msi
signtool verify /pa /all trace-app.exe   # 从 MSI 解出或从 ZIP 解出
```

**Step 7: 目录结构图补 Bundle.wxs**

当前第 19-21 行：

```
└── wix/
    └── Product.wxs          # WiX v4 产品声明
```

扩为：

```
└── wix/
    ├── Product.wxs          # WiX v4 MSI 产品声明
    └── Bundle.wxs           # WiX v4 Burn Bundle 声明（Phase 16）
```

**Step 8: `关键设计决定` 小节加一条 Bundle 相关说明**

在 `## 关键设计决定` 现有 4 条之后追加第 5 条：

```markdown
- **Bundle UpgradeCode 与 MSI UpgradeCode 完全独立**：Bundle 是
  `B98D8477-7730-4BC5-B177-8E00DC5C7DD0`，MSI 是
  `4F0FC3A3-C718-4DD4-BB01-0351E9960E8C`。Burn 把 Bundle 视为
  Windows Installer 层面的独立身份，共用 UpgradeCode 会让 MajorUpgrade
  分不清该升级哪一个，产出孤儿 ARP 条目。Chain 里 `MsiPackage Visible="no"`
  保证用户只在"程序与功能"里看到一条 Trace 条目（Bundle 的条目）。
```

**Step 9: 提交**

```bash
git add clients/trace-windows/installer/README.md
git commit -m "docs(installer): promote Trace-Setup-*.exe to primary artifact"
```

---

## Task 16.5: Phase 16 holistic final review

**目标**：用 Phase 16 的 4 次提交（16.1 Bundle.wxs + 16.2 build-msi.ps1 + 16.3 release.yml + 16.4 README）做一次 spec + quality 联合审阅，重点抓跨任务一致性（Bundle GUID vs MSI GUID、Setup.exe 文件名在四处一致、ATS 7 env 跳过分支不被 Bundle 引入的新签名点破坏、WiX Bal extension 命令在 README 和 release.yml 完全一致）。

**Files:**
- 无直接修改，由 reviewer 决定是否回填修复 commit

**Step 1: 汇总 Phase 16 提交**

Run:
```bash
git log --oneline HEAD~4..HEAD -- clients/trace-windows/installer/ .github/workflows/trace-windows-release.yml docs/plans/2026-04-23-phase-16-exe-bundle.md
```

Expected: 看到 plan + 16.1 + 16.2 + 16.3 + 16.4 共 5 次提交。

**Step 2: 对照 §0 前置决策逐条 check**

- [ ] 主产物名 `Trace-Setup-<ver>-<arch>.exe` 在 build-msi.ps1 / release.yml / README.md 全部同形
- [ ] Bundle UpgradeCode `B98D8477-7730-4BC5-B177-8E00DC5C7DD0` 仅在 Bundle.wxs 出现，不跑到 Product.wxs
- [ ] MSI UpgradeCode `4F0FC3A3-...` 仅在 Product.wxs，不跑到 Bundle.wxs
- [ ] Chain 里 `MsiPackage Visible="no"`
- [ ] BA 用 `bal:WixStandardBootstrapperApplication` Theme=`hyperlinkLicense`
- [ ] 签名链仍然是 `trace-app.exe → MSI → Burn engine → Setup.exe` 四段，全部通过 `Invoke-TrustedSign`
- [ ] `TRACE_ATS_DLIB` + 6 Azure env 的名字在 build-msi.ps1 与 release.yml 完全同名
- [ ] `wix extension add -g WixToolset.Bal.wixext/4.0.5` 在 release.yml 新增、README 前置清单同步新增
- [ ] stdout 三行（MSI + ZIP + Setup.exe）

**Step 3: 跨文件文件名一致性自动扫描**

Run:
```bash
grep -rn "Trace-Setup" \
    clients/trace-windows/installer/build-msi.ps1 \
    .github/workflows/trace-windows-release.yml \
    clients/trace-windows/installer/README.md
```

Expected: 每个文件里都能找到 `Trace-Setup-*` 或 `Trace-Setup-$Version-$Arch.exe` / `Trace-Setup-<ver>-<arch>.exe` 变体的引用。

**Step 4: 签名跳过分支 4 段联跑**

Run:
```bash
pwsh -NoProfile -Command @'
$scriptContent = Get-Content clients/trace-windows/installer/build-msi.ps1 -Raw
$ast = [System.Management.Automation.Language.Parser]::ParseInput($scriptContent, [ref]$null, [ref]$null)
$funcAst = $ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq "Invoke-TrustedSign" }, $true)[0]
Invoke-Expression $funcAst.Extent.Text
$InformationPreference = "Continue"
@("/tmp/p16-app.exe","/tmp/p16.msi","/tmp/p16-engine.exe","/tmp/p16-setup.exe") | ForEach-Object { "f" | Out-File $_ }
@("/tmp/p16-app.exe","/tmp/p16.msi","/tmp/p16-engine.exe","/tmp/p16-setup.exe") | ForEach-Object { Invoke-TrustedSign -Path $_ }
Remove-Item /tmp/p16-app.exe, /tmp/p16.msi, /tmp/p16-engine.exe, /tmp/p16-setup.exe
'@
```

Expected: 4 条 `skipping signing` 输出，exit 0。

**Step 5: 全量 YAML / PowerShell / WiX XML parse**

Run:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/trace-windows-release.yml')); yaml.safe_load(open('.github/workflows/trace-windows-build.yml'))" && \
pwsh -NoProfile -Command "[System.Management.Automation.Language.Parser]::ParseFile('$(pwd)/clients/trace-windows/installer/build-msi.ps1', [ref]\$null, [ref]\$errs); if (\$errs) { \$errs | ForEach-Object { Write-Error \$_ }; exit 1 } else { Write-Host 'parse ok' }" && \
xmllint --noout clients/trace-windows/installer/wix/Product.wxs clients/trace-windows/installer/wix/Bundle.wxs && echo "xml ok"
```

Expected: yaml 无异常 + `parse ok` + `xml ok`。

**Step 6: 派发 `superpowers:code-reviewer` 做最终审阅**

重点审：
- Bundle.wxs 与 Product.wxs 的 UpgradeCode 绝对隔离
- build-msi.ps1 新增的 4 段签名调用与 Phase 15 两段调用的 skip 语义一致
- Setup.exe 文件名在 4 个文件里完全同形
- release.yml 的 Bal extension add 步骤顺序正确（WiX CLI 安装之后、build-msi.ps1 调用之前）
- README 双击 Setup.exe 的描述与 `WixStandardBootstrapperApplication/hyperlinkLicense` 实际 UI 一致（欢迎页 → License 链接 → Install 按钮 → 进度 → Close）
- `wix burn detach` / `reattach` 失败时 `$LASTEXITCODE` 检查已写，不会静默成功
- Burn engine.exe 清理逻辑（reattach 后 Remove-Item）保证不会被 `upload-artifact` glob 意外收走（即便漏了清理，glob 也刻意收窄到 `Trace-Setup-*-<arch>.exe`，二道防线）

**Step 7: 根据 reviewer 反馈决定**

- reviewer 返回 ✅：Phase 16 关闭，可以准备 tag 发布
- reviewer 返回 ❌：按反馈回填 fix commit，循环再 review

---

## 附录 A：全部涉及文件速查

| 文件 | 动作 | 任务 |
|------|------|------|
| `clients/trace-windows/installer/wix/Bundle.wxs` | create | 16.1 |
| `clients/trace-windows/installer/build-msi.ps1` | modify | 16.2 |
| `.github/workflows/trace-windows-release.yml` | modify | 16.3 |
| `clients/trace-windows/installer/README.md` | modify | 16.4 |
| `docs/plans/2026-04-23-phase-16-exe-bundle.md` | create（本文件） | 计划 |

**不改**：
- `clients/trace-windows/installer/wix/Product.wxs`（Bundle 通过 `-d MsiPath=...` 引用 MSI 产物，MSI 定义本身无需动）
- `.github/workflows/trace-windows-build.yml`（Phase 14 per-push 工作流，Bundle 不进 every-push 路径）
- `clients/trace-windows/installer/scripts/*.py`（asset 生成脚本，Bundle 直接复用 MSI 的 LICENSE.rtf 与 trace.ico）
- `clients/trace-windows/installer/assets/*`（Bundle 的 `LicenseFile` / `LogoFile` 指向 Phase 14 已生成的同一组 asset）
- `clients/trace-windows/Cargo.toml`（版本号仍由 `[workspace.package].version` 驱动）

---

## 附录 B：失败场景手册

| 症状 | 可能原因 | 处置 |
|------|---------|------|
| `wix build Bundle.wxs` 报 `Unknown element bal:WixStandardBootstrapperApplication` | `WixToolset.Bal.wixext` 扩展没装 | 本地：`wix extension add -g WixToolset.Bal.wixext/4.0.5`；CI：检查 release.yml 的 "Add WiX Bal extension" 步骤是否 green |
| `wix burn detach` 报 `not a bundle` | 传给 detach 的是 MSI 不是 Setup.exe | 检查 `$SetupPath` 变量串错；build-msi.ps1 里 detach 的第一个位置参数必须是刚 `wix build Bundle.wxs` 产出的 `.exe` |
| `wix burn reattach` 产出的 Setup.exe 体积比 detach 前小很多 | reattach 时 engine 路径传错，WiX 把缺失当空引擎处理 | 确认 `$EnginePath` 在 detach 后真实存在且 `Invoke-TrustedSign` 之后没被清空；`Get-Item $EnginePath`.Length 应非零 |
| Setup.exe 签名通过但 SmartScreen 仍弹 "未知发布者" | engine 未被单独签 → 外壳签名但内部引擎裸奔 | 确认脚本顺序是 detach → sign engine → reattach → sign Setup.exe，不能跳过中间 sign engine 一步 |
| 卸载后"程序与功能"里仍残留 Trace 条目 | Bundle Chain 里 `Visible="no"` 写错成 `Visible="yes"`，MSI 注册了自己的 ARP 但没被 Bundle 卸载覆盖 | 手动 `msiexec /x {Product GUID}` 清残；修正 Bundle.wxs 后下一版 Bundle 装上去会被 MajorUpgrade 路径自动清理 |
| ARP 条目版本号不对（显示旧版） | Bundle 与 MSI 的 `Version` 不同源 | 检查 build-msi.ps1 是否把 `$Version` 同时传给两次 `wix build`；`-d Version=$Version` 必须两次都出现 |
| CI 把 engine-*.exe 当 artifact 上传 | build-msi.ps1 里 reattach 后 `Remove-Item` 被跳过；或 upload glob 太宽 | 双保险：脚本里留清理 + upload glob 收紧到 `Trace-Setup-*-<arch>.exe` |
| 本地 mac 上跑 `pwsh build-msi.ps1` 测 Bundle 构建失败 | WiX / Bundle 构建依赖 Windows PE 工具链，mac 跑不动 | 正常现象；mac 上只能跑签名跳过分支 + 字符串拼接 dry-run。真实 Bundle 构建在 Windows 机或 CI 上验证 |

---

*End of plan*
