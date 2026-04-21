//! 跨模块图标 glyph 全局唯一性测试。
//!
//! Windows 侧 Settings 窗口同时渲染 Theme 卡片(内含 [`ThemePreset`] 图标)
//! 和 Storage 卡片(内含 [`WriteMode`] 图标),如果两类枚举选中相同 unicode
//! 码位作为图标,用户在同一屏上会产生视觉混淆。放在 `tests/` 目录是为了
//! **强制跨模块联动**:两个枚举各自的单测只检查**本模块内**的唯一性,而
//! 这个集成测试拉通两者,任何一个模块单方面改 glyph 却忘了更新另一个都
//! 会在这里失败。
//!
//! 相关 issue 回顾:`WriteMode::File` 原本是 `"\u{25A4}"`(▤),与
//! `ThemePreset::Paper` 重复,已在 P12 sub-task 3 质量修复里把 File 改为
//! `"\u{1F4C4}"`(📄)。这个测试就是为了**防止未来再次回归**到重复状态。
//!
//! 加入新枚举:
//! * 扩展下面的 `glyphs()` vec;
//! * 任何失败意味着新 glyph 与现有集合撞车,换一个 unicode 码位即可。

use trace_core::{ThemePreset, WriteMode};

/// 汇总所有在设置面板上可能同屏出现的图标 glyph。
fn glyphs() -> Vec<&'static str> {
    let mut all = Vec::new();
    for m in [WriteMode::Dimension, WriteMode::Thread, WriteMode::File] {
        all.push(m.icon_glyph());
    }
    for p in [
        ThemePreset::Light,
        ThemePreset::Dark,
        ThemePreset::Paper,
        ThemePreset::Dune,
    ] {
        all.push(p.icon_glyph());
    }
    all
}

#[test]
fn write_mode_and_theme_preset_glyphs_are_globally_unique() {
    let all = glyphs();
    let unique: std::collections::HashSet<_> = all.iter().copied().collect();
    assert_eq!(
        all.len(),
        unique.len(),
        "跨模块 glyph 必须全局唯一: {all:?}"
    );
}

#[test]
fn every_glyph_is_non_empty() {
    // 守护:任何变体返回空串都会让 UI 上那一格塌掉。
    for g in glyphs() {
        assert!(!g.is_empty(), "glyph 必须非空,拿到 {g:?}");
    }
}
