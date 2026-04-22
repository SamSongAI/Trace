# Phase 14 — WiX v4 MSI 安装包 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** 为 Trace Windows 客户端交付可双击安装 / 卸载 / 升级的 MSI 安装包（x64），产物覆盖 WiX 源码、构建脚本、CI 集成与手动验收清单。

**Architecture:** 独立的 `clients/trace-windows/installer/` 子目录放 WiX v4 声明（`wix/Product.wxs`）、图标 / 许可证等静态资产（`assets/`）、本地构建脚本（`build-msi.ps1`）和本目录 README。构建流水线：`cargo build -p trace-app --release --target x86_64-pc-windows-msvc` → `wix build -arch x64 -d Version=$(cargo-metadata.version)`。Mac 开发机无法本地运行 WiX，因此所有 wix build/validate 都落到 GitHub Actions 的 `windows-latest` runner；mac 端只做 XML schema 静态校验与 PowerShell 语法检查。

**Tech Stack:** WiX Toolset v4（dotnet global tool）· PowerShell 7+（构建脚本）· Python 3 + Pillow（ICO 生成）· GitHub Actions windows-latest runner · 固定 UpgradeCode `{4F0FC3A3-C718-4DD4-BB01-0351E9960E8C}`（本次生成，永不改；ProductCode 每版用 `*`）。

---

## 前置约束

- **工作目录**：`/Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows/`。`clients/trace-windows/installer/` 不是 cargo crate，**不要**加进 `[workspace]` members。
- **不要触碰**：`.gitignore`（repo 根，预先改动留给 Sam）、`scripts/trace.sh`（Mac 构建）、`AGENTS.md`、`clients/trace-win/**`（旧 MVP）、`target/**`。
- **commit 规范**：`type(scope): 描述` 格式，爬升提交，explicit `git add <path>`，没有 `Co-Authored-By` 脚注，不允许 `--amend`。
- **TDD 适配**：WiX XML 与 PowerShell 脚本很难写单元测试；每个 task 的验证点是"能跑 / 能解析 / 能通过 schema 校验"，真正的完整验收在 Task 14.7 手动测清单。
- **UpgradeCode 固定**：`{4F0FC3A3-C718-4DD4-BB01-0351E9960E8C}`。任何 task 都不得更换；这是 MSI 升级检测的稳定锚点，一旦发布后更换将把老版本标记为无关产品，双装成立而非升级。

---

## Task 14.1 — 图标资产（多尺寸 .ico）

**动机**：WiX `<Icon>` 元素需要 `.ico` 文件；`ARPPRODUCTICON` 在控制面板「程序与功能」里显示的也是 `.ico`。现有 `assets/trace-32.png` 只有 32×32，不满足高 DPI 环境。

**Files:**
- Create: `clients/trace-windows/installer/assets/trace.ico`
- Create: `clients/trace-windows/installer/scripts/build-ico.py`
- Modify: 无

**Step 1: 创建脚本目录并写 ICO 生成器**

```bash
mkdir -p /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows/installer/scripts
mkdir -p /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows/installer/assets
```

写 `clients/trace-windows/installer/scripts/build-ico.py`：

```python
#!/usr/bin/env python3
"""Generate a multi-resolution Windows .ico from the source 32x32 PNG.

Input:  clients/trace-windows/assets/trace-32.png (RGBA, 32x32)
Output: clients/trace-windows/installer/assets/trace.ico
Sizes:  16, 24, 32, 48, 64, 128, 256 — covers taskbar, Start menu,
        tile, jumbo shell thumbnail across DPI scaling factors.

Implementation note: Pillow's ICO writer takes a single base image and
downsamples it internally via the `sizes=` parameter; `append_images=`
is silently ignored for this format. We therefore upscale the 32×32
source to 256×256 with Lanczos once, then let Pillow produce the seven
frames from that single base. The 32→256 upscale is lossy, but the
source asset is already 32×32 by product design (Phase 0–11), and
shell thumbnails rarely exceed 64×64 in practice.
"""
from pathlib import Path
from PIL import Image

REPO_ROOT = Path(__file__).resolve().parents[2]  # .../installer -> trace-windows
SRC = REPO_ROOT / "assets" / "trace-32.png"
OUT = REPO_ROOT / "installer" / "assets" / "trace.ico"

SIZES = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]


def main() -> None:
    # Explicit existence check before PIL: PIL's own error ("cannot identify
    # image file") is ambiguous (corrupt file vs. missing file) and dumps a
    # full traceback that buries the actual cause in CI output. A plain
    # SystemExit keeps the error surface consistent with the size check below.
    if not SRC.exists():
        raise SystemExit(f"source PNG not found: {SRC}")
    base = Image.open(SRC).convert("RGBA")
    if base.size != (32, 32):
        raise SystemExit(f"expected 32x32 source, got {base.size}")
    # Upscale once to the largest target so Pillow's internal downsampler
    # always shrinks (higher fidelity than repeated upscales).
    base_256 = base.resize((256, 256), Image.LANCZOS)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    base_256.save(OUT, format="ICO", sizes=SIZES)
    print(f"wrote {OUT} with {len(SIZES)} frames: {[s[0] for s in SIZES]}")


if __name__ == "__main__":
    main()
```

**Step 2: 运行脚本生成 .ico**

```bash
cd /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows
python3 installer/scripts/build-ico.py
```

期待输出：`wrote /Users/.../installer/assets/trace.ico with 7 frames: [16, 24, 32, 48, 64, 128, 256]`

**Step 3: 验证 .ico 文件**

```bash
file /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows/installer/assets/trace.ico
```

期待输出包含 `MS Windows icon resource - 7 icons`。

**Step 4: 提交**

```bash
cd /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows
git add installer/scripts/build-ico.py installer/assets/trace.ico
git commit -m "feat(installer): add multi-resolution trace.ico from PNG source

Generated via installer/scripts/build-ico.py (Pillow-based) from
assets/trace-32.png. Seven frames - 16, 24, 32, 48, 64, 128, 256 -
cover every Windows shell surface: taskbar, Start menu tile, ARP
list, jumbo shell thumbnail across DPI factors.

Implementation quirk documented in the script: Pillow's ICO writer
takes a single base image and downsamples internally via sizes=;
append_images= is silently ignored for this format. The script
upscales the 32x32 source to 256x256 once with Lanczos so every
subsequent derivation is a downsample (higher fidelity than repeated
upscales from the native 32x32)."
```

---

## Task 14.2 — 许可证页面（LICENSE.rtf）

**动机**：WiX 标准模板 `WixUI_InstallDir` 在欢迎页后展示 EULA，接口是 `<WixVariable Id="WixUILicenseRtf" Value="...">`，内容必须是 RTF 格式（富文本而非 Markdown / 纯文本）。用仓库 MIT 全文。

**Files:**
- Create: `clients/trace-windows/installer/assets/LICENSE.rtf`
- Create: `clients/trace-windows/installer/scripts/build-license-rtf.py`

**Step 1: 写 RTF 生成脚本**

`clients/trace-windows/installer/scripts/build-license-rtf.py`：

```python
#!/usr/bin/env python3
"""Convert the repository's MIT LICENSE to minimal RTF for WiX.

WiX UI templates embed the EULA as rtf1-flavoured RichText. Word /
Wordpad render it; we need only ASCII + line breaks so the RTF header
is intentionally the bare minimum that Windows Forms RichTextBox will
accept.
"""
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[4]  # repo root (.../Trace)
SRC = REPO_ROOT / "LICENSE"
OUT = REPO_ROOT / "clients" / "trace-windows" / "installer" / "assets" / "LICENSE.rtf"


def to_rtf(text: str) -> str:
    # Escape RTF control characters.
    escaped = text.replace("\\", "\\\\").replace("{", "\\{").replace("}", "\\}")
    # Convert newlines to \par so RichTextBox renders line breaks.
    paragraphs = escaped.splitlines()
    body = "\\par\n".join(paragraphs)
    return (
        "{\\rtf1\\ansi\\ansicpg1252\\deff0\\nouicompat"
        "\\deflang1033{\\fonttbl{\\f0\\fnil Segoe UI;}}\n"
        "\\viewkind4\\uc1\\pard\\f0\\fs18\n"
        f"{body}\\par\n"
        "}\n"
    )


def main() -> None:
    if not SRC.exists():
        raise SystemExit(f"LICENSE not found at {SRC}")
    text = SRC.read_text(encoding="utf-8")
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(to_rtf(text), encoding="ascii")
    print(f"wrote {OUT} ({OUT.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
```

**Step 2: 运行脚本**

```bash
cd /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows
python3 installer/scripts/build-license-rtf.py
```

期待输出：`wrote /Users/.../installer/assets/LICENSE.rtf (N bytes)`，其中 N 约为 MIT 文本长度的 1.1 倍。

**Step 3: 校验 RTF 可被 TextEdit 打开**

```bash
head -c 60 /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows/installer/assets/LICENSE.rtf
```

期待前缀 `{\rtf1\ansi\ansicpg1252...`。

**Step 4: 提交**

```bash
cd /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows
git add installer/scripts/build-license-rtf.py installer/assets/LICENSE.rtf
git commit -m "feat(installer): generate LICENSE.rtf from repository MIT text

WiX UI template WixUI_InstallDir expects an rtf1-flavoured EULA. The
build-license-rtf.py helper converts LICENSE to the minimum RTF
header Windows RichTextBox accepts. Keeps the source of truth in the
repo root so a license amendment re-generates on the next invocation."
```

---

## Task 14.3 — Product.wxs 主声明 + 升级策略

**动机**：WiX v4 产品声明需包含 `Package`、`MediaTemplate`、`Directory` 树、`ComponentGroup`、`Feature`、`MajorUpgrade`、`UI` 引用。这一 task 先落"能装能卸能升"的最小闭环，快捷方式和元数据留给 14.4。

**Files:**
- Create: `clients/trace-windows/installer/wix/Product.wxs`
- Create: `clients/trace-windows/installer/wix/Package.wixproj` *(仅 CI 里 dotnet CLI 需要，本地也能跑但非必需)*
- Modify: 无

**Step 1: 创建 wix 子目录并写 Product.wxs**

```bash
mkdir -p /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows/installer/wix
```

`clients/trace-windows/installer/wix/Product.wxs`：

```xml
<?xml version="1.0" encoding="utf-8"?>
<!--
  Trace Windows client MSI — WiX v4 declaration.

  Build:
    wix build -arch x64 \
      -d Version=0.1.0 \
      -d BinDir=..\..\target\x86_64-pc-windows-msvc\release \
      -d AssetsDir=..\assets \
      -ext WixToolset.UI.wixext \
      -o Trace-0.1.0-x64.msi Product.wxs

  UpgradeCode is fixed for the lifetime of this product. Changing it
  would make every future release look like a separate product to
  Windows Installer, enabling double-install instead of upgrade.
  Generated 2026-04-22; do not edit.
-->
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs"
     xmlns:ui="http://wixtoolset.org/schemas/v4/wxs/ui">
  <Package
      Name="Trace"
      Manufacturer="Sam Song"
      Version="$(Version)"
      UpgradeCode="4F0FC3A3-C718-4DD4-BB01-0351E9960E8C"
      Compressed="yes"
      Scope="perMachine"
      InstallerVersion="500">

    <SummaryInformation Description="Trace — macOS 风格的系统级快速捕获工具（Windows 版）" />

    <!-- Major Upgrade: 安装新版前卸载旧版，保留 %APPDATA%\Trace 用户数据 -->
    <MajorUpgrade
        Schedule="afterInstallInitialize"
        DowngradeErrorMessage="已安装更新的 Trace 版本，无法降级。" />

    <MediaTemplate EmbedCab="yes" />

    <!-- 目录结构: C:\Program Files\Trace\ -->
    <StandardDirectory Id="ProgramFiles64Folder">
      <Directory Id="INSTALLFOLDER" Name="Trace" />
    </StandardDirectory>

    <!-- 组件: trace-app.exe -->
    <ComponentGroup Id="TraceAppComponents" Directory="INSTALLFOLDER">
      <Component Id="TraceAppExe" Guid="*">
        <File Id="TraceAppExeFile"
              Source="$(BinDir)\trace-app.exe"
              KeyPath="yes" />
      </Component>
    </ComponentGroup>

    <!-- 主特性 -->
    <Feature Id="MainFeature" Title="Trace" Level="1"
             Description="Trace 主程序">
      <ComponentGroupRef Id="TraceAppComponents" />
    </Feature>

    <!-- UI: WixUI_InstallDir + EULA -->
    <ui:WixUI Id="WixUI_InstallDir" InstallDirectory="INSTALLFOLDER" />
    <WixVariable Id="WixUILicenseRtf" Value="$(AssetsDir)\LICENSE.rtf" />
  </Package>
</Wix>
```

**Step 2: Schema 静态校验（mac 可跑）**

```bash
xmllint --noout /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows/installer/wix/Product.wxs
```

期待：无输出（0 退出码）。Schema 语法错误会打印行号。

**Step 3: 提交**

```bash
cd /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows
git add installer/wix/Product.wxs
git commit -m "feat(installer): add minimal WiX v4 Product.wxs with MajorUpgrade

Defines per-machine x64 install into Program Files\\Trace with
trace-app.exe as the sole component, WixUI_InstallDir EULA flow, and
MajorUpgrade scheduling that uninstalls prior versions before laying
down the new one. UpgradeCode pinned to 4F0FC3A3-C718-4DD4-BB01-
0351E9960E8C; per WiX semantics this GUID MUST NOT change once
shipped. Shortcut wiring and ARP metadata arrive in the next commit."
```

---

## Task 14.4 — 快捷方式 + ARP 元数据 + 图标

**动机**：Windows 用户期待装完能在开始菜单搜 Trace 启动；控制面板「程序与功能」条目要有图标、发布者主页、卸载尺寸估算。这一 task 把 14.3 的 MSI 扩充到"体验完整"。

**Files:**
- Modify: `clients/trace-windows/installer/wix/Product.wxs:<end of Package>` （追加快捷方式 ComponentGroup、`<Icon>`、ARP Properties）

**Step 1: 在 Product.wxs 现有 Feature 之前、`Package` 闭合之前增补以下片段**

**1a.** 主 `<Package>` 标签上补充 `UpgradeStrategy`-relevant 属性。在 `InstallerVersion="500"` 之后追加：

```xml
      InstallerVersion="500"
      ProductName="Trace">
```

**1b.** 在 `MediaTemplate` 之前插入 `<Icon>` 与 ARP 属性：

```xml
    <Icon Id="TraceIcon.ico" SourceFile="$(AssetsDir)\trace.ico" />
    <Property Id="ARPPRODUCTICON" Value="TraceIcon.ico" />
    <Property Id="ARPHELPLINK" Value="https://github.com/SamSong1997/Trace" />
    <Property Id="ARPURLINFOABOUT" Value="https://github.com/SamSong1997/Trace" />
    <Property Id="ARPCONTACT" Value="samsong199705@gmail.com" />
    <Property Id="ARPNOMODIFY" Value="1" />
    <Property Id="ARPNOREPAIR" Value="1" />
```

**1c.** 扩充目录结构，在现有 `<StandardDirectory Id="ProgramFiles64Folder">` 之后追加 Start Menu 目录：

```xml
    <StandardDirectory Id="ProgramMenuFolder">
      <Directory Id="ApplicationProgramsFolder" Name="Trace" />
    </StandardDirectory>
    <StandardDirectory Id="DesktopFolder" />
```

**1d.** 在 `TraceAppComponents` ComponentGroup 之后新增两个 ComponentGroup：

```xml
    <!-- 开始菜单快捷方式 -->
    <ComponentGroup Id="StartMenuShortcuts" Directory="ApplicationProgramsFolder">
      <Component Id="StartMenuShortcut" Guid="*">
        <Shortcut Id="StartMenuShortcutExe"
                  Name="Trace"
                  Description="Trace — 快速捕获"
                  Target="[INSTALLFOLDER]trace-app.exe"
                  WorkingDirectory="INSTALLFOLDER"
                  Icon="TraceIcon.ico" />
        <RemoveFolder Id="RemoveApplicationProgramsFolder"
                      Directory="ApplicationProgramsFolder"
                      On="uninstall" />
        <RegistryValue Root="HKCU"
                       Key="Software\Trace\Installer"
                       Name="StartMenuShortcutInstalled"
                       Type="integer"
                       Value="1"
                       KeyPath="yes" />
      </Component>
    </ComponentGroup>

    <!-- 桌面快捷方式（可选 Feature） -->
    <ComponentGroup Id="DesktopShortcutComponents" Directory="DesktopFolder">
      <Component Id="DesktopShortcut" Guid="*">
        <Shortcut Id="DesktopShortcutExe"
                  Name="Trace"
                  Description="Trace — 快速捕获"
                  Target="[INSTALLFOLDER]trace-app.exe"
                  WorkingDirectory="INSTALLFOLDER"
                  Icon="TraceIcon.ico" />
        <RegistryValue Root="HKCU"
                       Key="Software\Trace\Installer"
                       Name="DesktopShortcutInstalled"
                       Type="integer"
                       Value="1"
                       KeyPath="yes" />
      </Component>
    </ComponentGroup>
```

**1e.** 扩充 `MainFeature` 的 `ComponentGroupRef` 引用；再加一个 `DesktopShortcutFeature`：

```xml
    <Feature Id="MainFeature" Title="Trace" Level="1"
             Description="Trace 主程序">
      <ComponentGroupRef Id="TraceAppComponents" />
      <ComponentGroupRef Id="StartMenuShortcuts" />
    </Feature>

    <Feature Id="DesktopShortcutFeature"
             Title="桌面快捷方式"
             Level="1"
             AllowAbsent="yes"
             Description="在桌面创建 Trace 图标">
      <ComponentGroupRef Id="DesktopShortcutComponents" />
    </Feature>
```

**Step 2: Schema 静态校验**

```bash
xmllint --noout /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows/installer/wix/Product.wxs
```

期待：0 退出码、无输出。

**Step 3: 人工快速浏览**

读整份 `Product.wxs`，确认：
- 每个 `Component` 有且仅有一个 `KeyPath="yes"`（`TraceAppExeFile` / `StartMenuShortcut` 的 RegistryValue / `DesktopShortcut` 的 RegistryValue）
- `Icon Id` 以 `.ico` 结尾（WiX v4 要求）
- `ComponentGroupRef` 没有遗漏

**Step 4: 提交**

```bash
cd /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows
git add installer/wix/Product.wxs
git commit -m "feat(installer): wire Start Menu + Desktop shortcuts with ARP metadata

Adds Start Menu shortcut (required) and Desktop shortcut (separate
AllowAbsent feature so uncheck-to-opt-out works), ARP icon /
help link / about URL / contact so the Programs and Features entry
presents a complete identity. Per-user HKCU RegistryValue acts as the
shortcut KeyPath since MSI requires one and shortcuts do not qualify."
```

---

## Task 14.5 — 本地构建脚本 build-msi.ps1

**动机**：Windows dev 机 / CI runner 上只需要运行一条命令即可产出 MSI；脚本统一两个输入（版本号 + 目标架构），输出统一命名的 MSI 放到 `installer/out/`。

**Files:**
- Create: `clients/trace-windows/installer/build-msi.ps1`
- Create: `clients/trace-windows/installer/.gitignore`

**Step 1: 写 PowerShell 构建脚本**

`clients/trace-windows/installer/build-msi.ps1`：

```powershell
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

.PARAMETER Version
    Override the version string embedded in the MSI. When omitted,
    defaults to the workspace-level version in Cargo.toml.

.PARAMETER Arch
    Target architecture. Currently only `x64` is supported. `arm64`
    will be wired in Phase 15 along with the release matrix.

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
    [ValidateSet('x64')]
    [string]$Arch = 'x64',
    [string]$OutDir = ''
)

$ErrorActionPreference = 'Stop'
$InformationPreference = 'Continue'

$InstallerRoot = $PSScriptRoot
$WorkspaceRoot = Split-Path -Parent $InstallerRoot
$AssetsDir = Join-Path $InstallerRoot 'assets'
$WixDir = Join-Path $InstallerRoot 'wix'

if (-not $OutDir) {
    $OutDir = Join-Path $InstallerRoot 'out'
}

# --- resolve version from Cargo.toml if not overridden ---------------------
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
$TargetTriple = switch ($Arch) {
    'x64' { 'x86_64-pc-windows-msvc' }
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
Write-Host $MsiPath
```

**Step 2: 写 installer 子目录的 `.gitignore`**

`clients/trace-windows/installer/.gitignore`：

```gitignore
# Build output
out/
*.msi
*.wixpdb
*.wixobj

# Cargo target noise if someone runs `wix build` from here by mistake
target/
```

**Step 3: 静态校验 PowerShell 语法（mac 可跑）**

```bash
pwsh -NoProfile -Command "Get-Command -Syntax ./build-msi.ps1" 2>&1 \
  || pwsh -NoProfile -Command "[ScriptBlock]::Create((Get-Content -Raw ./installer/build-msi.ps1)) | Out-Null"
```

在 `clients/trace-windows/` 下执行。期待：0 退出码。**若 mac 未装 pwsh**，跳过这步（Task 14.6 的 CI 会跑到）。

**Step 4: 提交**

```bash
cd /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows
git add installer/build-msi.ps1 installer/.gitignore
git commit -m "feat(installer): add PowerShell build-msi driver

Wraps the cargo + wix build pipeline into a single idempotent
PowerShell script. Resolves version from Cargo.toml workspace so
docs / releases / MSI filename share a single source of truth.
Current arch limited to x64; arm64 flag plumbing is reserved for
Phase 15 CI matrix."
```

---

## Task 14.6 — CI workflow 扩展

**动机**：mac dev 机跑不了 WiX，所以 MSI 的真正验证在 GitHub Actions。当前 `.github/workflows/trace-windows-build.yml` 只跑 `cargo build`；我们追加一个在 push 主分支时产出 MSI 工件的 job，Phase 15 再把签名和 Release 发布接上。

**Files:**
- Modify: `.github/workflows/trace-windows-build.yml` — 在现有 job 之后追加 `build-msi` job

**Step 1: 读现状**

```bash
cat /Users/apple/Desktop/ContextOS/Projects/Trace/.github/workflows/trace-windows-build.yml
```

记录下现有 `build` job 的 `runs-on`、checkout 步骤、cargo cache 方案。新 job 复用相同 key 避免重复下载依赖。

**Step 2: 在 workflow 末尾追加 build-msi job**

末尾添加（缩进与现有文件一致）：

```yaml
  build-msi:
    needs: build
    runs-on: windows-latest
    defaults:
      run:
        working-directory: clients/trace-windows
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust toolchain (stable + x86_64-pc-windows-msvc)
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-pc-windows-msvc

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            clients/trace-windows/target
          key: ${{ runner.os }}-cargo-msi-${{ hashFiles('clients/trace-windows/Cargo.lock') }}

      - name: Setup .NET 8 SDK
        uses: actions/setup-dotnet@v4
        with:
          dotnet-version: '8.0.x'

      - name: Install WiX v4 global tool
        run: dotnet tool install --global wix --version 4.0.5

      - name: Setup Python (for LICENSE.rtf regeneration)
        uses: actions/setup-python@v5
        with:
          python-version: '3.11'

      - name: Install Pillow
        run: python -m pip install Pillow==10.*

      - name: Regenerate ICO and LICENSE.rtf from source
        run: |
          python installer/scripts/build-ico.py
          python installer/scripts/build-license-rtf.py

      - name: Build MSI
        shell: pwsh
        run: ./installer/build-msi.ps1

      - name: Upload MSI artifact
        uses: actions/upload-artifact@v4
        with:
          name: trace-msi-x64
          path: clients/trace-windows/installer/out/*.msi
          if-no-files-found: error
          retention-days: 14
```

**Step 3: 静态校验 YAML 语法**

```bash
python3 -c "import yaml, sys; yaml.safe_load(open('/Users/apple/Desktop/ContextOS/Projects/Trace/.github/workflows/trace-windows-build.yml'))"
```

期待：0 退出码，无输出。

**Step 4: 提交**

```bash
cd /Users/apple/Desktop/ContextOS/Projects/Trace
git add .github/workflows/trace-windows-build.yml
git commit -m "ci(trace-windows): build MSI artifact on every push

Adds a build-msi job to the existing trace-windows workflow. Runs on
windows-latest, installs WiX v4 as a dotnet global tool, regenerates
ICO + LICENSE.rtf from source (so artifact authenticity is provable
from the commit tree alone), invokes installer/build-msi.ps1, and
uploads the resulting Trace-<ver>-x64.msi. Signing and Release
publication arrive in Phase 15."
```

---

## Task 14.7 — README + 手动验收 Checklist

**动机**：WiX 源 + CI 可以产出 MSI，但"装上去能用"需要手动在 Windows 11 上验证。把操作步骤和验收 checklist 落到 `installer/README.md`，便于 Sam（或协作者）在 VM 里跑一遍记录结果。

**Files:**
- Create: `clients/trace-windows/installer/README.md`

**Step 1: 写 installer/README.md**

```markdown
# Trace Windows 安装包（MSI）

本目录放 WiX v4 MSI 安装包的全部素材：WiX 源、图标 / 许可证资产、PowerShell 构建脚本、素材生成器。生成的 MSI 不入 git（见 `.gitignore`）。

## 目录结构

```
installer/
├── README.md               # 本文件
├── .gitignore              # 忽略 out/、*.msi、*.wixobj、*.wixpdb
├── build-msi.ps1           # 本地 / CI 构建入口（PowerShell 7+）
├── assets/
│   ├── trace.ico           # 多尺寸图标（由 build-ico.py 生成）
│   └── LICENSE.rtf         # EULA 页面（由 build-license-rtf.py 生成）
├── scripts/
│   ├── build-ico.py        # 从 assets/trace-32.png 合成多尺寸 ICO
│   └── build-license-rtf.py# 从仓库根 LICENSE 生成 RTF
└── wix/
    └── Product.wxs         # WiX v4 产品声明
```

## 本地构建（Windows dev 机）

前置：

1. Rust stable + `rustup target add x86_64-pc-windows-msvc`
2. .NET 8 SDK
3. WiX v4 global tool：`dotnet tool install --global wix --version 4.0.5`
4. PowerShell 7+（Windows 10 自带的 `powershell.exe` 是 5.1，不行）

构建：

```powershell
cd clients\trace-windows
pwsh installer\build-msi.ps1
# 产出：installer\out\Trace-<version>-x64.msi
```

版本号自动从 `Cargo.toml` 的 `[workspace.package] version` 取；如需覆盖：

```powershell
pwsh installer\build-msi.ps1 -Version 0.2.0-rc1
```

## 再生成 ICO / LICENSE.rtf

这两个资产从源数据重新派生，而非手工编辑：

```bash
python3 installer/scripts/build-ico.py
python3 installer/scripts/build-license-rtf.py
```

提交 ICO / LICENSE.rtf 时请一并提交生成脚本的改动。

## 手动验收 Checklist（Windows 11 VM）

在干净的 Windows 11 虚拟机上跑一遍，记录通过与否。出现任何 ❌ 立刻 file issue。

### 首次安装

- [ ] 双击 `Trace-0.1.0-x64.msi` → 弹出 WixUI_InstallDir 欢迎页
- [ ] 下一步 → EULA 页显示 MIT 许可证全文（英文）
- [ ] 勾选"我接受"→ 下一步可用
- [ ] 安装路径默认 `C:\Program Files\Trace\` → 下一步
- [ ] 点击"安装"→ UAC 提示 → 批准
- [ ] 安装完成页显示 → 点击"完成"
- [ ] 开始菜单搜索"Trace"能找到快捷方式
- [ ] 桌面出现 Trace 图标
- [ ] 控制面板 → 程序和功能 → Trace 条目显示：
  - [ ] 图标正确（多尺寸 .ico，256 渲染清晰）
  - [ ] 发布者 "Sam Song"
  - [ ] 版本 "0.1.0"
  - [ ] 帮助链接指向 https://github.com/SamSong1997/Trace
- [ ] 点开始菜单快捷方式 → Trace 启动、托盘图标出现、捕获面板可呼出
- [ ] 在设置面板里开启"开机自启"→ 重启 VM → Trace 自动启动

### 升级（将 0.1.0 → 0.2.0 作流程验证，哪怕 0.2.0 只是 bump 版本）

- [ ] 先装 0.1.0 → 设置改一些值 → 关闭
- [ ] 双击 Trace-0.2.0-x64.msi → 安装流程（无需先卸载）
- [ ] 安装完成后打开 Trace → 设置保留（vault 路径、热键、笔记库等）
- [ ] 控制面板里只显示 0.2.0 条目（0.1.0 被 MajorUpgrade 卸掉）

### 卸载

- [ ] 控制面板 → 程序和功能 → Trace → 卸载
- [ ] UAC 批准后卸载完成
- [ ] `C:\Program Files\Trace\` 消失
- [ ] 开始菜单 Trace 快捷方式消失
- [ ] 桌面 Trace 快捷方式消失
- [ ] `%APPDATA%\Trace\` 保留（用户数据不应被删）

### 降级（反例）

- [ ] 装 0.2.0 后尝试装 0.1.0 MSI → 弹出"已安装更新的 Trace 版本，无法降级。"且不继续

## CI 产物

GitHub Actions 的 `build-msi` job 会在每次 push 后产出 `trace-msi-x64` artifact（保留 14 天）。签名和 Release 发布由 Phase 15 负责。
```

**Step 2: 提交**

```bash
cd /Users/apple/Desktop/ContextOS/Projects/Trace/clients/trace-windows
git add installer/README.md
git commit -m "docs(installer): add build instructions and manual QA checklist

Documents local build prerequisites (Rust + .NET 8 + WiX global tool
+ PowerShell 7), regeneration commands for derived assets, and a
concrete install/upgrade/uninstall/downgrade-reject checklist to run
in a clean Windows 11 VM before cutting a release. Ties off Phase 14;
Phase 15 consumes this MSI for signing and release publication."
```

---

## 落地顺序 & 复核节奏

同一 session 内按 SDD 两阶段复核（spec → 代码质量）逐 task 推进：

1. Task 14.1 → implementer → spec reviewer → quality reviewer → 下一个
2. 依此类推至 14.7
3. 最后发起整份变更的 final review（整个 Phase 14 做一次 code-reviewer 扫尾）

完成后移交 Phase 15（签名 + Release workflow）。

## 已经交付到这份计划之外的保证

- **UpgradeCode**：`{4F0FC3A3-C718-4DD4-BB01-0351E9960E8C}`（已固化、已生成）
- **图标源**：`clients/trace-windows/assets/trace-32.png`（仓库已存在，Phase 0-11 已落盘）
- **License 源**：`LICENSE`（仓库根已存在 MIT 全文）
- **CI 入口**：`.github/workflows/trace-windows-build.yml`（已存在，只追加 job）
- **Cargo 版本字段**：`clients/trace-windows/Cargo.toml` `[workspace.package] version = "0.1.0"`（已存在）
