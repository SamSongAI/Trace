//! Localized UI strings for the Trace client.
//!
//! Ports `Sources/Trace/Utils/L10n.swift` to pure Rust. In the Swift reference
//! each getter reads `AppSettings.shared.language` implicitly; here the
//! [`Language`] is passed explicitly to every call, which keeps `trace-core`
//! free of global state and makes the string table trivially testable.
//!
//! All non-parameterized strings are returned as `&'static str` (zero
//! allocation). Only [`L10n::shortcut_conflict`] allocates because it
//! interpolates a runtime name into the template.
//!
//! [`Language::SystemDefault`] is treated as an unresolved sentinel: L10n
//! falls back to English so a pre-resolution call still yields a sensible
//! value. The platform layer is responsible for resolving the system locale
//! into a concrete variant before persistence.

use crate::models::Language;

/// Centralized localization strings, mirroring the Swift `L10n` enum.
///
/// All methods are associated functions — `L10n` is a zero-sized marker
/// type that is never instantiated. Call sites: `L10n::vault(lang)`.
pub struct L10n;

/// Picks the variant matching `lang`, falling back to English when the
/// language is still `SystemDefault`.
#[inline]
fn pick(lang: Language, zh: &'static str, ja: &'static str, en: &'static str) -> &'static str {
    match lang {
        Language::Zh => zh,
        Language::Ja => ja,
        Language::En | Language::SystemDefault => en,
    }
}

impl L10n {
    // --- Settings Sections

    pub fn language(lang: Language) -> &'static str {
        pick(lang, "语言", "言語", "Language")
    }

    /// Label for the "follow the OS" language option. The Mac reference has
    /// no equivalent entry because `AppLanguage` on macOS omits the
    /// `.systemDefault` sentinel — Swift resolves the preferred locale at
    /// read time and stores a concrete variant. On Windows the settings UI
    /// exposes "System default" as a first-class choice so the user can
    /// opt out of an explicit override, which means L10n needs its own
    /// string for the chip label.
    pub fn language_system_default(lang: Language) -> &'static str {
        pick(lang, "系统默认", "システム既定", "System default")
    }

    pub fn theme(lang: Language) -> &'static str {
        pick(lang, "主题", "テーマ", "Theme")
    }

    pub fn storage(lang: Language) -> &'static str {
        pick(lang, "保存位置", "保存先", "Storage")
    }

    pub fn quick_sections(lang: Language) -> &'static str {
        pick(lang, "快捷分类", "クイックセクション", "Quick Sections")
    }

    pub fn shortcuts(lang: Language) -> &'static str {
        pick(lang, "快捷键", "ショートカット", "Shortcuts")
    }

    pub fn system(lang: Language) -> &'static str {
        pick(lang, "系统", "システム", "System")
    }

    // --- Settings Labels

    pub fn write_mode(lang: Language) -> &'static str {
        pick(lang, "写入模式", "書き込みモード", "Write Mode")
    }

    pub fn vault(lang: Language) -> &'static str {
        pick(lang, "笔记库", "ノート保管庫", "Vault")
    }

    pub fn vault_hint_dimension(lang: Language) -> &'static str {
        pick(
            lang,
            "Obsidian Vault 根目录，或其他笔记库的根路径",
            "Obsidian Vault のルートディレクトリ、または他のノート保管庫のルートパス",
            "Root directory of your Obsidian Vault or note library",
        )
    }

    pub fn vault_hint_file(lang: Language) -> &'static str {
        pick(
            lang,
            "文档保存的文件夹路径",
            "ドキュメント保存先のフォルダパス",
            "Folder path for document storage",
        )
    }

    pub fn daily_folder(lang: Language) -> &'static str {
        pick(lang, "日记文件夹", "デイリーフォルダ", "Daily Folder")
    }

    pub fn daily_folder_hint(lang: Language) -> &'static str {
        pick(
            lang,
            "笔记库内存放日记的子文件夹名称，建议与 Obsidian 日记设置一致",
            "デイリーノート用のサブフォルダ名。Obsidian のデイリーノート設定と合わせてください",
            "Subfolder name for daily notes, should match your Obsidian daily notes settings",
        )
    }

    pub fn file_name_format(lang: Language) -> &'static str {
        pick(lang, "文件名格式", "ファイル名の形式", "File Name Format")
    }

    pub fn entry_format(lang: Language) -> &'static str {
        pick(lang, "条目格式", "エントリー形式", "Entry Format")
    }

    pub fn section_name(lang: Language) -> &'static str {
        pick(lang, "模块名", "セクション名", "Section Name")
    }

    pub fn launch_at_login(lang: Language) -> &'static str {
        pick(lang, "开机自启动", "ログイン時に起動", "Launch at Login")
    }

    // --- Settings Buttons

    pub fn browse(lang: Language) -> &'static str {
        pick(lang, "选择", "選択", "Browse")
    }

    pub fn choose_folder(lang: Language) -> &'static str {
        pick(lang, "指定位置", "場所を指定", "Set Location")
    }

    pub fn add_section(lang: Language) -> &'static str {
        pick(lang, "新增模块", "セクションを追加", "Add Section")
    }

    pub fn save(lang: Language) -> &'static str {
        pick(lang, "保存", "保存", "Save")
    }

    pub fn delete_section(lang: Language) -> &'static str {
        pick(lang, "删除模块", "セクションを削除", "Delete Section")
    }

    pub fn edit(lang: Language) -> &'static str {
        pick(lang, "修改", "変更", "Edit")
    }

    pub fn cancel(lang: Language) -> &'static str {
        pick(lang, "取消", "キャンセル", "Cancel")
    }

    // --- Shortcut Names

    pub fn shortcut_create(lang: Language) -> &'static str {
        pick(lang, "创建笔记", "ノートを作成", "Create Note")
    }

    pub fn shortcut_send(lang: Language) -> &'static str {
        pick(lang, "发送笔记", "ノートを送信", "Send Note")
    }

    pub fn shortcut_append(lang: Language) -> &'static str {
        pick(lang, "追加上一条", "前回に追加", "Append to Last")
    }

    pub fn shortcut_toggle_mode(lang: Language) -> &'static str {
        pick(
            lang,
            "切换写入模式",
            "書き込みモード切替",
            "Toggle Write Mode",
        )
    }

    pub fn shortcut_close_panel(lang: Language) -> &'static str {
        pick(lang, "关闭面板", "パネルを閉じる", "Close Panel")
    }

    pub fn shortcut_pin_panel(lang: Language) -> &'static str {
        pick(lang, "固定面板", "パネルを固定", "Pin Panel")
    }

    pub fn shortcut_switch_section(lang: Language) -> &'static str {
        pick(
            lang,
            "切换模块/线程",
            "セクション/スレッド切替",
            "Switch Section/Thread",
        )
    }

    // --- Shortcut Categories

    pub fn shortcut_category_global(lang: Language) -> &'static str {
        pick(lang, "全局", "グローバル", "Global")
    }

    pub fn shortcut_category_panel(lang: Language) -> &'static str {
        pick(lang, "面板内", "パネル内", "Panel")
    }

    // --- Shortcut Recorder

    pub fn recording(lang: Language) -> &'static str {
        pick(lang, "按键录制中…", "キー入力中…", "Recording...")
    }

    pub fn need_modifier_key(lang: Language) -> &'static str {
        pick(
            lang,
            "需要至少一个修饰键（⌘/⇧/⌥/⌃）",
            "修飾キーが1つ以上必要です（⌘/⇧/⌥/⌃）",
            "At least one modifier key required (⌘/⇧/⌥/⌃)",
        )
    }

    pub fn esc_reserved(lang: Language) -> &'static str {
        pick(
            lang,
            "Esc 已用于关闭面板",
            "Esc はパネルを閉じるために予約されています",
            "Esc is reserved for closing the panel",
        )
    }

    pub fn cmd_number_reserved(lang: Language) -> &'static str {
        pick(
            lang,
            "⌘1–9 已用于切换模块",
            "⌘1–9 はセクション切替に予約されています",
            "⌘1–9 is reserved for switching sections",
        )
    }

    /// Interpolates `name` into the "conflicts with <shortcut>" template.
    /// Mirrors Swift's `shortcutConflict(with:)`.
    ///
    /// Uses Rust's `format!` variable capture (not string replacement), so
    /// `name` values containing a literal `{name}` are passed through
    /// verbatim without re-expansion.
    pub fn shortcut_conflict(lang: Language, name: &str) -> String {
        match lang {
            Language::Zh => format!("与「{name}」冲突"),
            Language::Ja => format!("「{name}」と競合しています"),
            Language::En | Language::SystemDefault => format!("Conflicts with \"{name}\""),
        }
    }

    // --- Write Mode

    pub fn write_mode_daily_title(lang: Language) -> &'static str {
        pick(lang, "日记", "デイリー", "Daily")
    }

    pub fn write_mode_document_title(lang: Language) -> &'static str {
        pick(lang, "文档", "ドキュメント", "Document")
    }

    pub fn write_mode_daily_destination(lang: Language) -> &'static str {
        pick(
            lang,
            "追加到当天日记",
            "今日のデイリーに追加",
            "Append to today's daily",
        )
    }

    pub fn write_mode_document_destination(lang: Language) -> &'static str {
        pick(
            lang,
            "创建独立文档",
            "独立ドキュメントを作成",
            "Create standalone document",
        )
    }

    pub fn write_mode_daily_summary(lang: Language) -> &'static str {
        pick(
            lang,
            "追加到当天的日记文件，适合快速收集和后续整理。",
            "今日のデイリーファイルに追加。素早いメモや後での整理に最適。",
            "Append to today's daily file. Great for quick capture and later review.",
        )
    }

    pub fn write_mode_document_summary(lang: Language) -> &'static str {
        pick(
            lang,
            "每次新建一篇独立 Markdown 文档，适合沉淀为正式稿件。",
            "毎回独立した Markdown ドキュメントを作成。清書に最適。",
            "Create a standalone Markdown document each time. Good for polished writing.",
        )
    }

    pub fn write_mode_daily_target(lang: Language) -> &'static str {
        pick(
            lang,
            "按模块追加到当天日记，底部保留自定义模块切换。",
            "セクションごとに今日のデイリーに追加。下部でセクション切替可能。",
            "Append by section to today's daily note, with section switching at bottom.",
        )
    }

    pub fn write_mode_document_target(lang: Language) -> &'static str {
        pick(
            lang,
            "新建独立文件，可选标题，保存到指定目录。",
            "独立ファイルを新規作成。タイトルは任意、指定フォルダに保存。",
            "Create a standalone file with optional title, saved to the specified folder.",
        )
    }

    // --- Thread Mode

    pub fn write_mode_daily_compact(lang: Language) -> &'static str {
        pick(lang, "Daily", "Daily", "Daily")
    }

    pub fn write_mode_document_compact(lang: Language) -> &'static str {
        pick(lang, "文档", "ドキュメント", "Doc")
    }

    pub fn write_mode_thread_title(lang: Language) -> &'static str {
        pick(lang, "线程", "スレッド", "Thread")
    }

    pub fn write_mode_thread_compact(lang: Language) -> &'static str {
        pick(lang, "线程", "スレッド", "Thread")
    }

    pub fn write_mode_thread_destination(lang: Language) -> &'static str {
        pick(lang, "追加到线程", "スレッドに追加", "Append to thread")
    }

    pub fn write_mode_thread_summary(lang: Language) -> &'static str {
        pick(
            lang,
            "按主题追加到对应线程文件，适合连续追踪同一话题。",
            "テーマごとに対応するスレッドファイルに追加。同じトピックの継続的なトラッキングに最適。",
            "Append to corresponding thread file by topic. Great for continuous tracking of the same topic.",
        )
    }

    pub fn write_mode_thread_target(lang: Language) -> &'static str {
        pick(
            lang,
            "按主题追加到对应线程文件，方便连续追踪。",
            "テーマごとに対応するスレッドファイルに追加。継続的なトラッキングに最適。",
            "Append to corresponding thread file by topic for continuous tracking.",
        )
    }

    pub fn thread_placeholder(lang: Language) -> &'static str {
        pick(
            lang,
            "输入想法，追加到选中线程...",
            "アイデアを入力してスレッドに追加...",
            "Type your thought to append to thread...",
        )
    }

    pub fn no_thread_selected(lang: Language) -> &'static str {
        pick(
            lang,
            "请选择一个线程",
            "スレッドを選択してください",
            "Please select a thread",
        )
    }

    pub fn vault_hint_thread(lang: Language) -> &'static str {
        pick(
            lang,
            "线程文件将保存在此目录",
            "スレッドファイルはここに保存されます",
            "Thread files will be saved here",
        )
    }

    pub fn thread_management(lang: Language) -> &'static str {
        pick(lang, "线程管理", "スレッド管理", "Thread Management")
    }

    pub fn new_thread_default_name(lang: Language) -> &'static str {
        pick(lang, "新线程", "新しいスレッド", "New Thread")
    }

    pub fn add_thread(lang: Language) -> &'static str {
        pick(lang, "添加线程", "スレッドを追加", "Add Thread")
    }

    pub fn delete_thread(lang: Language) -> &'static str {
        pick(lang, "删除线程", "スレッドを削除", "Delete Thread")
    }

    pub fn thread_name(lang: Language) -> &'static str {
        pick(lang, "名称", "名前", "Name")
    }

    pub fn thread_target_file(lang: Language) -> &'static str {
        pick(lang, "目标文件路径", "対象ファイルパス", "Target file path")
    }

    pub fn folder_path(lang: Language) -> &'static str {
        pick(lang, "文件夹", "フォルダ", "Folder")
    }

    pub fn file_name(lang: Language) -> &'static str {
        pick(lang, "文件名", "ファイル名", "Filename")
    }

    pub fn root_folder(lang: Language) -> &'static str {
        pick(lang, "根目录", "ルート", "Root")
    }

    // --- Entry Theme Presets

    pub fn entry_code_block(lang: Language) -> &'static str {
        pick(
            lang,
            "代码块（推荐）",
            "コードブロック（推奨）",
            "Code Block (Recommended)",
        )
    }

    pub fn entry_plain_text(lang: Language) -> &'static str {
        pick(
            lang,
            "文本 + 时间戳",
            "テキスト＋タイムスタンプ",
            "Text + Timestamp",
        )
    }

    pub fn entry_quote(lang: Language) -> &'static str {
        pick(
            lang,
            "引用（Markdown）",
            "引用（Markdown）",
            "Quote (Markdown)",
        )
    }

    // --- Separator Styles

    pub fn separator_none(lang: Language) -> &'static str {
        pick(lang, "仅空行", "空行のみ", "Empty lines only")
    }

    pub fn separator_horizontal_rule(lang: Language) -> &'static str {
        pick(lang, "--- 分割线", "--- 区切り線", "--- divider")
    }

    pub fn separator_asterisk_rule(lang: Language) -> &'static str {
        pick(lang, "*** 分割线", "*** 区切り線", "*** divider")
    }

    // --- Vault Validation

    pub fn vault_empty(lang: Language) -> &'static str {
        pick(
            lang,
            "请先选择笔记库文件夹。",
            "ノート保管庫のフォルダを選択してください。",
            "Please select a vault folder first.",
        )
    }

    pub fn vault_not_exist(lang: Language) -> &'static str {
        pick(
            lang,
            "笔记库路径不存在，请重新选择。",
            "ノート保管庫のパスが存在しません。再選択してください。",
            "Vault path does not exist. Please select again.",
        )
    }

    pub fn vault_not_directory(lang: Language) -> &'static str {
        pick(
            lang,
            "笔记库路径必须是文件夹。",
            "ノート保管庫のパスはフォルダである必要があります。",
            "Vault path must be a directory.",
        )
    }

    pub fn vault_not_writable(lang: Language) -> &'static str {
        pick(
            lang,
            "笔记库路径不可写，请检查文件夹权限。",
            "ノート保管庫のパスに書き込めません。フォルダの権限を確認してください。",
            "Vault path is not writable. Check folder permissions.",
        )
    }

    // --- Capture Panel

    pub fn note_placeholder(lang: Language) -> &'static str {
        pick(
            lang,
            "输入笔记内容...",
            "ノートを入力...",
            "Type your note...",
        )
    }

    pub fn document_placeholder(lang: Language) -> &'static str {
        pick(
            lang,
            "输入文档内容...",
            "ドキュメントを入力...",
            "Type your document...",
        )
    }

    pub fn document_title_placeholder(lang: Language) -> &'static str {
        pick(lang, "标题（可选）", "タイトル（任意）", "Title (optional)")
    }

    pub fn pin_panel_help(lang: Language) -> &'static str {
        pick(
            lang,
            "固定面板，保存后不关闭 (⌘P)",
            "パネルを固定、保存後も閉じない (⌘P)",
            "Pin panel, stay open after save (⌘P)",
        )
    }

    pub fn settings_tooltip(lang: Language) -> &'static str {
        pick(lang, "设置", "設定", "Settings")
    }

    /// Title of the settings window itself (as opposed to the gear button's
    /// tooltip in the capture panel header). Mac renders the same literal in
    /// both spots, so the Windows port shares the translation table.
    pub fn settings(lang: Language) -> &'static str {
        pick(lang, "设置", "設定", "Settings")
    }

    // --- Toast & Alerts

    pub fn empty_not_saved(lang: Language) -> &'static str {
        pick(
            lang,
            "内容为空，未保存",
            "内容が空のため保存されませんでした",
            "Empty content, not saved",
        )
    }

    pub fn save_failed(lang: Language) -> &'static str {
        pick(lang, "保存失败", "保存に失敗しました", "Save Failed")
    }

    // --- Global Hotkey Alert

    pub fn hotkey_registration_failed(lang: Language) -> &'static str {
        pick(
            lang,
            "无法注册全局快捷键",
            "グローバルショートカットの登録に失敗しました",
            "Cannot register global hotkey",
        )
    }

    pub fn hotkey_conflict_message(lang: Language) -> &'static str {
        pick(
            lang,
            "当前快捷键可能与其他应用冲突，请前往设置修改快捷键。",
            "現在のショートカットが他のアプリと競合している可能性があります。設定で変更してください。",
            "The current shortcut may conflict with other apps. Go to Settings to change it.",
        )
    }

    pub fn open_settings(lang: Language) -> &'static str {
        pick(lang, "打开设置", "設定を開く", "Open Settings")
    }

    pub fn later(lang: Language) -> &'static str {
        pick(lang, "稍后", "後で", "Later")
    }

    // --- System Tray Menu

    /// "New Note" tray menu entry. Mirrors the first item of the macOS
    /// `setupStatusItem` menu in `AppDelegate.swift`, but unlike the Mac
    /// reference (hard-coded English) the Windows port localizes the label.
    pub fn new_note(lang: Language) -> &'static str {
        pick(lang, "新建笔记", "新規ノート", "New Note")
    }

    /// "Quit <AppName>" tray menu entry. Interpolates the application display
    /// name exactly like the Swift reference `"Quit \(BrandAssets.displayName)"`.
    ///
    /// Returns `String` (not `&'static str`) because each call synthesizes a
    /// fresh buffer containing the runtime-provided name.
    pub fn quit(lang: Language, app_name: &str) -> String {
        match lang {
            Language::Zh => format!("退出 {app_name}"),
            Language::Ja => format!("{app_name}を終了"),
            Language::En | Language::SystemDefault => format!("Quit {app_name}"),
        }
    }

    // --- Writer Errors

    pub fn vault_not_configured(lang: Language) -> &'static str {
        pick(
            lang,
            "笔记库路径未配置，请点击右上角 ⚙ 进入设置。",
            "ノート保管庫のパスが未設定です。右上の ⚙ をクリックして設定してください。",
            "Vault path not configured. Click the ⚙ icon to open Settings.",
        )
    }

    pub fn invalid_target_folder(lang: Language) -> &'static str {
        pick(
            lang,
            "目标目录必须是笔记库内的相对路径，且不能包含 ..",
            "対象ディレクトリは保管庫内の相対パスで、.. を含むことはできません",
            "Target must be a relative path within the vault and cannot contain ..",
        )
    }

    pub fn image_vault_not_configured(lang: Language) -> &'static str {
        pick(
            lang,
            "笔记库路径未配置或不可写，无法保存图片。",
            "ノート保管庫のパスが未設定または書き込み不可のため、画像を保存できません。",
            "Vault path not configured or not writable. Cannot save image.",
        )
    }

    pub fn image_encoding_failed(lang: Language) -> &'static str {
        pick(
            lang,
            "图片编码失败，无法写入 PNG 文件。",
            "画像のエンコードに失敗しました。PNG ファイルを書き込めません。",
            "Image encoding failed. Cannot write PNG file.",
        )
    }

    // --- Theme Descriptions

    pub fn theme_light_summary(lang: Language) -> &'static str {
        pick(
            lang,
            "参考 Obsidian Light 的灰白底色和紫色强调，更干净、更通用。",
            "Obsidian Light にインスパイアされたグレースケールとパープルアクセント。",
            "Clean grayscale with purple accent, inspired by Obsidian Light.",
        )
    }

    pub fn theme_dark_summary(lang: Language) -> &'static str {
        pick(
            lang,
            "黑白灰夜间主题，保留足够对比度，避免紫色品牌偏移。",
            "十分なコントラストを保つダークテーマ。",
            "Dark theme with strong contrast, no purple brand shift.",
        )
    }

    pub fn theme_paper_summary(lang: Language) -> &'static str {
        pick(
            lang,
            "纸张米白与墨黑正文，适合长时间阅读、整理和静态编辑。",
            "紙のアイボリーとインクブラックの本文。長時間の読書と編集に最適。",
            "Paper white with ink-black text, ideal for long reading and editing.",
        )
    }

    pub fn theme_dune_summary(lang: Language) -> &'static str {
        pick(
            lang,
            "燕麦底与陶土橙强调，整体更暖、更柔和，也更有材料感。",
            "オートミールの背景と陶土オレンジのアクセント。温かみのある質感。",
            "Oat background with clay-orange accent, warm and tactile.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Settings Sections

    #[test]
    fn language_section() {
        assert_eq!(L10n::language(Language::Zh), "语言");
        assert_eq!(L10n::language(Language::Ja), "言語");
        assert_eq!(L10n::language(Language::En), "Language");
    }

    #[test]
    fn l10n_language_system_default_covers_three_langs() {
        assert_eq!(L10n::language_system_default(Language::Zh), "系统默认");
        assert_eq!(
            L10n::language_system_default(Language::Ja),
            "システム既定"
        );
        assert_eq!(L10n::language_system_default(Language::En), "System default");
        // SystemDefault sentinel falls back to English, matching the rest of
        // the L10n table (see `pick` in this file).
        assert_eq!(
            L10n::language_system_default(Language::SystemDefault),
            "System default"
        );
    }

    #[test]
    fn theme_section() {
        assert_eq!(L10n::theme(Language::Zh), "主题");
        assert_eq!(L10n::theme(Language::Ja), "テーマ");
        assert_eq!(L10n::theme(Language::En), "Theme");
    }

    #[test]
    fn storage_section() {
        assert_eq!(L10n::storage(Language::Zh), "保存位置");
        assert_eq!(L10n::storage(Language::Ja), "保存先");
        assert_eq!(L10n::storage(Language::En), "Storage");
    }

    #[test]
    fn quick_sections_section() {
        assert_eq!(L10n::quick_sections(Language::Zh), "快捷分类");
        assert_eq!(L10n::quick_sections(Language::Ja), "クイックセクション");
        assert_eq!(L10n::quick_sections(Language::En), "Quick Sections");
    }

    #[test]
    fn shortcuts_section() {
        assert_eq!(L10n::shortcuts(Language::Zh), "快捷键");
        assert_eq!(L10n::shortcuts(Language::Ja), "ショートカット");
        assert_eq!(L10n::shortcuts(Language::En), "Shortcuts");
    }

    #[test]
    fn system_section() {
        assert_eq!(L10n::system(Language::Zh), "系统");
        assert_eq!(L10n::system(Language::Ja), "システム");
        assert_eq!(L10n::system(Language::En), "System");
    }

    // --- Settings Labels

    #[test]
    fn write_mode_label() {
        assert_eq!(L10n::write_mode(Language::Zh), "写入模式");
        assert_eq!(L10n::write_mode(Language::Ja), "書き込みモード");
        assert_eq!(L10n::write_mode(Language::En), "Write Mode");
    }

    #[test]
    fn vault_label() {
        assert_eq!(L10n::vault(Language::Zh), "笔记库");
        assert_eq!(L10n::vault(Language::Ja), "ノート保管庫");
        assert_eq!(L10n::vault(Language::En), "Vault");
    }

    #[test]
    fn vault_hint_dimension_label() {
        assert_eq!(
            L10n::vault_hint_dimension(Language::Zh),
            "Obsidian Vault 根目录，或其他笔记库的根路径"
        );
        assert_eq!(
            L10n::vault_hint_dimension(Language::Ja),
            "Obsidian Vault のルートディレクトリ、または他のノート保管庫のルートパス"
        );
        assert_eq!(
            L10n::vault_hint_dimension(Language::En),
            "Root directory of your Obsidian Vault or note library"
        );
    }

    #[test]
    fn vault_hint_file_label() {
        assert_eq!(L10n::vault_hint_file(Language::Zh), "文档保存的文件夹路径");
        assert_eq!(
            L10n::vault_hint_file(Language::Ja),
            "ドキュメント保存先のフォルダパス"
        );
        assert_eq!(
            L10n::vault_hint_file(Language::En),
            "Folder path for document storage"
        );
    }

    #[test]
    fn daily_folder_label() {
        assert_eq!(L10n::daily_folder(Language::Zh), "日记文件夹");
        assert_eq!(L10n::daily_folder(Language::Ja), "デイリーフォルダ");
        assert_eq!(L10n::daily_folder(Language::En), "Daily Folder");
    }

    #[test]
    fn daily_folder_hint_label() {
        assert_eq!(
            L10n::daily_folder_hint(Language::Zh),
            "笔记库内存放日记的子文件夹名称，建议与 Obsidian 日记设置一致"
        );
        assert_eq!(
            L10n::daily_folder_hint(Language::Ja),
            "デイリーノート用のサブフォルダ名。Obsidian のデイリーノート設定と合わせてください"
        );
        assert_eq!(
            L10n::daily_folder_hint(Language::En),
            "Subfolder name for daily notes, should match your Obsidian daily notes settings"
        );
    }

    #[test]
    fn file_name_format_label() {
        assert_eq!(L10n::file_name_format(Language::Zh), "文件名格式");
        assert_eq!(L10n::file_name_format(Language::Ja), "ファイル名の形式");
        assert_eq!(L10n::file_name_format(Language::En), "File Name Format");
    }

    #[test]
    fn entry_format_label() {
        assert_eq!(L10n::entry_format(Language::Zh), "条目格式");
        assert_eq!(L10n::entry_format(Language::Ja), "エントリー形式");
        assert_eq!(L10n::entry_format(Language::En), "Entry Format");
    }

    #[test]
    fn section_name_label() {
        assert_eq!(L10n::section_name(Language::Zh), "模块名");
        assert_eq!(L10n::section_name(Language::Ja), "セクション名");
        assert_eq!(L10n::section_name(Language::En), "Section Name");
    }

    #[test]
    fn launch_at_login_label() {
        assert_eq!(L10n::launch_at_login(Language::Zh), "开机自启动");
        assert_eq!(L10n::launch_at_login(Language::Ja), "ログイン時に起動");
        assert_eq!(L10n::launch_at_login(Language::En), "Launch at Login");
    }

    // --- Settings Buttons

    #[test]
    fn browse_button() {
        assert_eq!(L10n::browse(Language::Zh), "选择");
        assert_eq!(L10n::browse(Language::Ja), "選択");
        assert_eq!(L10n::browse(Language::En), "Browse");
    }

    #[test]
    fn choose_folder_button() {
        assert_eq!(L10n::choose_folder(Language::Zh), "指定位置");
        assert_eq!(L10n::choose_folder(Language::Ja), "場所を指定");
        assert_eq!(L10n::choose_folder(Language::En), "Set Location");
    }

    #[test]
    fn add_section_button() {
        assert_eq!(L10n::add_section(Language::Zh), "新增模块");
        assert_eq!(L10n::add_section(Language::Ja), "セクションを追加");
        assert_eq!(L10n::add_section(Language::En), "Add Section");
    }

    #[test]
    fn save_button() {
        assert_eq!(L10n::save(Language::Zh), "保存");
        assert_eq!(L10n::save(Language::Ja), "保存");
        assert_eq!(L10n::save(Language::En), "Save");
    }

    #[test]
    fn delete_section_button() {
        assert_eq!(L10n::delete_section(Language::Zh), "删除模块");
        assert_eq!(L10n::delete_section(Language::Ja), "セクションを削除");
        assert_eq!(L10n::delete_section(Language::En), "Delete Section");
    }

    #[test]
    fn edit_button() {
        assert_eq!(L10n::edit(Language::Zh), "修改");
        assert_eq!(L10n::edit(Language::Ja), "変更");
        assert_eq!(L10n::edit(Language::En), "Edit");
    }

    #[test]
    fn cancel_button() {
        assert_eq!(L10n::cancel(Language::Zh), "取消");
        assert_eq!(L10n::cancel(Language::Ja), "キャンセル");
        assert_eq!(L10n::cancel(Language::En), "Cancel");
    }

    // --- Shortcut Names

    #[test]
    fn shortcut_create_name() {
        assert_eq!(L10n::shortcut_create(Language::Zh), "创建笔记");
        assert_eq!(L10n::shortcut_create(Language::Ja), "ノートを作成");
        assert_eq!(L10n::shortcut_create(Language::En), "Create Note");
    }

    #[test]
    fn shortcut_send_name() {
        assert_eq!(L10n::shortcut_send(Language::Zh), "发送笔记");
        assert_eq!(L10n::shortcut_send(Language::Ja), "ノートを送信");
        assert_eq!(L10n::shortcut_send(Language::En), "Send Note");
    }

    #[test]
    fn shortcut_append_name() {
        assert_eq!(L10n::shortcut_append(Language::Zh), "追加上一条");
        assert_eq!(L10n::shortcut_append(Language::Ja), "前回に追加");
        assert_eq!(L10n::shortcut_append(Language::En), "Append to Last");
    }

    #[test]
    fn shortcut_toggle_mode_name() {
        assert_eq!(L10n::shortcut_toggle_mode(Language::Zh), "切换写入模式");
        assert_eq!(
            L10n::shortcut_toggle_mode(Language::Ja),
            "書き込みモード切替"
        );
        assert_eq!(
            L10n::shortcut_toggle_mode(Language::En),
            "Toggle Write Mode"
        );
    }

    #[test]
    fn shortcut_close_panel_name() {
        assert_eq!(L10n::shortcut_close_panel(Language::Zh), "关闭面板");
        assert_eq!(L10n::shortcut_close_panel(Language::Ja), "パネルを閉じる");
        assert_eq!(L10n::shortcut_close_panel(Language::En), "Close Panel");
    }

    #[test]
    fn shortcut_pin_panel_name() {
        assert_eq!(L10n::shortcut_pin_panel(Language::Zh), "固定面板");
        assert_eq!(L10n::shortcut_pin_panel(Language::Ja), "パネルを固定");
        assert_eq!(L10n::shortcut_pin_panel(Language::En), "Pin Panel");
    }

    #[test]
    fn shortcut_switch_section_name() {
        assert_eq!(L10n::shortcut_switch_section(Language::Zh), "切换模块/线程");
        assert_eq!(
            L10n::shortcut_switch_section(Language::Ja),
            "セクション/スレッド切替"
        );
        assert_eq!(
            L10n::shortcut_switch_section(Language::En),
            "Switch Section/Thread"
        );
    }

    // --- Shortcut Categories

    #[test]
    fn shortcut_category_global_label() {
        assert_eq!(L10n::shortcut_category_global(Language::Zh), "全局");
        assert_eq!(L10n::shortcut_category_global(Language::Ja), "グローバル");
        assert_eq!(L10n::shortcut_category_global(Language::En), "Global");
    }

    #[test]
    fn shortcut_category_panel_label() {
        assert_eq!(L10n::shortcut_category_panel(Language::Zh), "面板内");
        assert_eq!(L10n::shortcut_category_panel(Language::Ja), "パネル内");
        assert_eq!(L10n::shortcut_category_panel(Language::En), "Panel");
    }

    // --- Shortcut Recorder

    #[test]
    fn recording_message() {
        assert_eq!(L10n::recording(Language::Zh), "按键录制中…");
        assert_eq!(L10n::recording(Language::Ja), "キー入力中…");
        assert_eq!(L10n::recording(Language::En), "Recording...");
    }

    #[test]
    fn need_modifier_key_message() {
        assert_eq!(
            L10n::need_modifier_key(Language::Zh),
            "需要至少一个修饰键（⌘/⇧/⌥/⌃）"
        );
        assert_eq!(
            L10n::need_modifier_key(Language::Ja),
            "修飾キーが1つ以上必要です（⌘/⇧/⌥/⌃）"
        );
        assert_eq!(
            L10n::need_modifier_key(Language::En),
            "At least one modifier key required (⌘/⇧/⌥/⌃)"
        );
    }

    #[test]
    fn esc_reserved_message() {
        assert_eq!(L10n::esc_reserved(Language::Zh), "Esc 已用于关闭面板");
        assert_eq!(
            L10n::esc_reserved(Language::Ja),
            "Esc はパネルを閉じるために予約されています"
        );
        assert_eq!(
            L10n::esc_reserved(Language::En),
            "Esc is reserved for closing the panel"
        );
    }

    #[test]
    fn cmd_number_reserved_message() {
        assert_eq!(
            L10n::cmd_number_reserved(Language::Zh),
            "⌘1–9 已用于切换模块"
        );
        assert_eq!(
            L10n::cmd_number_reserved(Language::Ja),
            "⌘1–9 はセクション切替に予約されています"
        );
        assert_eq!(
            L10n::cmd_number_reserved(Language::En),
            "⌘1–9 is reserved for switching sections"
        );
    }

    // --- Write Mode

    #[test]
    fn write_mode_daily_title_label() {
        assert_eq!(L10n::write_mode_daily_title(Language::Zh), "日记");
        assert_eq!(L10n::write_mode_daily_title(Language::Ja), "デイリー");
        assert_eq!(L10n::write_mode_daily_title(Language::En), "Daily");
    }

    #[test]
    fn write_mode_document_title_label() {
        assert_eq!(L10n::write_mode_document_title(Language::Zh), "文档");
        assert_eq!(
            L10n::write_mode_document_title(Language::Ja),
            "ドキュメント"
        );
        assert_eq!(L10n::write_mode_document_title(Language::En), "Document");
    }

    #[test]
    fn write_mode_daily_destination_label() {
        assert_eq!(
            L10n::write_mode_daily_destination(Language::Zh),
            "追加到当天日记"
        );
        assert_eq!(
            L10n::write_mode_daily_destination(Language::Ja),
            "今日のデイリーに追加"
        );
        assert_eq!(
            L10n::write_mode_daily_destination(Language::En),
            "Append to today's daily"
        );
    }

    #[test]
    fn write_mode_document_destination_label() {
        assert_eq!(
            L10n::write_mode_document_destination(Language::Zh),
            "创建独立文档"
        );
        assert_eq!(
            L10n::write_mode_document_destination(Language::Ja),
            "独立ドキュメントを作成"
        );
        assert_eq!(
            L10n::write_mode_document_destination(Language::En),
            "Create standalone document"
        );
    }

    #[test]
    fn write_mode_daily_summary_label() {
        assert_eq!(
            L10n::write_mode_daily_summary(Language::Zh),
            "追加到当天的日记文件，适合快速收集和后续整理。"
        );
        assert_eq!(
            L10n::write_mode_daily_summary(Language::Ja),
            "今日のデイリーファイルに追加。素早いメモや後での整理に最適。"
        );
        assert_eq!(
            L10n::write_mode_daily_summary(Language::En),
            "Append to today's daily file. Great for quick capture and later review."
        );
    }

    #[test]
    fn write_mode_document_summary_label() {
        assert_eq!(
            L10n::write_mode_document_summary(Language::Zh),
            "每次新建一篇独立 Markdown 文档，适合沉淀为正式稿件。"
        );
        assert_eq!(
            L10n::write_mode_document_summary(Language::Ja),
            "毎回独立した Markdown ドキュメントを作成。清書に最適。"
        );
        assert_eq!(
            L10n::write_mode_document_summary(Language::En),
            "Create a standalone Markdown document each time. Good for polished writing."
        );
    }

    #[test]
    fn write_mode_daily_target_label() {
        assert_eq!(
            L10n::write_mode_daily_target(Language::Zh),
            "按模块追加到当天日记，底部保留自定义模块切换。"
        );
        assert_eq!(
            L10n::write_mode_daily_target(Language::Ja),
            "セクションごとに今日のデイリーに追加。下部でセクション切替可能。"
        );
        assert_eq!(
            L10n::write_mode_daily_target(Language::En),
            "Append by section to today's daily note, with section switching at bottom."
        );
    }

    #[test]
    fn write_mode_document_target_label() {
        assert_eq!(
            L10n::write_mode_document_target(Language::Zh),
            "新建独立文件，可选标题，保存到指定目录。"
        );
        assert_eq!(
            L10n::write_mode_document_target(Language::Ja),
            "独立ファイルを新規作成。タイトルは任意、指定フォルダに保存。"
        );
        assert_eq!(
            L10n::write_mode_document_target(Language::En),
            "Create a standalone file with optional title, saved to the specified folder."
        );
    }

    // --- Thread Mode

    #[test]
    fn write_mode_daily_compact_label() {
        assert_eq!(L10n::write_mode_daily_compact(Language::Zh), "Daily");
        assert_eq!(L10n::write_mode_daily_compact(Language::Ja), "Daily");
        assert_eq!(L10n::write_mode_daily_compact(Language::En), "Daily");
    }

    #[test]
    fn write_mode_document_compact_label() {
        assert_eq!(L10n::write_mode_document_compact(Language::Zh), "文档");
        assert_eq!(
            L10n::write_mode_document_compact(Language::Ja),
            "ドキュメント"
        );
        assert_eq!(L10n::write_mode_document_compact(Language::En), "Doc");
    }

    #[test]
    fn write_mode_thread_title_label() {
        assert_eq!(L10n::write_mode_thread_title(Language::Zh), "线程");
        assert_eq!(L10n::write_mode_thread_title(Language::Ja), "スレッド");
        assert_eq!(L10n::write_mode_thread_title(Language::En), "Thread");
    }

    #[test]
    fn write_mode_thread_compact_label() {
        assert_eq!(L10n::write_mode_thread_compact(Language::Zh), "线程");
        assert_eq!(L10n::write_mode_thread_compact(Language::Ja), "スレッド");
        assert_eq!(L10n::write_mode_thread_compact(Language::En), "Thread");
    }

    #[test]
    fn write_mode_thread_destination_label() {
        assert_eq!(
            L10n::write_mode_thread_destination(Language::Zh),
            "追加到线程"
        );
        assert_eq!(
            L10n::write_mode_thread_destination(Language::Ja),
            "スレッドに追加"
        );
        assert_eq!(
            L10n::write_mode_thread_destination(Language::En),
            "Append to thread"
        );
    }

    #[test]
    fn write_mode_thread_summary_label() {
        assert_eq!(
            L10n::write_mode_thread_summary(Language::Zh),
            "按主题追加到对应线程文件，适合连续追踪同一话题。"
        );
        assert_eq!(
            L10n::write_mode_thread_summary(Language::Ja),
            "テーマごとに対応するスレッドファイルに追加。同じトピックの継続的なトラッキングに最適。"
        );
        assert_eq!(
            L10n::write_mode_thread_summary(Language::En),
            "Append to corresponding thread file by topic. Great for continuous tracking of the same topic."
        );
    }

    #[test]
    fn write_mode_thread_target_label() {
        assert_eq!(
            L10n::write_mode_thread_target(Language::Zh),
            "按主题追加到对应线程文件，方便连续追踪。"
        );
        assert_eq!(
            L10n::write_mode_thread_target(Language::Ja),
            "テーマごとに対応するスレッドファイルに追加。継続的なトラッキングに最適。"
        );
        assert_eq!(
            L10n::write_mode_thread_target(Language::En),
            "Append to corresponding thread file by topic for continuous tracking."
        );
    }

    #[test]
    fn thread_placeholder_label() {
        assert_eq!(
            L10n::thread_placeholder(Language::Zh),
            "输入想法，追加到选中线程..."
        );
        assert_eq!(
            L10n::thread_placeholder(Language::Ja),
            "アイデアを入力してスレッドに追加..."
        );
        assert_eq!(
            L10n::thread_placeholder(Language::En),
            "Type your thought to append to thread..."
        );
    }

    #[test]
    fn no_thread_selected_label() {
        assert_eq!(L10n::no_thread_selected(Language::Zh), "请选择一个线程");
        assert_eq!(
            L10n::no_thread_selected(Language::Ja),
            "スレッドを選択してください"
        );
        assert_eq!(
            L10n::no_thread_selected(Language::En),
            "Please select a thread"
        );
    }

    #[test]
    fn vault_hint_thread_label() {
        assert_eq!(
            L10n::vault_hint_thread(Language::Zh),
            "线程文件将保存在此目录"
        );
        assert_eq!(
            L10n::vault_hint_thread(Language::Ja),
            "スレッドファイルはここに保存されます"
        );
        assert_eq!(
            L10n::vault_hint_thread(Language::En),
            "Thread files will be saved here"
        );
    }

    #[test]
    fn thread_management_label() {
        assert_eq!(L10n::thread_management(Language::Zh), "线程管理");
        assert_eq!(L10n::thread_management(Language::Ja), "スレッド管理");
        assert_eq!(L10n::thread_management(Language::En), "Thread Management");
    }

    #[test]
    fn new_thread_default_name_label() {
        assert_eq!(L10n::new_thread_default_name(Language::Zh), "新线程");
        assert_eq!(
            L10n::new_thread_default_name(Language::Ja),
            "新しいスレッド"
        );
        assert_eq!(L10n::new_thread_default_name(Language::En), "New Thread");
    }

    #[test]
    fn add_thread_label() {
        assert_eq!(L10n::add_thread(Language::Zh), "添加线程");
        assert_eq!(L10n::add_thread(Language::Ja), "スレッドを追加");
        assert_eq!(L10n::add_thread(Language::En), "Add Thread");
    }

    #[test]
    fn delete_thread_label() {
        assert_eq!(L10n::delete_thread(Language::Zh), "删除线程");
        assert_eq!(L10n::delete_thread(Language::Ja), "スレッドを削除");
        assert_eq!(L10n::delete_thread(Language::En), "Delete Thread");
    }

    #[test]
    fn thread_name_label() {
        assert_eq!(L10n::thread_name(Language::Zh), "名称");
        assert_eq!(L10n::thread_name(Language::Ja), "名前");
        assert_eq!(L10n::thread_name(Language::En), "Name");
    }

    #[test]
    fn thread_target_file_label() {
        assert_eq!(L10n::thread_target_file(Language::Zh), "目标文件路径");
        assert_eq!(L10n::thread_target_file(Language::Ja), "対象ファイルパス");
        assert_eq!(L10n::thread_target_file(Language::En), "Target file path");
    }

    #[test]
    fn folder_path_label() {
        assert_eq!(L10n::folder_path(Language::Zh), "文件夹");
        assert_eq!(L10n::folder_path(Language::Ja), "フォルダ");
        assert_eq!(L10n::folder_path(Language::En), "Folder");
    }

    #[test]
    fn file_name_label() {
        assert_eq!(L10n::file_name(Language::Zh), "文件名");
        assert_eq!(L10n::file_name(Language::Ja), "ファイル名");
        assert_eq!(L10n::file_name(Language::En), "Filename");
    }

    #[test]
    fn root_folder_label() {
        assert_eq!(L10n::root_folder(Language::Zh), "根目录");
        assert_eq!(L10n::root_folder(Language::Ja), "ルート");
        assert_eq!(L10n::root_folder(Language::En), "Root");
    }

    // --- Entry Theme Presets

    #[test]
    fn entry_code_block_preset() {
        assert_eq!(L10n::entry_code_block(Language::Zh), "代码块（推荐）");
        assert_eq!(
            L10n::entry_code_block(Language::Ja),
            "コードブロック（推奨）"
        );
        assert_eq!(
            L10n::entry_code_block(Language::En),
            "Code Block (Recommended)"
        );
    }

    #[test]
    fn entry_plain_text_preset() {
        assert_eq!(L10n::entry_plain_text(Language::Zh), "文本 + 时间戳");
        assert_eq!(
            L10n::entry_plain_text(Language::Ja),
            "テキスト＋タイムスタンプ"
        );
        assert_eq!(L10n::entry_plain_text(Language::En), "Text + Timestamp");
    }

    #[test]
    fn entry_quote_preset() {
        assert_eq!(L10n::entry_quote(Language::Zh), "引用（Markdown）");
        assert_eq!(L10n::entry_quote(Language::Ja), "引用（Markdown）");
        assert_eq!(L10n::entry_quote(Language::En), "Quote (Markdown)");
    }

    // --- Separator Styles

    #[test]
    fn separator_none_label() {
        assert_eq!(L10n::separator_none(Language::Zh), "仅空行");
        assert_eq!(L10n::separator_none(Language::Ja), "空行のみ");
        assert_eq!(L10n::separator_none(Language::En), "Empty lines only");
    }

    #[test]
    fn separator_horizontal_rule_label() {
        assert_eq!(L10n::separator_horizontal_rule(Language::Zh), "--- 分割线");
        assert_eq!(
            L10n::separator_horizontal_rule(Language::Ja),
            "--- 区切り線"
        );
        assert_eq!(L10n::separator_horizontal_rule(Language::En), "--- divider");
    }

    #[test]
    fn separator_asterisk_rule_label() {
        assert_eq!(L10n::separator_asterisk_rule(Language::Zh), "*** 分割线");
        assert_eq!(L10n::separator_asterisk_rule(Language::Ja), "*** 区切り線");
        assert_eq!(L10n::separator_asterisk_rule(Language::En), "*** divider");
    }

    // --- Vault Validation

    #[test]
    fn vault_empty_message() {
        assert_eq!(L10n::vault_empty(Language::Zh), "请先选择笔记库文件夹。");
        assert_eq!(
            L10n::vault_empty(Language::Ja),
            "ノート保管庫のフォルダを選択してください。"
        );
        assert_eq!(
            L10n::vault_empty(Language::En),
            "Please select a vault folder first."
        );
    }

    #[test]
    fn vault_not_exist_message() {
        assert_eq!(
            L10n::vault_not_exist(Language::Zh),
            "笔记库路径不存在，请重新选择。"
        );
        assert_eq!(
            L10n::vault_not_exist(Language::Ja),
            "ノート保管庫のパスが存在しません。再選択してください。"
        );
        assert_eq!(
            L10n::vault_not_exist(Language::En),
            "Vault path does not exist. Please select again."
        );
    }

    #[test]
    fn vault_not_directory_message() {
        assert_eq!(
            L10n::vault_not_directory(Language::Zh),
            "笔记库路径必须是文件夹。"
        );
        assert_eq!(
            L10n::vault_not_directory(Language::Ja),
            "ノート保管庫のパスはフォルダである必要があります。"
        );
        assert_eq!(
            L10n::vault_not_directory(Language::En),
            "Vault path must be a directory."
        );
    }

    #[test]
    fn vault_not_writable_message() {
        assert_eq!(
            L10n::vault_not_writable(Language::Zh),
            "笔记库路径不可写，请检查文件夹权限。"
        );
        assert_eq!(
            L10n::vault_not_writable(Language::Ja),
            "ノート保管庫のパスに書き込めません。フォルダの権限を確認してください。"
        );
        assert_eq!(
            L10n::vault_not_writable(Language::En),
            "Vault path is not writable. Check folder permissions."
        );
    }

    // --- Capture Panel

    #[test]
    fn note_placeholder_label() {
        assert_eq!(L10n::note_placeholder(Language::Zh), "输入笔记内容...");
        assert_eq!(L10n::note_placeholder(Language::Ja), "ノートを入力...");
        assert_eq!(L10n::note_placeholder(Language::En), "Type your note...");
    }

    #[test]
    fn document_placeholder_label() {
        assert_eq!(L10n::document_placeholder(Language::Zh), "输入文档内容...");
        assert_eq!(
            L10n::document_placeholder(Language::Ja),
            "ドキュメントを入力..."
        );
        assert_eq!(
            L10n::document_placeholder(Language::En),
            "Type your document..."
        );
    }

    #[test]
    fn document_title_placeholder_label() {
        assert_eq!(
            L10n::document_title_placeholder(Language::Zh),
            "标题（可选）"
        );
        assert_eq!(
            L10n::document_title_placeholder(Language::Ja),
            "タイトル（任意）"
        );
        assert_eq!(
            L10n::document_title_placeholder(Language::En),
            "Title (optional)"
        );
    }

    #[test]
    fn pin_panel_help_label() {
        assert_eq!(
            L10n::pin_panel_help(Language::Zh),
            "固定面板，保存后不关闭 (⌘P)"
        );
        assert_eq!(
            L10n::pin_panel_help(Language::Ja),
            "パネルを固定、保存後も閉じない (⌘P)"
        );
        assert_eq!(
            L10n::pin_panel_help(Language::En),
            "Pin panel, stay open after save (⌘P)"
        );
    }

    #[test]
    fn settings_tooltip_label() {
        assert_eq!(L10n::settings_tooltip(Language::Zh), "设置");
        assert_eq!(L10n::settings_tooltip(Language::Ja), "設定");
        assert_eq!(L10n::settings_tooltip(Language::En), "Settings");
    }

    #[test]
    fn settings_window_title() {
        assert_eq!(L10n::settings(Language::Zh), "设置");
        assert_eq!(L10n::settings(Language::Ja), "設定");
        assert_eq!(L10n::settings(Language::En), "Settings");
    }

    // --- Toast & Alerts

    #[test]
    fn empty_not_saved_message() {
        assert_eq!(L10n::empty_not_saved(Language::Zh), "内容为空，未保存");
        assert_eq!(
            L10n::empty_not_saved(Language::Ja),
            "内容が空のため保存されませんでした"
        );
        assert_eq!(
            L10n::empty_not_saved(Language::En),
            "Empty content, not saved"
        );
    }

    #[test]
    fn save_failed_message() {
        assert_eq!(L10n::save_failed(Language::Zh), "保存失败");
        assert_eq!(L10n::save_failed(Language::Ja), "保存に失敗しました");
        assert_eq!(L10n::save_failed(Language::En), "Save Failed");
    }

    // --- Global Hotkey Alert

    #[test]
    fn hotkey_registration_failed_message() {
        assert_eq!(
            L10n::hotkey_registration_failed(Language::Zh),
            "无法注册全局快捷键"
        );
        assert_eq!(
            L10n::hotkey_registration_failed(Language::Ja),
            "グローバルショートカットの登録に失敗しました"
        );
        assert_eq!(
            L10n::hotkey_registration_failed(Language::En),
            "Cannot register global hotkey"
        );
    }

    #[test]
    fn hotkey_conflict_message_body() {
        assert_eq!(
            L10n::hotkey_conflict_message(Language::Zh),
            "当前快捷键可能与其他应用冲突，请前往设置修改快捷键。"
        );
        assert_eq!(
            L10n::hotkey_conflict_message(Language::Ja),
            "現在のショートカットが他のアプリと競合している可能性があります。設定で変更してください。"
        );
        assert_eq!(
            L10n::hotkey_conflict_message(Language::En),
            "The current shortcut may conflict with other apps. Go to Settings to change it."
        );
    }

    #[test]
    fn open_settings_button() {
        assert_eq!(L10n::open_settings(Language::Zh), "打开设置");
        assert_eq!(L10n::open_settings(Language::Ja), "設定を開く");
        assert_eq!(L10n::open_settings(Language::En), "Open Settings");
    }

    #[test]
    fn later_button() {
        assert_eq!(L10n::later(Language::Zh), "稍后");
        assert_eq!(L10n::later(Language::Ja), "後で");
        assert_eq!(L10n::later(Language::En), "Later");
    }

    // --- Writer Errors

    #[test]
    fn vault_not_configured_message() {
        assert_eq!(
            L10n::vault_not_configured(Language::Zh),
            "笔记库路径未配置，请点击右上角 ⚙ 进入设置。"
        );
        assert_eq!(
            L10n::vault_not_configured(Language::Ja),
            "ノート保管庫のパスが未設定です。右上の ⚙ をクリックして設定してください。"
        );
        assert_eq!(
            L10n::vault_not_configured(Language::En),
            "Vault path not configured. Click the ⚙ icon to open Settings."
        );
    }

    #[test]
    fn invalid_target_folder_message() {
        assert_eq!(
            L10n::invalid_target_folder(Language::Zh),
            "目标目录必须是笔记库内的相对路径，且不能包含 .."
        );
        assert_eq!(
            L10n::invalid_target_folder(Language::Ja),
            "対象ディレクトリは保管庫内の相対パスで、.. を含むことはできません"
        );
        assert_eq!(
            L10n::invalid_target_folder(Language::En),
            "Target must be a relative path within the vault and cannot contain .."
        );
    }

    #[test]
    fn image_vault_not_configured_message() {
        assert_eq!(
            L10n::image_vault_not_configured(Language::Zh),
            "笔记库路径未配置或不可写，无法保存图片。"
        );
        assert_eq!(
            L10n::image_vault_not_configured(Language::Ja),
            "ノート保管庫のパスが未設定または書き込み不可のため、画像を保存できません。"
        );
        assert_eq!(
            L10n::image_vault_not_configured(Language::En),
            "Vault path not configured or not writable. Cannot save image."
        );
    }

    #[test]
    fn image_encoding_failed_message() {
        assert_eq!(
            L10n::image_encoding_failed(Language::Zh),
            "图片编码失败，无法写入 PNG 文件。"
        );
        assert_eq!(
            L10n::image_encoding_failed(Language::Ja),
            "画像のエンコードに失敗しました。PNG ファイルを書き込めません。"
        );
        assert_eq!(
            L10n::image_encoding_failed(Language::En),
            "Image encoding failed. Cannot write PNG file."
        );
    }

    // --- Theme Descriptions

    #[test]
    fn theme_light_summary_description() {
        assert_eq!(
            L10n::theme_light_summary(Language::Zh),
            "参考 Obsidian Light 的灰白底色和紫色强调，更干净、更通用。"
        );
        assert_eq!(
            L10n::theme_light_summary(Language::Ja),
            "Obsidian Light にインスパイアされたグレースケールとパープルアクセント。"
        );
        assert_eq!(
            L10n::theme_light_summary(Language::En),
            "Clean grayscale with purple accent, inspired by Obsidian Light."
        );
    }

    #[test]
    fn theme_dark_summary_description() {
        assert_eq!(
            L10n::theme_dark_summary(Language::Zh),
            "黑白灰夜间主题，保留足够对比度，避免紫色品牌偏移。"
        );
        assert_eq!(
            L10n::theme_dark_summary(Language::Ja),
            "十分なコントラストを保つダークテーマ。"
        );
        assert_eq!(
            L10n::theme_dark_summary(Language::En),
            "Dark theme with strong contrast, no purple brand shift."
        );
    }

    #[test]
    fn theme_paper_summary_description() {
        assert_eq!(
            L10n::theme_paper_summary(Language::Zh),
            "纸张米白与墨黑正文，适合长时间阅读、整理和静态编辑。"
        );
        assert_eq!(
            L10n::theme_paper_summary(Language::Ja),
            "紙のアイボリーとインクブラックの本文。長時間の読書と編集に最適。"
        );
        assert_eq!(
            L10n::theme_paper_summary(Language::En),
            "Paper white with ink-black text, ideal for long reading and editing."
        );
    }

    #[test]
    fn theme_dune_summary_description() {
        assert_eq!(
            L10n::theme_dune_summary(Language::Zh),
            "燕麦底与陶土橙强调，整体更暖、更柔和，也更有材料感。"
        );
        assert_eq!(
            L10n::theme_dune_summary(Language::Ja),
            "オートミールの背景と陶土オレンジのアクセント。温かみのある質感。"
        );
        assert_eq!(
            L10n::theme_dune_summary(Language::En),
            "Oat background with clay-orange accent, warm and tactile."
        );
    }

    // --- SystemDefault fallback

    #[test]
    fn system_default_falls_back_to_english() {
        // Spot-check across a variety of keys that SystemDefault returns the
        // English variant until the platform layer resolves it.
        assert_eq!(
            L10n::language(Language::SystemDefault),
            L10n::language(Language::En)
        );
        assert_eq!(
            L10n::theme(Language::SystemDefault),
            L10n::theme(Language::En)
        );
        assert_eq!(
            L10n::vault_hint_dimension(Language::SystemDefault),
            L10n::vault_hint_dimension(Language::En)
        );
        assert_eq!(
            L10n::write_mode_thread_summary(Language::SystemDefault),
            L10n::write_mode_thread_summary(Language::En)
        );
        assert_eq!(
            L10n::theme_dune_summary(Language::SystemDefault),
            L10n::theme_dune_summary(Language::En)
        );
        assert_eq!(
            L10n::shortcut_conflict(Language::SystemDefault, "Send Note"),
            L10n::shortcut_conflict(Language::En, "Send Note")
        );
    }

    // --- shortcut_conflict (parameterized)

    #[test]
    fn shortcut_conflict_interpolates_name() {
        assert_eq!(
            L10n::shortcut_conflict(Language::Zh, "发送笔记"),
            "与「发送笔记」冲突"
        );
        assert_eq!(
            L10n::shortcut_conflict(Language::Ja, "ノートを送信"),
            "「ノートを送信」と競合しています"
        );
        assert_eq!(
            L10n::shortcut_conflict(Language::En, "Send Note"),
            "Conflicts with \"Send Note\""
        );
    }

    #[test]
    fn shortcut_conflict_preserves_bracket_glyphs() {
        let zh = L10n::shortcut_conflict(Language::Zh, "创建笔记");
        assert!(zh.contains('「'), "zh variant must contain 「: {zh}");
        assert!(zh.contains('」'), "zh variant must contain 」: {zh}");

        let ja = L10n::shortcut_conflict(Language::Ja, "ノートを作成");
        assert!(ja.contains('「'), "ja variant must contain 「: {ja}");
        assert!(ja.contains('」'), "ja variant must contain 」: {ja}");

        let en = L10n::shortcut_conflict(Language::En, "Create Note");
        assert!(
            en.contains('"'),
            "en variant must wrap the name in ASCII double-quotes: {en}"
        );
        assert_eq!(en.matches('"').count(), 2, "exactly two quotes expected");
    }

    // --- System Tray Menu

    #[test]
    fn new_note_menu_label() {
        assert_eq!(L10n::new_note(Language::Zh), "新建笔记");
        assert_eq!(L10n::new_note(Language::Ja), "新規ノート");
        assert_eq!(L10n::new_note(Language::En), "New Note");
    }

    #[test]
    fn new_note_system_default_falls_back_to_english() {
        assert_eq!(
            L10n::new_note(Language::SystemDefault),
            L10n::new_note(Language::En)
        );
    }

    #[test]
    fn quit_interpolates_app_name() {
        assert_eq!(L10n::quit(Language::Zh, "Trace"), "退出 Trace");
        assert_eq!(L10n::quit(Language::Ja, "Trace"), "Traceを終了");
        assert_eq!(L10n::quit(Language::En, "Trace"), "Quit Trace");
    }

    #[test]
    fn quit_system_default_falls_back_to_english() {
        assert_eq!(
            L10n::quit(Language::SystemDefault, "Trace"),
            L10n::quit(Language::En, "Trace")
        );
    }

    #[test]
    fn quit_returns_owned_string_containing_app_name() {
        // Assigning to a `String` binding is a compile-time assertion that
        // `quit` doesn't return `&'static str` — this would fail to build if
        // the signature regressed to a borrowed return type.
        let rendered: String = L10n::quit(Language::En, "MyCoolApp");
        assert!(
            rendered.contains("MyCoolApp"),
            "quit output must embed the passed app name, got {rendered:?}"
        );
    }

    #[test]
    fn quit_handles_empty_app_name_without_panicking() {
        // Degenerate case: passing an empty name should produce a prefix-only
        // string (possibly with a trailing space) rather than panicking.
        assert_eq!(L10n::quit(Language::En, ""), "Quit ");
        assert_eq!(L10n::quit(Language::Zh, ""), "退出 ");
        assert_eq!(L10n::quit(Language::Ja, ""), "を終了");
    }
}
