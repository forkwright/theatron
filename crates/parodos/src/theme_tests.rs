use super::*;

struct TestEnv {
    vars: Vec<(&'static str, &'static str)>,
}

impl TestEnv {
    fn new(vars: &[(&'static str, &'static str)]) -> Self {
        Self {
            vars: vars.to_vec(),
        }
    }
}

impl Env for TestEnv {
    fn var(&self, name: &str) -> Option<String> {
        for (key, value) in &self.vars {
            if *key == name {
                return Some((*value).to_owned());
            }
        }
        None
    }
}

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
fn for_mode_with_env_auto_detects_mode_and_depth() {
    let env = TestEnv::new(&[("COLORFGBG", "0;15"), ("COLORTERM", "truecolor")]);
    let theme = Theme::for_mode_with_env(None, &env);

    assert_eq!(theme.mode, ThemeMode::Light);
    assert_eq!(theme.depth, ColorDepth::TrueColor);
}

#[test]
fn for_mode_with_env_respects_explicit_mode() {
    let env = TestEnv::new(&[("COLORFGBG", "0;15"), ("TERM", "xterm-256color")]);
    let theme = Theme::for_mode_with_env(Some(ThemeMode::Dark), &env);

    assert_eq!(theme.mode, ThemeMode::Dark);
    assert_eq!(theme.depth, ColorDepth::Color256);
}

#[test]
fn detect_background_uses_last_colorfgbg_component() {
    let light = TestEnv::new(&[("COLORFGBG", "15;0;8")]);
    let dark = TestEnv::new(&[("COLORFGBG", "15;8;0")]);

    assert_eq!(detect_background(&light), ThemeMode::Light);
    assert_eq!(detect_background(&dark), ThemeMode::Dark);
}

#[test]
fn detect_background_defaults_dark_when_missing_or_invalid() {
    let missing = TestEnv::new(&[]);
    let invalid = TestEnv::new(&[("COLORFGBG", "15;not-a-color")]);

    assert_eq!(detect_background(&missing), ThemeMode::Dark);
    assert_eq!(detect_background(&invalid), ThemeMode::Dark);
}

#[test]
fn detect_color_depth_detects_truecolor_env_vars() {
    let truecolor = TestEnv::new(&[("COLORTERM", "truecolor")]);
    let bit24 = TestEnv::new(&[("COLORTERM", "24bit")]);
    let term_program = TestEnv::new(&[("TERM_PROGRAM", "WezTerm")]);

    assert_eq!(detect_color_depth(&truecolor), ColorDepth::TrueColor);
    assert_eq!(detect_color_depth(&bit24), ColorDepth::TrueColor);
    assert_eq!(detect_color_depth(&term_program), ColorDepth::TrueColor);
}

#[test]
fn detect_color_depth_requires_supported_vte_version() {
    let supported = TestEnv::new(&[("VTE_VERSION", "3600")]);
    let newer = TestEnv::new(&[("VTE_VERSION", "7000")]);
    let old = TestEnv::new(&[("VTE_VERSION", "0001")]);
    let invalid = TestEnv::new(&[("VTE_VERSION", "vte-3600")]);

    assert_eq!(detect_color_depth(&supported), ColorDepth::TrueColor);
    assert_eq!(detect_color_depth(&newer), ColorDepth::TrueColor);
    assert_eq!(detect_color_depth(&old), ColorDepth::Basic);
    assert_eq!(detect_color_depth(&invalid), ColorDepth::Basic);
}

#[test]
fn detect_color_depth_detects_256_color_fallbacks() {
    let term = TestEnv::new(&[("TERM", "xterm-256color")]);
    let tmux = TestEnv::new(&[("TMUX", "/tmp/tmux-1000/default,123,0")]);

    assert_eq!(detect_color_depth(&term), ColorDepth::Color256);
    assert_eq!(detect_color_depth(&tmux), ColorDepth::Color256);
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
