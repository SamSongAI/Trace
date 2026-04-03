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
  <a href="https://github.com/SamSong1997/Trace/releases/latest">下载</a> · <a href="#2-功能">功能</a> · <a href="#3-配合-obsidian-使用">Obsidian 配置</a> · <a href="#5-快捷键">快捷键</a>
</p>

---

## 1. 下载安装

从 [GitHub Releases](https://github.com/SamSong1997/Trace/releases/latest) 下载最新的 `Trace.dmg`。

1. 打开 DMG，将 Trace 拖入 Applications 文件夹
2. 启动 Trace
3. 点击右上角 ⚙ 进入设置，配置笔记库路径

系统要求：macOS 13+。

> 开发者也可以从源码构建：`git clone` → `./scripts/trace.sh install`

## 2. 功能

- **全局快捷键** — 从任何应用唤起捕获面板，快捷键可自定义
- **日记模式 / 文档模式** — 日记追加到当天文件，文档创建独立 .md 文件，`⇧Tab` 切换
- **自定义分类** — 日记模式下可配置分类按钮（如"想法""待办""灵感"），对应日记中的标题
- **独立路径** — 日记和文档可以指向不同的文件夹
- **Pin 模式** — 保存后面板不关闭，适合连续记录
- **原生 macOS** — 不是 Electron，不是 Web 套壳
- **零网络** — 没有账号，没有遥测，没有云端

## 3. 配合 Obsidian 使用

Trace 写入本地 `.md` 文件，天然适配 Obsidian。关键是让写入路径和 Obsidian 对齐。

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

> Trace 不绑定 Obsidian。任何本地 Markdown 笔记系统（Logseq、Typora、iA Writer 等）都可以配合使用。

## 4. 为什么做 Trace

灵感来的时候，你正在做别的事。`⌘Tab` 切到笔记应用去记录，注意力就碎了。

Trace 在操作系统层面解决这个问题。一个快捷键，一个浮动面板，写完即走。你从未离开当前的工作。

设计原则：

- **捕获优先于整理** — 先留住想法，分类是后面的事
- **零摩擦** — 全局快捷键唤起，没有加载时间
- **本地优先** — 文件在你的硬盘上，没有云端
- **轻量聚焦** — 只做捕获这一件事

## 5. 快捷键

| 快捷键 | 功能 |
|---|---|
| `⌘N` | 唤起捕获面板（可自定义） |
| `⌘Enter` | 保存 |
| `⌘⇧Enter` | 追加到上一条 |
| `⇧Tab` | 切换日记 / 文档模式 |
| `⌘P` | 切换 Pin 模式 |
| `⌘1-9` | 快速切换分类 |
| `Esc` | 关闭面板 |

## 6. 起源

Trace 的前身是 [ObsidianFlashNote](https://github.com/SamSong1997/ObsidianFlashNote)——一个 Hammerspoon Lua 脚本。验证了"不离开当前应用就能捕获想法"这个概念后，用 SwiftUI 从零重写为原生 macOS 应用。

## 7. 联系

作者：**Sam Song**

- X / Twitter: [@SamSongAI](https://twitter.com/SamSongAI)
- WeChat: 扫码添加 👇

<p align="center">
  <img src="Sources/Trace/Resources/wechat-qr.jpg" width="200" alt="WeChat QR" />
</p>

## License

[MIT](LICENSE) — 自由使用、fork、改造。
