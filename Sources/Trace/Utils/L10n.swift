import Foundation

/// Centralized localization strings. All UI text reads from `AppSettings.shared.language`.
/// Views that observe `AppSettings` will automatically refresh when language changes.
enum L10n {
    private static func s(_ zh: String, _ ja: String, _ en: String) -> String {
        switch AppSettings.shared.language {
        case .zh: return zh
        case .ja: return ja
        case .en: return en
        }
    }

    // MARK: - Settings Sections

    static var language: String { s("语言", "言語", "Language") }
    static var theme: String { s("主题", "テーマ", "Theme") }
    static var storage: String { s("保存位置", "保存先", "Storage") }
    static var quickSections: String { s("快捷分类", "クイックセクション", "Quick Sections") }
    static var shortcuts: String { s("快捷键", "ショートカット", "Shortcuts") }
    static var system: String { s("系统", "システム", "System") }

    // MARK: - Settings Labels

    static var writeMode: String { s("写入模式", "書き込みモード", "Write Mode") }
    static var vault: String { s("笔记库", "ノート保管庫", "Vault") }
    static var vaultHintDimension: String {
        s("Obsidian Vault 根目录，或其他笔记库的根路径",
          "Obsidian Vault のルートディレクトリ、または他のノート保管庫のルートパス",
          "Root directory of your Obsidian Vault or note library")
    }
    static var dailyFolder: String { s("日记文件夹", "デイリーフォルダ", "Daily Folder") }
    static var dailyFolderHint: String {
        s("笔记库内存放日记的子文件夹名称，建议与 Obsidian 日记设置一致",
          "デイリーノート用のサブフォルダ名。Obsidian のデイリーノート設定と合わせてください",
          "Subfolder name for daily notes, should match your Obsidian daily notes settings")
    }
    static var imageAssetsFolder: String { s("图片文件夹", "画像フォルダ", "Image Assets Folder") }
    static var imageAssetsFolderHint: String {
        s("粘贴图片的存放子文件夹名称，默认为 assets",
          "貼り付け画像の保存先サブフォルダ名。デフォルトは assets",
          "Subfolder name for pasted images, defaults to assets")
    }
    static var fileNameFormat: String { s("文件名格式", "ファイル名の形式", "File Name Format") }
    static var entryFormat: String { s("条目格式", "エントリー形式", "Entry Format") }
    static var sectionName: String { s("模块名", "セクション名", "Section Name") }
    static var launchAtLogin: String { s("开机自启动", "ログイン時に起動", "Launch at Login") }

    // MARK: - Settings Buttons

    static var browse: String { s("选择", "選択", "Browse") }
    static var chooseFolder: String { s("指定位置", "場所を指定", "Set Location") }
    static var addSection: String { s("新增模块", "セクションを追加", "Add Section") }
    static var save: String { s("保存", "保存", "Save") }
    static var deleteSection: String { s("删除模块", "セクションを削除", "Delete Section") }
    static var edit: String { s("修改", "変更", "Edit") }
    static var cancel: String { s("取消", "キャンセル", "Cancel") }

    // MARK: - Shortcut Names

    static var shortcutCreate: String { s("创建笔记", "ノートを作成", "Create Note") }
    static var shortcutSend: String { s("发送笔记", "ノートを送信", "Send Note") }
    static var shortcutAppend: String { s("追加上一条", "前回に追加", "Append to Last") }
    static var shortcutToggleMode: String { s("切换写入模式", "書き込みモード切替", "Toggle Write Mode") }
    static var shortcutClosePanel: String { s("关闭面板", "パネルを閉じる", "Close Panel") }
    static var shortcutPinPanel: String { s("固定面板", "パネルを固定", "Pin Panel") }
    static var shortcutSwitchSection: String { s("切换模块/线程", "セクション/スレッド切替", "Switch Section/Thread") }

    // MARK: - Shortcut Categories

    static var shortcutCategoryGlobal: String { s("全局", "グローバル", "Global") }
    static var shortcutCategoryPanel: String { s("面板内", "パネル内", "Panel") }

    // MARK: - Shortcut Recorder

    static var recording: String { s("按键录制中…", "キー入力中…", "Recording...") }
    static var needModifierKey: String {
        s("需要至少一个修饰键（⌘/⇧/⌥/⌃）",
          "修飾キーが1つ以上必要です（⌘/⇧/⌥/⌃）",
          "At least one modifier key required (⌘/⇧/⌥/⌃)")
    }
    static var escReserved: String {
        s("Esc 已用于关闭面板",
          "Esc はパネルを閉じるために予約されています",
          "Esc is reserved for closing the panel")
    }
    static var cmdNumberReserved: String {
        s("⌘1–9 已用于切换模块",
          "⌘1–9 はセクション切替に予約されています",
          "⌘1–9 is reserved for switching sections")
    }
    static func shortcutConflict(with name: String) -> String {
        s("与「\(name)」冲突",
          "「\(name)」と競合しています",
          "Conflicts with \"\(name)\"")
    }

    // MARK: - Write Mode

    static var writeModeDailyTitle: String { s("日记", "デイリー", "Daily") }
    static var writeModeDailyDestination: String { s("追加到当天日记", "今日のデイリーに追加", "Append to today's daily") }
    static var writeModeDailySummary: String {
        s("追加到当天的日记文件，适合快速收集和后续整理。",
          "今日のデイリーファイルに追加。素早いメモや後での整理に最適。",
          "Append to today's daily file. Great for quick capture and later review.")
    }
    static var writeModeDailyTarget: String {
        s("按模块追加到当天日记，底部保留自定义模块切换。",
          "セクションごとに今日のデイリーに追加。下部でセクション切替可能。",
          "Append by section to today's daily note, with section switching at bottom.")
    }

    // MARK: - Thread Mode

    static var writeModeDailyCompact: String { s("Note", "Note", "Note") }
    static var writeModeThreadTitle: String { s("线程", "スレッド", "Thread") }
    static var writeModeThreadCompact: String { s("Thread", "Thread", "Thread") }
    static var writeModeThreadDestination: String { s("追加到线程", "スレッドに追加", "Append to thread") }
    static var writeModeThreadSummary: String {
        s("按主题追加到对应线程文件，适合连续追踪同一话题。",
          "テーマごとに対応するスレッドファイルに追加。同じトピックの継続的なトラッキングに最適。",
          "Append to corresponding thread file by topic. Great for continuous tracking of the same topic.")
    }
    static var writeModeThreadTarget: String {
        s("按主题追加到对应线程文件，方便连续追踪。",
          "テーマごとに対応するスレッドファイルに追加。継続的なトラッキングに最適。",
          "Append to corresponding thread file by topic for continuous tracking.")
    }

    // MARK: - Agent Mode

    static var writeModeAgentCompact: String { s("Agent", "Agent", "Agent") }
    static var threadPlaceholder: String {
        s("输入想法，追加到选中线程...",
          "アイデアを入力してスレッドに追加...",
          "Type your thought to append to thread...")
    }
    static var noThreadSelected: String {
        s("请选择一个线程",
          "スレッドを選択してください",
          "Please select a thread")
    }
    static var vaultHintThread: String {
        s("线程文件将保存在此目录",
          "スレッドファイルはここに保存されます",
          "Thread files will be saved here")
    }
    static var threadManagement: String { s("线程管理", "スレッド管理", "Thread Management") }
    static var newThreadDefaultName: String { s("新线程", "新しいスレッド", "New Thread") }
    static var addThread: String { s("添加线程", "スレッドを追加", "Add Thread") }
    static var deleteThread: String { s("删除线程", "スレッドを削除", "Delete Thread") }
    static var threadName: String { s("名称", "名前", "Name") }
    static var threadTargetFile: String { s("目标文件路径", "対象ファイルパス", "Target file path") }
    static var folderPath: String { s("文件夹", "フォルダ", "Folder") }
    static var fileName: String { s("文件名", "ファイル名", "Filename") }
    static var rootFolder: String { s("根目录", "ルート", "Root") }

    // MARK: - Entry Theme Presets

    static var entryCodeBlock: String { s("代码块（推荐）", "コードブロック（推奨）", "Code Block (Recommended)") }
    static var entryPlainText: String { s("文本 + 时间戳", "テキスト＋タイムスタンプ", "Text + Timestamp") }
    static var entryQuote: String { s("引用（Markdown）", "引用（Markdown）", "Quote (Markdown)") }

    // MARK: - Separator Styles

    static var separatorNone: String { s("仅空行", "空行のみ", "Empty lines only") }
    static var separatorHorizontalRule: String { s("--- 分割线", "--- 区切り線", "--- divider") }
    static var separatorAsteriskRule: String { s("*** 分割线", "*** 区切り線", "*** divider") }

    // MARK: - Vault Validation

    static var vaultEmpty: String {
        s("请先选择笔记库文件夹。",
          "ノート保管庫のフォルダを選択してください。",
          "Please select a vault folder first.")
    }
    static var vaultNotExist: String {
        s("笔记库路径不存在，请重新选择。",
          "ノート保管庫のパスが存在しません。再選択してください。",
          "Vault path does not exist. Please select again.")
    }
    static var vaultNotDirectory: String {
        s("笔记库路径必须是文件夹。",
          "ノート保管庫のパスはフォルダである必要があります。",
          "Vault path must be a directory.")
    }
    static var vaultNotWritable: String {
        s("笔记库路径不可写，请检查文件夹权限。",
          "ノート保管庫のパスに書き込めません。フォルダの権限を確認してください。",
          "Vault path is not writable. Check folder permissions.")
    }

    // MARK: - Capture Panel

    static var notePlaceholder: String { s("输入笔记内容...", "ノートを入力...", "Type your note...") }
    static var pinPanelHelp: String {
        s("固定面板，保存后不关闭 (⌘P)",
          "パネルを固定、保存後も閉じない (⌘P)",
          "Pin panel, stay open after save (⌘P)")
    }
    static var settingsTooltip: String { s("设置", "設定", "Settings") }

    // MARK: - Toast & Alerts

    static var emptyNotSaved: String { s("内容为空，未保存", "内容が空のため保存されませんでした", "Empty content, not saved") }
    static var saveFailed: String { s("保存失败", "保存に失敗しました", "Save Failed") }
    static var savedTo: String {
        s("已保存到「%@」",
          "「%@」に保存しました",
          "Saved to \"%@\"")
    }

    // MARK: - Global Hotkey Alert

    static var hotkeyRegistrationFailed: String {
        s("无法注册全局快捷键",
          "グローバルショートカットの登録に失敗しました",
          "Cannot register global hotkey")
    }
    static var hotkeyConflictMessage: String {
        s("当前快捷键可能与其他应用冲突，请前往设置修改快捷键。",
          "現在のショートカットが他のアプリと競合している可能性があります。設定で変更してください。",
          "The current shortcut may conflict with other apps. Go to Settings to change it.")
    }
    static var openSettings: String { s("打开设置", "設定を開く", "Open Settings") }
    static var later: String { s("稍后", "後で", "Later") }

    // MARK: - Writer Errors

    static var vaultNotConfigured: String {
        s("笔记库路径未配置，请点击右上角 ⚙ 进入设置。",
          "ノート保管庫のパスが未設定です。右上の ⚙ をクリックして設定してください。",
          "Vault path not configured. Click the ⚙ icon to open Settings.")
    }
    static var invalidTargetFolder: String {
        s("目标目录必须是笔记库内的相对路径，且不能包含 ..",
          "対象ディレクトリは保管庫内の相対パスで、.. を含むことはできません",
          "Target must be a relative path within the vault and cannot contain ..")
    }
    static var imageVaultNotConfigured: String {
        s("笔记库路径未配置或不可写，无法保存图片。",
          "ノート保管庫のパスが未設定または書き込み不可のため、画像を保存できません。",
          "Vault path not configured or not writable. Cannot save image.")
    }
    static var imageEncodingFailed: String {
        s("图片编码失败，无法写入 PNG 文件。",
          "画像のエンコードに失敗しました。PNG ファイルを書き込めません。",
          "Image encoding failed. Cannot write PNG file.")
    }
    static var imagePastedOne: String {
        s("已粘贴 1 张图片",
          "画像 1 枚を貼り付けました",
          "Pasted 1 image")
    }
    static var imagePastedMultiple: String {
        s("已粘贴 %d 张图片",
          "画像 %d 枚を貼り付けました",
          "Pasted %d images")
    }
    static var imagePasteFailed: String {
        s("图片粘贴失败",
          "画像の貼り付けに失敗しました",
          "Image paste failed")
    }
    static var imageNoImageFound: String {
        s("剪贴板中没有图片",
          "クリップボードに画像がありません",
          "No image found in clipboard")
    }

    // MARK: - Theme Descriptions

    static var themeLightSummary: String {
        s("参考 Obsidian Light 的灰白底色和紫色强调，更干净、更通用。",
          "Obsidian Light にインスパイアされたグレースケールとパープルアクセント。",
          "Clean grayscale with purple accent, inspired by Obsidian Light.")
    }
    static var themeDarkSummary: String {
        s("黑白灰夜间主题，保留足够对比度，避免紫色品牌偏移。",
          "十分なコントラストを保つダークテーマ。",
          "Dark theme with strong contrast, no purple brand shift.")
    }
    static var themePaperSummary: String {
        s("纸张米白与墨黑正文，适合长时间阅读、整理和静态编辑。",
          "紙のアイボリーとインクブラックの本文。長時間の読書と編集に最適。",
          "Paper white with ink-black text, ideal for long reading and editing.")
    }
    static var themeDuneSummary: String {
        s("燕麦底与陶土橙强调，整体更暖、更柔和，也更有材料感。",
          "オートミールの背景と陶土オレンジのアクセント。温かみのある質感。",
          "Oat background with clay-orange accent, warm and tactile.")
    }

    // MARK: - AI Agent
    static var ai: String { s("AI 助手", "AI アシスタント", "AI Agent") }
    static var aiSettings: String { s("AI 设置", "AI 設定", "AI Settings") }
    static var aiEnable: String { s("启用 AI 助手", "AI アシスタントを有効化", "Enable AI Agent") }
    static var aiApiKey: String { s("API Key", "API キー", "API Key") }
    static var aiApiKeyHint: String {
        s("OpenAI / Anthropic / 其他兼容服务的 API Key",
          "OpenAI / Anthropic / 互換サービスの API キー",
          "API key for OpenAI, Anthropic, or any OpenAI-compatible service")
    }
    static var aiModel: String { s("模型", "モデル", "Model") }
    static var aiModelHint: String {
        s("如 gpt-4o-mini, claude-3-5-sonnet, 或本地 Ollama 模型名",
          "例: gpt-4o-mini, claude-3-5-sonnet, またはローカル Ollama モデル名",
          "e.g. gpt-4o-mini, claude-3-5-sonnet, or local Ollama model name")
    }
    static var aiEndpoint: String { s("API 地址", "API エンドポイント", "API Endpoint") }
    static var aiEndpointHint: String {
        s("OpenAI 兼容的 API 地址，本地 Ollama 用 http://localhost:11434/v1",
          "OpenAI 互換の API エンドポイント。ローカル Ollama は http://localhost:11434/v1",
          "OpenAI-compatible API endpoint. For local Ollama: http://localhost:11434/v1")
    }
    static var aiNotConfigured: String {
        s("AI 助手尚未配置。\n请在设置中填写 API Key 后使用。",
          "AI アシスタントが未設定です。\n設定で API キーを入力してください。",
          "AI Agent is not configured.\nPlease set your API key in Settings.")
    }
    static var aiInputPlaceholder: String { s("问点什么…", "何か聞く…", "Ask anything…") }
    static var aiAgentTooltip: String { s("AI 助手", "AI アシスタント", "AI Agent") }
}
