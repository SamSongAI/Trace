# Phase 17 — 精简 release 资产 Implementation Plan

> **For Claude:** 本 plan 走 manual execution（按 SDD 决策树 tightly coupled 分支）—
> 任务按文件拆，单点改动，执行阶段由同一个 session 手动完成；最终一次
> `superpowers:code-reviewer` subagent 做 holistic sign-off。

**Goal:** 把 release 产物从 6 文件（2 arch × 3 格式：Setup.exe + MSI + portable.zip）精简到 2 文件（`Trace-<ver>-<arch>.exe` × x64/arm64），去掉对中文用户反直觉的 "Setup" 命名前缀。

**Architecture:** 纯删减 + 重命名。Bundle.wxs 和 Product.wxs 的实际 WiX 源文件不动，只改 build-msi.ps1 的文件名变量 + 删 portable zip 生成块；release.yml 的 artifact glob 收窄；README 把三种安装方式段落压成一种。MSI 继续作为 Bundle 内嵌的中间产物生成 + 签名（Bundle.wxs 需要 `MsiPath` 变量），只是不再对外发布。

**Tech Stack:** 无新增依赖。PowerShell 7 / WiX v4 / GitHub Actions —— 全是 Phase 14-16 已经到位的工具链。

---

## §0 决策记录

### 0.1 为什么 6 → 2

| 格式 | Trace 用户里的占比 | 保留价值 | 决策 |
|------|--------------------|----------|------|
| `Trace-<ver>-<arch>.exe` (Burn bundle) | ~95% | 普通用户一键装 | **保留** |
| `Trace-<ver>-<arch>.msi` | ~0（企业 IT 部署场景不存在） | 重复品（已内嵌在 Bundle 里） | **不发布**，继续作中间产物 |
| `Trace-<ver>-<arch>-portable.zip` | <5%（无 admin 的公司电脑） | 边缘受众 | **删除** |

定位决定取舍：Trace 是 macOS 风格的个人快速捕获工具，目标用户是设计师 / 程序员 / 知识工作者 —— 企业批量部署不是场景，企业锁 admin 的用户大概率也不是 Trace 目标人群。Sam 明确指令："用户只需要一个 exe。"

### 0.2 为什么去掉 "Setup" 前缀

- Windows 英文圈惯例：微信 / 钉钉 / WPS / 剪映都叫 `*Setup.exe`
- 中文用户第一眼反应：**"设置文件？"** —— 反直觉
- Obsidian / Notion 这类清爽型桌面应用的命名：`Obsidian-1.x.exe` / `Notion-x.x.x.exe` —— 直接 `<AppName>-<version>.exe`，双击后 UI 自己会告诉用户这是装东西，文件名不需要再重复

改名后用户看到 `Trace-0.2.0-x64.exe` 的心智与 macOS 下看到 `Trace.app` 一致。

### 0.3 MSI 保留为中间产物的理由

- Bundle.wxs 的 `<MsiPackage SourceFile="$(MsiPath)" />` 必须有一个真实 MSI 文件才能内嵌
- 签名链不能缩短：`trace-app.exe` (pre-WiX) → `MSI` (post-WiX) → `engine.exe` (detach) → `Setup.exe` (reattach) —— MSI 必须单独签名一次，否则 Windows Installer 在解包时会校验失败
- 保留 MSI 的本地调试价值：开发者可以 `signtool verify /pa /all Trace-<ver>-<arch>.msi` 验证签名链

只是 release.yml 的 artifact glob 不再包含它，CI 不 upload 给 Release。

### 0.4 stdout 协议调整

```powershell
# 旧（3 行，MSI 主产物在第一位）
Write-Host $MsiPath
Write-Host $ZipPath
Write-Host $SetupPath

# 新（2 行，Setup.exe 主产物在第一位）
Write-Host $SetupPath
Write-Host $MsiPath
```

- `$ZipPath` 变量整体删除（配套删 portable zip 生成块）
- 顺序调整：Setup.exe 是主发行物，放第一位；MSI 保留在第二行供本地调试
- CI 不依赖 stdout（CI 用 artifact glob），所以协议变更不会 break 任何现有消费者

---

## §1 任务列表

### Task 17.1: build-msi.ps1 精简

**Files:**
- Modify: `clients/trace-windows/installer/build-msi.ps1`

**改动点：**

1. **重命名 `$SetupName`**（~L277）
   - 旧：`$SetupName = "Trace-Setup-$Version-$Arch.exe"`
   - 新：`$SetupName = "Trace-$Version-$Arch.exe"`

2. **更新 bundle 段头部注释**（~L265-276）
   - 把 `# --- wix build bundle (Trace-Setup-<ver>-<arch>.exe) ---` 改成 `# --- wix build bundle (Trace-<ver>-<arch>.exe) ---`
   - 注释正文里的 `Trace-Setup-*.exe` 改成 `Trace-<ver>-<arch>.exe`

3. **删除 portable zip 生成块**（L341-388 整段）
   - 从 `# --- portable zip ---` 注释开始
   - 到 `Write-Information ("produced {0} ({1:N0} bytes)" -f $ZipPath, $zipSize)` 结束
   - 含：`$RepoRoot` / `$LicensePath` / `$ZipName` / `$ZipPath` / `$Staging` 所有变量、`Copy-Item` / `Compress-Archive` 调用、zip 大小打印

4. **stdout 协议缩到 2 行 + 顺序调整**（L390-398）
   - 把注释里的 `$zip` 示例行删掉
   - 注释里的 `$setup` / `$msi` 顺序对调，`$setup` 放第一个
   - `Write-Host` 调用顺序：`$SetupPath` 放第一行（主产物），`$MsiPath` 放第二行（本地调试）
   - 把"Keep MSI first for back-compat"那句改成"Setup.exe 为主产物，首行输出；MSI 供本地调试保留第二行"

**测试:**
- 本地 mac 无 pwsh，无法直接 parse-check；依赖 Edit 工具的精确替换 + 人肉 re-read
- Holistic reviewer 会做文本层面的协议一致性检查
- 下一次 CI run（tag push）会做运行时验证

**Commit:**
```
feat(installer): slim build-msi.ps1 to Trace-<ver>-<arch>.exe only
```

---

### Task 17.2: release.yml artifact glob 收窄

**Files:**
- Modify: `.github/workflows/trace-windows-release.yml:127-130`

**改动点：**

```yaml
# 旧
path: |
  clients/trace-windows/installer/out/Trace-Setup-*-${{ matrix.arch }}.exe
  clients/trace-windows/installer/out/Trace-*-${{ matrix.arch }}.msi
  clients/trace-windows/installer/out/Trace-*-${{ matrix.arch }}-portable.zip

# 新
path: |
  clients/trace-windows/installer/out/Trace-*-${{ matrix.arch }}.exe
```

**关键：** glob 仍带 `${{ matrix.arch }}` 后缀，防止 x64 和 arm64 交叉污染。

**注释更新：**
- 把 glob 上方 "Primary artifact is the WiX Burn Bundle..." 段改成对应新命名和"唯一产物"措辞
- 把 "engine-*.exe from a failed reattach would never be mistaken" 这句保留（engine 清理防御仍然有效）
- 不再提 MSI / ZIP

**Commit:**
```
ci(trace-windows): release only Trace-<ver>-<arch>.exe
```

---

### Task 17.3: Bundle.wxs 头部注释示例命令

**Files:**
- Modify: `clients/trace-windows/installer/wix/Bundle.wxs:11`

**改动点：**

```xml
<!-- 旧 -->
-o ..\out\Trace-Setup-0.1.0-x64.exe Bundle.wxs

<!-- 新 -->
-o ..\out\Trace-0.1.0-x64.exe Bundle.wxs
```

只改这一行。Bundle.wxs 的 WiX 语义不动。

**Commit:**
```
docs(installer): update Bundle.wxs build example to new filename
```

---

### Task 17.4: README.md 精简 + 重命名

**Files:**
- Modify: `clients/trace-windows/installer/README.md`

**改动点清单：**

1. **产物命名全局替换：** `Trace-Setup-*` → `Trace-*`
   - `Trace-Setup-<ver>-x64.exe` → `Trace-<ver>-x64.exe`
   - 所有 signtool verify 示例、下载流程、发布物列表里的 Setup.exe 引用

2. **"三种安装方式"段落压成一种：**
   - 删掉描述裸 MSI 双击的段落
   - 删掉描述 portable zip 解压即用的段落
   - 只保留 Burn bundle 双击流程：Welcome → 许可链接 → Install → Close

3. **升级路径段落：**
   - 删掉 "msiexec /i Trace-*.msi" 高级路径
   - 只描述 "双击新版 `Trace-<ver>-<arch>.exe`，Burn 的 related-bundle detection 自动卸载旧版"

4. **目录树 / Prerequisites / Build 块：**
   - 目录树说明不变（Bundle.wxs / Product.wxs / build-msi.ps1 都还在）
   - Prerequisites 步骤 5 (WiX Bal extension) 保留
   - Build 命令块的输出示例从 3 个文件改成 2 个文件（Setup.exe 主 + MSI 中间产物）并说明 MSI 只用于本地调试、不发布

5. **Tag-release 产物清单：**
   - 6 个文件改成 2 个：`Trace-<ver>-x64.exe` / `Trace-<ver>-arm64.exe`
   - 说明 MSI + ZIP 已从 release 移除，理由简要引用 Phase 17 决策（0.1-0.3）

6. **Portable zip 相关 checklist / troubleshooting：**
   - 所有"解压 zip 到 %USERPROFILE%\..."指南删除
   - 所有"portable 模式不写注册表"类补丁说明删除

**Commit:**
```
docs(installer): slim install guide to single .exe distribution
```

---

### Task 17.5: Holistic review

**Workflow:**

1. 跑完 Task 17.1-17.4，4 个 commit 都落到 main
2. 起一个 `superpowers:code-reviewer` subagent，prompt 自包含，列出四个 commit SHA + §0 决策清单
3. Reviewer 检查项：
   - 命名一致性：仓库里所有活跃文件的 `Trace-Setup-` 残留必须为零（docs/plans/ 除外）
   - 产物一致性：build-msi.ps1 不再生成 zip，release.yml 不再上传 zip/msi
   - 签名链完整：trace-app.exe → MSI → engine.exe → Setup.exe 四段仍在
   - MSI 作为中间产物的反向检验：`wix build Bundle.wxs -d MsiPath=...` 依然需要 MSI 文件，build-msi.ps1 的 MSI 生成块没被误删
   - 文档一致性：README 的产物清单 / Bundle.wxs 头部示例 / release.yml glob 三者文件名一致
   - stdout 协议：两行 + Setup.exe 第一行
4. 如有反馈，同 Phase 16 节奏：一个修复 commit 搞定所有 issue，再 dispatch 第二轮 reviewer 确认

**收尾:**
- TodoWrite 标所有任务 completed
- 文字汇报 commit 链 + tag-ready 状态
- 不自作主张打 tag

---

## §2 风险与 Roll-back

### 2.1 CI 没跑过 Phase 17 代码

- 本地 mac 无 pwsh / 无 wix，无法运行时验证
- 风险：build-msi.ps1 精简后 PowerShell 语法错 / 变量未定义漏网
- 缓解：Edit 工具精确替换、reviewer 文本层面 double-check、下一次 tag push CI 会暴露
- Roll-back：5 个 commit 可 `git revert` 回 `26d7156`

### 2.2 用户下载到 `Trace-<ver>.exe` 后不知道要做啥

- 双击后 Burn UI 自己会渲染 Welcome 页 —— UX 不受影响
- Release notes / README 会解释这是 installer；目标用户从 GitHub Release 页进来，有 release title "Trace v0.2.0" 上下文

### 2.3 裸 MSI 需求突然出现

- 比如某天某个企业用户要求 `msiexec /i`
- Roll-back 方案：临时恢复 Task 17.2 的 release.yml glob 的 MSI 一行即可；MSI 文件仍在 `installer/out/` 里构建，不需要改 build-msi.ps1
- 彻底复原还可以从 `26d7156` 反向 cherry-pick

---

## §3 §0 决策交叉核对清单（执行中逐项 ✓）

- [ ] `$SetupName` = `"Trace-$Version-$Arch.exe"` ✓ 0.2
- [ ] portable zip 块删除 ✓ 0.1
- [ ] release.yml artifact 只含 `Trace-*.exe` ✓ 0.1
- [ ] MSI 继续生成 + 签名（build-msi.ps1 L213-263 不动）✓ 0.3
- [ ] stdout 两行，Setup.exe 第一位 ✓ 0.4
- [ ] Bundle.wxs 示例命令改名 ✓ 0.2
- [ ] README 所有 `Trace-Setup-` 替换为 `Trace-` ✓ 0.2
- [ ] docs/plans/*.md 保持原样（历史记录）

---

## §4 估算规模

- 5 个任务（含 review）
- 4 个活跃 commit + 至多 1 个 review-fix commit
- 纯删减 + 重命名，没有新设计决策
- Phase 16 review-fix 阶段已验证的节奏：manual execution + 最终 holistic review
