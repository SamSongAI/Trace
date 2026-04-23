# Trace Windows 安装程序

本目录放 WiX v4 安装程序的全部素材：MSI 源（`wix/Product.wxs`）、Burn Bundle
源（`wix/Bundle.wxs`）、图标 / 许可证资产、PowerShell 构建脚本、素材生成器。
最终发布物是 `Trace-<ver>-<arch>.exe`（Burn Bundle）；中间产物 MSI 在本地构
建目录里可见，供调试和签名验证，但不对外发布。生成的二进制不入 git（见
`.gitignore`）。

## 目录结构

```
installer/
├── README.md                  # 本文件
├── .gitignore                 # 忽略 out/、*.msi、*.wixobj、*.wixpdb
├── build-msi.ps1              # 本地 / CI 构建入口（PowerShell 7+）
├── assets/
│   ├── trace.ico              # 多尺寸图标（由 build-ico.py 生成）
│   └── LICENSE.rtf            # EULA 页面（由 build-license-rtf.py 生成）
├── scripts/
│   ├── build-ico.py           # 从 assets/trace-32.png 合成多尺寸 ICO
│   └── build-license-rtf.py   # 从仓库根 LICENSE 生成 RTF
└── wix/
    ├── Product.wxs            # WiX v4 MSI 产品声明
    └── Bundle.wxs             # WiX v4 Burn Bundle 声明（Phase 16）
```

Release 发布通过仓库根的 `.github/workflows/trace-windows-release.yml`
驱动，见下方 [Release 发布流水线](#release-发布流水线phase-15) 一节。

## 本地构建（Windows dev 机）

前置：

1. Rust stable + `rustup target add x86_64-pc-windows-msvc`
2. .NET 8 SDK
3. WiX v4 global tool：`dotnet tool install --global wix --version 4.0.5`
4. WiX UI 扩展：`wix extension add -g WixToolset.UI.wixext/4.0.5`
5. WiX Bal 扩展：`wix extension add -g WixToolset.Bal.wixext/4.0.5`
   （Bundle.wxs 引用 `bal:WixStandardBootstrapperApplication`，未装此扩展
   `wix build Bundle.wxs` 会直接报 `Unknown element`。）
6. PowerShell 7+（Windows 10 自带的 `powershell.exe` 是 5.1，不行）
7. Python 3.11+ 与 Pillow 10.x：`pip install Pillow==10.*`
   （**每次构建都会跑**，用来再生成 `trace.ico` 和 `LICENSE.rtf`；Python 解释器在 Windows 下必须以 `python` 命令可用）

构建：

```powershell
cd clients\trace-windows
pwsh installer\build-msi.ps1
# 产出两个文件（Phase 17 起）：
#   installer\out\Trace-<version>-x64.exe   ← 主产物（双击即装，release 唯一发布物）
#   installer\out\Trace-<version>-x64.msi   ← 中间产物（Bundle 内嵌用 + 本地 signtool 验证；不对外发布）
```

版本号自动从 `Cargo.toml` 的 `[workspace.package] version` 取；如需覆盖：

```powershell
pwsh installer\build-msi.ps1 -Version 0.2.0-rc1
```

`build-msi.ps1` 会在 `wix build` 之前自动重新生成 `trace.ico` 和
`LICENSE.rtf`，所以修改源（`assets/trace-32.png` 或仓库根 `LICENSE`）后
直接跑构建即可，无需手工再生。

## 手动再生成 ICO / LICENSE.rtf

脚本独立可运行，可在 mac / Linux dev 机上预览派生结果：

```bash
python3 clients/trace-windows/installer/scripts/build-ico.py
python3 clients/trace-windows/installer/scripts/build-license-rtf.py
```

两个脚本都是幂等的：相同输入 → 相同输出。提交 ICO / LICENSE.rtf 时请一并
提交生成脚本的改动。

## 关键设计决定

- **UpgradeCode 永不更换**：`4F0FC3A3-C718-4DD4-BB01-0351E9960E8C` 已固化
  在 `wix/Product.wxs`。任何时候改掉这 GUID 会让 Windows Installer 视新
  版本为独立产品，破坏 MajorUpgrade 路径、导致双装。
- **%APPDATA%\Trace\ 归用户所有**：MSI 从不触碰此目录，所以升级和卸载都
  不会破坏用户笔记、设置、每日文件。
- **perMachine + HKMU KeyPath**：安装到 `Program Files\Trace\`，快捷方式
  组件的 Registry KeyPath 走 `HKMU`（WiX 魔法：perMachine → HKLM，
  perUser → HKCU），避开"perMachine + HKCU"组合在多用户机上导致的
  反复 self-repair。
- **ARPNOMODIFY / ARPNOREPAIR**：没有 Modify / Repair 自定义操作，直接
  关掉两个按钮，避免用户点了什么也不发生。
- **Bundle UpgradeCode 与 MSI UpgradeCode 完全独立**：Bundle 是
  `B98D8477-7730-4BC5-B177-8E00DC5C7DD0`，MSI 是
  `4F0FC3A3-C718-4DD4-BB01-0351E9960E8C`。Burn 把 Bundle 视为
  Windows Installer 层面的独立身份，共用 UpgradeCode 会让 MajorUpgrade
  分不清该升级哪一个，产出孤儿 ARP 条目。Chain 里 `MsiPackage Visible="no"`
  保证用户只在"程序与功能"里看到一条 Trace 条目（Bundle 的条目）。

## 手动验收 Checklist（Windows 11 VM）

在干净的 Windows 11 虚拟机上跑一遍，记录通过与否。出现任何 ❌ 立刻 file
issue。

### 首次安装

- [ ] 双击 `Trace-0.1.0-x64.exe` → 弹出 Trace 欢迎页（WiX Burn
      的 WixStandardBootstrapperApplication/HyperlinkLicense 主题，
      显示 "Trace" 标题 + 图标 + License 超链接）
- [ ] 点 "License" 超链接 → 弹出的 RTF 里是 MIT 许可证全文（英文）
- [ ] 点 "Install" → UAC 提示 → 批准
- [ ] 安装进度条走完 → 完成页 → 点 "Close"
- [ ] 开始菜单搜索"Trace"能找到快捷方式
- [ ] 桌面出现 Trace 图标（若安装时未反勾"桌面快捷方式"特性）
- [ ] 控制面板 → 程序和功能 → Trace 条目显示：
  - [ ] 条目名 "Trace"，且只有一条（内部 MSI 被 Bundle 的 `Visible="no"` 藏掉）
  - [ ] 图标正确（多尺寸 .ico，256 渲染清晰）
  - [ ] 发布者 "Sam Song"
  - [ ] 版本 "0.1.0"
  - [ ] 帮助链接指向 <https://github.com/SamSong1997/Trace>
  - [ ] "更改"与"修复"按钮不可见（ARPNOMODIFY/ARPNOREPAIR 生效）
- [ ] 点开始菜单快捷方式 → Trace 启动、托盘图标出现、捕获面板可呼出
- [ ] 在设置面板里开启"开机自启"→ 重启 VM → Trace 自动启动

### 升级（0.1.0 → 0.2.0，哪怕 0.2.0 只是 bump 版本）

- [ ] 先装 0.1.0 → 在设置里改几个值（vault 路径、热键） → 关闭
- [ ] 双击 `Trace-0.2.0-x64.exe` → Burn UI 走完（无需先卸载）
- [ ] 安装完成后打开 Trace → 设置保留（vault 路径、热键、笔记库等）
- [ ] 控制面板里只显示 0.2.0 条目（旧 Bundle 由 Burn 的 related-bundle
      detection 自动卸载；内部 MSI 由 Product.wxs 的 MajorUpgrade 替换）
- [ ] `%APPDATA%\Trace\` 内的 notes / settings 原封不动

### 多用户自修复回归（perMachine + HKMU 关键检查）

- [ ] 以用户 A 身份安装 Trace → 点快捷方式正常启动
- [ ] 切换到用户 B（或在 VM 里新建一个本地账户登录） → 点开始菜单 Trace
  快捷方式 → **不应该**弹 UAC 或"正在配置 Trace"的 self-repair 对话
- [ ] 用户 B 下 Trace 正常启动（使用用户 B 自己的 `%APPDATA%\Trace\`）

### 卸载

- [ ] 控制面板 → 程序和功能 → Trace → 卸载
- [ ] UAC 批准后卸载完成
- [ ] `C:\Program Files\Trace\` 消失
- [ ] 开始菜单 Trace 快捷方式消失
- [ ] `Start Menu\Programs\Trace\` 子目录消失（RemoveFolder 生效）
- [ ] 桌面 Trace 快捷方式消失
- [ ] `%APPDATA%\Trace\` 保留（用户数据不应被删）

### 降级（反例）

- [ ] 装 0.2.0 后尝试双击 `Trace-0.1.0-x64.exe` → 弹出"已安装更新的 Trace
  版本，无法降级。"且不继续（降级拦截由内嵌 MSI 的 MajorUpgrade
  DowngradeErrorMessage 触发，Burn 把它透传到 UI）

### 签名产物（仅 Release workflow 产物）

- [ ] `signtool verify /pa /all Trace-<ver>-x64.exe` 返回
      Successfully verified（外壳 + 内部 Burn engine 都带签名）
- [ ] （可选，本地调试）`signtool verify /pa /all Trace-<ver>-x64.msi`
      返回 Successfully verified —— MSI 不发布到 Release，但本地
      `installer\out\` 里还有，用来独立校验 Bundle 内嵌包的签名链
- [ ] `Trace-<ver>-x64.exe` 属性对话框 → 数字签名 → 看到 Sam Song
      签名 + ACS 时间戳
- [ ] UAC 弹窗显示 "已验证的发布者：Sam Song"（不是"未知发布者"）
- [ ] SmartScreen 不再弹"Windows 已保护你的电脑"警告（可能需要少量
      下载量后 Microsoft reputation 才完全建立，新证书前几次安装可能
      仍会被拦，但签名有效性已建立）
- [ ] arm64 机器上双击 `Trace-<ver>-arm64.exe` 安装成功（可选，视硬件
      条件；x64 机器上没有简便的干跑方式 —— Burn Bundle 不接受
      `msiexec /a` 这类 MSI-only 的 administrative install 参数）

## CI 产物

GitHub Actions 的 `build-msi` job 会在每次 push 后产出 `trace-msi-x64`
artifact（保留 14 天）。代码签名和 Release 发布由 Phase 15 负责。

## Release 发布流水线（Phase 15）

`trace-windows-release.yml` 负责对齐 tag 的正式发布：`push tags
v*.*.*` 会并行构建 x64 与 arm64 签名的 `Trace-<ver>-<arch>.exe`
（WiX Burn Bundle），并自动发布到 GitHub Releases。Phase 17 起，
release 只产出这一种文件 —— MSI 在构建目录里仍然生成并参与签名链，
但不作为 Release artifact 上传（已内嵌在 Bundle 里，外发只会制造"选
择困难"的噪音）；portable zip 整体移除。

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

# 4. 打开 Releases 页面核对（2 个文件：x64 / arm64 各 1 个）：
#    Trace-0.2.0-x64.exe       ← Burn Bundle，双击即装
#    Trace-0.2.0-arm64.exe     ← Burn Bundle，双击即装
#
#    Phase 17 起这是唯一 release artifact。中间产物 Trace-0.2.0-*.msi
#    还在 CI runner 的 installer/out/ 里（供签名 + Bundle 内嵌用），
#    但 upload-artifact 的 glob 已经排除掉，不会流到 Release 页面。
```

预发布 tag（带 `-`，如 `v0.2.0-rc1`）会自动标记为 Pre-release。

### 验证签名

在 Windows 机器上：

```powershell
# signtool 在 "Windows Kits\10\bin\<ver>\x64\signtool.exe" 下
# 主产物：
signtool verify /pa /all Trace-0.2.0-x64.exe

# 本地调试（可选）：MSI 不发布到 Release，但本地构建目录里有；展开后
# 还能抽出内嵌的 trace-app.exe 单独验签
signtool verify /pa /all Trace-0.2.0-x64.msi
signtool verify /pa /all trace-app.exe   # 用 msiexec /a 展开后从 Program Files\Trace\ 拿
```

输出应该包含 `Successfully verified` 和时间戳信息（来自
`timestamp.acs.microsoft.com`）。

或者在 Explorer 里右键 `Trace-<ver>-<arch>.exe` → 属性 → 数字签名，
看到 "Sam Song"（通过 Microsoft Identity Verification CA 颁发的证书）。
