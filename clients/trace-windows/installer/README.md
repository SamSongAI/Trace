# Trace Windows 安装包（MSI）

本目录放 WiX v4 MSI 安装包的全部素材：WiX 源、图标 / 许可证资产、PowerShell
构建脚本、素材生成器。生成的 MSI 不入 git（见 `.gitignore`）。

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
    └── Product.wxs            # WiX v4 产品声明
```

## 本地构建（Windows dev 机）

前置：

1. Rust stable + `rustup target add x86_64-pc-windows-msvc`
2. .NET 8 SDK
3. WiX v4 global tool：`dotnet tool install --global wix --version 4.0.5`
4. WiX UI 扩展：`wix extension add -g WixToolset.UI.wixext/4.0.5`
5. PowerShell 7+（Windows 10 自带的 `powershell.exe` 是 5.1，不行）
6. Python 3.11+ 与 Pillow 10.x（仅在需要再生成 ICO 时）：`pip install Pillow==10.*`

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

## 手动验收 Checklist（Windows 11 VM）

在干净的 Windows 11 虚拟机上跑一遍，记录通过与否。出现任何 ❌ 立刻 file
issue。

### 首次安装

- [ ] 双击 `Trace-0.1.0-x64.msi` → 弹出 WixUI_InstallDir 欢迎页
- [ ] 下一步 → EULA 页显示 MIT 许可证全文（英文）
- [ ] 勾选"我接受"→ 下一步可用
- [ ] 安装路径默认 `C:\Program Files\Trace\` → 下一步
- [ ] 点击"安装"→ UAC 提示 → 批准
- [ ] 安装完成页显示 → 点击"完成"
- [ ] 开始菜单搜索"Trace"能找到快捷方式
- [ ] 桌面出现 Trace 图标（若安装时未反勾"桌面快捷方式"特性）
- [ ] 控制面板 → 程序和功能 → Trace 条目显示：
  - [ ] 图标正确（多尺寸 .ico，256 渲染清晰）
  - [ ] 发布者 "Sam Song"
  - [ ] 版本 "0.1.0"
  - [ ] 帮助链接指向 <https://github.com/SamSong1997/Trace>
  - [ ] "更改"与"修复"按钮不可见（ARPNOMODIFY/ARPNOREPAIR 生效）
- [ ] 点开始菜单快捷方式 → Trace 启动、托盘图标出现、捕获面板可呼出
- [ ] 在设置面板里开启"开机自启"→ 重启 VM → Trace 自动启动

### 升级（0.1.0 → 0.2.0，哪怕 0.2.0 只是 bump 版本）

- [ ] 先装 0.1.0 → 在设置里改几个值（vault 路径、热键） → 关闭
- [ ] 双击 Trace-0.2.0-x64.msi → 安装流程（无需先卸载）
- [ ] 安装完成后打开 Trace → 设置保留（vault 路径、热键、笔记库等）
- [ ] 控制面板里只显示 0.2.0 条目（0.1.0 被 MajorUpgrade 卸掉）
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

- [ ] 装 0.2.0 后尝试装 0.1.0 MSI → 弹出"已安装更新的 Trace 版本，无法
  降级。"且不继续

## CI 产物

GitHub Actions 的 `build-msi` job 会在每次 push 后产出 `trace-msi-x64`
artifact（保留 14 天）。代码签名和 Release 发布由 Phase 15 负责。
