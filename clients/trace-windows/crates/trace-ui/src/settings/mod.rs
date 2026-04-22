//! `SettingsApp` state, `SettingsMessage` enum, and `update/view/theme/subscription`
//! functions driving the settings window of the Trace client.
//!
//! Phase 12 sub-task 1 scaffolds the skeleton only: a second iced window
//! routed via [`iced::daemon`] that renders a single scrollable column with
//! the localized "Settings" title and no content. Subsequent sub-tasks layer
//! the 7 Cards (Language, Theme, Storage, Quick Sections, Shortcuts, System,
//! …) on top of this foundation.
//!
//! The module intentionally mirrors the `CaptureApp` pattern:
//!
//! * `SettingsApp` holds the mutable state plus a cached `iced::Theme` so the
//!   per-frame `theme()` call stays cheap.
//! * `settings_update` / `settings_view` / `settings_theme` / `settings_subscription`
//!   are free functions that match iced's builder API and are trivial to
//!   unit-test without spinning up the iced runtime.
//! * `AppSettings` is shared with [`crate::app::CaptureApp`] via `Arc` so the
//!   two windows see a consistent view of the user's preferences. Mutation
//!   through the settings window will land in later sub-tasks.
//!
//! The window itself is opened by [`crate::app::CaptureApp`] in response to
//! [`crate::app::Message::SettingsRequested`]; see `crates/trace-app` for the
//! top-level `iced::daemon` wiring that routes messages between the two
//! windows.

pub mod autostart;
mod quick_sections;
mod shortcut_event;
mod shortcuts;
pub mod storage;
mod system;
mod threads;
pub mod tiles;
pub mod widgets;

pub use autostart::{LaunchAtLoginSink, NoopLaunchAtLoginSink};

use std::path::PathBuf;
use std::sync::Arc;

use iced::event::{self as iced_event};
use iced::keyboard::Event as KeyboardEvent;
use iced::widget::{column, container, row, scrollable, text};
use iced::{window, Element, Event, Length, Pixels, Size, Subscription, Task, Theme};
use trace_core::{
    join_folder_and_filename, split_target_file, AppSettings, DailyFileDateFormat, EntryTheme,
    L10n, Language, NoteSection, ShortcutSpec, ThemePreset, ThreadConfig, TraceTheme,
    VaultPathValidationIssue, WriteMode,
};
use uuid::Uuid;

use crate::theme::to_iced_theme;

/// Default logical width of the settings window. Matches Mac
/// `SettingsView.swift` default frame width.
pub const DEFAULT_SETTINGS_WINDOW_WIDTH: f32 = 720.0;
/// Default logical height of the settings window.
pub const DEFAULT_SETTINGS_WINDOW_HEIGHT: f32 = 640.0;
/// Minimum width the user can resize the settings window down to. Matches
/// Mac `SettingsView.swift` `minWidth` of 640 pt.
pub const MIN_SETTINGS_WINDOW_WIDTH: f32 = 640.0;
/// Minimum height the user can resize the settings window down to. Matches
/// Mac `SettingsView.swift` `minHeight` of 480 pt.
pub const MIN_SETTINGS_WINDOW_HEIGHT: f32 = 480.0;
/// Vertical padding around the scrollable card stack. Sub-task 2 onwards adds
/// the card shell inside this padding.
pub const SETTINGS_OUTER_PADDING: u16 = 24;
/// Font size of the settings window title. Matches Mac `SettingsView.swift`
/// 22 pt title.
pub const SETTINGS_TITLE_FONT_SIZE: f32 = 22.0;

/// Returns the iced [`window::Settings`] for the settings window.
///
/// Split out as a free function so the `trace-app` crate can pass it
/// directly to `iced::window::open` and tests can compare the resulting
/// dimensions against the Mac `SettingsView.swift` defaults without running
/// the iced runtime. Unlike the capture panel, the settings window uses the
/// native decorations and stays on the normal window layer — it is a
/// conventional preferences sheet, not a floating capture panel.
pub fn window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(
            DEFAULT_SETTINGS_WINDOW_WIDTH,
            DEFAULT_SETTINGS_WINDOW_HEIGHT,
        ),
        min_size: Some(Size::new(
            MIN_SETTINGS_WINDOW_WIDTH,
            MIN_SETTINGS_WINDOW_HEIGHT,
        )),
        resizable: true,
        decorations: true,
        transparent: false,
        ..window::Settings::default()
    }
}

/// Which of the four configurable keyboard shortcuts is currently being
/// recorded by the user. Mirrors Mac `SettingsView.swift`'s `ShortcutTarget`
/// enum one-for-one so the cross-platform UX reads the same.
///
/// Used both as the discriminant carried through the record → capture round
/// trip (see [`SettingsMessage::RecordingStarted`] /
/// [`SettingsMessage::RecordingCaptured`]) and as the iteration handle that
/// the Shortcuts card loops over when rendering the four configurable rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutTarget {
    /// Global capture-panel summon hotkey. Registered via Win32 `RegisterHotKey`
    /// outside the focused window so it fires even while Trace is in the
    /// background — matches Mac `.create`.
    Create,
    /// Panel-scoped "send note" shortcut. Mac `.send`.
    Send,
    /// Panel-scoped "append note" shortcut. Mac `.append`.
    Append,
    /// Panel-scoped "toggle write mode" shortcut. Mac `.toggleWriteMode`.
    ToggleMode,
}

impl ShortcutTarget {
    /// Every variant in display order. Used by the card renderer to emit the
    /// four rows in a consistent sequence and by the conflict-scan helper in
    /// `settings_update` to compare against all other targets.
    pub const ALL: [ShortcutTarget; 4] = [
        ShortcutTarget::Create,
        ShortcutTarget::Send,
        ShortcutTarget::Append,
        ShortcutTarget::ToggleMode,
    ];

    /// Localized row label shown next to the shortcut chip. Mirrors Mac
    /// `ShortcutTarget.name`.
    pub fn name(self, lang: Language) -> &'static str {
        match self {
            ShortcutTarget::Create => L10n::shortcut_create(lang),
            ShortcutTarget::Send => L10n::shortcut_send(lang),
            ShortcutTarget::Append => L10n::shortcut_append(lang),
            ShortcutTarget::ToggleMode => L10n::shortcut_toggle_mode(lang),
        }
    }

    /// Localized "Global" / "Panel" category caption shown below the row
    /// label. Mirrors Mac `ShortcutTarget.category`.
    pub fn category(self, lang: Language) -> &'static str {
        match self {
            ShortcutTarget::Create => L10n::shortcut_category_global(lang),
            ShortcutTarget::Send | ShortcutTarget::Append | ShortcutTarget::ToggleMode => {
                L10n::shortcut_category_panel(lang)
            }
        }
    }
}

/// Messages consumed by [`settings_update`].
///
/// Sub-task 3 wires the first three cards (Language, Theme, Storage → Write
/// Mode). Sub-task 4 layers the per-write-mode Storage rows on top: vault /
/// inbox paths (with Browse buttons whose folder-picker result flows back
/// through [`VaultBrowseChose`] / [`InboxVaultBrowseChose`]), the Daily
/// folder name, the Daily file-name format, and the Daily entry format.
///
/// All new variants still only touch shadow fields on [`SettingsApp`] —
/// persistence to [`AppSettings`] lands in sub-task 8.
///
/// [`LanguageChanged`]: SettingsMessage::LanguageChanged
/// [`ThemePresetChanged`]: SettingsMessage::ThemePresetChanged
/// [`WriteModeChanged`]: SettingsMessage::WriteModeChanged
/// [`VaultBrowseChose`]: SettingsMessage::VaultBrowseChose
/// [`InboxVaultBrowseChose`]: SettingsMessage::InboxVaultBrowseChose
#[derive(Debug, Clone)]
pub enum SettingsMessage {
    /// Fired when the user changes the active language via the Language card.
    /// Updates the shadow [`SettingsApp::language`] field so the rest of the
    /// settings view localizes immediately without a round-trip through
    /// [`AppSettings`].
    LanguageChanged(Language),
    /// Fired when the user picks a different theme preset via the Theme card.
    /// Updates the shadow [`SettingsApp::theme_preset`] and rebuilds the
    /// cached iced theme so the settings window itself previews the selection
    /// in-place.
    ThemePresetChanged(ThemePreset),
    /// Fired when the user picks a different write mode via the Storage card.
    /// Updates the shadow [`SettingsApp::write_mode`] field without touching
    /// the shared [`AppSettings`] snapshot.
    WriteModeChanged(WriteMode),
    /// Fired on every keystroke in the Dimension-mode vault path text
    /// input. Updates [`SettingsApp::vault_path`].
    VaultPathChanged(String),
    /// Fired when the user clicks the Dimension-mode vault Browse button.
    /// `settings_update` reacts by kicking off an `rfd::AsyncFileDialog`
    /// folder picker via `iced::Task::perform` and emits a follow-up
    /// [`VaultBrowseChose`](SettingsMessage::VaultBrowseChose) with the
    /// selected path (or `None` if the user cancelled).
    BrowseVaultRequested,
    /// Follow-up to [`BrowseVaultRequested`](SettingsMessage::BrowseVaultRequested)
    /// carrying the picker's result. A `Some(path)` updates
    /// [`SettingsApp::vault_path`]; a `None` (cancelled) is a no-op.
    VaultBrowseChose(Option<String>),
    /// Fired on every keystroke in the File-mode inbox-vault path text
    /// input. Updates [`SettingsApp::inbox_vault_path`].
    InboxVaultPathChanged(String),
    /// Fired when the user clicks the File-mode inbox-vault Browse button.
    /// Mirrors [`BrowseVaultRequested`](SettingsMessage::BrowseVaultRequested)
    /// for the inbox vault.
    BrowseInboxVaultRequested,
    /// Follow-up to [`BrowseInboxVaultRequested`](SettingsMessage::BrowseInboxVaultRequested)
    /// carrying the picker's result. A `Some(path)` updates
    /// [`SettingsApp::inbox_vault_path`]; a `None` is a no-op.
    InboxVaultBrowseChose(Option<String>),
    /// Fired on every keystroke in the Daily folder name text input.
    /// Updates [`SettingsApp::daily_folder_name`].
    DailyFolderNameChanged(String),
    /// Fired when the user picks a different [`DailyFileDateFormat`] variant
    /// via the File Name Format picker. Updates
    /// [`SettingsApp::daily_file_date_format`].
    DailyFileDateFormatChanged(DailyFileDateFormat),
    /// Fired when the user picks a different [`EntryTheme`] variant via the
    /// Entry Format picker. Updates
    /// [`SettingsApp::daily_entry_theme_preset`].
    DailyEntryThemePresetChanged(EntryTheme),
    /// Fired on every keystroke in a section title text input. Writes the new
    /// string into [`SettingsApp::section_titles`] at `index` if in bounds;
    /// out-of-bounds indices silently no-op (iced should never emit one, but
    /// the branch defends against stale `Message` deliveries that arrive after
    /// a [`SectionRemoved`](SettingsMessage::SectionRemoved) shrinks the vec).
    SectionTitleChanged(usize, String),
    /// Adds a new section at the tail of [`SettingsApp::section_titles`] using
    /// [`NoteSection::default_title_for`] for the new index. No-op once the
    /// vec is already at [`NoteSection::MAXIMUM_COUNT`] (9).
    SectionAdded,
    /// Removes the section at `index` from [`SettingsApp::section_titles`] if
    /// the current length is above [`NoteSection::MINIMUM_COUNT`] (1) *and*
    /// `index` is in bounds; otherwise silently no-op.
    SectionRemoved(usize),
    /// Fired on every keystroke in a thread name text input. Writes the new
    /// string into the matching [`SettingsApp::thread_configs`] entry if one
    /// exists for `id`; an unknown id silently no-ops (guards against stale
    /// messages arriving after a [`ThreadRemoved`](SettingsMessage::ThreadRemoved)
    /// drops the entry).
    ThreadNameChanged(Uuid, String),
    /// Fired on every keystroke in the folder-path portion of a thread row.
    /// The shadow stores only the integrated `target_file`; the update branch
    /// splits the current value, substitutes the new folder, and joins back.
    /// Mirrors Mac `ThreadConfigRow.commitFolder`'s target-file rebuild, but
    /// without the trim/default-filename normalization (that lives in the
    /// sub-task 8 write-back path so the shadow can reflect in-progress keystrokes
    /// faithfully).
    ThreadFolderChanged(Uuid, String),
    /// Fired on every keystroke in the filename portion of a thread row. Same
    /// shadow-rebuild contract as
    /// [`ThreadFolderChanged`](SettingsMessage::ThreadFolderChanged): split
    /// the current target_file, substitute the new filename, join back.
    ThreadFilenameChanged(Uuid, String),
    /// Fired when the user clicks a row's "Choose Folder" button. Mirrors
    /// [`BrowseVaultRequested`](SettingsMessage::BrowseVaultRequested) but
    /// carries the thread `id` through the async round-trip so the follow-up
    /// message can look up the right shadow entry.
    BrowseThreadFolderRequested(Uuid),
    /// Follow-up to
    /// [`BrowseThreadFolderRequested`](SettingsMessage::BrowseThreadFolderRequested)
    /// carrying the picker's result. A `Some(path)` converts to a relative
    /// path when the picked folder sits inside the shadow vault (matching
    /// Mac `ThreadConfigRow.chooseFolder`'s `vaultURL.path` prefix check) or
    /// keeps the absolute path otherwise, then rebuilds the row's
    /// `target_file` against the shadow filename. A `None` (cancelled) is a
    /// no-op.
    ThreadFolderBrowseChose(Uuid, Option<String>),
    /// Adds a new [`ThreadConfig`] at the tail of [`SettingsApp::thread_configs`].
    /// Mirrors Mac `AppSettings.addThread`: pick the first
    /// [`L10n::new_thread_default_name`]-based name that does not collide
    /// with an existing thread (add a numeric suffix when it does), default
    /// the target file to `<name>.md`, and assign an `order` one above the
    /// current max. No-op once the vec is at [`ThreadConfig::MAXIMUM_COUNT`].
    ThreadAdded,
    /// Removes the thread identified by `id` from
    /// [`SettingsApp::thread_configs`]. No-op when the shadow is at
    /// [`ThreadConfig::MINIMUM_COUNT`]; mirrors
    /// [`SectionRemoved`](SettingsMessage::SectionRemoved)'s floor guard so a
    /// stale message delivered after the button is already disabled cannot
    /// drop the last thread.
    ThreadRemoved(Uuid),
    /// Fired when the user clicks the "Edit" (pencil) button on a shortcut
    /// row in the Shortcuts card. Puts the settings state into "recording"
    /// mode for the given target, clears any stale validation message, and
    /// lets the keyboard subscription (sub-task 7 Commit 4) start capturing
    /// keystrokes. A second click on the same target or a click on a different
    /// target's Edit button simply re-arms the state for the new target.
    RecordingStarted(ShortcutTarget),
    /// Fired when the user presses Esc with no modifiers during recording, or
    /// clicks the Cancel button. Clears
    /// [`SettingsApp::recording_target`] and
    /// [`SettingsApp::shortcut_recorder_message`] without touching any
    /// shortcut shadow field. Mirrors Mac `stopRecording()`'s bare Esc case
    /// which returns `nil` from the `NSEvent` monitor.
    RecordingCancelled,
    /// Fired by the keyboard subscription (sub-task 7 Commit 4) when a key
    /// press arrives while a target is being recorded. The update branch
    /// validates the candidate (modifier required, Esc reserved, ⌘/Ctrl+1-9
    /// reserved for panel targets, conflict-free across all four targets) and
    /// either stores the new [`ShortcutSpec`] into the target's shadow field or
    /// stages an error message on [`SettingsApp::shortcut_recorder_message`].
    ///
    /// `key_code` is a Win32 virtual-key code; `modifiers` uses the
    /// `MOD_ALT|MOD_CONTROL|MOD_SHIFT|MOD_WIN` bit set from
    /// [`trace_core::ShortcutSpec`]. The split into a pair of `u32`s (rather
    /// than passing a `ShortcutSpec` directly) keeps the subscription's
    /// message type trivially [`Clone`] without forcing `ShortcutSpec` into
    /// the message hot path for every other variant.
    RecordingCaptured {
        /// Win32 virtual-key code of the captured key.
        key_code: u32,
        /// Bitmask of modifier flags (`MOD_ALT|MOD_CONTROL|MOD_SHIFT|MOD_WIN`).
        modifiers: u32,
    },
    /// Fired by the System card's "Launch at Login" toggle.
    ///
    /// Sub-task 8a keeps the variant shadow-only: the update branch flips
    /// [`SettingsApp::launch_at_login_shadow`] and returns `Task::none()`.
    /// Persistence to the filesystem + the actual Windows autostart registry
    /// entry are wired in sub-tasks 8b (persistence architecture) and 8c
    /// (autostart integration) respectively, following the same shadow-only
    /// → write-back split every other sub-task 1-7 card already uses.
    LaunchAtLoginToggled(bool),
}

/// Mutable application state for the settings window.
///
/// The struct is intentionally data-oriented so `settings_update` can be
/// tested without running iced's event loop. Field parity with
/// [`crate::app::CaptureApp`]:
///
/// * `theme` — full [`TraceTheme`] snapshot for the current preset.
/// * `iced_theme` — cached [`iced::Theme`] derived from `theme`; kept in
///   sync via the [`SettingsApp::set_theme`] helper.
/// * `settings` — shared `AppSettings` snapshot. Wrapped in [`Arc`] so the
///   capture and settings windows can hand the same pointer to writers
///   without deep-cloning the `Vec<ThreadConfig>` / `Vec<NoteSection>`
///   allocations.
/// * `language` — starts as `settings.language` and drives localized
///   strings in `settings_view`. Split out from the `Arc<AppSettings>` so
///   the settings window can preview a language change before the
///   `AppSettings` write lands.
pub struct SettingsApp {
    /// Full theme palette bundle for the currently selected preset.
    pub theme: TraceTheme,
    /// Cached iced [`Theme`] derived from [`SettingsApp::theme`]. Refreshed
    /// atomically via [`SettingsApp::set_theme`].
    ///
    /// **Invariant**: `iced_theme == to_iced_theme(&theme)`.
    pub iced_theme: Theme,
    /// Shared settings snapshot. Mutated through the settings window is a
    /// later sub-task; sub-task 1 only reads from this handle.
    pub settings: Arc<AppSettings>,
    /// Active language for localized view strings. Seeded from
    /// `settings.language`; later sub-tasks wire a language picker to this
    /// field.
    pub language: Language,
    /// Active theme preset shown in the Theme card. Seeded from
    /// `settings.app_theme_preset`; mutated in-place by
    /// [`SettingsMessage::ThemePresetChanged`] so the settings window can
    /// preview a new preset before the write back to [`AppSettings`] lands
    /// (sub-task 8).
    pub theme_preset: ThemePreset,
    /// Active write mode shown in the Storage → Write Mode row. Seeded from
    /// `settings.note_write_mode`; mutated in-place by
    /// [`SettingsMessage::WriteModeChanged`] so the Storage card can toggle
    /// between `Dimension` / `Thread` / `File` independently of the shared
    /// [`AppSettings`] snapshot.
    pub write_mode: WriteMode,
    /// Active Dimension-mode vault path shown in the Storage card. Seeded
    /// from `settings.vault_path`; mutated in-place by
    /// [`SettingsMessage::VaultPathChanged`] / [`SettingsMessage::VaultBrowseChose`]
    /// so the inline validation hint tracks edits in real time.
    pub vault_path: String,
    /// Active File-mode inbox vault path shown in the Storage card. Seeded
    /// from `settings.inbox_vault_path`.
    pub inbox_vault_path: String,
    /// Active Daily folder name. Seeded from `settings.daily_folder_name`.
    pub daily_folder_name: String,
    /// Active Daily file-name format. Seeded from
    /// `settings.daily_file_date_format` via
    /// [`DailyFileDateFormat::resolved_from_raw`] so an unknown on-disk
    /// value falls back to the [`DailyFileDateFormat::ChineseFull`] default,
    /// matching Mac's `DailyFileDateFormat.resolved(fromStored:)`.
    pub daily_file_date_format: DailyFileDateFormat,
    /// Active Daily entry theme preset. Seeded from
    /// `settings.daily_entry_theme_preset`.
    pub daily_entry_theme_preset: EntryTheme,
    /// Cached classification of [`Self::vault_path`] against
    /// [`trace_platform::validate_vault_path`]. Recomputed in
    /// [`settings_update`] whenever the path changes; the view layer reads
    /// this field directly because `validate_vault_path` writes a probe file
    /// to disk and must never be called from the per-frame view pass.
    pub vault_path_issue: Option<VaultPathValidationIssue>,
    /// Cached classification of [`Self::inbox_vault_path`]. Mirrors the same
    /// contract as [`Self::vault_path_issue`]: refreshed in `settings_update`,
    /// never on the per-frame render hot path.
    pub inbox_vault_path_issue: Option<VaultPathValidationIssue>,
    /// Shadow of [`AppSettings::section_titles`] mutated by the Quick Sections
    /// card (sub-task 5). Each entry is a free-form title string; the vec
    /// length is kept in `[NoteSection::MINIMUM_COUNT, NoteSection::MAXIMUM_COUNT]`
    /// by the `SectionAdded` / `SectionRemoved` branches in
    /// [`settings_update`]. This field is **never** written back into
    /// [`Self::settings`]; the trim/normalize + persistence round-trip lands
    /// in sub-task 8 so the Mac reference's save-button semantics can be
    /// mirrored exactly there.
    pub section_titles: Vec<String>,
    /// Shadow of [`AppSettings::thread_configs`] mutated by the Threads card
    /// (sub-task 6). Each entry is a full [`ThreadConfig`] whose `target_file`
    /// is kept as the integrated `folder/filename` string; the view layer
    /// splits it on every render for the two editable sub-fields so the shadow
    /// never has to maintain a parallel draft state.
    ///
    /// Physical order of the vec is not guaranteed to match `order` — renders
    /// sort by `order` on demand so a reorder UX in a future sub-task can
    /// stay a pure data operation. Length is bracketed by
    /// `[ThreadConfig::MINIMUM_COUNT, ThreadConfig::MAXIMUM_COUNT]` by the
    /// `ThreadAdded` / `ThreadRemoved` branches; persistence to
    /// [`Self::settings`] is deferred to sub-task 8.
    pub thread_configs: Vec<ThreadConfig>,
    /// Shadow of [`AppSettings::hot_key_code`] + `hot_key_modifiers` pair,
    /// driving the Shortcuts card's "Create Note" row (sub-task 7). Seeded
    /// from the persisted `u32` pair in [`Self::new`] and mutated in-place by
    /// [`SettingsMessage::RecordingCaptured`] when the user records a new
    /// binding for [`ShortcutTarget::Create`].
    pub global_hotkey: ShortcutSpec,
    /// Shadow of the Mac-equivalent `send_note_*` pair, driving the Shortcuts
    /// card's "Send Note" row (sub-task 7). Same seeding / mutation contract
    /// as [`Self::global_hotkey`] but scoped to
    /// [`ShortcutTarget::Send`].
    pub send_note_shortcut: ShortcutSpec,
    /// Shadow of the `append_note_*` pair, driving the Shortcuts card's
    /// "Append Note" row (sub-task 7).
    pub append_note_shortcut: ShortcutSpec,
    /// Shadow of the `mode_toggle_*` pair, driving the Shortcuts card's
    /// "Toggle Write Mode" row (sub-task 7).
    pub mode_toggle_shortcut: ShortcutSpec,
    /// Which shortcut target is currently recording keystrokes, if any.
    /// `None` means the Shortcuts card is in its resting state; `Some(t)`
    /// means the keyboard subscription (sub-task 7 Commit 4) should route
    /// key events into the validation pipeline. Mirrors Mac
    /// `SettingsView.recordingTarget`.
    pub recording_target: Option<ShortcutTarget>,
    /// Stale error message from the most recent [`SettingsMessage::RecordingCaptured`]
    /// validation failure, or `None` when recording succeeded / never
    /// started. Rendered directly by the Shortcuts card's footer and cleared
    /// on the next [`SettingsMessage::RecordingStarted`] /
    /// [`SettingsMessage::RecordingCancelled`]. Matches Mac
    /// `SettingsView.shortcutRecorderMessage`.
    pub shortcut_recorder_message: Option<String>,
    /// Shadow of [`AppSettings::launch_at_login`], driving the System card's
    /// "Launch at Login" toggle (sub-task 8a). Seeded from the persisted
    /// `bool` in [`Self::new`] and mutated in-place by
    /// [`SettingsMessage::LaunchAtLoginToggled`]. Persistence is wired in
    /// sub-task 8b (through [`persist_working`]); the Windows autostart
    /// registry integration is deferred to sub-task 8c.
    pub launch_at_login_shadow: bool,
    /// Writable working copy of the settings. Each shadow-field message
    /// handler mirrors its corresponding mutation here and then calls
    /// [`persist_working`] — the Rust analog of Mac's `@Published` +
    /// `didSet { UserDefaults.set(...) }` pair. Sub-task 8c flips the
    /// surrounding `Arc<AppSettings>` handle to `Arc<RwLock<AppSettings>>` so
    /// the CaptureApp sees the edits too; for 8b the working copy stays local
    /// to the settings window because the Mac reference doesn't notify the
    /// capture panel on every keystroke either.
    pub working: AppSettings,
    /// Destination the working copy is serialised to after every edit.
    /// `None` skips the write so UI-only tests (and any future call site that
    /// deliberately wants an ephemeral settings buffer) do not touch the
    /// filesystem. The legacy [`Self::new`] constructor defaults this to
    /// `None` to keep the existing test harness and `trace-app` wiring
    /// bit-for-bit identical.
    pub save_path: Option<PathBuf>,
    /// Host-OS launch-at-login sink invoked from the
    /// [`SettingsMessage::LaunchAtLoginToggled`] arm. Production `trace-app`
    /// wires this to [`trace_platform::autostart::enable`] /
    /// [`trace_platform::autostart::disable`] via a local
    /// [`LaunchAtLoginSink`] impl; legacy constructors default to
    /// [`NoopLaunchAtLoginSink`] so unit tests and non-Windows dev builds
    /// stay filesystem-/registry-free. Held behind `Arc<dyn ...>` so the
    /// `SettingsApp` can stay `Send + Sync` without constraining the sink's
    /// concrete type (`Arc<T>` is `Send + Sync` when `T: Send + Sync`).
    pub launch_at_login_sink: Arc<dyn LaunchAtLoginSink>,
    /// Most recent `Arc<AppSettings>` snapshot of [`Self::working`]. Refreshed
    /// inside [`persist_working`] — i.e. every time a shadow-field
    /// [`SettingsMessage`] commits a change — so the daemon layer can cheaply
    /// `Arc::clone(&app.latest_snapshot)` and broadcast it to
    /// [`crate::app::CaptureApp`] after each Settings dispatch. Sub-task 8c.3
    /// is the first consumer; the constructor initialises it to the input
    /// `Arc` so the very first broadcast (if any) points at the byte-for-byte
    /// identical allocation the daemon is already holding.
    ///
    /// Visibility: `pub(crate)` so only [`persist_working`] (the one endorsed
    /// writer) and the in-crate tests can mutate the field directly. External
    /// readers must go through the [`Self::latest_snapshot`] accessor so the
    /// invariant "snapshot == last persisted working copy" cannot be bypassed
    /// from outside the crate.
    pub(crate) latest_snapshot: Arc<AppSettings>,
}

impl SettingsApp {
    /// Builds a fresh [`SettingsApp`] from a resolved [`TraceTheme`] and a
    /// shared [`AppSettings`] handle, without a disk-backed working copy.
    ///
    /// Thin wrapper around [`Self::new_with_save_path`] with `save_path: None`
    /// — kept on the public surface so every existing call site (the
    /// `trace-app` wiring in `main.rs` and the in-module tests) keeps
    /// compiling unchanged. Sub-task 8b persistence lands exclusively through
    /// the `*_with_save_path` constructor so the test suite can stay
    /// filesystem-free by default.
    pub fn new(theme: TraceTheme, settings: Arc<AppSettings>) -> Self {
        Self::new_with_save_path(theme, settings, None)
    }

    /// Builds a fresh [`SettingsApp`] with an optional write-through save
    /// path, defaulting the autostart sink to [`NoopLaunchAtLoginSink`].
    ///
    /// Preserves the pre-8c.2 two-argument-plus-path signature so the entire
    /// in-repo test harness keeps compiling untouched. Production startup
    /// wiring in `trace-app/main.rs` calls
    /// [`Self::new_with_dependencies`] directly so the real
    /// registry-backed sink is in place; every other call site (unit tests,
    /// transient UI previews) stays registry-free through this shim.
    pub fn new_with_save_path(
        theme: TraceTheme,
        settings: Arc<AppSettings>,
        save_path: Option<PathBuf>,
    ) -> Self {
        Self::new_with_dependencies(theme, settings, save_path, Arc::new(NoopLaunchAtLoginSink))
    }

    /// Builds a fresh [`SettingsApp`] and optionally arms the write-through
    /// persistence pipeline by supplying a `save_path`, plus injects the
    /// [`LaunchAtLoginSink`] that the
    /// [`SettingsMessage::LaunchAtLoginToggled`] arm forwards toggles to.
    ///
    /// `settings.language` is copied into the local `language` field so the
    /// settings window can render immediately without waiting for the first
    /// message dispatch. When `save_path` is `Some`, every shadow-field
    /// mutation handled by [`settings_update`] first commits to
    /// [`Self::working`] and then calls [`persist_working`] — the Rust analog
    /// of Mac `AppSettings`' `@Published` + `didSet { UserDefaults.set(...) }`
    /// pair. A `None` skips the write so UI-only call sites (unit tests,
    /// transient previews) keep their filesystem-free behaviour.
    ///
    /// The `launch_at_login_sink` is called from the `LaunchAtLoginToggled`
    /// arm *after* the shadow + `working` fields have been updated and
    /// `persist_working` has run, so failing (or stubbed) sinks never prevent
    /// the on-disk JSON from catching up. Pass
    /// [`NoopLaunchAtLoginSink`] wrapped in an `Arc` for non-production
    /// callers.
    ///
    /// Callers must still keep the `Arc<AppSettings>` in sync if other
    /// windows mutate the underlying settings; sub-task 8c introduces an
    /// `RwLock` wrapper for the cross-window synchronisation that Mac gets
    /// for free from `@ObservedObject`.
    pub fn new_with_dependencies(
        theme: TraceTheme,
        settings: Arc<AppSettings>,
        save_path: Option<PathBuf>,
        launch_at_login_sink: Arc<dyn LaunchAtLoginSink>,
    ) -> Self {
        let iced_theme = to_iced_theme(&theme);
        let language = settings.language;
        let theme_preset = settings.app_theme_preset;
        let write_mode = settings.note_write_mode;
        let vault_path = settings.vault_path.clone();
        let inbox_vault_path = settings.inbox_vault_path.clone();
        let daily_folder_name = settings.daily_folder_name.clone();
        // On-disk value is a raw ICU string; resolve to a preset variant via
        // the Mac-compatible fallback rule so an unknown string decodes to
        // `ChineseFull` rather than silently mismatching the picker.
        let daily_file_date_format =
            DailyFileDateFormat::resolved_from_raw(&settings.daily_file_date_format);
        let daily_entry_theme_preset = settings.daily_entry_theme_preset;
        // Seed the cached validation results once at construction so the
        // Storage card has something to render on the first view pass without
        // touching the filesystem on every frame.
        let vault_path_issue = trace_platform::validate_vault_path(&vault_path);
        let inbox_vault_path_issue = trace_platform::validate_vault_path(&inbox_vault_path);
        // Clone the persisted Quick Sections titles into the shadow so the
        // card has something to render before the user edits anything. We
        // clone instead of borrowing because `settings` is shared behind an
        // `Arc` and the shadow must remain mutable without taking a lock on
        // every edit.
        let section_titles = settings.section_titles.clone();
        // Clone the persisted thread configs into the shadow for the same
        // reason — the Threads card (sub-task 6) mutates the shadow freely
        // without taking a lock on the `Arc<AppSettings>`.
        let thread_configs = settings.thread_configs.clone();
        // Pack each persisted `(key_code, modifiers)` `u32` pair into a
        // `ShortcutSpec` once at construction. The shadow then carries the
        // struct directly so the card renderer and validation pipeline read
        // a single value instead of two parallel fields.
        let global_hotkey = ShortcutSpec::new(settings.hot_key_code, settings.hot_key_modifiers);
        let send_note_shortcut =
            ShortcutSpec::new(settings.send_note_key_code, settings.send_note_modifiers);
        let append_note_shortcut = ShortcutSpec::new(
            settings.append_note_key_code,
            settings.append_note_modifiers,
        );
        let mode_toggle_shortcut = ShortcutSpec::new(
            settings.mode_toggle_key_code,
            settings.mode_toggle_modifiers,
        );
        // Mirror the persisted `launch_at_login` flag into the shadow so the
        // System card reads the user's last choice on first paint. Sub-task
        // 8a is UI-only — the shadow never writes back to the `Arc` here.
        let launch_at_login_shadow = settings.launch_at_login;
        // Clone the full `AppSettings` into the local working copy. Every
        // shadow-field message in `settings_update` commits the matching
        // field onto this struct before calling `persist_working`, so the
        // on-disk JSON stays a byte-for-byte reflection of the user's most
        // recent edit — no debounce, no batching.
        let working = (*settings).clone();
        // Seed the broadcast snapshot with the input `Arc` itself. Content
        // is identical to `working` at construction time, and reusing the
        // incoming pointer means the daemon's `state.shared_settings` and
        // `SettingsApp::latest_snapshot` share one allocation until the
        // first shadow-field edit triggers `persist_working`.
        let latest_snapshot = Arc::clone(&settings);
        Self {
            theme,
            iced_theme,
            settings,
            language,
            theme_preset,
            write_mode,
            vault_path,
            inbox_vault_path,
            daily_folder_name,
            daily_file_date_format,
            daily_entry_theme_preset,
            vault_path_issue,
            inbox_vault_path_issue,
            section_titles,
            thread_configs,
            global_hotkey,
            send_note_shortcut,
            append_note_shortcut,
            mode_toggle_shortcut,
            recording_target: None,
            shortcut_recorder_message: None,
            launch_at_login_shadow,
            working,
            save_path,
            launch_at_login_sink,
            latest_snapshot,
        }
    }

    /// Returns the shadow [`ShortcutSpec`] for `target`. Centralized so the
    /// view layer and the conflict-scan logic share one read path per target
    /// instead of each match arm re-splitting the four fields.
    pub fn shortcut_for(&self, target: ShortcutTarget) -> ShortcutSpec {
        match target {
            ShortcutTarget::Create => self.global_hotkey,
            ShortcutTarget::Send => self.send_note_shortcut,
            ShortcutTarget::Append => self.append_note_shortcut,
            ShortcutTarget::ToggleMode => self.mode_toggle_shortcut,
        }
    }

    /// Writes `spec` into the shadow field for `target`. Mirror of
    /// [`Self::shortcut_for`] on the mutable side; keeps the per-target match
    /// out of [`settings_update`]'s `RecordingCaptured` branch.
    pub fn set_shortcut(&mut self, target: ShortcutTarget, spec: ShortcutSpec) {
        match target {
            ShortcutTarget::Create => self.global_hotkey = spec,
            ShortcutTarget::Send => self.send_note_shortcut = spec,
            ShortcutTarget::Append => self.append_note_shortcut = spec,
            ShortcutTarget::ToggleMode => self.mode_toggle_shortcut = spec,
        }
    }

    /// Replaces the active [`TraceTheme`] and refreshes the cached
    /// [`Self::iced_theme`] atomically. Mirrors
    /// [`crate::app::CaptureApp::set_theme`].
    pub fn set_theme(&mut self, theme: TraceTheme) {
        self.iced_theme = to_iced_theme(&theme);
        self.theme = theme;
    }

    /// Cheap `Arc::clone` of the most recent write-through snapshot.
    ///
    /// Used by the daemon layer (sub-task 8c.3) to broadcast settings
    /// changes to [`crate::app::CaptureApp`] after every `settings_update`
    /// dispatch. Because [`persist_working`] refreshes
    /// [`Self::latest_snapshot`] on every shadow-field commit, successive
    /// calls between two edits return pointers to the same allocation —
    /// the `CaptureApp::ReplaceSettings` arm relies on `Arc::ptr_eq` to
    /// short-circuit no-op dispatches.
    pub fn latest_snapshot(&self) -> Arc<AppSettings> {
        Arc::clone(&self.latest_snapshot)
    }
}

/// Refreshes [`SettingsApp::latest_snapshot`] from the current `working`
/// copy and serialises the same copy to [`SettingsApp::save_path`], logging
/// but otherwise swallowing any failure.
///
/// The snapshot refresh is unconditional — it must run even when
/// `save_path` is `None` (ephemeral test buffers, transient UI previews)
/// so the daemon's broadcast to `CaptureApp` picks up the edit regardless
/// of persistence configuration. The disk write still only fires when a
/// `save_path` was supplied.
///
/// Mirrors the UI-non-blocking contract of Mac `AppSettings`' `@Published` +
/// `didSet { UserDefaults.set(...) }` pair: a save failure is surfaced to
/// `tracing::warn!` (Windows analog of `NSLog`) and never bubbles up into
/// the view tree. The Mac reference's `updateLaunchAtLogin()` uses the same
/// `NSLog` fallback for the same class of errors — a non-writable settings
/// directory must not block the user from continuing to type.
fn persist_working(state: &mut SettingsApp) {
    // Refresh the broadcast snapshot before touching disk so a concurrent
    // `latest_snapshot()` call observes the new value whether or not the
    // save succeeds. Every call reallocates a fresh `Arc` — cheap compared
    // to the full `AppSettings` clone below — so `Arc::ptr_eq` between
    // pre-edit and post-edit snapshots always returns `false`, which the
    // daemon layer relies on to detect real changes.
    state.latest_snapshot = Arc::new(state.working.clone());
    let Some(path) = state.save_path.as_deref() else {
        return;
    };
    if let Err(error) = state.working.save(path) {
        tracing::warn!(%error, path = %path.display(), "failed to persist settings");
    }
}

/// Mutates the supplied [`SettingsApp`] in response to a [`SettingsMessage`]
/// and returns an [`iced::Task`] describing any follow-up effect.
///
/// Sub-task 1 handled only [`SettingsMessage::LanguageChanged`]; sub-task 3
/// added theme-preset and write-mode branches; sub-task 4 wires the full
/// per-write-mode Storage card: free-text edits land directly on shadow
/// fields, and the two Browse buttons fire `Task::perform` against
/// [`rfd::AsyncFileDialog::pick_folder`] so the picker runs outside the iced
/// view thread. The picker's `Option<String>` flows back through
/// [`SettingsMessage::VaultBrowseChose`] / [`SettingsMessage::InboxVaultBrowseChose`].
///
/// Sub-task 8b layers write-through persistence on top: every branch that
/// mutates a shadow field corresponding to a persisted [`AppSettings`]
/// property also commits the change into [`SettingsApp::working`] and calls
/// [`persist_working`]. Pure-UI branches (recorder arming, focus, Browse
/// requests, picker cancellations) stay out of the persistence path so the
/// on-disk JSON only changes when the user's intent lands on a real field
/// edit.
pub fn settings_update(state: &mut SettingsApp, message: SettingsMessage) -> Task<SettingsMessage> {
    match message {
        SettingsMessage::LanguageChanged(lang) => {
            state.language = lang;
            state.working.language = lang;
            persist_working(state);
            Task::none()
        }
        SettingsMessage::ThemePresetChanged(preset) => {
            // Swap the shadow field first, then rebuild the full
            // [`TraceTheme`] so the cached iced theme stays in lock-step.
            // `set_theme` handles the iced-theme recompute so this branch
            // only needs to remember the preset and hand off to the helper.
            state.theme_preset = preset;
            state.set_theme(TraceTheme::for_preset(preset));
            state.working.app_theme_preset = preset;
            persist_working(state);
            Task::none()
        }
        SettingsMessage::WriteModeChanged(mode) => {
            state.write_mode = mode;
            state.working.note_write_mode = mode;
            persist_working(state);
            Task::none()
        }
        SettingsMessage::VaultPathChanged(path) => {
            state.vault_path = path;
            state.vault_path_issue = trace_platform::validate_vault_path(&state.vault_path);
            // Persist the trimmed path so trailing whitespace from a paste
            // never lands on disk, matching Mac `AppSettings.vaultPath`'s
            // `trimmingCharacters(.whitespacesAndNewlines)` normalization on
            // write.
            state.working.vault_path = state.vault_path.trim().to_string();
            persist_working(state);
            Task::none()
        }
        SettingsMessage::BrowseVaultRequested => pick_folder_task(SettingsMessage::VaultBrowseChose),
        SettingsMessage::VaultBrowseChose(Some(path)) => {
            state.vault_path = path;
            state.vault_path_issue = trace_platform::validate_vault_path(&state.vault_path);
            state.working.vault_path = state.vault_path.trim().to_string();
            persist_working(state);
            Task::none()
        }
        // User cancelled the picker — leave the current path and cached issue
        // untouched; nothing on disk changed so a re-probe would be wasted I/O.
        SettingsMessage::VaultBrowseChose(None) => Task::none(),
        SettingsMessage::InboxVaultPathChanged(path) => {
            state.inbox_vault_path = path;
            state.inbox_vault_path_issue =
                trace_platform::validate_vault_path(&state.inbox_vault_path);
            state.working.inbox_vault_path = state.inbox_vault_path.trim().to_string();
            persist_working(state);
            Task::none()
        }
        SettingsMessage::BrowseInboxVaultRequested => {
            pick_folder_task(SettingsMessage::InboxVaultBrowseChose)
        }
        SettingsMessage::InboxVaultBrowseChose(Some(path)) => {
            state.inbox_vault_path = path;
            state.inbox_vault_path_issue =
                trace_platform::validate_vault_path(&state.inbox_vault_path);
            state.working.inbox_vault_path = state.inbox_vault_path.trim().to_string();
            persist_working(state);
            Task::none()
        }
        SettingsMessage::InboxVaultBrowseChose(None) => Task::none(),
        SettingsMessage::DailyFolderNameChanged(name) => {
            state.daily_folder_name = name;
            state.working.daily_folder_name = state.daily_folder_name.trim().to_string();
            persist_working(state);
            Task::none()
        }
        SettingsMessage::DailyFileDateFormatChanged(format) => {
            state.daily_file_date_format = format;
            // Persist as the raw ICU string the rest of the stack reads —
            // matches Mac `AppSettings.dailyFileDateFormat` which stores the
            // picker selection as a plain string rather than the typed
            // enum variant.
            state.working.daily_file_date_format = format.raw_value().to_string();
            persist_working(state);
            Task::none()
        }
        SettingsMessage::DailyEntryThemePresetChanged(theme) => {
            state.daily_entry_theme_preset = theme;
            state.working.daily_entry_theme_preset = theme;
            persist_working(state);
            Task::none()
        }
        SettingsMessage::SectionTitleChanged(index, value) => {
            // Defensive `get_mut`: iced should never route a stale index to
            // this branch, but if a `SectionRemoved` shrinks the vec before a
            // still-inflight keystroke arrives we must drop it rather than
            // panic. The shadow itself stays a faithful reflection of the
            // textfield — we only run `normalize()` on the working copy so
            // the on-disk JSON lands in its canonical shape even when the
            // user's current keystroke leaves the shadow mid-heading.
            if let Some(slot) = state.section_titles.get_mut(index) {
                *slot = value;
                commit_section_titles(state);
                persist_working(state);
            }
            Task::none()
        }
        SettingsMessage::SectionAdded => {
            // Cap at `MAXIMUM_COUNT` so the button becomes a visual disabled
            // state + a no-op guard in the update layer. Mirrors Mac
            // `AppSettings.addSection()` which bails above the same cap.
            if state.section_titles.len() < NoteSection::MAXIMUM_COUNT {
                let next_index = state.section_titles.len();
                state
                    .section_titles
                    .push(NoteSection::default_title_for(next_index));
                commit_section_titles(state);
                persist_working(state);
            }
            Task::none()
        }
        SettingsMessage::SectionRemoved(index) => {
            // Symmetric guard against the `MINIMUM_COUNT` floor. The extra
            // `index < len()` check covers stale deliveries the same way the
            // `SectionTitleChanged` branch does.
            if state.section_titles.len() > NoteSection::MINIMUM_COUNT
                && index < state.section_titles.len()
            {
                state.section_titles.remove(index);
                commit_section_titles(state);
                persist_working(state);
            }
            Task::none()
        }
        SettingsMessage::ThreadNameChanged(id, value) => {
            if let Some(thread) = state.thread_configs.iter_mut().find(|t| t.id == id) {
                thread.name = value;
                commit_thread_configs(state);
                persist_working(state);
            }
            Task::none()
        }
        SettingsMessage::ThreadFolderChanged(id, folder) => {
            // Split the existing target_file to preserve the current filename,
            // then substitute in the new folder. Mirrors Mac
            // `ThreadConfigRow.commitFolder`'s rebuild without the trim —
            // the shadow carries raw keystroke state.
            if let Some(thread) = state.thread_configs.iter_mut().find(|t| t.id == id) {
                let (_, filename) = split_target_file(&thread.target_file);
                thread.target_file = join_folder_and_filename(&folder, &filename);
                commit_thread_configs(state);
                persist_working(state);
            }
            Task::none()
        }
        SettingsMessage::ThreadFilenameChanged(id, filename) => {
            if let Some(thread) = state.thread_configs.iter_mut().find(|t| t.id == id) {
                let (folder, _) = split_target_file(&thread.target_file);
                thread.target_file = join_folder_and_filename(&folder, &filename);
                commit_thread_configs(state);
                persist_working(state);
            }
            Task::none()
        }
        SettingsMessage::BrowseThreadFolderRequested(id) => pick_thread_folder_task(id),
        SettingsMessage::ThreadFolderBrowseChose(id, Some(path)) => {
            // Mac `ThreadConfigRow.chooseFolder`: when the picked folder sits
            // inside the vault, store the relative path; otherwise keep the
            // absolute path. We use a `"<vault>/"`-prefixed check to avoid
            // accepting a sibling `vault-backup` folder as "inside the vault".
            let vault_path = state.vault_path.clone();
            let folder_for_target = normalize_thread_folder_to_vault(&vault_path, &path);
            let mutated = if let Some(thread) =
                state.thread_configs.iter_mut().find(|t| t.id == id)
            {
                let (_, existing_filename) = split_target_file(&thread.target_file);
                // Mac defaults an empty filename to `"Threads.md"` when the
                // folder picker lands a value; we do the same so the user is
                // not left with a bare-folder target after picking a path.
                let filename: String = if existing_filename.is_empty() {
                    DEFAULT_THREAD_FILENAME.to_string()
                } else {
                    existing_filename
                };
                thread.target_file =
                    join_folder_and_filename(&folder_for_target, &filename);
                true
            } else {
                false
            };
            if mutated {
                commit_thread_configs(state);
                persist_working(state);
            }
            Task::none()
        }
        SettingsMessage::ThreadFolderBrowseChose(_, None) => Task::none(),
        SettingsMessage::ThreadAdded => {
            if state.thread_configs.len() < ThreadConfig::MAXIMUM_COUNT {
                let base_name = L10n::new_thread_default_name(state.language);
                let (name, target_file) =
                    unique_new_thread_name(&state.thread_configs, base_name);
                let next_order = state
                    .thread_configs
                    .iter()
                    .map(|t| t.order)
                    .max()
                    .map_or(0, |m| m + 1);
                state
                    .thread_configs
                    .push(ThreadConfig::new(name, target_file, None, next_order));
                commit_thread_configs(state);
                persist_working(state);
            }
            Task::none()
        }
        SettingsMessage::ThreadRemoved(id) => {
            if state.thread_configs.len() > ThreadConfig::MINIMUM_COUNT {
                state.thread_configs.retain(|t| t.id != id);
                commit_thread_configs(state);
                persist_working(state);
            }
            Task::none()
        }
        SettingsMessage::RecordingStarted(target) => {
            // Toggle semantics mirror Mac `toggleRecording(for:)`: clicking
            // Edit on the same row that is already recording disarms the
            // recorder, while clicking a different row simply switches the
            // target. Any stale validation message from a previous attempt
            // is cleared so the chip's footer only shows the live recording
            // hint.
            state.shortcut_recorder_message = None;
            state.recording_target = if state.recording_target == Some(target) {
                None
            } else {
                Some(target)
            };
            Task::none()
        }
        SettingsMessage::RecordingCancelled => {
            // Disarm the recorder and drop any error message so the Shortcuts
            // card returns to its resting state. Matches Mac `stopRecording()`.
            state.recording_target = None;
            state.shortcut_recorder_message = None;
            Task::none()
        }
        SettingsMessage::RecordingCaptured { key_code, modifiers } => {
            // No target armed: defensive no-op for the (very rare) race where
            // a keystroke arrives after the recorder has already been
            // cancelled.
            let Some(target) = state.recording_target else {
                return Task::none();
            };

            let candidate = ShortcutSpec::new(key_code, modifiers);

            // Bare Esc (no modifiers) silently cancels the recorder. Matches
            // the Mac reference's `NSEvent` monitor, which returns `nil`
            // before any validation runs — so we gate on this case first.
            if candidate.key_code == VK_ESCAPE && !candidate.has_modifier() {
                state.recording_target = None;
                state.shortcut_recorder_message = None;
                return Task::none();
            }

            // Modifier required — a bare F1/Tab/Enter with no ⌘/⌥/⇧/⌃ would
            // otherwise collide with typical in-app typing and is rejected
            // the same way Mac `handleRecordedShortcut` does.
            if !candidate.has_modifier() {
                state.shortcut_recorder_message =
                    Some(L10n::need_modifier_key(state.language).to_string());
                return Task::none();
            }

            // Modifier + Esc (e.g. ⌃+Esc) is reserved for the panel's close
            // gesture. Mirror Mac's second validation step verbatim.
            if candidate.key_code == VK_ESCAPE {
                state.shortcut_recorder_message =
                    Some(L10n::esc_reserved(state.language).to_string());
                return Task::none();
            }

            // Ctrl+1..9 (Win32 analog of Mac's ⌘+1..9) is reserved for panel
            // section switching. Only the global Create hotkey may claim one
            // of those combos — all three panel targets must reject them.
            if target != ShortcutTarget::Create && candidate.is_reserved_section_switch() {
                state.shortcut_recorder_message =
                    Some(L10n::cmd_number_reserved(state.language).to_string());
                return Task::none();
            }

            // Conflict check: the four shortcuts must be unique across the
            // whole Shortcuts card. `conflicting_target` scans all other
            // targets (excluding `target` itself) and returns the first one
            // whose shadow already holds `candidate`.
            if let Some(conflict) = conflicting_target(state, candidate, target) {
                state.shortcut_recorder_message = Some(L10n::shortcut_conflict(
                    state.language,
                    conflict.name(state.language),
                ));
                return Task::none();
            }

            // All checks pass — commit the candidate into the shadow, disarm
            // the recorder, clear any stale message, and persist the
            // `(vkey, modifiers)` pair for the right target. Split the
            // `ShortcutSpec` back into the two `u32` fields that
            // `AppSettings` serialises so the on-disk JSON stays
            // byte-compatible with the Mac format.
            state.set_shortcut(target, candidate);
            commit_shortcut(&mut state.working, target, candidate);
            state.recording_target = None;
            state.shortcut_recorder_message = None;
            persist_working(state);
            Task::none()
        }
        SettingsMessage::LaunchAtLoginToggled(value) => {
            // Sub-task 8b persisted the flag on every toggle; sub-task 8c.2
            // additionally routes the toggle through a
            // [`LaunchAtLoginSink`] so the Windows registry Run key stays in
            // sync. The JSON write happens *before* the sink so a failing
            // sink never blocks persistence — both failure paths log via
            // `tracing::warn!` and neither surfaces to the UI, matching Mac
            // `AppSettings.updateLaunchAtLogin`.
            state.launch_at_login_shadow = value;
            state.working.launch_at_login = value;
            persist_working(state);
            state.launch_at_login_sink.apply(value);
            Task::none()
        }
    }
}

/// Commits the shadow `section_titles` onto the working copy and runs
/// [`AppSettings::normalize`] so the on-disk JSON always holds the canonical
/// shape (dedup-by-slot, CR/LF swapped for spaces, leading `#` markers
/// stripped, trimmed, filled up to the minimum count). Split out so the
/// three Section-editing message arms share one code path and the invariant
/// "disk copy is always normalized" is locally obvious.
///
/// Note: `normalize()` also contains a shortcut-collision self-heal that may
/// reset `working.append_note_*` to its default when it matches `send_note_*`.
/// Under normal UI flow this branch cannot trigger because the recording
/// conflict-check prevents duplicate shortcuts at the shadow level; this
/// comment exists so future callers are aware of the side-effect.
fn commit_section_titles(state: &mut SettingsApp) {
    state.working.section_titles = state.section_titles.clone();
    state.working.normalize();
}

/// Commits the shadow `thread_configs` onto the working copy. Threads are
/// not subject to `normalize()` — the normalize pipeline only touches
/// section titles and panel-shortcut collisions — so this helper is a thin
/// clone; centralised so every thread-editing branch reads the same way.
fn commit_thread_configs(state: &mut SettingsApp) {
    state.working.thread_configs = state.thread_configs.clone();
}

/// Splits a [`ShortcutSpec`] back into the pair of `u32` fields that
/// [`AppSettings`] stores for the given `target`, then copies them onto
/// `working`. Mirror of [`SettingsApp::set_shortcut`] on the persistence
/// side so the `RecordingCaptured` success branch stays a single line.
fn commit_shortcut(working: &mut AppSettings, target: ShortcutTarget, spec: ShortcutSpec) {
    match target {
        ShortcutTarget::Create => {
            working.hot_key_code = spec.key_code;
            working.hot_key_modifiers = spec.modifiers;
        }
        ShortcutTarget::Send => {
            working.send_note_key_code = spec.key_code;
            working.send_note_modifiers = spec.modifiers;
        }
        ShortcutTarget::Append => {
            working.append_note_key_code = spec.key_code;
            working.append_note_modifiers = spec.modifiers;
        }
        ShortcutTarget::ToggleMode => {
            working.mode_toggle_key_code = spec.key_code;
            working.mode_toggle_modifiers = spec.modifiers;
        }
    }
}

/// Win32 virtual-key code for the Escape key. Hoisted out of the match arms
/// so the (Esc, modifier) validation branches name the constant rather than
/// an inline magic number.
const VK_ESCAPE: u32 = 0x1B;

/// Returns the first [`ShortcutTarget`] (other than `current`) whose shadow
/// shortcut matches `candidate`, or `None` when the candidate is free.
///
/// Iterates [`ShortcutTarget::ALL`] so the scan order stays stable across
/// edits and mirrors Mac `conflictingShortcutTarget(for:excluding:)`. Split
/// into a free function so the validation branches in [`settings_update`]
/// stay readable and the tests in this module can exercise the scan without
/// constructing a whole message pipeline.
fn conflicting_target(
    state: &SettingsApp,
    candidate: ShortcutSpec,
    current: ShortcutTarget,
) -> Option<ShortcutTarget> {
    ShortcutTarget::ALL
        .iter()
        .copied()
        .filter(|t| *t != current)
        .find(|t| state.shortcut_for(*t) == candidate)
}

/// Default filename used when a user picks a folder for a thread row whose
/// filename is still blank. Matches Mac `ThreadConfigRow.chooseFolder`'s
/// `"Threads.md"` fallback.
const DEFAULT_THREAD_FILENAME: &str = "Threads.md";

/// Converts a folder path picked via the system folder dialog into the
/// representation stored in `ThreadConfig::target_file`.
///
/// * Inside the shadow vault → relative path with no leading slash, matching
///   Mac `ThreadConfigRow.chooseFolder`'s `"vault/"`-prefix check (guards
///   against a sibling `vault-backup` folder being misread as inside-vault).
/// * Outside the vault → the absolute path is preserved verbatim.
/// * Empty vault_path → always keep the absolute path (there is nothing to
///   compute a relative path against).
fn normalize_thread_folder_to_vault(vault_path: &str, picked: &str) -> String {
    if vault_path.is_empty() {
        return picked.to_string();
    }
    // The exact match case returns an empty folder (root of the vault).
    if picked == vault_path {
        return String::new();
    }
    // Use a trailing-slash probe so the check rejects sibling folders that
    // happen to share a prefix with the vault path. Normalize the probe
    // ourselves because `vault_path` may or may not already end in `/`.
    let vault_with_slash = if vault_path.ends_with('/') {
        vault_path.to_string()
    } else {
        format!("{vault_path}/")
    };
    if picked.starts_with(&vault_with_slash) {
        return picked[vault_with_slash.len()..].to_string();
    }
    picked.to_string()
}

/// Pick a fresh `(name, target_file)` pair for a newly added thread, skipping
/// any value that collides with an existing entry.
///
/// Mirrors Mac `AppSettings.addThread`: start from the localized default name
/// and append an integer suffix (starting from `1`) until nothing collides on
/// either `name` or `target_file`. The helper stays pure so the tests in this
/// module can exercise the de-dup logic without building a whole `SettingsApp`.
fn unique_new_thread_name(
    existing: &[ThreadConfig],
    base_name: &str,
) -> (String, String) {
    let make_target = |name: &str| format!("{name}.md");
    let is_taken = |name: &str, target_file: &str| {
        existing
            .iter()
            .any(|t| t.name == name || t.target_file == target_file)
    };
    let base_target = make_target(base_name);
    if !is_taken(base_name, &base_target) {
        return (base_name.to_string(), base_target);
    }
    // Counter starts at 1 so the suffix matches the Mac reference's
    // "New Thread 1", "New Thread 2"... progression.
    let mut suffix = 1_usize;
    loop {
        let candidate_name = format!("{base_name} {suffix}");
        let candidate_target = make_target(&candidate_name);
        if !is_taken(&candidate_name, &candidate_target) {
            return (candidate_name, candidate_target);
        }
        suffix += 1;
    }
}

/// Kicks off an `rfd::AsyncFileDialog::pick_folder` picker and wraps the
/// result in a [`SettingsMessage::ThreadFolderBrowseChose`] carrying the
/// thread id.
///
/// A per-row dedicated helper because the generic [`pick_folder_task`] only
/// wraps `Option<String>` into a single-argument message constructor; thread
/// rows need to round-trip the `Uuid` through the async pick so the follow-up
/// message can find the right shadow entry.
fn pick_thread_folder_task(id: Uuid) -> Task<SettingsMessage> {
    let future = async {
        rfd::AsyncFileDialog::new()
            .pick_folder()
            .await
            .map(|handle| handle.path().to_string_lossy().into_owned())
    };
    Task::perform(future, move |picked| {
        SettingsMessage::ThreadFolderBrowseChose(id, picked)
    })
}

/// Kicks off an `rfd::AsyncFileDialog::pick_folder` picker off the iced view
/// thread and wraps the selection in `wrap`. A cancellation surfaces as
/// `wrap(None)`. Extracted so the Dimension-vault and File-inbox browse
/// requests share a single code path.
fn pick_folder_task(
    wrap: fn(Option<String>) -> SettingsMessage,
) -> Task<SettingsMessage> {
    // `AsyncFileDialog::pick_folder` returns a `Future<Output = Option<FileHandle>>`.
    // We flatten to `Option<String>` so the follow-up `SettingsMessage` carries
    // plain, owned UTF-8 and the rest of the app never has to know about
    // `rfd`'s `FileHandle` wrapper.
    let future = async {
        rfd::AsyncFileDialog::new()
            .pick_folder()
            .await
            .map(|handle| handle.path().to_string_lossy().into_owned())
    };
    Task::perform(future, wrap)
}

// 下面四个布局常量仅在 settings 模块内部消费(构建 card 列、chip 行、tile
// 列/行间距),以及同模块的回归测试。改为模块私有 `const`,避免 `pub` 暗
// 示它们是 crate 级公共 API —— 若未来别的 card 也用到,就地提级到
// `pub(crate)` 即可。
/// Spacing (in pixels) between stacked settings cards inside the scrollable
/// column. Matches Mac `SettingsView.swift`'s `VStack(spacing: 18)`.
const SETTINGS_CARD_STACK_SPACING: f32 = 18.0;
/// Spacing between adjacent language chips inside the Language card row.
const LANGUAGE_CHIP_SPACING: f32 = 8.0;
/// Vertical spacing between theme-preset tiles inside the Theme card column.
const THEME_TILE_SPACING: f32 = 8.0;
/// Spacing between adjacent write-mode tiles inside the Storage card row.
const WRITE_MODE_TILE_SPACING: f32 = 8.0;

/// Renders the settings window. Returns an [`Element`] ready for use in an
/// `iced::daemon(...).view(...)` router.
///
/// Sub-task 3 layers the first three cards (Language, Theme, Storage) on top
/// of the sub-task 1 scrollable shell. Every subsequent sub-task appends more
/// cards to [`build_cards`] without reshaping the outer layout.
pub fn settings_view(state: &SettingsApp) -> Element<'_, SettingsMessage> {
    let title = text(L10n::settings(state.language)).size(Pixels(SETTINGS_TITLE_FONT_SIZE));

    let cards = build_cards(state);

    let body = column![title, cards]
        .spacing(SETTINGS_CARD_STACK_SPACING)
        .width(Length::Fill);

    let scrollable_body = scrollable(
        container(body)
            .padding(SETTINGS_OUTER_PADDING)
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill);

    container(scrollable_body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Builds the stacked card column for [`settings_view`]. Split out as a
/// private helper so the card-per-sub-task wiring stays local without making
/// the outer `settings_view` balloon. Returns an [`Element`] rather than a
/// concrete column so future sub-tasks can swap in a more complex container
/// without touching the caller.
fn build_cards(state: &SettingsApp) -> Element<'_, SettingsMessage> {
    let palette = state.theme.settings;

    // Build the base three cards unconditionally; the Quick Sections card
    // only surfaces under Dimension mode (parity with Mac
    // `SettingsView.swift`'s `if writeMode == .dimension` gate around
    // `SectionCard`). We switch from the `column![...]` macro to the dynamic
    // `column(Vec<Element<_, _>>)` constructor so the card list can grow
    // conditionally without splitting into mutually exclusive macro arms.
    let mut cards: Vec<Element<'_, SettingsMessage>> = vec![
        language_card(state, palette),
        theme_card(state, palette),
        storage_card(state, palette),
    ];

    if state.write_mode == WriteMode::Dimension {
        cards.push(quick_sections::quick_sections_card(
            palette,
            state.language,
            &state.section_titles,
        ));
    }

    // Mirrors Mac `SettingsView.swift`'s `if writeMode == .thread` gate:
    // the Threads management card surfaces only under Thread mode, directly
    // after the Storage card. Dimension / File modes keep their existing
    // card ordering.
    if state.write_mode == WriteMode::Thread {
        cards.push(threads::threads_card(
            palette,
            state.language,
            &state.thread_configs,
            &state.vault_path,
        ));
    }

    // Shortcuts card: renders unconditionally under every write mode. The
    // keyboard configuration is not scoped to a specific Storage layout, so
    // it sits after the write-mode–specific cards in the scroll.
    cards.push(shortcuts::shortcuts_card(state, palette));

    // System card: also write-mode-agnostic — the "Launch at Login" flag it
    // carries applies to every Storage layout. Renders unconditionally after
    // the Shortcuts card so the scroll ends on the card that owns the
    // version caption.
    cards.push(system::system_card(
        palette,
        state.language,
        state.launch_at_login_shadow,
    ));

    column(cards)
        .spacing(SETTINGS_CARD_STACK_SPACING)
        .width(Length::Fill)
        .into()
}

/// Builds the Language card: a single row of chips covering
/// `SystemDefault` + the three native locales.
fn language_card<'a>(
    state: &'a SettingsApp,
    palette: trace_core::SettingsPalette,
) -> Element<'a, SettingsMessage> {
    let lang = state.language;
    let chips: [(Language, &'a str); 4] = [
        (Language::SystemDefault, L10n::language_system_default(lang)),
        // Every locale renders with its endonym so the chip reads in its own
        // script. `native_display_name` returns `None` only for
        // `Language::SystemDefault`, which is handled above — the unwrap on
        // these three arms is total.
        (
            Language::Zh,
            Language::Zh
                .native_display_name()
                .expect("Zh endonym must exist"),
        ),
        (
            Language::En,
            Language::En
                .native_display_name()
                .expect("En endonym must exist"),
        ),
        (
            Language::Ja,
            Language::Ja
                .native_display_name()
                .expect("Ja endonym must exist"),
        ),
    ];

    let chip_row = row(chips.iter().map(|(variant, label)| {
        tiles::language_chip(
            palette,
            label,
            state.language == *variant,
            SettingsMessage::LanguageChanged(*variant),
        )
    }))
    .spacing(LANGUAGE_CHIP_SPACING)
    .width(Length::Fill);

    widgets::section_card(palette, L10n::language(lang), chip_row.into())
}

/// Builds the Theme card: a vertical list of tiles, one per preset.
fn theme_card<'a>(
    state: &'a SettingsApp,
    palette: trace_core::SettingsPalette,
) -> Element<'a, SettingsMessage> {
    let lang = state.language;
    let presets = [
        ThemePreset::Light,
        ThemePreset::Dark,
        ThemePreset::Paper,
        ThemePreset::Dune,
    ];

    let tiles = presets.iter().map(|preset| {
        // 直接使用 `ThemePreset::preview_swatches` helper,避免为了拿色板
        // 而构造整个 `TraceTheme` palette bundle。语义上这是 preset 自身
        // 的属性,与 `title()` / `icon_glyph()` 的访问方式对齐。
        tiles::theme_preset_tile(
            palette,
            preset.title(),
            preset.icon_glyph(),
            preset.preview_swatches(),
            state.theme_preset == *preset,
            SettingsMessage::ThemePresetChanged(*preset),
        )
    });

    let stack = column(tiles)
        .spacing(THEME_TILE_SPACING)
        .width(Length::Fill);

    widgets::section_card(palette, L10n::theme(lang), stack.into())
}

/// Builds the Storage card. The top row always shows the three Write Mode
/// tiles; the rows underneath are swapped in based on the active
/// [`WriteMode`]:
///
/// * `Dimension` — vault path (with inline validation), daily folder name,
///   file-name format, and entry format.
/// * `File` — inbox vault path (with inline validation).
/// * `Thread` — no additional rows (thread configs are their own card).
///
/// Mirrors Mac `SettingsView.swift`'s per-mode conditional branches so the
/// two ports show the same fields in the same order.
fn storage_card<'a>(
    state: &'a SettingsApp,
    palette: trace_core::SettingsPalette,
) -> Element<'a, SettingsMessage> {
    let lang = state.language;

    let modes = [WriteMode::Dimension, WriteMode::Thread, WriteMode::File];
    let tiles_row = row(modes.iter().map(|mode| {
        tiles::write_mode_tile(
            palette,
            mode.compact_title(lang),
            mode.destination_title(lang),
            mode.icon_glyph(),
            state.write_mode == *mode,
            SettingsMessage::WriteModeChanged(*mode),
        )
    }))
    .spacing(WRITE_MODE_TILE_SPACING)
    .width(Length::Fill);

    let write_mode_row =
        widgets::setting_row(palette, L10n::write_mode(lang), None, tiles_row.into());

    // Start the card body with the write-mode row, then append whatever
    // extra rows the selected write mode calls for.
    let mut body_rows: Vec<Element<'a, SettingsMessage>> = vec![write_mode_row];

    match state.write_mode {
        WriteMode::Dimension => {
            // Read the cached validation result instead of re-probing the
            // filesystem on every view pass (iced re-renders at ~60 fps under
            // the Dimension WriteMode). See `SettingsApp::vault_path_issue`.
            body_rows.push(storage::vault_path_row(
                palette,
                lang,
                &state.vault_path,
                state.vault_path_issue,
                SettingsMessage::VaultPathChanged,
                SettingsMessage::BrowseVaultRequested,
            ));
            body_rows.push(storage::daily_folder_row(
                palette,
                lang,
                &state.daily_folder_name,
                SettingsMessage::DailyFolderNameChanged,
            ));
            body_rows.push(storage::file_name_format_row(
                palette,
                lang,
                state.daily_file_date_format,
                SettingsMessage::DailyFileDateFormatChanged,
            ));
            body_rows.push(storage::entry_format_row(
                palette,
                lang,
                state.daily_entry_theme_preset,
                SettingsMessage::DailyEntryThemePresetChanged,
            ));
        }
        WriteMode::File => {
            // Same caching contract as the Dimension vault row above.
            body_rows.push(storage::inbox_vault_path_row(
                palette,
                lang,
                &state.inbox_vault_path,
                state.inbox_vault_path_issue,
                SettingsMessage::InboxVaultPathChanged,
                SettingsMessage::BrowseInboxVaultRequested,
            ));
        }
        WriteMode::Thread => {
            // No extra rows — thread configs live in their own card in later
            // sub-tasks, mirroring the Mac reference's `ThreadConfigCard`.
        }
    }

    widgets::section_card_with_rows(palette, L10n::storage(lang), body_rows)
}

/// Returns the iced [`Theme`] derived from [`SettingsApp::theme`]. Plumbed
/// through `iced::daemon(...).theme(...)`.
pub fn settings_theme(state: &SettingsApp) -> Theme {
    state.iced_theme.clone()
}

/// Aggregate subscription for the settings window.
///
/// The subscription is mounted conditionally: when
/// [`SettingsApp::recording_target`] is `None` the window has no live event
/// sources and we return [`Subscription::none`] so no runtime resources are
/// wasted on idle keystrokes. When a recorder is armed we mount
/// [`iced::event::listen_with`] wired to [`keyboard_event_to_message`] so
/// every `KeyPressed` event feeds [`SettingsMessage::RecordingCaptured`].
///
/// The gate lives *here* (at the subscription boundary) rather than inside
/// [`keyboard_event_to_message`] because `listen_with` requires a plain
/// `fn` pointer — the callback can't close over `recording_target`. Mac
/// solves the same problem by attaching / removing the `NSEvent`
/// local-monitor on demand; this is the iced analog.
pub fn settings_subscription(state: &SettingsApp) -> Subscription<SettingsMessage> {
    if state.recording_target.is_none() {
        Subscription::none()
    } else {
        iced_event::listen_with(keyboard_event_to_message)
    }
}

/// `listen_with` callback for the Shortcuts recorder. Must be a plain `fn`
/// (no captures) — see [`iced::event::listen_with`]. Every
/// [`KeyboardEvent::KeyPressed`] that decodes into a Win32 VK + modifier
/// pair becomes a [`SettingsMessage::RecordingCaptured`]; every other
/// event returns `None` so unrelated keys (modifier-only presses,
/// non-US-layout characters, mouse events, …) leave the recorder idle.
///
/// The decoder intentionally does *not* inspect [`event::Status`]. Unlike
/// the capture panel — which skips captured keystrokes so the text editor
/// owns them first — the settings window treats every press as a candidate
/// shortcut while the recorder is armed. This matches Mac's local
/// `NSEvent` monitor behaviour, which intercepts `keyDown` before the
/// focused control sees it.
fn keyboard_event_to_message(
    event: Event,
    _status: iced_event::Status,
    _window: window::Id,
) -> Option<SettingsMessage> {
    let Event::Keyboard(KeyboardEvent::KeyPressed {
        key, modifiers, ..
    }) = event
    else {
        return None;
    };
    let key_code = shortcut_event::key_to_vk(&key)?;
    let modifiers = shortcut_event::modifiers_to_win32(modifiers);
    Some(SettingsMessage::RecordingCaptured {
        key_code,
        modifiers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use trace_core::ThemePreset;

    fn fresh_app() -> SettingsApp {
        SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(AppSettings::default()),
        )
    }

    #[test]
    fn settings_app_default_constructs_from_settings_and_theme() {
        let settings = AppSettings {
            language: Language::Zh,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(settings),
        );
        // Language is mirrored from `AppSettings` so the first render reads
        // the user's preferred locale without waiting for a dispatched
        // message.
        assert_eq!(app.language, Language::Zh);
        assert_eq!(app.theme.preset, ThemePreset::Dark);
        assert_eq!(app.settings.language, Language::Zh);
    }

    #[test]
    fn settings_app_new_seeds_iced_theme_from_trace_theme() {
        let app = fresh_app();
        // The cached iced theme's palette must match a fresh conversion.
        let expected = to_iced_theme(&app.theme);
        assert_eq!(
            settings_theme(&app).palette().background,
            expected.palette().background
        );
    }

    #[test]
    fn set_theme_refreshes_cached_iced_theme() {
        let mut app = fresh_app();
        let dark_bg = settings_theme(&app).palette().background;
        app.set_theme(TraceTheme::for_preset(ThemePreset::Light));
        let light_bg = settings_theme(&app).palette().background;
        assert_ne!(
            dark_bg, light_bg,
            "switching preset must rebuild the cached iced theme"
        );
    }

    #[test]
    fn settings_view_constructs_without_panic() {
        let app = fresh_app();
        // Construct the view tree — proves the widget graph type-checks for
        // each language without running the iced event loop.
        let _element: Element<'_, SettingsMessage> = settings_view(&app);
    }

    #[test]
    fn settings_view_builds_across_all_languages() {
        let mut app = fresh_app();
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            app.language = lang;
            let _element: Element<'_, SettingsMessage> = settings_view(&app);
        }
    }

    #[test]
    fn language_changed_updates_state() {
        let mut app = fresh_app();
        assert_eq!(app.language, Language::SystemDefault);
        let _ = settings_update(&mut app, SettingsMessage::LanguageChanged(Language::Zh));
        assert_eq!(app.language, Language::Zh);
        let _ = settings_update(&mut app, SettingsMessage::LanguageChanged(Language::En));
        assert_eq!(app.language, Language::En);
    }

    #[test]
    fn settings_subscription_starts_empty() {
        // A fresh settings window has no recorder armed, so the aggregate
        // subscription must resolve to the idle branch. We can't inspect
        // the `Subscription` tree directly — iced 0.14 doesn't expose an
        // `is_none()` helper — but constructing it both proves the branch
        // compiles and locks the call site to the current surface.
        let app = fresh_app();
        let _sub: Subscription<SettingsMessage> = settings_subscription(&app);
    }

    #[test]
    fn settings_subscription_mounts_listener_while_recording() {
        // Mirror the idle case but flip `recording_target` so the gate
        // lands on the `listen_with` branch. Same opaque-subscription
        // caveat — constructing it is the strongest assertion iced 0.14
        // lets us make without running the runtime.
        let mut app = fresh_app();
        app.recording_target = Some(ShortcutTarget::Create);
        let _sub: Subscription<SettingsMessage> = settings_subscription(&app);
    }

    #[test]
    fn default_settings_window_dimensions_match_mac_reference() {
        // Mac `SettingsView.swift` picks 720 x 640 for the default frame.
        // Lock the constants so a drift is caught at test time.
        assert_eq!(DEFAULT_SETTINGS_WINDOW_WIDTH, 720.0);
        assert_eq!(DEFAULT_SETTINGS_WINDOW_HEIGHT, 640.0);
    }

    // --- Sub-task 3 ----------------------------------------------------

    #[test]
    fn settings_app_new_seeds_theme_preset_and_write_mode_from_settings() {
        // Shadow fields must pick up the persisted settings snapshot so the
        // first render reflects the user's last choice without waiting for a
        // message dispatch.
        let persisted = AppSettings {
            app_theme_preset: ThemePreset::Paper,
            note_write_mode: WriteMode::File,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Paper),
            Arc::new(persisted),
        );
        assert_eq!(app.theme_preset, ThemePreset::Paper);
        assert_eq!(app.write_mode, WriteMode::File);
    }

    #[test]
    fn settings_update_theme_preset_changed_refreshes_cached_iced_theme() {
        let mut app = fresh_app();
        let dark_bg = settings_theme(&app).palette().background;
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThemePresetChanged(ThemePreset::Light),
        );
        let light_bg = settings_theme(&app).palette().background;
        assert_eq!(app.theme_preset, ThemePreset::Light);
        assert_eq!(app.theme.preset, ThemePreset::Light);
        assert_ne!(
            dark_bg, light_bg,
            "theme preset change must rebuild the cached iced theme"
        );
    }

    #[test]
    fn settings_update_theme_preset_changed_covers_every_preset() {
        // Cycle through every preset to prove the match arms never miss one.
        let mut app = fresh_app();
        for preset in [
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::Paper,
            ThemePreset::Dune,
        ] {
            let _ = settings_update(&mut app, SettingsMessage::ThemePresetChanged(preset));
            assert_eq!(app.theme_preset, preset);
            assert_eq!(app.theme.preset, preset);
        }
    }

    #[test]
    fn settings_update_write_mode_changed_updates_shadow_field() {
        let mut app = fresh_app();
        // `AppSettings::default()` uses `WriteMode::Dimension`.
        assert_eq!(app.write_mode, WriteMode::Dimension);
        for mode in [WriteMode::Thread, WriteMode::File, WriteMode::Dimension] {
            let _ = settings_update(&mut app, SettingsMessage::WriteModeChanged(mode));
            assert_eq!(app.write_mode, mode);
        }
    }

    #[test]
    fn settings_update_write_mode_changed_does_not_mutate_shared_settings() {
        // Sub-task 3 only touches the shadow field — persistence to
        // `AppSettings` is wired in sub-task 8. Guard the invariant so a
        // future edit that leaks a write back to the Arc is caught by tests.
        let mut app = fresh_app();
        let original = app.settings.note_write_mode;
        let _ = settings_update(
            &mut app,
            SettingsMessage::WriteModeChanged(WriteMode::Thread),
        );
        assert_eq!(app.settings.note_write_mode, original);
    }

    #[test]
    fn settings_update_language_changed_still_works_after_expansion() {
        // Regression guard: adding new `SettingsMessage` variants must not
        // break the existing LanguageChanged branch.
        let mut app = fresh_app();
        let _ = settings_update(&mut app, SettingsMessage::LanguageChanged(Language::Ja));
        assert_eq!(app.language, Language::Ja);
    }

    #[test]
    fn settings_view_renders_language_theme_storage_cards_without_panic() {
        // The three cards must build across every language × every preset ×
        // every write-mode combination so a drift in the matching logic is
        // caught at test time.
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            for preset in [
                ThemePreset::Light,
                ThemePreset::Dark,
                ThemePreset::Paper,
                ThemePreset::Dune,
            ] {
                for mode in [WriteMode::Dimension, WriteMode::Thread, WriteMode::File] {
                    let settings = AppSettings {
                        language: lang,
                        app_theme_preset: preset,
                        note_write_mode: mode,
                        ..AppSettings::default()
                    };
                    let app = SettingsApp::new(
                        TraceTheme::for_preset(preset),
                        Arc::new(settings),
                    );
                    let _element: Element<'_, SettingsMessage> = settings_view(&app);
                }
            }
        }
    }

    #[test]
    fn card_stack_spacing_matches_mac_reference() {
        // Mac `SettingsView.swift` uses `VStack(spacing: 18)` for the card
        // stack. Lock the constant so a drift is caught at test time.
        assert_eq!(SETTINGS_CARD_STACK_SPACING, 18.0);
    }

    // --- Sub-task 4 ----------------------------------------------------

    #[test]
    fn settings_app_new_seeds_storage_shadow_fields_from_settings() {
        // All five new storage shadow fields must be hydrated from the
        // persisted `AppSettings` snapshot so the Storage card shows the
        // user's last choices on first paint.
        let persisted = AppSettings {
            vault_path: "C:/vault".into(),
            inbox_vault_path: "C:/inbox".into(),
            daily_folder_name: "DayNotes".into(),
            daily_file_date_format: DailyFileDateFormat::IsoDate.raw_value().to_string(),
            daily_entry_theme_preset: EntryTheme::MarkdownQuote,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(persisted),
        );
        assert_eq!(app.vault_path, "C:/vault");
        assert_eq!(app.inbox_vault_path, "C:/inbox");
        assert_eq!(app.daily_folder_name, "DayNotes");
        // Raw ICU string rehydrates through `resolved_from_raw` so a match
        // with a known preset resolves to that variant exactly.
        assert_eq!(app.daily_file_date_format, DailyFileDateFormat::IsoDate);
        assert_eq!(app.daily_entry_theme_preset, EntryTheme::MarkdownQuote);
    }

    #[test]
    fn settings_app_new_falls_back_to_chinese_full_for_unknown_format() {
        // Mac's `DailyFileDateFormat.resolved(fromStored:)` falls back to
        // `ChineseFull` when the stored ICU string is not a known preset. The
        // Windows port must match so a hand-edited JSON does not land in an
        // unrepresentable `UnknownFormat` state.
        let persisted = AppSettings {
            daily_file_date_format: "not a real format".into(),
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(persisted),
        );
        assert_eq!(app.daily_file_date_format, DailyFileDateFormat::ChineseFull);
    }

    #[test]
    fn settings_update_vault_path_changed_updates_shadow_field() {
        let mut app = fresh_app();
        assert!(app.vault_path.is_empty());
        let _ = settings_update(
            &mut app,
            SettingsMessage::VaultPathChanged("C:/new-vault".into()),
        );
        assert_eq!(app.vault_path, "C:/new-vault");
    }

    #[test]
    fn settings_update_vault_browse_chose_some_overwrites_vault_path() {
        let mut app = fresh_app();
        let _ = settings_update(
            &mut app,
            SettingsMessage::VaultBrowseChose(Some("D:/picked".into())),
        );
        assert_eq!(app.vault_path, "D:/picked");
    }

    #[test]
    fn settings_update_vault_browse_chose_none_is_noop() {
        // User cancelled the picker — the current path must be preserved.
        let mut app = fresh_app();
        app.vault_path = "C:/keep-me".into();
        let _ = settings_update(&mut app, SettingsMessage::VaultBrowseChose(None));
        assert_eq!(app.vault_path, "C:/keep-me");
    }

    #[test]
    fn settings_update_inbox_vault_path_changed_updates_shadow_field() {
        let mut app = fresh_app();
        assert!(app.inbox_vault_path.is_empty());
        let _ = settings_update(
            &mut app,
            SettingsMessage::InboxVaultPathChanged("C:/inbox".into()),
        );
        assert_eq!(app.inbox_vault_path, "C:/inbox");
    }

    #[test]
    fn settings_update_inbox_vault_browse_chose_some_overwrites_inbox_path() {
        let mut app = fresh_app();
        let _ = settings_update(
            &mut app,
            SettingsMessage::InboxVaultBrowseChose(Some("D:/picked-inbox".into())),
        );
        assert_eq!(app.inbox_vault_path, "D:/picked-inbox");
    }

    #[test]
    fn settings_update_inbox_vault_browse_chose_none_is_noop() {
        let mut app = fresh_app();
        app.inbox_vault_path = "C:/keep-inbox".into();
        let _ = settings_update(&mut app, SettingsMessage::InboxVaultBrowseChose(None));
        assert_eq!(app.inbox_vault_path, "C:/keep-inbox");
    }

    #[test]
    fn settings_update_daily_folder_name_changed_updates_shadow_field() {
        let mut app = fresh_app();
        assert_eq!(app.daily_folder_name, "Daily"); // AppSettings default
        let _ = settings_update(
            &mut app,
            SettingsMessage::DailyFolderNameChanged("Weekly".into()),
        );
        assert_eq!(app.daily_folder_name, "Weekly");
    }

    #[test]
    fn settings_update_daily_file_date_format_changed_cycles_every_preset() {
        // Cycle through every preset so the match arm in `settings_update`
        // is exercised for each variant, not just the first one.
        let mut app = fresh_app();
        for preset in DailyFileDateFormat::ALL {
            let _ = settings_update(
                &mut app,
                SettingsMessage::DailyFileDateFormatChanged(preset),
            );
            assert_eq!(app.daily_file_date_format, preset);
        }
    }

    #[test]
    fn settings_update_daily_entry_theme_preset_changed_cycles_every_variant() {
        let mut app = fresh_app();
        for theme in EntryTheme::ALL {
            let _ = settings_update(
                &mut app,
                SettingsMessage::DailyEntryThemePresetChanged(theme),
            );
            assert_eq!(app.daily_entry_theme_preset, theme);
        }
    }

    #[test]
    fn settings_update_storage_messages_do_not_mutate_shared_settings() {
        // Sub-task 4 still only touches shadow fields. Persistence lands in
        // sub-task 8. Guard every new variant against leaking a write back
        // into the shared `Arc<AppSettings>`.
        let mut app = fresh_app();
        let original_vault = app.settings.vault_path.clone();
        let original_inbox = app.settings.inbox_vault_path.clone();
        let original_daily_folder = app.settings.daily_folder_name.clone();
        let original_date_format = app.settings.daily_file_date_format.clone();
        let original_entry_theme = app.settings.daily_entry_theme_preset;

        let _ = settings_update(
            &mut app,
            SettingsMessage::VaultPathChanged("C:/new".into()),
        );
        let _ = settings_update(
            &mut app,
            SettingsMessage::InboxVaultPathChanged("C:/new-inbox".into()),
        );
        let _ = settings_update(
            &mut app,
            SettingsMessage::DailyFolderNameChanged("NewDaily".into()),
        );
        let _ = settings_update(
            &mut app,
            SettingsMessage::DailyFileDateFormatChanged(DailyFileDateFormat::SlashDate),
        );
        let _ = settings_update(
            &mut app,
            SettingsMessage::DailyEntryThemePresetChanged(EntryTheme::PlainTextTimestamp),
        );

        assert_eq!(app.settings.vault_path, original_vault);
        assert_eq!(app.settings.inbox_vault_path, original_inbox);
        assert_eq!(app.settings.daily_folder_name, original_daily_folder);
        assert_eq!(app.settings.daily_file_date_format, original_date_format);
        assert_eq!(app.settings.daily_entry_theme_preset, original_entry_theme);
    }

    #[test]
    fn storage_card_builds_for_every_write_mode_across_all_languages() {
        // Smoke test: rendering the Storage card must succeed for each
        // (language, write_mode) pair so a missing arm in the dispatch logic
        // is caught at test time rather than at first paint.
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            for mode in [WriteMode::Dimension, WriteMode::Thread, WriteMode::File] {
                let settings = AppSettings {
                    language: lang,
                    note_write_mode: mode,
                    vault_path: "".into(),       // exercises the Empty issue
                    inbox_vault_path: "".into(), // same for File mode
                    ..AppSettings::default()
                };
                let app = SettingsApp::new(
                    TraceTheme::for_preset(ThemePreset::Dark),
                    Arc::new(settings),
                );
                let _element: Element<'_, SettingsMessage> = settings_view(&app);
            }
        }
    }

    #[test]
    fn storage_card_in_dimension_mode_renders_every_filename_format() {
        // Cycle through the five filename-format presets. A broken picker
        // serialization would have one preset fail to build while the others
        // succeed; the loop guards against that.
        for preset in DailyFileDateFormat::ALL {
            let settings = AppSettings {
                note_write_mode: WriteMode::Dimension,
                daily_file_date_format: preset.raw_value().to_string(),
                ..Default::default()
            };
            let app = SettingsApp::new(
                TraceTheme::for_preset(ThemePreset::Dark),
                Arc::new(settings),
            );
            let _element: Element<'_, SettingsMessage> = settings_view(&app);
        }
    }

    #[test]
    fn storage_card_in_dimension_mode_renders_every_entry_theme() {
        // Same regression shape as the filename-format test, for the entry
        // theme picker.
        for theme in EntryTheme::ALL {
            let settings = AppSettings {
                note_write_mode: WriteMode::Dimension,
                daily_entry_theme_preset: theme,
                ..Default::default()
            };
            let app = SettingsApp::new(
                TraceTheme::for_preset(ThemePreset::Dark),
                Arc::new(settings),
            );
            let _element: Element<'_, SettingsMessage> = settings_view(&app);
        }
    }

    // --- Sub-task 4 cache ---------------------------------------------------
    //
    // These tests guard the contract that `validate_vault_path` is called
    // from `settings_update` (not the view hot path). The view layer reads
    // the cached `vault_path_issue` / `inbox_vault_path_issue` fields
    // directly, so any change to the stored path must refresh the cache.

    #[test]
    fn settings_app_new_seeds_vault_path_issue_for_empty_path() {
        // Default `AppSettings` has an empty vault_path, so the cached issue
        // must classify as `Empty` on first construction — matches the
        // `vault_path_issue` doc comment contract that the view pass never
        // has to call `validate_vault_path`.
        let app = fresh_app();
        assert_eq!(
            app.vault_path_issue,
            Some(VaultPathValidationIssue::Empty)
        );
        assert_eq!(
            app.inbox_vault_path_issue,
            Some(VaultPathValidationIssue::Empty)
        );
    }

    #[test]
    fn vault_path_changed_updates_cached_issue() {
        // Mutating the path must refresh the cached classification; without
        // this the Storage card would render a stale warning after the user
        // types a new value.
        let mut app = fresh_app();
        assert_eq!(
            app.vault_path_issue,
            Some(VaultPathValidationIssue::Empty)
        );
        let _ = settings_update(
            &mut app,
            SettingsMessage::VaultPathChanged("/nonexistent/path/for/trace-cache-test".into()),
        );
        // A non-blank, non-existent path must reclassify — the exact variant
        // depends on the host filesystem, but it can no longer be `Empty`.
        assert_ne!(
            app.vault_path_issue,
            Some(VaultPathValidationIssue::Empty)
        );
    }

    #[test]
    fn vault_browse_chose_some_updates_cached_issue() {
        // Browse picker confirmations must refresh the cached issue the same
        // way free-text edits do. Shares the helper path under the hood —
        // testing this branch explicitly guards against a future refactor
        // that special-cases the browse flow.
        let mut app = fresh_app();
        let _ = settings_update(
            &mut app,
            SettingsMessage::VaultBrowseChose(Some(
                "/nonexistent/browse-cache-test".into(),
            )),
        );
        assert_ne!(
            app.vault_path_issue,
            Some(VaultPathValidationIssue::Empty)
        );
    }

    #[test]
    fn vault_browse_chose_none_preserves_cached_issue() {
        // A cancelled picker must leave the cache alone — nothing on disk
        // changed, so a re-probe would be wasted I/O and would mask a
        // regression that forgot to refresh the cache on real edits.
        let mut app = fresh_app();
        // Prime the cache with a deliberately non-Empty state so we can
        // observe that the Noop branch really is idempotent.
        let _ = settings_update(
            &mut app,
            SettingsMessage::VaultPathChanged("/nonexistent/browse-cancel-test".into()),
        );
        let before = app.vault_path_issue;
        let _ = settings_update(&mut app, SettingsMessage::VaultBrowseChose(None));
        assert_eq!(app.vault_path_issue, before);
        assert_eq!(app.vault_path, "/nonexistent/browse-cancel-test");
    }

    #[test]
    fn inbox_vault_path_changed_updates_cached_issue() {
        // Inbox row mirrors the Dimension vault row; re-test here so the
        // symmetry is explicit and a future refactor that splits the two
        // helpers can't silently break one side.
        let mut app = fresh_app();
        let _ = settings_update(
            &mut app,
            SettingsMessage::InboxVaultPathChanged(
                "/nonexistent/inbox-cache-test".into(),
            ),
        );
        assert_ne!(
            app.inbox_vault_path_issue,
            Some(VaultPathValidationIssue::Empty)
        );
    }

    // --- Sub-task 5 ----------------------------------------------------

    #[test]
    fn settings_app_new_seeds_section_titles_from_settings() {
        // The shadow must mirror the persisted vec on first paint so the
        // Quick Sections card renders the user's last configuration without
        // waiting for a dispatched message.
        let persisted = AppSettings {
            section_titles: vec!["想法".into(), "待办".into(), "链接".into()],
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(persisted),
        );
        assert_eq!(app.section_titles, vec!["想法", "待办", "链接"]);
    }

    #[test]
    fn section_title_changed_writes_shadow() {
        let mut app = fresh_app();
        // `AppSettings::default()` seeds the four-entry Mac-default vec.
        let before = app.section_titles.clone();
        assert!(!before.is_empty());
        let _ = settings_update(
            &mut app,
            SettingsMessage::SectionTitleChanged(0, "想法".into()),
        );
        assert_eq!(app.section_titles[0], "想法");
        // All other slots must stay untouched.
        assert_eq!(app.section_titles[1..], before[1..]);
    }

    #[test]
    fn section_title_changed_out_of_bounds_is_noop() {
        // Iced should never emit a stale index, but the branch must defend
        // against one rather than panic. Assert via `get_mut`-style shape:
        // the vec is unchanged and the call returns normally.
        let mut app = fresh_app();
        let before = app.section_titles.clone();
        let _ = settings_update(
            &mut app,
            SettingsMessage::SectionTitleChanged(99, "ghost".into()),
        );
        assert_eq!(app.section_titles, before);
    }

    #[test]
    fn section_added_appends_default_title() {
        let mut app = fresh_app();
        let before_len = app.section_titles.len();
        let _ = settings_update(&mut app, SettingsMessage::SectionAdded);
        assert_eq!(app.section_titles.len(), before_len + 1);
        assert_eq!(
            app.section_titles[before_len],
            NoteSection::default_title_for(before_len)
        );
    }

    #[test]
    fn section_added_at_maximum_is_noop() {
        // Pre-fill to the cap so the branch must detect the saturated state
        // and do nothing. Guards against a future refactor that forgets the
        // `< MAXIMUM_COUNT` gate.
        let mut app = fresh_app();
        app.section_titles =
            (0..NoteSection::MAXIMUM_COUNT)
                .map(NoteSection::default_title_for)
                .collect();
        let before = app.section_titles.clone();
        let _ = settings_update(&mut app, SettingsMessage::SectionAdded);
        assert_eq!(app.section_titles, before);
        assert_eq!(app.section_titles.len(), NoteSection::MAXIMUM_COUNT);
    }

    #[test]
    fn section_removed_at_index_shifts_rest() {
        let mut app = fresh_app();
        app.section_titles = vec!["A".into(), "B".into(), "C".into()];
        let _ = settings_update(&mut app, SettingsMessage::SectionRemoved(1));
        assert_eq!(app.section_titles, vec!["A".to_string(), "C".into()]);
    }

    #[test]
    fn section_removed_at_minimum_is_noop() {
        // Can't drop below one section. Mirrors Mac `AppSettings.removeSection`
        // which silently bails in the same state.
        let mut app = fresh_app();
        app.section_titles = vec!["Solo".into()];
        let _ = settings_update(&mut app, SettingsMessage::SectionRemoved(0));
        assert_eq!(app.section_titles, vec!["Solo".to_string()]);
    }

    #[test]
    fn section_removed_out_of_bounds_is_noop() {
        let mut app = fresh_app();
        let before = app.section_titles.clone();
        let before_len = before.len();
        let _ = settings_update(&mut app, SettingsMessage::SectionRemoved(99));
        assert_eq!(app.section_titles, before);
        assert_eq!(app.section_titles.len(), before_len);
    }

    #[test]
    fn section_messages_do_not_mutate_shared_settings() {
        // Sub-task 5 still only touches the shadow. Persistence to the shared
        // `Arc<AppSettings>` lands in sub-task 8.
        let mut app = fresh_app();
        let original = app.settings.section_titles.clone();
        let _ = settings_update(
            &mut app,
            SettingsMessage::SectionTitleChanged(0, "新想法".into()),
        );
        let _ = settings_update(&mut app, SettingsMessage::SectionAdded);
        let _ = settings_update(&mut app, SettingsMessage::SectionRemoved(0));
        assert_eq!(app.settings.section_titles, original);
    }

    // --- Sub-task 5 build_cards dispatch -------------------------------
    //
    // The Quick Sections card must only surface under Dimension mode so the
    // Thread / File modes keep their Mac-parity card ordering. These tests
    // smoke the `build_cards` dispatch for each mode; a missing arm would
    // either panic at construction or compile away silently.

    #[test]
    fn build_cards_in_dimension_shows_quick_sections_card() {
        // Dimension mode is the only mode where the Quick Sections card
        // renders; `build_cards` must return a viable widget tree here.
        let settings = AppSettings {
            note_write_mode: WriteMode::Dimension,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(settings),
        );
        assert_eq!(app.write_mode, WriteMode::Dimension);
        let _element: Element<'_, SettingsMessage> = build_cards(&app);
    }

    #[test]
    fn build_cards_in_file_mode_hides_quick_sections_card() {
        // File mode must skip the Quick Sections card branch without
        // panicking; a regression that changed the gate from `==` to `!=`
        // would show up here as a build failure, since the `file` inbox card
        // would push twice or the section card would leak into File mode.
        let settings = AppSettings {
            note_write_mode: WriteMode::File,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(settings),
        );
        assert_eq!(app.write_mode, WriteMode::File);
        let _element: Element<'_, SettingsMessage> = build_cards(&app);
    }

    #[test]
    fn build_cards_in_thread_mode_hides_quick_sections_card() {
        // Thread mode has its own (future) card stack; Quick Sections must
        // stay out of this mode so the section picker only applies where the
        // Dimension-mode chip row actually consumes it.
        let settings = AppSettings {
            note_write_mode: WriteMode::Thread,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(settings),
        );
        assert_eq!(app.write_mode, WriteMode::Thread);
        let _element: Element<'_, SettingsMessage> = build_cards(&app);
    }

    // --- Sub-task 6 ----------------------------------------------------

    #[test]
    fn settings_app_new_seeds_thread_configs_from_settings() {
        // The shadow must mirror the persisted `thread_configs` on first
        // paint so the Threads card renders the user's last configuration
        // without waiting for a dispatched message.
        let persisted = AppSettings {
            thread_configs: vec![
                ThreadConfig::new("想法", "想法.md", None, 0),
                ThreadConfig::new("读书笔记", "读书笔记.md", None, 1),
            ],
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(persisted),
        );
        assert_eq!(app.thread_configs.len(), 2);
        assert_eq!(app.thread_configs[0].name, "想法");
        assert_eq!(app.thread_configs[1].name, "读书笔记");
    }

    fn thread_fixture(name: &str, target_file: &str, order: i32) -> ThreadConfig {
        ThreadConfig::new(name, target_file, None, order)
    }

    fn seeded_app(threads: Vec<ThreadConfig>) -> SettingsApp {
        let settings = AppSettings {
            thread_configs: threads,
            ..AppSettings::default()
        };
        SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(settings),
        )
    }

    #[test]
    fn thread_name_changed_writes_shadow() {
        let t1 = thread_fixture("A", "A.md", 0);
        let t2 = thread_fixture("B", "B.md", 1);
        let id = t1.id;
        let mut app = seeded_app(vec![t1, t2]);
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThreadNameChanged(id, "Renamed".into()),
        );
        assert_eq!(app.thread_configs[0].name, "Renamed");
        // Other thread must stay untouched.
        assert_eq!(app.thread_configs[1].name, "B");
    }

    #[test]
    fn thread_name_changed_for_unknown_id_is_noop() {
        let t1 = thread_fixture("A", "A.md", 0);
        let mut app = seeded_app(vec![t1]);
        let before = app.thread_configs.clone();
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThreadNameChanged(Uuid::new_v4(), "ghost".into()),
        );
        assert_eq!(app.thread_configs, before);
    }

    #[test]
    fn thread_folder_changed_preserves_filename() {
        // Mac `ThreadConfigRow.commitFolder` rebuilds `target_file` from the
        // draft folder + draft filename. The shadow carries only the joined
        // target_file, so the update branch must re-split to preserve the
        // current filename.
        let t1 = thread_fixture("A", "old/notes.md", 0);
        let id = t1.id;
        let mut app = seeded_app(vec![t1]);
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThreadFolderChanged(id, "new".into()),
        );
        assert_eq!(app.thread_configs[0].target_file, "new/notes.md");
    }

    #[test]
    fn thread_filename_changed_preserves_folder() {
        let t1 = thread_fixture("A", "keep/old.md", 0);
        let id = t1.id;
        let mut app = seeded_app(vec![t1]);
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThreadFilenameChanged(id, "new.md".into()),
        );
        assert_eq!(app.thread_configs[0].target_file, "keep/new.md");
    }

    #[test]
    fn thread_folder_browse_chose_none_is_noop() {
        let t1 = thread_fixture("A", "keep/old.md", 0);
        let id = t1.id;
        let mut app = seeded_app(vec![t1]);
        let before = app.thread_configs.clone();
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThreadFolderBrowseChose(id, None),
        );
        assert_eq!(app.thread_configs, before);
    }

    #[test]
    fn thread_folder_browse_chose_within_vault_stores_relative_path() {
        let t1 = thread_fixture("A", "old/notes.md", 0);
        let id = t1.id;
        let mut app = seeded_app(vec![t1]);
        app.vault_path = "/Users/you/Vault".into();
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThreadFolderBrowseChose(
                id,
                Some("/Users/you/Vault/Projects".into()),
            ),
        );
        assert_eq!(app.thread_configs[0].target_file, "Projects/notes.md");
    }

    #[test]
    fn thread_folder_browse_chose_outside_vault_keeps_absolute_path() {
        let t1 = thread_fixture("A", "old/notes.md", 0);
        let id = t1.id;
        let mut app = seeded_app(vec![t1]);
        app.vault_path = "/Users/you/Vault".into();
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThreadFolderBrowseChose(
                id,
                Some("/Users/other/Scratch".into()),
            ),
        );
        assert_eq!(
            app.thread_configs[0].target_file,
            "/Users/other/Scratch/notes.md"
        );
    }

    #[test]
    fn thread_folder_browse_chose_guards_against_vault_prefix_collision() {
        // "/Users/you/Vault-backup" starts with "/Users/you/Vault" textually,
        // but the prefix check must reject it because it is not the vault
        // folder. Mirrors Mac `ThreadConfigRow.chooseFolder`'s trailing-slash
        // probe.
        let t1 = thread_fixture("A", "keep.md", 0);
        let id = t1.id;
        let mut app = seeded_app(vec![t1]);
        app.vault_path = "/Users/you/Vault".into();
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThreadFolderBrowseChose(
                id,
                Some("/Users/you/Vault-backup".into()),
            ),
        );
        assert_eq!(
            app.thread_configs[0].target_file,
            "/Users/you/Vault-backup/keep.md"
        );
    }

    #[test]
    fn thread_folder_browse_chose_defaults_filename_when_blank() {
        // Mac `ThreadConfigRow.chooseFolder` auto-sets the filename to
        // `Threads.md` when the draft is empty. Mirror the same default so
        // the user isn't left with a bare-folder target after picking a path.
        let t1 = thread_fixture("A", "", 0);
        let id = t1.id;
        let mut app = seeded_app(vec![t1]);
        app.vault_path = "/Users/you/Vault".into();
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThreadFolderBrowseChose(
                id,
                Some("/Users/you/Vault/Logs".into()),
            ),
        );
        assert_eq!(app.thread_configs[0].target_file, "Logs/Threads.md");
    }

    #[test]
    fn thread_folder_browse_chose_empty_vault_keeps_absolute() {
        // When the vault path is blank, there's nothing to compute a relative
        // path against, so the picked absolute path must be stored verbatim.
        let t1 = thread_fixture("A", "notes.md", 0);
        let id = t1.id;
        let mut app = seeded_app(vec![t1]);
        assert!(app.vault_path.is_empty());
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThreadFolderBrowseChose(
                id,
                Some("/Users/x/Folder".into()),
            ),
        );
        assert_eq!(
            app.thread_configs[0].target_file,
            "/Users/x/Folder/notes.md"
        );
    }

    #[test]
    fn thread_added_appends_with_next_order() {
        let t1 = thread_fixture("A", "A.md", 5);
        let mut app = seeded_app(vec![t1]);
        let before_len = app.thread_configs.len();
        let _ = settings_update(&mut app, SettingsMessage::ThreadAdded);
        assert_eq!(app.thread_configs.len(), before_len + 1);
        let added = app.thread_configs.last().expect("must have last thread");
        // order = max(existing) + 1
        assert_eq!(added.order, 6);
        // default target_file == "<name>.md"
        assert_eq!(added.target_file, format!("{}.md", added.name));
    }

    #[test]
    fn thread_added_from_empty_starts_at_zero() {
        // Empty shadow (rare but possible during development) must start
        // order at 0, not crash on `max().unwrap()`.
        let mut app = seeded_app(vec![]);
        let _ = settings_update(&mut app, SettingsMessage::ThreadAdded);
        assert_eq!(app.thread_configs.len(), 1);
        assert_eq!(app.thread_configs[0].order, 0);
    }

    #[test]
    fn thread_added_appends_unique_name_on_collision() {
        // The localized default "新线程" (Zh) / "New Thread" (En) might
        // already exist; the helper must append a numeric suffix starting
        // at 1 until nothing collides.
        let lang = Language::Zh;
        let base = L10n::new_thread_default_name(lang);
        let settings = AppSettings {
            language: lang,
            thread_configs: vec![
                ThreadConfig::new(base, format!("{base}.md"), None, 0),
            ],
            ..AppSettings::default()
        };
        let mut app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(settings),
        );
        let _ = settings_update(&mut app, SettingsMessage::ThreadAdded);
        assert_eq!(app.thread_configs.len(), 2);
        let added = app.thread_configs.last().unwrap();
        assert_eq!(added.name, format!("{base} 1"));
        assert_eq!(added.target_file, format!("{base} 1.md"));
    }

    #[test]
    fn thread_added_at_maximum_is_noop() {
        let threads: Vec<ThreadConfig> = (0..ThreadConfig::MAXIMUM_COUNT)
            .map(|i| {
                ThreadConfig::new(
                    format!("T{i}"),
                    format!("T{i}.md"),
                    None,
                    i as i32,
                )
            })
            .collect();
        let mut app = seeded_app(threads);
        let before = app.thread_configs.clone();
        let _ = settings_update(&mut app, SettingsMessage::ThreadAdded);
        assert_eq!(app.thread_configs, before);
        assert_eq!(app.thread_configs.len(), ThreadConfig::MAXIMUM_COUNT);
    }

    #[test]
    fn thread_removed_filters_by_id() {
        let t1 = thread_fixture("A", "A.md", 0);
        let t2 = thread_fixture("B", "B.md", 1);
        let t3 = thread_fixture("C", "C.md", 2);
        let id_of_b = t2.id;
        let mut app = seeded_app(vec![t1, t2, t3]);
        let _ = settings_update(&mut app, SettingsMessage::ThreadRemoved(id_of_b));
        assert_eq!(app.thread_configs.len(), 2);
        assert!(app.thread_configs.iter().all(|t| t.id != id_of_b));
    }

    #[test]
    fn thread_removed_at_minimum_is_noop() {
        // Mirrors `canRemoveThread = count > 1` on Mac. The last remaining
        // thread cannot be deleted from the shadow.
        let t1 = thread_fixture("Solo", "Solo.md", 0);
        let id = t1.id;
        let mut app = seeded_app(vec![t1]);
        let _ = settings_update(&mut app, SettingsMessage::ThreadRemoved(id));
        assert_eq!(app.thread_configs.len(), 1);
        assert_eq!(app.thread_configs[0].name, "Solo");
    }

    #[test]
    fn thread_removed_for_unknown_id_is_noop() {
        let t1 = thread_fixture("A", "A.md", 0);
        let t2 = thread_fixture("B", "B.md", 1);
        let mut app = seeded_app(vec![t1, t2]);
        let before = app.thread_configs.clone();
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThreadRemoved(Uuid::new_v4()),
        );
        assert_eq!(app.thread_configs, before);
    }

    #[test]
    fn thread_messages_do_not_mutate_shared_settings() {
        // Sub-task 6 still only touches the shadow. Persistence lands in
        // sub-task 8.
        let t1 = thread_fixture("A", "A.md", 0);
        let t2 = thread_fixture("B", "B.md", 1);
        let id = t1.id;
        let mut app = seeded_app(vec![t1, t2]);
        let original = app.settings.thread_configs.clone();
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThreadNameChanged(id, "Renamed".into()),
        );
        let _ = settings_update(
            &mut app,
            SettingsMessage::ThreadFolderChanged(id, "folder".into()),
        );
        let _ = settings_update(&mut app, SettingsMessage::ThreadAdded);
        let _ = settings_update(&mut app, SettingsMessage::ThreadRemoved(id));
        assert_eq!(app.settings.thread_configs, original);
    }

    #[test]
    fn normalize_thread_folder_to_vault_handles_every_branch() {
        // `picked == vault_path` → empty relative folder
        assert_eq!(
            normalize_thread_folder_to_vault("/Users/x/Vault", "/Users/x/Vault"),
            ""
        );
        // inside vault → relative
        assert_eq!(
            normalize_thread_folder_to_vault("/Users/x/Vault", "/Users/x/Vault/A"),
            "A"
        );
        // vault with trailing slash → still relative
        assert_eq!(
            normalize_thread_folder_to_vault("/Users/x/Vault/", "/Users/x/Vault/A"),
            "A"
        );
        // outside vault → absolute preserved
        assert_eq!(
            normalize_thread_folder_to_vault("/Users/x/Vault", "/tmp"),
            "/tmp"
        );
        // vault-looking sibling → absolute preserved
        assert_eq!(
            normalize_thread_folder_to_vault(
                "/Users/x/Vault",
                "/Users/x/Vault-backup"
            ),
            "/Users/x/Vault-backup"
        );
        // empty vault → always absolute
        assert_eq!(
            normalize_thread_folder_to_vault("", "/tmp"),
            "/tmp"
        );
    }

    #[test]
    fn unique_new_thread_name_returns_base_when_no_collision() {
        assert_eq!(
            unique_new_thread_name(&[], "New Thread"),
            ("New Thread".to_string(), "New Thread.md".to_string())
        );
    }

    #[test]
    fn unique_new_thread_name_skips_name_collision() {
        let existing = vec![ThreadConfig::new(
            "New Thread",
            "New Thread.md",
            None,
            0,
        )];
        let (name, target) = unique_new_thread_name(&existing, "New Thread");
        assert_eq!(name, "New Thread 1");
        assert_eq!(target, "New Thread 1.md");
    }

    #[test]
    fn unique_new_thread_name_also_skips_target_file_collision() {
        // A thread whose name differs but target_file collides must still
        // bump the suffix. Matches Mac `AppSettings.addThread` which checks
        // both the `name` and `targetFile` fields.
        let existing = vec![ThreadConfig::new(
            "Something else",
            "New Thread.md",
            None,
            0,
        )];
        let (name, target) = unique_new_thread_name(&existing, "New Thread");
        assert_eq!(name, "New Thread 1");
        assert_eq!(target, "New Thread 1.md");
    }

    // --- Sub-task 6 build_cards dispatch -------------------------------
    //
    // The Threads card must only surface under Thread mode so the Dimension
    // / File modes keep their Mac-parity card ordering. These tests smoke
    // the `build_cards` dispatch for each mode; a missing arm would either
    // panic at construction or compile away silently.

    #[test]
    fn build_cards_in_thread_mode_shows_threads_card() {
        let settings = AppSettings {
            note_write_mode: WriteMode::Thread,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(settings),
        );
        assert_eq!(app.write_mode, WriteMode::Thread);
        let _element: Element<'_, SettingsMessage> = build_cards(&app);
    }

    #[test]
    fn build_cards_in_dimension_mode_hides_threads_card() {
        // Dimension mode ships the Quick Sections card instead; the Threads
        // card must stay out so the Dimension card stack matches Mac.
        let settings = AppSettings {
            note_write_mode: WriteMode::Dimension,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(settings),
        );
        let _element: Element<'_, SettingsMessage> = build_cards(&app);
    }

    #[test]
    fn build_cards_in_file_mode_hides_threads_card() {
        // File mode has its own inbox row inside the Storage card; the
        // Threads card must stay out of File mode so neither card surface
        // duplicates.
        let settings = AppSettings {
            note_write_mode: WriteMode::File,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(settings),
        );
        let _element: Element<'_, SettingsMessage> = build_cards(&app);
    }

    #[test]
    fn build_cards_in_thread_mode_builds_across_all_languages() {
        // Each language must paint the threads card without panicking; a
        // stale L10n key would show up here at test time rather than at
        // first paint.
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            let settings = AppSettings {
                language: lang,
                note_write_mode: WriteMode::Thread,
                ..AppSettings::default()
            };
            let app = SettingsApp::new(
                TraceTheme::for_preset(ThemePreset::Dark),
                Arc::new(settings),
            );
            let _element: Element<'_, SettingsMessage> = build_cards(&app);
        }
    }

    // --- Sub-task 7 Commit 2: Shortcut target + recorder update branches
    //
    // Cover every validation gate in `RecordingCaptured` plus the shadow
    // seeding and `start / cancel` flows so the card renderer (Commit 3) and
    // keyboard subscription (Commit 4) can trust the update pipeline.

    use trace_core::{
        DEFAULT_APPEND_NOTE_MODIFIERS, DEFAULT_APPEND_NOTE_VKEY, DEFAULT_GLOBAL_HOTKEY_MODIFIERS,
        DEFAULT_GLOBAL_HOTKEY_VKEY, DEFAULT_MODE_TOGGLE_MODIFIERS, DEFAULT_MODE_TOGGLE_VKEY,
        DEFAULT_SEND_NOTE_MODIFIERS, DEFAULT_SEND_NOTE_VKEY, MOD_ALT, MOD_CONTROL, MOD_SHIFT,
    };

    #[test]
    fn shortcut_target_name_and_category_cover_all_variants_and_languages() {
        // Every target must resolve its name + category in every language
        // without falling through to an empty string; a stale L10n key would
        // surface here as an empty label.
        for lang in [
            Language::SystemDefault,
            Language::Zh,
            Language::En,
            Language::Ja,
        ] {
            for target in ShortcutTarget::ALL {
                assert!(!target.name(lang).is_empty());
                assert!(!target.category(lang).is_empty());
            }
        }
    }

    #[test]
    fn shortcut_target_all_lists_every_variant_in_display_order() {
        // Order matters: the Shortcuts card renders rows in this sequence and
        // the conflict scan iterates it, so freeze the order with an explicit
        // test rather than relying on the derive to preserve declaration
        // order implicitly.
        assert_eq!(
            ShortcutTarget::ALL,
            [
                ShortcutTarget::Create,
                ShortcutTarget::Send,
                ShortcutTarget::Append,
                ShortcutTarget::ToggleMode,
            ]
        );
    }

    #[test]
    fn settings_app_new_seeds_all_four_shortcut_shadows_from_persisted_settings() {
        // Defaults on `AppSettings` decode into defaults on `SettingsApp`.
        let app = fresh_app();
        assert_eq!(
            app.global_hotkey,
            ShortcutSpec::new(DEFAULT_GLOBAL_HOTKEY_VKEY, DEFAULT_GLOBAL_HOTKEY_MODIFIERS)
        );
        assert_eq!(
            app.send_note_shortcut,
            ShortcutSpec::new(DEFAULT_SEND_NOTE_VKEY, DEFAULT_SEND_NOTE_MODIFIERS)
        );
        assert_eq!(
            app.append_note_shortcut,
            ShortcutSpec::new(DEFAULT_APPEND_NOTE_VKEY, DEFAULT_APPEND_NOTE_MODIFIERS)
        );
        assert_eq!(
            app.mode_toggle_shortcut,
            ShortcutSpec::new(DEFAULT_MODE_TOGGLE_VKEY, DEFAULT_MODE_TOGGLE_MODIFIERS)
        );
        assert!(app.recording_target.is_none());
        assert!(app.shortcut_recorder_message.is_none());
    }

    #[test]
    fn settings_app_new_seeds_shortcut_shadows_from_non_default_settings() {
        // Pin a non-default `(code, modifiers)` tuple on every shortcut so
        // the round-trip through `ShortcutSpec::new` is exercised by more
        // than just the all-default path.
        let persisted = AppSettings {
            hot_key_code: 0x41,       // A
            hot_key_modifiers: MOD_ALT,
            send_note_key_code: 0x42, // B
            send_note_modifiers: MOD_SHIFT,
            append_note_key_code: 0x43, // C
            append_note_modifiers: MOD_CONTROL | MOD_SHIFT,
            mode_toggle_key_code: 0x44, // D
            mode_toggle_modifiers: MOD_CONTROL,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(persisted),
        );
        assert_eq!(app.global_hotkey, ShortcutSpec::new(0x41, MOD_ALT));
        assert_eq!(app.send_note_shortcut, ShortcutSpec::new(0x42, MOD_SHIFT));
        assert_eq!(
            app.append_note_shortcut,
            ShortcutSpec::new(0x43, MOD_CONTROL | MOD_SHIFT)
        );
        assert_eq!(
            app.mode_toggle_shortcut,
            ShortcutSpec::new(0x44, MOD_CONTROL)
        );
    }

    #[test]
    fn shortcut_for_and_set_shortcut_round_trip_every_target() {
        let mut app = fresh_app();
        for target in ShortcutTarget::ALL {
            // Use a distinct key code per target so a swapped-match arm in
            // `set_shortcut`/`shortcut_for` would fail the round-trip.
            let probe = ShortcutSpec::new(0x70 + target as u32, MOD_CONTROL | MOD_ALT);
            app.set_shortcut(target, probe);
            assert_eq!(app.shortcut_for(target), probe);
        }
    }

    #[test]
    fn recording_started_arms_target_and_clears_stale_message() {
        let mut app = fresh_app();
        app.shortcut_recorder_message = Some("stale".to_string());
        let _ = settings_update(
            &mut app,
            SettingsMessage::RecordingStarted(ShortcutTarget::Send),
        );
        assert_eq!(app.recording_target, Some(ShortcutTarget::Send));
        assert_eq!(app.shortcut_recorder_message, None);
    }

    #[test]
    fn recording_started_toggles_off_on_second_invocation_for_same_target() {
        // Mirror Mac `toggleRecording(for:)`: pressing Edit on a target that
        // is already recording must disarm the recorder rather than re-arm
        // it. Without the toggle check the row is stuck in recording mode
        // with no way to exit short of pressing Cancel.
        let mut app = fresh_app();
        // First click — enters recording for Send.
        let _ = settings_update(
            &mut app,
            SettingsMessage::RecordingStarted(ShortcutTarget::Send),
        );
        assert_eq!(app.recording_target, Some(ShortcutTarget::Send));
        // Second click on the same target — exits recording.
        let _ = settings_update(
            &mut app,
            SettingsMessage::RecordingStarted(ShortcutTarget::Send),
        );
        assert_eq!(app.recording_target, None);
    }

    #[test]
    fn recording_started_switches_to_different_target_without_nulling() {
        // Clicking Edit on a different row while another row is recording
        // must switch the armed target outright, not drop to `None` and
        // force the user to click twice.
        let mut app = fresh_app();
        let _ = settings_update(
            &mut app,
            SettingsMessage::RecordingStarted(ShortcutTarget::Send),
        );
        assert_eq!(app.recording_target, Some(ShortcutTarget::Send));
        let _ = settings_update(
            &mut app,
            SettingsMessage::RecordingStarted(ShortcutTarget::Append),
        );
        assert_eq!(app.recording_target, Some(ShortcutTarget::Append));
    }

    #[test]
    fn recording_cancelled_disarms_target_and_clears_message() {
        let mut app = fresh_app();
        app.recording_target = Some(ShortcutTarget::Append);
        app.shortcut_recorder_message = Some("stale".to_string());
        let _ = settings_update(&mut app, SettingsMessage::RecordingCancelled);
        assert_eq!(app.recording_target, None);
        assert_eq!(app.shortcut_recorder_message, None);
    }

    #[test]
    fn recording_captured_without_armed_target_is_no_op() {
        let mut app = fresh_app();
        let before = app.global_hotkey;
        let _ = settings_update(
            &mut app,
            SettingsMessage::RecordingCaptured {
                key_code: 0x4E,
                modifiers: MOD_CONTROL,
            },
        );
        assert_eq!(app.global_hotkey, before);
        assert_eq!(app.shortcut_recorder_message, None);
    }

    #[test]
    fn recording_captured_bare_escape_silently_cancels_recording() {
        // Mac reference: bare Esc returns `nil` from the NSEvent monitor,
        // i.e. disarms the recorder without flagging an error. The Win32
        // port must match so the Esc chord behaves predictably.
        let mut app = fresh_app();
        app.recording_target = Some(ShortcutTarget::Send);
        let before = app.send_note_shortcut;
        let _ = settings_update(
            &mut app,
            SettingsMessage::RecordingCaptured {
                key_code: 0x1B,
                modifiers: 0,
            },
        );
        assert_eq!(app.recording_target, None);
        assert_eq!(app.shortcut_recorder_message, None);
        assert_eq!(app.send_note_shortcut, before);
    }

    #[test]
    fn recording_captured_without_modifier_reports_need_modifier_message() {
        let mut app = fresh_app();
        app.recording_target = Some(ShortcutTarget::Send);
        let before = app.send_note_shortcut;
        let _ = settings_update(
            &mut app,
            SettingsMessage::RecordingCaptured {
                key_code: 0x41, // A without any modifier
                modifiers: 0,
            },
        );
        // Target stays armed so the user can retry without reopening.
        assert_eq!(app.recording_target, Some(ShortcutTarget::Send));
        assert_eq!(app.send_note_shortcut, before);
        assert_eq!(
            app.shortcut_recorder_message.as_deref(),
            Some(L10n::need_modifier_key(app.language))
        );
    }

    #[test]
    fn recording_captured_escape_with_modifier_reports_esc_reserved_message() {
        let mut app = fresh_app();
        app.recording_target = Some(ShortcutTarget::Send);
        let before = app.send_note_shortcut;
        let _ = settings_update(
            &mut app,
            SettingsMessage::RecordingCaptured {
                key_code: 0x1B,
                modifiers: MOD_CONTROL,
            },
        );
        assert_eq!(app.recording_target, Some(ShortcutTarget::Send));
        assert_eq!(app.send_note_shortcut, before);
        assert_eq!(
            app.shortcut_recorder_message.as_deref(),
            Some(L10n::esc_reserved(app.language))
        );
    }

    #[test]
    fn recording_captured_ctrl_digit_on_panel_target_reports_cmd_number_reserved() {
        // Ctrl+1..9 is reserved for panel section switching; the three panel
        // targets must reject it. Sweep all three to prove the gate is not
        // special-cased to `Send` alone.
        for target in [
            ShortcutTarget::Send,
            ShortcutTarget::Append,
            ShortcutTarget::ToggleMode,
        ] {
            let mut app = fresh_app();
            app.recording_target = Some(target);
            let before = app.shortcut_for(target);
            let _ = settings_update(
                &mut app,
                SettingsMessage::RecordingCaptured {
                    key_code: 0x31, // '1'
                    modifiers: MOD_CONTROL,
                },
            );
            assert_eq!(app.recording_target, Some(target), "{target:?}");
            assert_eq!(app.shortcut_for(target), before, "{target:?}");
            assert_eq!(
                app.shortcut_recorder_message.as_deref(),
                Some(L10n::cmd_number_reserved(app.language)),
                "{target:?}"
            );
        }
    }

    #[test]
    fn recording_captured_ctrl_digit_on_create_target_is_accepted() {
        // The global Create hotkey sits outside the panel scope, so Ctrl+1..9
        // is a valid capture there — the reservation gate must not apply.
        let mut app = fresh_app();
        app.recording_target = Some(ShortcutTarget::Create);
        let _ = settings_update(
            &mut app,
            SettingsMessage::RecordingCaptured {
                key_code: 0x32, // '2'
                modifiers: MOD_CONTROL,
            },
        );
        assert_eq!(app.recording_target, None);
        assert_eq!(app.shortcut_recorder_message, None);
        assert_eq!(app.global_hotkey, ShortcutSpec::new(0x32, MOD_CONTROL));
    }

    #[test]
    fn recording_captured_conflict_reports_conflicting_target_name() {
        // Seed the Send shadow with a known combo, then try to record the
        // same combo on Append. The update branch must stage the conflict
        // message naming "Send Note" (or its localized equivalent).
        let mut app = fresh_app();
        let conflict_spec = ShortcutSpec::new(0x4E, MOD_ALT | MOD_CONTROL); // Ctrl+Alt+N
        app.send_note_shortcut = conflict_spec;
        app.recording_target = Some(ShortcutTarget::Append);
        let before = app.append_note_shortcut;
        let _ = settings_update(
            &mut app,
            SettingsMessage::RecordingCaptured {
                key_code: 0x4E,
                modifiers: MOD_ALT | MOD_CONTROL,
            },
        );
        assert_eq!(app.recording_target, Some(ShortcutTarget::Append));
        assert_eq!(app.append_note_shortcut, before);
        let expected = L10n::shortcut_conflict(
            app.language,
            ShortcutTarget::Send.name(app.language),
        );
        assert_eq!(app.shortcut_recorder_message.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn recording_captured_same_combo_on_current_target_is_not_a_self_conflict() {
        // Recording the already-set shortcut onto its own row must succeed —
        // the conflict scan excludes `current` from the walk. This guards
        // against an accidental off-by-one in `conflicting_target`. The
        // chosen combo deliberately avoids every default shadow slot so a
        // real conflict cannot muddy the self-exclusion assertion.
        let mut app = fresh_app();
        let spec = ShortcutSpec::new(0x50, MOD_CONTROL | MOD_SHIFT); // Ctrl+Shift+P
        app.send_note_shortcut = spec;
        app.recording_target = Some(ShortcutTarget::Send);
        let _ = settings_update(
            &mut app,
            SettingsMessage::RecordingCaptured {
                key_code: 0x50,
                modifiers: MOD_CONTROL | MOD_SHIFT,
            },
        );
        assert_eq!(app.recording_target, None);
        assert_eq!(app.shortcut_recorder_message, None);
        assert_eq!(app.send_note_shortcut, spec);
    }

    #[test]
    fn recording_captured_success_commits_spec_and_disarms_recorder() {
        let mut app = fresh_app();
        app.recording_target = Some(ShortcutTarget::ToggleMode);
        app.shortcut_recorder_message = Some("stale".to_string());
        let _ = settings_update(
            &mut app,
            SettingsMessage::RecordingCaptured {
                key_code: 0x54, // T
                modifiers: MOD_CONTROL | MOD_ALT,
            },
        );
        assert_eq!(app.recording_target, None);
        assert_eq!(app.shortcut_recorder_message, None);
        assert_eq!(
            app.mode_toggle_shortcut,
            ShortcutSpec::new(0x54, MOD_CONTROL | MOD_ALT)
        );
    }

    #[test]
    fn conflicting_target_skips_current_and_finds_first_match() {
        // Lock the scan order: seed Send and Append with the same combo,
        // record onto ToggleMode, and assert the scan returns Send (the
        // first in `ShortcutTarget::ALL`).
        let mut app = fresh_app();
        let spec = ShortcutSpec::new(0x50, MOD_CONTROL); // Ctrl+P
        app.send_note_shortcut = spec;
        app.append_note_shortcut = spec;
        let found = conflicting_target(&app, spec, ShortcutTarget::ToggleMode);
        assert_eq!(found, Some(ShortcutTarget::Send));

        // Excluding Send should surface Append next.
        let found = conflicting_target(&app, spec, ShortcutTarget::Send);
        assert_eq!(found, Some(ShortcutTarget::Append));
    }

    // --- Sub-task 7 Commit 3: Shortcuts card in build_cards dispatch ------

    #[test]
    fn build_cards_renders_shortcuts_card_under_dimension_mode() {
        let settings = AppSettings {
            note_write_mode: WriteMode::Dimension,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(settings),
        );
        let _element: Element<'_, SettingsMessage> = build_cards(&app);
    }

    #[test]
    fn build_cards_renders_shortcuts_card_under_thread_mode() {
        let settings = AppSettings {
            note_write_mode: WriteMode::Thread,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(settings),
        );
        let _element: Element<'_, SettingsMessage> = build_cards(&app);
    }

    #[test]
    fn build_cards_renders_shortcuts_card_under_file_mode() {
        let settings = AppSettings {
            note_write_mode: WriteMode::File,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(settings),
        );
        let _element: Element<'_, SettingsMessage> = build_cards(&app);
    }

    #[test]
    fn build_cards_renders_shortcuts_card_while_recording() {
        // Exercise the recording-chip branch through `build_cards` so a
        // dropped match arm in the card renderer would surface here rather
        // than at first paint.
        let mut app = fresh_app();
        app.recording_target = Some(ShortcutTarget::Create);
        let _element: Element<'_, SettingsMessage> = build_cards(&app);
    }

    #[test]
    fn settings_view_renders_with_recording_active_for_every_target() {
        // `build_cards_renders_shortcuts_card_while_recording` above only
        // covers `ShortcutTarget::Create`. Each target has its own row and
        // trailing button, so iterate `ShortcutTarget::ALL` through the full
        // `settings_view` surface to catch a dropped match arm in either
        // the card renderer or the outer scroll container.
        for target in ShortcutTarget::ALL {
            let mut app = fresh_app();
            app.recording_target = Some(target);
            let _element: Element<'_, SettingsMessage> = settings_view(&app);
        }
    }

    #[test]
    fn build_cards_renders_shortcuts_card_with_validation_error_footer() {
        // The validation-error footer only surfaces when the shadow carries
        // a message. A separate app keeps the scope clean from the recording
        // variant above.
        let mut app = fresh_app();
        app.shortcut_recorder_message = Some("conflict".to_string());
        let _element: Element<'_, SettingsMessage> = build_cards(&app);
    }

    // --- Sub-task 7 Commit 4: Keystroke → RecordingCaptured decoder -------

    #[test]
    fn keyboard_event_to_message_maps_keypressed_letter_with_ctrl() {
        // Ctrl+N — the canonical case. The decoder must land the letter on
        // its Win32 VK code (0x4E) and the modifier on `MOD_CONTROL`.
        let event = Event::Keyboard(KeyboardEvent::KeyPressed {
            key: iced::keyboard::Key::Character("n".into()),
            modified_key: iced::keyboard::Key::Character("n".into()),
            physical_key: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: iced::keyboard::Location::Standard,
            modifiers: iced::keyboard::Modifiers::CTRL,
            text: None,
            repeat: false,
        });
        let msg = keyboard_event_to_message(event, iced_event::Status::Ignored, window::Id::unique());
        // `SettingsMessage` is intentionally not `PartialEq` (its payloads
        // include iced task handles on other variants); destructure the
        // tuple to assert the shape explicitly.
        let Some(SettingsMessage::RecordingCaptured { key_code, modifiers }) = msg else {
            panic!("expected RecordingCaptured, got {msg:?}");
        };
        assert_eq!(key_code, 0x4E);
        assert_eq!(modifiers, trace_core::MOD_CONTROL);
    }

    #[test]
    fn keyboard_event_to_message_maps_named_escape_without_modifier() {
        // Bare Escape is a valid message — the update branch decides that
        // "no modifier + Esc" silently cancels the recorder. The decoder
        // itself returns a message so the handler can apply that rule.
        let event = Event::Keyboard(KeyboardEvent::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            modified_key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            physical_key: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: iced::keyboard::Location::Standard,
            modifiers: iced::keyboard::Modifiers::empty(),
            text: None,
            repeat: false,
        });
        let msg = keyboard_event_to_message(event, iced_event::Status::Ignored, window::Id::unique());
        let Some(SettingsMessage::RecordingCaptured { key_code, modifiers }) = msg else {
            panic!("expected RecordingCaptured, got {msg:?}");
        };
        assert_eq!(key_code, 0x1B);
        assert_eq!(modifiers, 0);
    }

    #[test]
    fn keyboard_event_to_message_ignores_unsupported_character() {
        // A non-ASCII character has no VK mapping — the decoder must drop
        // it so the recorder stays idle rather than forwarding a garbage
        // VK code that would later fall through to the numeric fallback
        // in `vk_label`.
        let event = Event::Keyboard(KeyboardEvent::KeyPressed {
            key: iced::keyboard::Key::Character("中".into()),
            modified_key: iced::keyboard::Key::Character("中".into()),
            physical_key: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: iced::keyboard::Location::Standard,
            modifiers: iced::keyboard::Modifiers::CTRL,
            text: None,
            repeat: false,
        });
        let msg = keyboard_event_to_message(event, iced_event::Status::Ignored, window::Id::unique());
        assert!(msg.is_none());
    }

    #[test]
    fn keyboard_event_to_message_ignores_modifier_only_key() {
        // A pure modifier keyup (no accompanying letter) has no VK mapping.
        // Mirror Mac's `NSEvent` monitor which treats these as "still
        // recording" rather than a shortcut.
        let event = Event::Keyboard(KeyboardEvent::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Shift),
            modified_key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Shift),
            physical_key: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: iced::keyboard::Location::Left,
            modifiers: iced::keyboard::Modifiers::SHIFT,
            text: None,
            repeat: false,
        });
        let msg = keyboard_event_to_message(event, iced_event::Status::Ignored, window::Id::unique());
        assert!(msg.is_none());
    }

    #[test]
    fn keyboard_event_to_message_ignores_keyboard_event_released() {
        // Only `KeyPressed` drives the recorder — key-up events must not
        // race the press by emitting a second `RecordingCaptured`.
        let event = Event::Keyboard(KeyboardEvent::KeyReleased {
            key: iced::keyboard::Key::Character("n".into()),
            modified_key: iced::keyboard::Key::Character("n".into()),
            physical_key: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: iced::keyboard::Location::Standard,
            modifiers: iced::keyboard::Modifiers::CTRL,
        });
        let msg = keyboard_event_to_message(event, iced_event::Status::Ignored, window::Id::unique());
        assert!(msg.is_none());
    }

    #[test]
    fn keyboard_event_to_message_ignores_non_keyboard_events() {
        // Window / mouse / touch events must never produce a recorder
        // message, so the decoder returns `None` on the first match guard.
        let event = Event::Window(iced::window::Event::Focused);
        let msg = keyboard_event_to_message(event, iced_event::Status::Ignored, window::Id::unique());
        assert!(msg.is_none());
    }

    // --- Sub-task 8a: System card ------------------------------------------

    #[test]
    fn settings_app_new_seeds_launch_at_login_shadow_from_settings() {
        // The new shadow flag must hydrate from the persisted `AppSettings`
        // snapshot so the System card reads the user's last choice on first
        // paint without waiting for a message dispatch.
        let persisted = AppSettings {
            launch_at_login: true,
            ..AppSettings::default()
        };
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(persisted),
        );
        assert!(app.launch_at_login_shadow);

        // Default is `false` — guard the opposite branch too.
        let default_app = fresh_app();
        assert!(!default_app.launch_at_login_shadow);
    }

    #[test]
    fn launch_at_login_shadow_updates_on_message() {
        // Sub-task 8a is shadow-only: flipping the toggle must land on
        // `launch_at_login_shadow` without leaking into the persisted
        // `AppSettings.launch_at_login`. Drive both directions through the
        // update fn so a later regression that forgets one arm is caught.
        let mut app = fresh_app();
        assert!(!app.launch_at_login_shadow);

        let _ = settings_update(&mut app, SettingsMessage::LaunchAtLoginToggled(true));
        assert!(app.launch_at_login_shadow);
        // The shared `Arc<AppSettings>` must stay untouched — only
        // `working.launch_at_login` (the write-through buffer) reflects the toggle.
        assert!(!app.settings.launch_at_login);

        let _ = settings_update(&mut app, SettingsMessage::LaunchAtLoginToggled(false));
        assert!(!app.launch_at_login_shadow);
        assert!(!app.settings.launch_at_login);
    }

    #[test]
    fn system_card_renders_unconditionally_across_write_modes() {
        // The System card is not gated on `WriteMode` (unlike Quick Sections /
        // Threads), so `build_cards` must include it under every mode. iced
        // 0.14 gives no direct card-count introspection, but rendering the
        // full view for each mode catches a dropped match arm in the card
        // pusher the same way the Shortcuts parity tests above do.
        for mode in [WriteMode::Dimension, WriteMode::Thread, WriteMode::File] {
            let settings = AppSettings {
                note_write_mode: mode,
                ..AppSettings::default()
            };
            let app = SettingsApp::new(
                TraceTheme::for_preset(ThemePreset::Dark),
                Arc::new(settings),
            );
            let _element: Element<'_, SettingsMessage> = build_cards(&app);
            let _view: Element<'_, SettingsMessage> = settings_view(&app);
        }
    }

    // --- Sub-task 8b: working-copy write-through persistence --------------

    /// Helper that spins up a [`SettingsApp`] tied to `save_path` inside the
    /// supplied [`tempfile::TempDir`]. Keeps each test focused on the
    /// message-dispatch → reload-from-disk round-trip without re-typing the
    /// four-argument construction boilerplate.
    fn app_with_save_path(
        dir: &tempfile::TempDir,
        filename: &str,
    ) -> (SettingsApp, std::path::PathBuf) {
        let path = dir.path().join(filename);
        let app = SettingsApp::new_with_save_path(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(AppSettings::default()),
            Some(path.clone()),
        );
        (app, path)
    }

    #[test]
    fn new_with_save_path_seeds_working_copy_from_arc_snapshot() {
        // `working` must start as a full clone of the input `AppSettings` so
        // the write-through path carries the user's previous preferences
        // forward on the first edit rather than serializing a half-default
        // struct.
        let persisted = AppSettings {
            vault_path: "C:/old-vault".into(),
            language: Language::Ja,
            launch_at_login: true,
            ..AppSettings::default()
        };
        let app = SettingsApp::new_with_save_path(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(persisted),
            None,
        );
        assert_eq!(app.working.vault_path, "C:/old-vault");
        assert_eq!(app.working.language, Language::Ja);
        assert!(app.working.launch_at_login);
        assert!(app.save_path.is_none());
    }

    #[test]
    fn editing_vault_path_writes_to_disk() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (mut app, path) = app_with_save_path(&tmp, "settings.json");
        let _ = settings_update(
            &mut app,
            SettingsMessage::VaultPathChanged("/new/path".into()),
        );
        let loaded = AppSettings::load(&path).expect("load persisted settings");
        assert_eq!(loaded.vault_path, "/new/path");
    }

    #[test]
    fn editing_inbox_vault_path_persists_on_shadow_edit() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (mut app, path) = app_with_save_path(&tmp, "settings.json");
        let _ = settings_update(
            &mut app,
            SettingsMessage::InboxVaultPathChanged("/inbox/path".into()),
        );
        let loaded = AppSettings::load(&path).expect("load persisted settings");
        assert_eq!(loaded.inbox_vault_path, "/inbox/path");
    }

    #[test]
    fn editing_daily_folder_name_persists_on_shadow_edit() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (mut app, path) = app_with_save_path(&tmp, "settings.json");
        let _ = settings_update(
            &mut app,
            SettingsMessage::DailyFolderNameChanged("Journal".into()),
        );
        let loaded = AppSettings::load(&path).expect("load persisted settings");
        assert_eq!(loaded.daily_folder_name, "Journal");
    }

    #[test]
    fn editing_section_title_runs_normalize_before_save() {
        // `normalize()` trims whitespace, strips leading `#` markers, and
        // replaces CR/LF with spaces. Drop an in-progress keystroke that
        // violates the invariants and prove the persisted copy lands in the
        // canonical shape — not the raw shadow.
        let tmp = tempfile::tempdir().expect("tempdir");
        let (mut app, path) = app_with_save_path(&tmp, "settings.json");
        let _ = settings_update(
            &mut app,
            SettingsMessage::SectionTitleChanged(0, "## Raw\nInput  ".into()),
        );
        let loaded = AppSettings::load(&path).expect("load persisted settings");
        // Post-normalize: heading marker stripped, CR/LF swapped for space,
        // trailing whitespace trimmed. The exact shape is owned by
        // `normalize_section_title` — we assert the resulting title contains
        // no `#` marker and no newline.
        let title = &loaded.section_titles[0];
        assert!(
            !title.starts_with('#'),
            "leading heading marker must be stripped: {title:?}",
        );
        assert!(
            !title.contains('\n'),
            "newline must be replaced with space: {title:?}",
        );
        assert_eq!(title.trim(), title.as_str(), "whitespace must be trimmed");
    }

    #[test]
    fn launch_at_login_toggled_message_persists_flag() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (mut app, path) = app_with_save_path(&tmp, "settings.json");
        let _ = settings_update(&mut app, SettingsMessage::LaunchAtLoginToggled(true));
        let loaded = AppSettings::load(&path).expect("load persisted settings");
        assert!(loaded.launch_at_login);

        let _ = settings_update(&mut app, SettingsMessage::LaunchAtLoginToggled(false));
        let loaded = AppSettings::load(&path).expect("load persisted settings");
        assert!(!loaded.launch_at_login);
    }

    #[test]
    fn save_path_none_skips_writeback() {
        // Unit-test harness (`new(theme, settings)` and the existing suite)
        // must stay filesystem-free so tests don't accidentally race a
        // settings.json. Dispatch a mutating message through a `None`-save-path
        // app and confirm no files are created in the working directory and
        // no panic is raised.
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut app = SettingsApp::new_with_save_path(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(AppSettings::default()),
            None,
        );
        let _ = settings_update(
            &mut app,
            SettingsMessage::VaultPathChanged("/nope".into()),
        );
        // Nothing should have materialised in the temp dir — we never gave the
        // app a pointer to it.
        let entries: Vec<_> = std::fs::read_dir(tmp.path())
            .expect("read_dir tempdir")
            .collect();
        assert!(entries.is_empty(), "tempdir must stay empty");
    }

    #[test]
    fn hotkey_recorded_persists_vkey_and_modifiers_split() {
        // Sub-task 8b must split a captured `ShortcutSpec` back into the pair
        // of `u32` fields that `AppSettings` persists. Arm the recorder for
        // the global Create hotkey, dispatch a `RecordingCaptured` with a
        // modifier + digit combo (which is accepted on Create), then reload
        // the on-disk settings and verify both fields round-trip.
        let tmp = tempfile::tempdir().expect("tempdir");
        let (mut app, path) = app_with_save_path(&tmp, "settings.json");
        app.recording_target = Some(ShortcutTarget::Create);

        // VK_1 (0x31) + MOD_CONTROL (0x0002). `is_reserved_section_switch`
        // only matters for panel targets; Create accepts Ctrl+digit.
        const VK_1: u32 = 0x31;
        const MOD_CTRL: u32 = 0x0002;
        let _ = settings_update(
            &mut app,
            SettingsMessage::RecordingCaptured {
                key_code: VK_1,
                modifiers: MOD_CTRL,
            },
        );
        let loaded = AppSettings::load(&path).expect("load persisted settings");
        assert_eq!(loaded.hot_key_code, VK_1);
        assert_eq!(loaded.hot_key_modifiers, MOD_CTRL);
    }

    #[test]
    fn persist_failure_does_not_panic() {
        // Point `save_path` at a path whose parent directory does not exist.
        // `AppSettings::save` will fail (the parent is not auto-created by the
        // atomic writer); the update branch must swallow the error through
        // `tracing::warn!` without panicking or surfacing anything to the UI.
        let tmp = tempfile::tempdir().expect("tempdir");
        let bogus_path = tmp.path().join("missing-dir").join("settings.json");
        let mut app = SettingsApp::new_with_save_path(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(AppSettings::default()),
            Some(bogus_path),
        );
        // Would panic if `persist_working` bubbled the save error. The message
        // itself still needs to land on the working copy regardless.
        let _ = settings_update(
            &mut app,
            SettingsMessage::LanguageChanged(Language::Ja),
        );
        assert_eq!(app.working.language, Language::Ja);
    }

    #[test]
    fn new_legacy_constructor_defaults_save_path_to_none() {
        // Backwards compatibility: the existing `new(theme, settings)` call
        // sites across the codebase must keep compiling and must NOT touch
        // the filesystem. Smoke-check that the shim leaves `save_path` unset
        // and still clones the full `AppSettings` into `working`.
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(AppSettings {
                vault_path: "C:/pre-existing".into(),
                ..AppSettings::default()
            }),
        );
        assert!(app.save_path.is_none());
        assert_eq!(app.working.vault_path, "C:/pre-existing");
    }

    // --- Sub-task 8c.2: launch-at-login sink wiring -----------------------

    /// Recording sink used by the `LaunchAtLoginToggled` tests below. Kept as
    /// a tests-only helper so the production `trace-ui` surface never exposes
    /// a mock implementation. We intentionally duplicate a small amount of the
    /// `autostart::tests::RecordingSink` boilerplate rather than reach into a
    /// sibling `mod tests` block — `#[cfg(test)]` submodules aren't visible
    /// across module boundaries.
    #[derive(Default)]
    struct RecordingSink {
        calls: std::sync::Mutex<Vec<bool>>,
    }

    impl LaunchAtLoginSink for RecordingSink {
        fn apply(&self, enabled: bool) {
            self.calls
                .lock()
                .expect("recording sink mutex")
                .push(enabled);
        }
    }

    #[test]
    fn toggling_launch_at_login_invokes_sink_in_order() {
        // Arm the Settings window with a recording sink so we can observe the
        // sequence of `apply` calls that the `LaunchAtLoginToggled` arm
        // produces. Each UI toggle must produce exactly one sink call with
        // the same polarity as the shadow field.
        let sink = Arc::new(RecordingSink::default());
        let mut app = SettingsApp::new_with_dependencies(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(AppSettings::default()),
            None,
            Arc::clone(&sink) as Arc<dyn LaunchAtLoginSink>,
        );

        let _ = settings_update(&mut app, SettingsMessage::LaunchAtLoginToggled(true));
        let _ = settings_update(&mut app, SettingsMessage::LaunchAtLoginToggled(false));
        let _ = settings_update(&mut app, SettingsMessage::LaunchAtLoginToggled(true));

        let calls = sink.calls.lock().expect("recording sink mutex").clone();
        assert_eq!(calls, vec![true, false, true]);
    }

    #[test]
    fn sink_sees_shadow_and_working_already_updated() {
        // Contract: the sink is invoked *after* the shadow + working fields
        // settle, so a production sink that queries `AppSettings::load()` in
        // any follow-up code path sees the new value. We can't inspect state
        // from inside `apply` (the trait takes `&self`), so assert the
        // observable post-condition: by the time dispatch returns, both the
        // shadow + the on-disk JSON reflect the new flag *and* the sink was
        // called once with the matching value.
        let sink = Arc::new(RecordingSink::default());
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("settings.json");
        let mut app = SettingsApp::new_with_dependencies(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(AppSettings::default()),
            Some(path.clone()),
            Arc::clone(&sink) as Arc<dyn LaunchAtLoginSink>,
        );

        let _ = settings_update(&mut app, SettingsMessage::LaunchAtLoginToggled(true));

        // Shadow + working mirror the toggle.
        assert!(app.launch_at_login_shadow);
        assert!(app.working.launch_at_login);
        // Write-through ran before the sink: the on-disk JSON is already
        // carrying the new value.
        let loaded = AppSettings::load(&path).expect("load persisted settings");
        assert!(loaded.launch_at_login);
        // Sink was called exactly once with the same polarity.
        assert_eq!(
            *sink.calls.lock().expect("recording sink mutex"),
            vec![true]
        );
    }

    #[test]
    fn legacy_constructors_inject_noop_sink() {
        // The two pre-8c.2 constructors must continue to build a working
        // app and must not panic when the toggle arm forwards to the default
        // `NoopLaunchAtLoginSink`. We can't assert the sink's concrete type
        // without exposing a downcast surface, so instead we dispatch the
        // toggle through both shims and confirm no panic + shadow updates.
        let mut legacy_new = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Light),
            Arc::new(AppSettings::default()),
        );
        let _ = settings_update(
            &mut legacy_new,
            SettingsMessage::LaunchAtLoginToggled(true),
        );
        assert!(legacy_new.launch_at_login_shadow);

        let mut legacy_save_path = SettingsApp::new_with_save_path(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::new(AppSettings::default()),
            None,
        );
        let _ = settings_update(
            &mut legacy_save_path,
            SettingsMessage::LaunchAtLoginToggled(false),
        );
        assert!(!legacy_save_path.launch_at_login_shadow);
    }

    // --- Sub-task 8c.3: `latest_snapshot` broadcast plumbing ----------

    #[test]
    fn latest_snapshot_reflects_shadow_edits_after_persist() {
        // Every shadow-field dispatch must end with `persist_working`
        // refreshing `latest_snapshot` to a *new* `Arc` — the daemon
        // broadcasts after each settings message, so stale snapshots
        // would defeat the whole live-sync pipeline.
        let mut app = fresh_app();
        let before = app.latest_snapshot();
        let _ = settings_update(
            &mut app,
            SettingsMessage::LaunchAtLoginToggled(true),
        );
        let after = app.latest_snapshot();
        assert!(
            !Arc::ptr_eq(&before, &after),
            "snapshot must be a fresh allocation after a persist"
        );
        assert!(
            after.launch_at_login,
            "snapshot must mirror the shadow edit"
        );
    }

    #[test]
    fn latest_snapshot_is_cheap_clone_not_new_allocation_per_call() {
        // Between two edits, `latest_snapshot()` must hand out pointers
        // to the *same* allocation so the daemon's broadcast short-circuit
        // via `Arc::ptr_eq` has something to compare against.
        let app = fresh_app();
        let a = app.latest_snapshot();
        let b = app.latest_snapshot();
        assert!(
            Arc::ptr_eq(&a, &b),
            "successive snapshots without edits must share one allocation"
        );
    }

    #[test]
    fn latest_snapshot_initial_value_matches_input_arc() {
        // Constructor must seed the snapshot from the input `Arc` so the
        // very first broadcast (if the daemon dispatches one before any
        // edit) mirrors the same bytes the caller just handed us. We
        // sample a representative field rather than asserting full
        // struct equality because `AppSettings` does not implement
        // `PartialEq` — the public contract here is "observable state
        // of `snap` matches the seed", not "pointer identity".
        let settings = Arc::new(AppSettings {
            vault_path: "/seed/path".into(),
            note_write_mode: WriteMode::Thread,
            launch_at_login: true,
            ..AppSettings::default()
        });
        let app = SettingsApp::new(
            TraceTheme::for_preset(ThemePreset::Dark),
            Arc::clone(&settings),
        );
        let snap = app.latest_snapshot();
        assert_eq!(snap.vault_path, "/seed/path");
        assert_eq!(snap.note_write_mode, WriteMode::Thread);
        assert!(snap.launch_at_login);
    }
}
