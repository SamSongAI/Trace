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
  <a href="#设计哲学">设计哲学</a> · <a href="#工作原理">工作原理</a> · <a href="#核心功能">功能</a> · <a href="#与-obsidian-配合使用">Obsidian</a> · <a href="#安装">安装</a> · <a href="#联系">联系</a>
</p>

---

## 为什么做 Trace

每个知识工作者都有同样的问题：灵感来的时候，你正在做别的事。

两个选择都很糟——要么忘掉它，要么打断当前的工作去记录。每一次 `⌘Tab` 切换窗口，注意力就碎了一次。回到心流状态的代价远比你想象的高。

**Trace 在操作系统层面解决这个问题。** 一个快捷键唤起浮动面板，写完按 `⌘Enter`，内容直接写入本地 Markdown 文件。你从未离开过当前的工作。

这不是又一个笔记应用。这是一个**注意力保护工具**。

## 设计哲学

- **捕获优先于整理** — 先留住想法，分类是后面的事。Trace 只做捕获这一步，把整理留给你的笔记系统。
- **零摩擦** — 全局快捷键唤起，写完即走。没有加载时间，没有多余的步骤。
- **本地优先** — 没有账号，没有云端，没有遥测。你的文件就在你的硬盘上。
- **轻量聚焦** — 不做编辑器，不做知识库，不做 Markdown 渲染。只把"记下来"这件事做到极致。

## 工作原理

```
任意应用  ──快捷键──▶  Trace 浮动面板  ──⌘Enter──▶  本地 .md 文件
                      (系统级悬浮)                  (直接文件写入)
```

Trace 通过 `FileManager` 直接写入本地 `.md` 文件。没有插件依赖，没有同步冲突，没有中间件，没有网络请求。纯粹的本地文件 I/O。

## 核心功能

- **全局快捷键** — 从任何应用唤起捕获面板，快捷键可自定义。
- **双写入模式** — **日记模式**（追加到当天的日记文件）或 **文档模式**（创建独立 .md 文件），用 `⇧Tab` 一键切换。
- **自定义快捷分类** — 日记模式下，底部显示分类按钮（如"想法""待办""灵感"），每个分类对应日记文件中的一个标题。分类数量和名称完全可自定义。
- **独立笔记库路径** — 日记和文档可以指向不同的文件夹，互不干扰。
- **Pin 模式** — 连续捕获场景下，面板保存后不关闭，适合高频记录。
- **原生 macOS 体验** — SwiftUI + AppKit 构建，不是 Electron，不是 Web 套壳。
- **零网络** — 没有账号，没有遥测，没有云端。你的数据完全属于你。

## 与 Obsidian 配合使用

Trace 直接写入本地 `.md` 文件，天然适配 Obsidian。关键是让 Trace 的写入路径和 Obsidian 的读取路径对齐。

### 日记模式：对齐 Obsidian Daily Notes

Trace 的日记模式会将内容追加到当天的日记文件。文件的最终路径由三个设置决定：

```
笔记库路径 / 日记文件夹 / 文件名格式.md
```

你需要确保这三个值和 Obsidian 的日记插件设置一致：

| Trace 设置 | 对应 Obsidian 日记插件设置 | 示例 |
|---|---|---|
| **笔记库** | Vault 根目录路径 | `/Users/you/MyVault` |
| **日记文件夹** | 新笔记的存放位置 | `Daily` |
| **文件名格式** | 日期格式 | `yyyy M月d日 EEEE` |

**配置步骤：**

1. 打开 Obsidian → 设置 → 日记 → 查看「新笔记的存放位置」和「日期格式」
2. 打开 Trace → 设置 → 日记模式
3. **笔记库**：填 Obsidian Vault 的根目录（包含 `.obsidian` 文件夹的那一层）
4. **日记文件夹**：填 Obsidian 日记插件中「新笔记的存放位置」的值（默认 `Daily`）
5. **文件名格式**：选择和 Obsidian 日期格式一致的选项

对齐后，Trace 写入的日记文件就是 Obsidian 打开的同一个文件。在 Obsidian 中按 `⌘D` 打开今天的日记，就能看到刚才通过 Trace 写入的内容。

### 文档模式：独立 .md 文件

文档模式会在指定文件夹中创建独立的 `.md` 文件。将路径指向 Vault 内的任意文件夹即可，打开 Obsidian 就能看到新文件。

### 不绑定 Obsidian

Trace 本质是一个 Markdown 文件写入工具。任何基于本地 `.md` 文件的笔记系统（Logseq、Typora、iA Writer 等）都可以配合使用。不需要 Obsidian 运行，不需要安装插件。

## 快捷键

| 快捷键 | 功能 |
|---|---|
| `⌘N` | 唤起捕获面板（可自定义） |
| `⌘Enter` | 保存到当前分类 |
| `⌘⇧Enter` | 追加到上一条记录 |
| `⇧Tab` | 切换日记 / 文档模式 |
| `⌘P` | 切换 Pin 模式 |
| `⌘1-9` | 快速切换分类 |
| `Esc` | 关闭面板 |

## 安装

环境要求：**Xcode 16+**，**macOS 13+**。

```bash
git clone https://github.com/SamSong1997/Trace.git
cd Trace
./scripts/trace.sh install       # 构建并安装到 /Applications
./scripts/trace.sh launch-app    # 启动 Trace
```

首次运行后，点击菜单栏图标 → 设置 → 配置笔记库路径 → 开始使用。

## 项目结构

```
Sources/Trace/
├── App/          # 应用生命周期、菜单栏
├── UI/
│   ├── Capture/  # 浮动捕获面板
│   └── Settings/ # 设置界面
├── Services/     # 快捷键注册、文件写入、持久化
├── Models/       # 数据模型
├── Utils/        # 主题、快捷键、品牌资源
└── Resources/    # 图标、资源文件
```

**技术栈：** SwiftUI + AppKit · `CGEvent` 全局快捷键 · `FileManager` 直接 .md 写入 · 零外部依赖

```bash
swift build                   # 编译
swift test                    # 测试
./scripts/trace.sh check      # 编译 + 测试
```

## 起源

Trace 的前身是 [ObsidianFlashNote](https://github.com/SamSong1997/ObsidianFlashNote)——一个 Hammerspoon Lua 脚本，验证了"不离开当前应用就能捕获想法"这个概念。后来用 SwiftUI 从零重写为原生 macOS 应用。

改名是因为定位更清晰了。这不是"闪念笔记"，而是为每一个值得记住的想法**留下痕迹**——在它消失之前。

## 联系

作者：**Sam Song**

Trace 是 [SOTA Sync](https://sotasync.com) 的一部分——关于 AI、生产力和构建手艺的工具与思考。

- X / Twitter: [@SamSongAI](https://twitter.com/SamSongAI)
- WeChat: 扫码添加 👇

<p align="center">
  <img src="Sources/Trace/Resources/wechat-qr.jpg" width="200" alt="WeChat QR" />
</p>

## License

[MIT](LICENSE) — 自由使用、fork、改造。
