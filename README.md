<p align="center">
  <img src="Sources/Trace/Resources/logo.png" width="128" height="128" alt="Trace logo" />
</p>

<h1 align="center">Trace</h1>

<p align="center">
  <strong>macOS 系统级快速捕获工具，直接写入 Markdown 文件。</strong><br/>
  在任何应用中按下快捷键，浮动面板出现，写完即走。
</p>

<p align="center">
  <em>万象皆留痕。</em>
</p>

<p align="center">
  <a href="#1-为什么做-trace">产品灵感</a> · <a href="#2-功能">功能</a> · <a href="#3-配合-obsidian-使用">Obsidian 配置</a> · <a href="https://github.com/SamSongAI/Trace/releases/latest">下载</a> · <a href="README_EN.md">English</a>
</p>

---

## 1. 为什么做 Trace

灵感来的时候，你正在做别的事。

两个选择都很糟——要么忘掉它，要么 `⌘Tab` 切到笔记应用去记录。每一次切换窗口，注意力就碎了一次。回到心流状态的代价远比你想象的高。

**Trace 在操作系统层面解决这个问题。** 一个快捷键唤起浮动面板，写完按 `⌘Enter`，内容直接写入本地 Markdown 文件。你从未离开过当前的工作。

这不是又一个笔记应用。这是一个**注意力保护工具**。

**设计原则：**

- **捕获优先于整理** — 先留住想法，分类是后面的事。Trace 只做捕获，把整理留给你的笔记系统
- **零摩擦** — 全局快捷键唤起，没有加载时间，没有多余的步骤
- **本地优先** — 没有账号，没有云端，没有遥测。你的文件就在你的硬盘上
- **轻量聚焦** — 不做编辑器，不做知识库。只把"记下来"这件事做到极致

## 2. 功能

```
任意应用  ──快捷键──▶  Trace 浮动面板  ──⌘Enter──▶  本地 .md 文件
                      (系统级悬浮)                  (直接文件写入)
```

- **全局快捷键** — 从任何应用唤起捕获面板，快捷键可自定义
- **日记模式 / 文档模式** — 日记追加到当天文件，文档创建独立 .md 文件，`⇧Tab` 一键切换
- **自定义分类** — 日记模式下可配置分类按钮（如"想法""待办""灵感"），对应日记中的标题
- **独立路径** — 日记和文档可以指向不同的文件夹，互不干扰
- **Pin 模式** — 保存后面板不关闭，适合连续记录
- **原生 macOS** — SwiftUI + AppKit 构建，不是 Electron，不是 Web 套壳
- **零网络** — 没有账号，没有遥测，没有云端。你的数据完全属于你

## 3. 配合 Obsidian 使用

Trace 写入本地 `.md` 文件，天然适配 Obsidian。关键是让 Trace 的写入路径和 Obsidian 的读取路径对齐。

日记文件的最终路径由三个设置决定：

```
笔记库 / 日记文件夹 / 文件名格式.md
```

确保这三个值和 Obsidian 日记插件一致即可：

| Trace 设置 | 对应 Obsidian 日记设置 | 示例 |
|---|---|---|
| **笔记库** | Vault 根目录 | `/Users/you/MyVault` |
| **日记文件夹** | 新笔记的存放位置 | `Daily` |
| **文件名格式** | 日期格式 | `yyyy M月d日 EEEE` |

**配置步骤：**

1. Obsidian → 设置 → 日记 → 记下「存放位置」和「日期格式」
2. Trace → ⚙ 设置 → 日记模式
3. **笔记库**：填 Vault 根目录（包含 `.obsidian` 文件夹的那一层）
4. **日记文件夹**：填 Obsidian 中「存放位置」的值（默认 `Daily`）
5. **文件名格式**：选择与 Obsidian 一致的格式

对齐后，在 Obsidian 按 `⌘D` 打开今天的日记，就能看到 Trace 写入的内容。

> Trace 不绑定 Obsidian。任何本地 Markdown 笔记系统（Logseq、Typora、iA Writer 等）都可以配合使用。不需要 Obsidian 运行，不需要安装插件。

## 4. 快捷键

| 快捷键 | 功能 |
|---|---|
| `⌘N` | 唤起捕获面板（可自定义） |
| `⌘Enter` | 保存 |
| `⌘⇧Enter` | 追加到上一条 |
| `⇧Tab` | 切换日记 / 文档模式 |
| `⌘P` | 切换 Pin 模式 |
| `⌘1-9` | 快速切换分类 |
| `Esc` | 关闭面板 |

## 5. 下载安装

从 [GitHub Releases](https://github.com/SamSongAI/Trace/releases/latest) 下载最新的 `Trace.dmg`。

1. 打开 DMG 后，你会看到 `Trace.app` 和 `Applications` 图标，把 `Trace.app` 直接拖到 `Applications`
2. 首次打开时，macOS 可能提示"无法验证开发者"。解决方法：
   - **方式一**：右键点击 Trace.app → 选择「打开」→ 在弹窗中点击「打开」
   - **方式二**：打开终端，运行 `xattr -cr /Applications/Trace.app`，然后正常启动
3. 点击右上角 ⚙ 进入设置，配置笔记库路径

系统要求：macOS 13+。

## 6. 起源

Trace 的前身是 [ObsidianFlashNote](https://github.com/SamSongAI/ObsidianFlashNote)——一个 Hammerspoon Lua 脚本。验证了"不离开当前应用就能捕获想法"这个概念后，用 SwiftUI 从零重写为原生 macOS 应用。

改名是因为定位更清晰了。这不是"闪念笔记"，而是为每一个值得记住的想法**留下痕迹**——在它消失之前。

## 7. 联系

作者：**Sam Song**

- X / Twitter: [@SamSongAI](https://twitter.com/SamSongAI)
- WeChat: 扫码添加 👇

<p align="center">
  <img src="Sources/Trace/Resources/wechat-qr.jpg" width="200" alt="WeChat QR" />
</p>

## License

[MIT](LICENSE) — 自由使用、fork、改造。
