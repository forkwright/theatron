use super::*;

#[test]
fn truecolor_palette_has_correct_depth() {
    let theme = Theme::truecolor();
    assert_eq!(theme.depth, ColorDepth::TrueColor);
    assert_eq!(theme.mode, ResolvedTheme::Dark);
}

#[test]
fn color256_palette_has_correct_depth() {
    let theme = Theme::color256();
    assert_eq!(theme.depth, ColorDepth::Color256);
    assert_eq!(theme.mode, ResolvedTheme::Dark);
}

#[test]
fn basic_palette_has_correct_depth() {
    let theme = Theme::basic();
    assert_eq!(theme.depth, ColorDepth::Basic);
    assert_eq!(theme.mode, ResolvedTheme::Dark);
}

#[test]
fn truecolor_light_palette_has_correct_mode() {
    let theme = Theme::truecolor_light();
    assert_eq!(theme.depth, ColorDepth::TrueColor);
    assert_eq!(theme.mode, ResolvedTheme::Light);
}

#[test]
fn color256_light_palette_has_correct_mode() {
    let theme = Theme::color256_light();
    assert_eq!(theme.depth, ColorDepth::Color256);
    assert_eq!(theme.mode, ResolvedTheme::Light);
}

#[test]
fn basic_light_palette_has_correct_mode() {
    let theme = Theme::basic_light();
    assert_eq!(theme.depth, ColorDepth::Basic);
    assert_eq!(theme.mode, ResolvedTheme::Light);
}

#[test]
fn for_mode_dark_returns_dark() {
    let theme = Theme::for_mode(Some(ResolvedTheme::Dark));
    assert_eq!(theme.mode, ResolvedTheme::Dark);
}

#[test]
fn for_mode_light_returns_light() {
    let theme = Theme::for_mode(Some(ResolvedTheme::Light));
    assert_eq!(theme.mode, ResolvedTheme::Light);
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

/// In-memory [`Env`] for detection tests.
struct TestEnv {
    vars: std::collections::HashMap<String, String>,
}

impl TestEnv {
    fn new(pairs: &[(&str, &str)]) -> Self {
        Self {
            vars: pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        }
    }
}

impl Env for TestEnv {
    fn var(&self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }
}

#[test]
fn colorterm_truecolor_detects_truecolor_depth() {
    let env = TestEnv::new(&[("COLORTERM", "truecolor")]);
    assert_eq!(detect_color_depth(&env), ColorDepth::TrueColor);
}

#[test]
fn colorterm_24bit_detects_truecolor_depth() {
    let env = TestEnv::new(&[("COLORTERM", "24bit")]);
    assert_eq!(detect_color_depth(&env), ColorDepth::TrueColor);
}

#[test]
fn term_program_kitty_detects_truecolor_depth() {
    let env = TestEnv::new(&[("TERM_PROGRAM", "kitty")]);
    assert_eq!(detect_color_depth(&env), ColorDepth::TrueColor);
}

#[test]
fn vte_version_at_or_above_3600_detects_truecolor() {
    let env = TestEnv::new(&[("VTE_VERSION", "3600")]);
    assert_eq!(detect_color_depth(&env), ColorDepth::TrueColor);
    let env = TestEnv::new(&[("VTE_VERSION", "7800")]);
    assert_eq!(detect_color_depth(&env), ColorDepth::TrueColor);
}

#[test]
fn vte_version_below_3600_is_not_truecolor() {
    // VTE 0.28 (GNOME Terminal pre-3.12) predates TrueColor support.
    let env = TestEnv::new(&[("VTE_VERSION", "2800"), ("TERM", "xterm-256color")]);
    assert_eq!(detect_color_depth(&env), ColorDepth::Color256);
    let env = TestEnv::new(&[("VTE_VERSION", "0001")]);
    assert_eq!(detect_color_depth(&env), ColorDepth::Basic);
}

#[test]
fn vte_version_non_numeric_is_ignored() {
    let env = TestEnv::new(&[("VTE_VERSION", "not-a-version")]);
    assert_eq!(detect_color_depth(&env), ColorDepth::Basic);
}

#[test]
fn term_256color_detects_color256_depth() {
    let env = TestEnv::new(&[("TERM", "xterm-256color")]);
    assert_eq!(detect_color_depth(&env), ColorDepth::Color256);
}

#[test]
fn tmux_detects_color256_depth() {
    let env = TestEnv::new(&[("TMUX", "/tmp/tmux-1000/default,1234,0")]);
    assert_eq!(detect_color_depth(&env), ColorDepth::Color256);
}

#[test]
fn empty_env_detects_basic_depth_and_dark_background() {
    let env = TestEnv::new(&[]);
    assert_eq!(detect_color_depth(&env), ColorDepth::Basic);
    assert_eq!(detect_background(&env), ResolvedTheme::Dark);
}

#[test]
fn colorfgbg_light_background_detected() {
    let env = TestEnv::new(&[("COLORFGBG", "0;15")]);
    assert_eq!(detect_background(&env), ResolvedTheme::Light);
}

#[test]
fn colorfgbg_dark_background_detected() {
    let env = TestEnv::new(&[("COLORFGBG", "15;0")]);
    assert_eq!(detect_background(&env), ResolvedTheme::Dark);
}

#[test]
fn colorfgbg_bg_index_7_is_light() {
    // WHY (#183): the documented boundary is "indices 0-6 dark, 7+ light";
    // bg=7 previously fell through to Dark due to an off-by-one `>= 8` check.
    let env = TestEnv::new(&[("COLORFGBG", "0;7")]);
    assert_eq!(detect_background(&env), ResolvedTheme::Light);
}

#[test]
fn colorfgbg_bg_index_6_is_dark() {
    // Boundary companion to the bg==7 case above: 6 stays on the dark side.
    let env = TestEnv::new(&[("COLORFGBG", "0;6")]);
    assert_eq!(detect_background(&env), ResolvedTheme::Dark);
}

#[test]
fn colorfgbg_three_component_uses_last_value() {
    let env = TestEnv::new(&[("COLORFGBG", "15;0;0")]);
    assert_eq!(detect_background(&env), ResolvedTheme::Dark);
}

#[test]
fn colorfgbg_garbage_defaults_to_dark() {
    let env = TestEnv::new(&[("COLORFGBG", "default;default")]);
    assert_eq!(detect_background(&env), ResolvedTheme::Dark);
}

#[test]
fn for_mode_with_env_auto_detects_light_truecolor() {
    let env = TestEnv::new(&[("COLORTERM", "truecolor"), ("COLORFGBG", "0;15")]);
    let theme = Theme::for_mode_with_env(None, &env);
    assert_eq!(theme.mode, ResolvedTheme::Light);
    assert_eq!(theme.depth, ColorDepth::TrueColor);
}

#[test]
fn for_mode_with_env_explicit_mode_overrides_detection() {
    let env = TestEnv::new(&[("COLORTERM", "truecolor"), ("COLORFGBG", "0;15")]);
    let theme = Theme::for_mode_with_env(Some(ResolvedTheme::Dark), &env);
    assert_eq!(theme.mode, ResolvedTheme::Dark);
    assert_eq!(theme.depth, ColorDepth::TrueColor);
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
fn palette_groups_swap_as_a_unit_between_modes() {
    // WHY: the grouped-struct design exists so a mode change replaces every
    // group at once -- assert the groups genuinely differ across modes.
    let dark = Theme::truecolor();
    let light = Theme::truecolor_light();
    assert_ne!(dark.text.fg, light.text.fg);
    assert_ne!(dark.colors.accent, light.colors.accent);
    assert_ne!(dark.borders.normal, light.borders.normal);
    assert_ne!(dark.status.error, light.status.error);
    assert_ne!(dark.code.bg, light.code.bg);
    assert_ne!(dark.thinking.fg, light.thinking.fg);
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
fn resolved_theme_predicates_are_mutually_exclusive() {
    for mode in ResolvedTheme::all() {
        assert_ne!(mode.is_dark(), mode.is_light());
    }
}

#[test]
fn reexported_from_label_recognizes_lowercase_config_strings() {
    // The pre-#129 parodos parser accepted lowercase "dark"/"light";
    // the unified themelion::ThemeMode::from_label must keep that.
    assert_eq!(ThemeMode::from_label("dark"), Some(ThemeMode::Dark));
    assert_eq!(ThemeMode::from_label("light"), Some(ThemeMode::Light));
    assert_eq!(ThemeMode::from_label("DARK"), Some(ThemeMode::Dark));
    assert_eq!(ThemeMode::from_label("LIGHT"), Some(ThemeMode::Light));
    assert_eq!(ThemeMode::from_label(""), None);
    assert_eq!(ThemeMode::from_label("auto"), None);
    assert_eq!(ThemeMode::from_label("dark "), None);
}

#[test]
fn resolved_theme_all_covers_the_terminal_palette_domain() {
    let variants = ResolvedTheme::all();
    assert_eq!(variants.len(), 2);
    assert!(variants.contains(&ResolvedTheme::Dark));
    assert!(variants.contains(&ResolvedTheme::Light));
}

#[test]
fn forced_bridges_preference_to_terminal_palette() {
    // The canonical round-trip for a TUI consumer: parse a stored
    // preference, then let System fall back to terminal detection.
    let env = TestEnv::new(&[("COLORTERM", "truecolor"), ("COLORFGBG", "0;15")]);

    let dark = ThemeMode::from_label("dark").and_then(ThemeMode::forced);
    assert_eq!(
        Theme::for_mode_with_env(dark, &env).mode,
        ResolvedTheme::Dark
    );

    let system = ThemeMode::from_label("system").and_then(ThemeMode::forced);
    assert_eq!(system, None);
    // System defers to the terminal environment (light COLORFGBG here).
    assert_eq!(
        Theme::for_mode_with_env(system, &env).mode,
        ResolvedTheme::Light
    );
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
