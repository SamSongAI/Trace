<p align="center">
  <img src="Sources/Trace/Resources/logo.png" width="128" height="128" alt="Trace logo" />
</p>

<h1 align="center">Trace</h1>

<p align="center">
  <strong>System-level quick capture for Obsidian on macOS.</strong><br/>
  Press <code>⌘N</code> anywhere. Write to your Daily Note in 2 seconds. Never leave your current app.
</p>

<p align="center">
  <em>Thought is leverage. Leave a trace.</em>
</p>

<p align="center">
  <a href="#why">Why</a> · <a href="#how-it-works">How It Works</a> · <a href="#features">Features</a> · <a href="#installation">Install</a> · <a href="#connect">Connect</a>
</p>

---

## Why

Every knowledge worker has the same problem: a thought hits you mid-task, and you have two bad options — lose it, or break your flow to write it down.

Obsidian plugins don't help here. They only work when Obsidian is open. Every time you `⌘Tab` to capture a thought, your attention fragments. That context switch isn't 2 seconds — it's 2 minutes of getting back into the zone.

**Trace operates at the OS level.** One hotkey from any app, a floating panel appears, you type, `⌘Enter`, gone. Your Daily Note gets the entry. You never left your work.

This isn't about note-taking. It's about **protecting your attention while capturing every signal**.

## How It Works

```
Any App  ──⌘N──▶  Trace Panel  ──⌘Enter──▶  Obsidian Vault
                  (floating)                 (direct .md write)
```

Trace writes directly to your Obsidian vault via `FileManager`. No plugins. No sync conflicts. No middleware. No network calls. Pure local file I/O.

## Features

- **Global hotkey** — `⌘N` (customizable) from any app. Obsidian doesn't need to be open.
- **5 capture zones** — Note / Clip / Link / Task / Project. Each maps to a heading in your Daily Note.
- **Dual write mode** — Daily Note (append to today's sections) or Inbox (standalone .md files). Toggle with `⇧Tab`.
- **Pin mode** — Rapid-fire capture. Panel stays open after each save.
- **Native macOS** — SwiftUI + AppKit. Not Electron, not a web wrapper. Feels like part of the OS.
- **Zero network** — No accounts, no telemetry, no cloud. Your vault is yours.

## The Problem with Alternatives

| | Trace | QuickAdd (plugin) | Alfred / Raycast |
|---|---|---|---|
| Obsidian must be open | **No** | Yes | No |
| Zone-based Daily Note capture | **5 zones** | Needs config | No |
| Direct .md write | **Yes** | Yes | Via plugin |
| Native macOS UI | **Yes** | Web render | Yes |
| Runs as standalone app | **Yes** | No | Yes |

QuickAdd is great — inside Obsidian. Alfred can append text — to plain files. Trace is purpose-built for one thing: **getting thoughts into your Obsidian vault at OS speed**.

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `⌘N` | Open capture panel (customizable) |
| `⌘Enter` | Save to current zone |
| `⌘⇧Enter` | Append to last entry in same zone |
| `⇧Tab` | Toggle Daily / Inbox mode |
| `Esc` | Dismiss |

## Installation

Prerequisites: **Xcode 16+** and **macOS 13+**.

```bash
git clone https://github.com/SamSong1997/Trace.git
cd Trace
./scripts/trace.sh install       # build + copy to /Applications
./scripts/trace.sh launch-app    # start Trace
```

**First run:** Menu bar icon → Settings → set your Vault path → done.

## Under the Hood

```
Sources/Trace/
├── App/          # App lifecycle, menu bar
├── UI/
│   ├── Capture/  # Floating capture panel
│   └── Settings/ # Preferences window
├── Services/     # Hotkey registration, Daily Note writer, persistence
├── Models/       # Data models
├── Utils/        # Theme, keyboard shortcuts, brand assets
└── Resources/    # Icons, assets
```

**Stack:** SwiftUI + AppKit · `CGEvent` global hotkey · `FileManager` direct .md write · Zero dependencies.

```bash
swift build                   # compile
swift test                    # test
./scripts/trace.sh check      # build + test
```

## Origin Story

Trace started as [ObsidianFlashNote](https://github.com/SamSong1997/ObsidianFlashNote) — a Hammerspoon Lua script that proved the concept: you can capture thoughts without leaving your current app. The response was clear enough that I rewrote it from scratch as a native macOS app in SwiftUI.

The name changed because the mission became clearer. This isn't about "flash notes." It's about leaving a **trace** of every thought that matters — before it evaporates.

> 万象皆留痕。

## Connect

Built by **Sam Song**.

I'm building Trace as part of [SOTA Sync](https://sotasync.com) — tools and thinking around AI, productivity, and the craft of building.

- X / Twitter: [@SamSongAI](https://twitter.com/SamSongAI)
- WeChat: scan to connect 👇

<p align="center">
  <img src="Sources/Trace/Resources/wechat-qr.jpg" width="200" alt="WeChat QR" />
</p>

## License

[MIT](LICENSE) — use it, fork it, make it yours.
