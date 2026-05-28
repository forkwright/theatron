use super::*;

#[test]
fn truecolor_palette_has_correct_depth() {
    let theme = Theme::truecolor();
    assert_eq!(theme.depth, ColorDepth::TrueColor);
    assert_eq!(theme.mode, ThemeMode::Dark);
}

#[test]
fn color256_palette_has_correct_depth() {
    let theme = Theme::color256();
    assert_eq!(theme.depth, ColorDepth::Color256);
    assert_eq!(theme.mode, ThemeMode::Dark);
}

#[test]
fn basic_palette_has_correct_depth() {
    let theme = Theme::basic();
    assert_eq!(theme.depth, ColorDepth::Basic);
    assert_eq!(theme.mode, ThemeMode::Dark);
}

#[test]
fn truecolor_light_palette_has_correct_mode() {
    let theme = Theme::truecolor_light();
    assert_eq!(theme.depth, ColorDepth::TrueColor);
    assert_eq!(theme.mode, ThemeMode::Light);
}

#[test]
fn color256_light_palette_has_correct_mode() {
    let theme = Theme::color256_light();
    assert_eq!(theme.depth, ColorDepth::Color256);
    assert_eq!(theme.mode, ThemeMode::Light);
}

#[test]
fn basic_light_palette_has_correct_mode() {
    let theme = Theme::basic_light();
    assert_eq!(theme.depth, ColorDepth::Basic);
    assert_eq!(theme.mode, ThemeMode::Light);
}

#[test]
fn for_mode_dark_returns_dark() {
    let theme = Theme::for_mode(Some(ThemeMode::Dark));
    assert_eq!(theme.mode, ThemeMode::Dark);
}

#[test]
fn for_mode_light_returns_light() {
    let theme = Theme::for_mode(Some(ThemeMode::Light));
    assert_eq!(theme.mode, ThemeMode::Light);
}

#[test]
fn style_fg_uses_fg_color() {
    let theme = Theme::truecolor();
    let style = theme.style_fg();
    assert_eq!(style.fg, Some(theme.text.fg));
}

#[test]
fn style_muted_uses_fg_muted_color() {
    let theme = Theme::truecolor();
    let style = theme.style_muted();
    assert_eq!(style.fg, Some(theme.text.fg_muted));
}

#[test]
fn style_accent_bold_has_bold_modifier() {
    let theme = Theme::truecolor();
    let style = theme.style_accent_bold();
    assert!(style.add_modifier.contains(Modifier::BOLD));
    assert_eq!(style.fg, Some(theme.colors.accent));
}

#[test]
fn style_user_has_bold() {
    let theme = Theme::basic();
    let style = theme.style_user();
    assert!(style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn style_surface_sets_bg() {
    let theme = Theme::truecolor();
    let style = theme.style_surface();
    assert_eq!(style.bg, Some(theme.colors.surface));
}

#[test]
fn style_inline_code_uses_warning_fg() {
    let theme = Theme::truecolor();
    let style = theme.style_inline_code();
    assert_eq!(style.fg, Some(theme.status.warning));
    assert_eq!(style.bg, Some(theme.code.bg));
}

#[test]
fn spinner_frame_cycles() {
    let f0 = spinner_frame(0);
    let f3 = spinner_frame(3);
    assert_ne!(f0, f3);
    // After a full cycle, it wraps
    let total = BRAILLE_SPINNER.len() * 3;
    assert_eq!(
        spinner_frame(0),
        spinner_frame(u64::try_from(total).unwrap_or(u64::MAX))
    );
}

#[test]
fn spinner_frame_all_braille() {
    for frame in BRAILLE_SPINNER {
        assert!(
            ('\u{2800}'..='\u{28FF}').contains(frame),
            "spinner frame {frame:?} is not a braille character"
        );
    }
}

#[test]
fn detect_returns_valid_depth() {
    let theme = Theme::detect();
    let _ = theme.depth;
}

#[test]
fn all_palettes_have_reset_bg() {
    for theme in [
        Theme::truecolor(),
        Theme::color256(),
        Theme::basic(),
        Theme::truecolor_light(),
        Theme::color256_light(),
        Theme::basic_light(),
    ] {
        assert_eq!(theme.colors.bg, Color::Reset);
    }
}

#[test]
fn light_palettes_have_dark_text() {
    let theme = Theme::truecolor_light();
    // Text on a light background must be dark
    if let Color::Rgb(r, g, b) = theme.text.fg {
        assert!(
            r < 100 && g < 100 && b < 100,
            "light theme fg should be dark: ({r}, {g}, {b})"
        );
    }
}

#[test]
fn theme_static_is_accessible() {
    let _ = THEME.depth;
}

#[test]
fn struct_of_structs_groups_are_populated() {
    let theme = Theme::truecolor();
    let _ = theme.colors.accent;
    let _ = theme.text.fg;
    let _ = theme.borders.normal;
    let _ = theme.status.success;
    let _ = theme.code.fg;
    let _ = theme.thinking.fg;
}

#[test]
fn color_depth_is_truecolor_returns_true_only_for_truecolor() {
    assert!(ColorDepth::TrueColor.is_truecolor());
    assert!(!ColorDepth::Color256.is_truecolor());
    assert!(!ColorDepth::Basic.is_truecolor());
}

#[test]
fn color_depth_is_256_returns_true_only_for_color256() {
    assert!(ColorDepth::Color256.is_256());
    assert!(!ColorDepth::TrueColor.is_256());
    assert!(!ColorDepth::Basic.is_256());
}

#[test]
fn color_depth_is_basic_returns_true_only_for_basic() {
    assert!(ColorDepth::Basic.is_basic());
    assert!(!ColorDepth::TrueColor.is_basic());
    assert!(!ColorDepth::Color256.is_basic());
}

#[test]
fn color_depth_has_256_returns_true_for_truecolor_and_256() {
    assert!(ColorDepth::TrueColor.has_256());
    assert!(ColorDepth::Color256.has_256());
    assert!(!ColorDepth::Basic.has_256());
}

#[test]
fn theme_mode_is_dark_returns_true_only_for_dark() {
    assert!(ThemeMode::Dark.is_dark());
    assert!(!ThemeMode::Light.is_dark());
}

#[test]
fn theme_mode_is_light_returns_true_only_for_light() {
    assert!(ThemeMode::Light.is_light());
    assert!(!ThemeMode::Dark.is_light());
}

#[test]
fn theme_mode_predicates_are_mutually_exclusive() {
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        assert_ne!(mode.is_dark(), mode.is_light());
    }
}

#[test]
fn theme_mode_from_label_recognizes_canonical_lowercase() {
    assert_eq!(ThemeMode::from_label("dark"), Some(ThemeMode::Dark));
    assert_eq!(ThemeMode::from_label("light"), Some(ThemeMode::Light));
}

#[test]
fn theme_mode_from_label_is_case_insensitive() {
    assert_eq!(ThemeMode::from_label("Dark"), Some(ThemeMode::Dark));
    assert_eq!(ThemeMode::from_label("DARK"), Some(ThemeMode::Dark));
    assert_eq!(ThemeMode::from_label("Light"), Some(ThemeMode::Light));
    assert_eq!(ThemeMode::from_label("LIGHT"), Some(ThemeMode::Light));
}

#[test]
fn theme_mode_from_label_returns_none_for_unrecognized_labels() {
    assert_eq!(ThemeMode::from_label(""), None);
    assert_eq!(ThemeMode::from_label("system"), None);
    assert_eq!(ThemeMode::from_label("auto"), None);
    assert_eq!(ThemeMode::from_label("nope"), None);
    assert_eq!(ThemeMode::from_label("dark "), None);
}

#[test]
fn theme_mode_all_returns_every_variant() {
    let variants = ThemeMode::all();
    assert_eq!(variants.len(), 2);
    assert!(variants.contains(&ThemeMode::Dark));
    assert!(variants.contains(&ThemeMode::Light));
}

#[test]
fn theme_mode_all_round_trips_through_from_label_and_display() {
    for mode in ThemeMode::all() {
        let label = match mode {
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        };
        assert_eq!(ThemeMode::from_label(label), Some(mode));
    }
}

#[test]
fn color_depth_predicates_form_an_exhaustive_partition() {
    // Exactly one of is_truecolor / is_256 / is_basic is true
    // for any given variant.
    for depth in [
        ColorDepth::TrueColor,
        ColorDepth::Color256,
        ColorDepth::Basic,
    ] {
        let count = u32::from(depth.is_truecolor())
            + u32::from(depth.is_256())
            + u32::from(depth.is_basic());
        assert_eq!(count, 1, "exactly one predicate true for {depth:?}");
    }
}
