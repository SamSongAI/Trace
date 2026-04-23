<p align="center">
  <img src="Sources/Trace/Resources/logo.png" width="128" height="128" alt="Trace logo" />
</p>

<h1 align="center">Trace</h1>

<p align="center">
  <strong>System-level quick capture for macOS. Writes directly to Markdown files.</strong><br/>
  Press a hotkey from any app. Floating panel appears. Type. Done.
</p>

<p align="center">
  <a href="#1-why-trace">Why</a> · <a href="#2-features">Features</a> · <a href="#3-use-with-obsidian">Obsidian Setup</a> · <a href="CHANGELOG.md">Changelog</a> · <a href="https://github.com/SamSongAI/Trace/releases/latest">Download</a> · <a href="README.md">中文</a>
</p>

---

## 1. Why Trace

When a thought hits mid-task, you have two bad options: lose it, or break your flow to write it down.

Every `⌘Tab` to your note app fragments your attention. The context switch costs way more than the few seconds it takes. Getting back into the zone is the real price.

**Trace solves this at the OS level.** One hotkey summons a floating panel. Type your thought, press `⌘Enter`, it's written to a local Markdown file. You never left your work.

This isn't another note-taking app. It's an **attention protection tool**.

**Design principles:**

- **Capture before organize** — Save the thought first. Sorting is your note app's job
- **Zero friction** — Global hotkey, no load time, no extra steps
- **Local first** — No accounts, no cloud, no telemetry. Your files stay on your disk
- **Do one thing well** — No editor, no knowledge base. Just capture, done right

## 2. Features

```
Any App  ──hotkey──▶  Trace Panel  ──⌘Enter──▶  Local .md file
                     (floating)                 (direct file write)
```

- **Global hotkey** — Summon the capture panel from any app, fully customizable
- **Diary / Thread / Document mode** — Diary appends to today's file; Thread appends by topic to a chosen thread file; Document creates a standalone .md. Cycle modes with `⇧Tab`
- **Custom categories** — Configurable buttons (e.g. "Ideas", "Tasks", "Links") that map to headings in your daily file
- **Thread management** — Preconfigure thread targets for ongoing topics, projects, or problem tracks
- **Separate paths** — Diary and documents can point to different folders
- **Pin mode** — Panel stays open after save for rapid-fire capture
- **Native macOS** — SwiftUI + AppKit. Not Electron, not a web wrapper
- **Zero network** — No accounts, no telemetry, no cloud. Your data is yours

## 3. Use with Obsidian

Trace writes local `.md` files, making it a natural fit for Obsidian. The key is aligning Trace's write path with Obsidian's read path.

The final file path for diary entries is determined by three settings:

```
Vault Path / Diary Folder / Filename Format.md
```

Match these with your Obsidian Daily Notes plugin:

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

If you use Thread mode, you can also point thread files at any Markdown path inside the same vault, so diary entries, topic threads, and standalone documents can live in one local workflow.

> Trace doesn't require Obsidian. Any local Markdown system (Logseq, Typora, iA Writer, etc.) works. Obsidian doesn't need to be running. No plugins required.

## 4. Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `⌘N` | Open capture panel (customizable) |
| `⌘Enter` | Save |
| `⌘⇧Enter` | Append to last entry |
| `⇧Tab` | Cycle Diary / Thread / Document mode |
| `⌘P` | Toggle Pin mode |
| `⌘1-9` | Switch categories or threads |
| `Esc` | Dismiss |

## 5. Download

All platform installers live on [GitHub Releases](https://github.com/SamSongAI/Trace/releases/latest).

### macOS

Grab `Trace.dmg`.

1. Open the DMG. You'll see `Trace.app` and an `Applications` shortcut. Drag `Trace.app` onto `Applications`
2. On first launch, macOS may show "unverified developer" warning. To fix:
   - **Option A**: Right-click Trace.app → "Open" → click "Open" in the dialog
   - **Option B**: Run `xattr -cr /Applications/Trace.app` in Terminal, then launch normally
3. Click ⚙ in the top-right corner to configure your vault path

Requires macOS 13+.

### Windows

Pick the installer for your CPU architecture:

- **Intel / AMD desktop** (most users): `Trace-0.2.0-x64.exe`
- **ARM laptop** (e.g. Surface Pro X): `Trace-0.2.0-arm64.exe`

1. Double-click the installer. Windows SmartScreen will show a blue "Windows protected your PC" warning — the current build is unsigned. Click "More info" → "Run anyway" to proceed (signed builds via SignPath Foundation are planned)
2. Accept the license in the bootstrapper, click Install. The installer writes to `C:\Program Files\Trace\` and prompts once for UAC
3. Start Menu and Desktop shortcuts are created automatically. On first run, open Settings to configure your vault path

Requires Windows 10 or newer.

## 6. Origin

Trace started as [ObsidianFlashNote](https://github.com/SamSongAI/ObsidianFlashNote) — a Hammerspoon Lua script that proved the concept. After validating that "capture without leaving your current app" was worth solving, it was rewritten from scratch as a native macOS app in SwiftUI.

The name changed because the mission became clearer. This isn't about "flash notes." It's about leaving a **trace** of every thought that matters — before it evaporates.

## 7. Contact

Built by **Sam Song**.

- X / Twitter: [@SamSongAI](https://twitter.com/SamSongAI)

## License

[MIT](LICENSE) — use it, fork it, make it yours.
