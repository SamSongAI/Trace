<p align="center">
  <img src="Sources/Trace/Resources/logo.png" width="128" height="128" alt="Trace logo" />
</p>

<h1 align="center">Trace</h1>

<p align="center">
  <strong>System-level quick capture for macOS. Writes directly to Markdown files.</strong><br/>
  Press a hotkey from any app. Floating panel appears. Type. Done.
</p>

<p align="center">
  <a href="https://github.com/SamSong1997/Trace/releases/latest">Download</a> · <a href="#2-features">Features</a> · <a href="#3-use-with-obsidian">Obsidian Setup</a> · <a href="#5-keyboard-shortcuts">Shortcuts</a> · <a href="README.md">中文</a>
</p>

---

## 1. Download

Get the latest `Trace.dmg` from [GitHub Releases](https://github.com/SamSong1997/Trace/releases/latest).

1. Open the DMG and drag Trace into Applications
2. Launch Trace
3. Click ⚙ in the top-right corner to configure your vault path

Requires macOS 13+.

> Developers can also build from source: `git clone` → `./scripts/trace.sh install`

## 2. Features

- **Global hotkey** — Summon the capture panel from any app, fully customizable
- **Diary / Document mode** — Diary appends to today's file; Document creates a standalone .md file. Toggle with `⇧Tab`
- **Custom categories** — Configurable category buttons (e.g. "Ideas", "Tasks", "Links") that map to headings in your daily file
- **Separate paths** — Diary and documents can point to different folders
- **Pin mode** — Panel stays open after save for rapid-fire capture
- **Native macOS** — Not Electron, not a web wrapper
- **Zero network** — No accounts, no telemetry, no cloud

## 3. Use with Obsidian

Trace writes local `.md` files, making it a natural fit for Obsidian. The key is aligning Trace's write path with Obsidian's read path.

The final file path for diary entries is determined by three settings:

```
Vault Path / Diary Folder / Filename Format.md
```

Match these with your Obsidian Daily Notes plugin settings:

| Trace Setting | Obsidian Equivalent | Example |
|---|---|---|
| **Vault** | Vault root directory | `/Users/you/MyVault` |
| **Diary Folder** | New file location | `Daily` |
| **Filename Format** | Date format | `yyyy-MM-dd` |

**Setup steps:**

1. Obsidian → Settings → Daily Notes → note the "New file location" and "Date format"
2. Trace → ⚙ Settings → Diary mode
3. **Vault**: your Obsidian vault root (the folder containing `.obsidian`)
4. **Diary Folder**: same value as Obsidian's "New file location" (default: `Daily`)
5. **Filename Format**: choose the format matching Obsidian

Once aligned, press `⌘D` in Obsidian to open today's note — you'll see what Trace just wrote.

> Trace doesn't require Obsidian. Any local Markdown system (Logseq, Typora, iA Writer, etc.) works.

## 4. Why Trace

When a thought hits mid-task, you have two bad options: lose it, or break your flow to write it down. Every `⌘Tab` to your note app fragments your attention.

Trace solves this at the OS level. One hotkey, a floating panel, write and go. You never leave your current work.

Design principles:

- **Capture before organize** — Save the thought first, sort it later
- **Zero friction** — Global hotkey, no load time
- **Local first** — Files on your disk, no cloud
- **Do one thing well** — Capture only, nothing else

## 5. Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `⌘N` | Open capture panel (customizable) |
| `⌘Enter` | Save |
| `⌘⇧Enter` | Append to last entry |
| `⇧Tab` | Toggle Diary / Document mode |
| `⌘P` | Toggle Pin mode |
| `⌘1-9` | Switch categories |
| `Esc` | Dismiss |

## 6. Origin

Trace started as [ObsidianFlashNote](https://github.com/SamSong1997/ObsidianFlashNote) — a Hammerspoon Lua script that proved the concept. It was rewritten from scratch as a native macOS app in SwiftUI.

## 7. Contact

Built by **Sam Song**.

- X / Twitter: [@SamSongAI](https://twitter.com/SamSongAI)
- WeChat: scan to connect 👇

<p align="center">
  <img src="Sources/Trace/Resources/wechat-qr.jpg" width="200" alt="WeChat QR" />
</p>

## License

[MIT](LICENSE) — use it, fork it, make it yours.
