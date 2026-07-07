//! Canonical fleet theme vocabulary + OS preference detection.
//!
//! Owns the fleet-wide theme types: [`ThemeMode`] (user preference,
//! Dark/Light/System) and [`ResolvedTheme`] (concrete brightness after
//! resolving `System`). Terminal consumers (`parodos`) re-export these
//! with default features off; the Dioxus components (`ThemeProvider`,
//! `ThemeToggle`) live in `crate::provider` behind the `dioxus`
//! feature.

/// User-selected theme preference.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    /// Force dark mode regardless of system preference.
    Dark,
    /// Force light mode regardless of system preference.
    Light,
    /// Follow the OS/desktop environment preference.
    System,
}

/// Concrete theme after resolving system preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResolvedTheme {
    /// Dark theme.
    Dark,
    /// Light theme.
    Light,
}

impl ResolvedTheme {
    /// Returns `"dark"` or `"light"` — matches the `[data-theme="…"]`
    /// CSS selector value applied by `ThemeProvider` (available with
    /// the `dioxus` feature).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    /// Whether the resolved theme is dark.
    ///
    /// Convenience predicate: `theme.is_dark()` reads better than
    /// `theme == ResolvedTheme::Dark` at consumer call sites.
    #[must_use]
    pub const fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }

    /// Whether the resolved theme is light.
    ///
    /// Convenience predicate: `theme.is_light()` reads better than
    /// `theme == ResolvedTheme::Light` at consumer call sites.
    #[must_use]
    pub const fn is_light(self) -> bool {
        matches!(self, Self::Light)
    }

    /// Parse a `ResolvedTheme` from its [`as_str`](Self::as_str) value
    /// — i.e. the lowercase `[data-theme="…"]` attribute value that
    /// `ThemeProvider` applies to the DOM.
    ///
    /// Recognises `"dark"` and `"light"` (case-sensitive — they
    /// match the canonical attribute lowercase). Returns `None`
    /// for any other input.
    ///
    /// Round-trips with [`as_str`](Self::as_str) for any
    /// `ResolvedTheme`:
    ///
    /// ```
    /// use themelion::ResolvedTheme;
    /// for theme in [ResolvedTheme::Dark, ResolvedTheme::Light] {
    ///     assert_eq!(ResolvedTheme::parse_data_attr(theme.as_str()), Some(theme));
    /// }
    /// ```
    ///
    /// Useful for consumers reading the `[data-theme]` attribute
    /// back off the DOM (e.g. tests, settings round-trip). The
    /// name is deliberately specific — `from_str` would clash with
    /// `std::str::FromStr` conventions, and the function only
    /// accepts the canonical attribute value, not arbitrary
    /// strings.
    ///
    /// Parallels `themelion::ThemeMode::from_label` (PR #57).
    #[must_use]
    pub fn parse_data_attr(s: &str) -> Option<Self> {
        match s {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            _ => None,
        }
    }

    /// All `ResolvedTheme` variants, in canonical order:
    /// `[Dark, Light]`.
    ///
    /// Useful for exhaustiveness tests and any consumer iterating
    /// every possible resolved value. Symmetric with
    /// `themelion::ThemeMode::all` (PR #57); the resolved array has
    /// two elements (no `System` since this is the post-resolve
    /// enum).
    #[must_use]
    pub const fn all() -> [Self; 2] {
        [Self::Dark, Self::Light]
    }
}

impl ThemeMode {
    /// Whether this mode is `Dark`.
    ///
    /// Note this is the *user preference* — to ask whether the
    /// rendered theme is dark (after resolving `System`), call
    /// [`resolve`](Self::resolve)`().is_dark()`.
    #[must_use]
    pub const fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }

    /// Whether this mode is `Light`.
    ///
    /// Note this is the *user preference* — to ask whether the
    /// rendered theme is light (after resolving `System`), call
    /// [`resolve`](Self::resolve)`().is_light()`.
    #[must_use]
    pub const fn is_light(self) -> bool {
        matches!(self, Self::Light)
    }

    /// Whether this mode follows the OS preference (`System`).
    ///
    /// `is_system()` is true exactly when [`resolve`](Self::resolve)
    /// would consult the desktop-environment preference rather
    /// than returning a forced value.
    #[must_use]
    pub const fn is_system(self) -> bool {
        matches!(self, Self::System)
    }

    /// Cycle to the next mode: Dark -> Light -> System -> Dark.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::System,
            Self::System => Self::Dark,
        }
    }

    /// Resolve to a concrete theme, evaluating system preference when needed.
    #[must_use]
    pub fn resolve(self) -> ResolvedTheme {
        match self {
            Self::Dark => ResolvedTheme::Dark,
            Self::Light => ResolvedTheme::Light,
            Self::System => detect_system_preference(),
        }
    }

    /// Human-readable label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::System => "System",
        }
    }

    /// Unicode icon for the current mode.
    #[must_use]
    pub fn icon(self) -> &'static str {
        match self {
            Self::Dark => "\u{263E}",
            Self::Light => "\u{2600}",
            Self::System => "\u{25D0}",
        }
    }

    /// Parse a [`ThemeMode`] from its [`label`](Self::label) string.
    ///
    /// Returns `None` if the input doesn't match any known label.
    /// Useful for consumers round-tripping the mode through a
    /// settings-storage layer that persists the human-readable
    /// label (e.g. `bathron::settings`).
    ///
    /// Recognized labels (case-insensitive): `"Dark"`, `"Light"`,
    /// `"System"`. This is the forgiving human-input channel — it
    /// accepts any casing so hand-edited settings files and legacy
    /// lowercase stores parse without pre-normalization. For a strict
    /// wire format use [`slug`](Self::slug) / [`from_slug`](Self::from_slug).
    ///
    /// # Examples
    ///
    /// ```
    /// use themelion::ThemeMode;
    /// assert_eq!(ThemeMode::from_label("Dark"), Some(ThemeMode::Dark));
    /// assert_eq!(ThemeMode::from_label("dark"), Some(ThemeMode::Dark));
    /// assert_eq!(ThemeMode::from_label("SYSTEM"), Some(ThemeMode::System));
    /// assert_eq!(ThemeMode::from_label("garbage"), None);
    /// ```
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        match label.to_ascii_lowercase().as_str() {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            "system" => Some(Self::System),
            _ => None,
        }
    }

    /// The concrete theme this preference forces, or `None` when it
    /// defers to the environment (`System`).
    ///
    /// Bridge for consumers with their own environment-detection seam:
    /// [`resolve`](Self::resolve) consults the *desktop* environment
    /// (`GTK_THEME` / `COLORFGBG`) for `System`, which is the wrong
    /// probe for a terminal app. `parodos::Theme::for_mode(mode.forced())`
    /// instead lets the terminal substrate run its own detection when
    /// the preference is `System`.
    ///
    /// # Examples
    ///
    /// ```
    /// use themelion::{ResolvedTheme, ThemeMode};
    /// assert_eq!(ThemeMode::Dark.forced(), Some(ResolvedTheme::Dark));
    /// assert_eq!(ThemeMode::Light.forced(), Some(ResolvedTheme::Light));
    /// assert_eq!(ThemeMode::System.forced(), None);
    /// ```
    #[must_use]
    pub const fn forced(self) -> Option<ResolvedTheme> {
        match self {
            Self::Dark => Some(ResolvedTheme::Dark),
            Self::Light => Some(ResolvedTheme::Light),
            Self::System => None,
        }
    }

    /// All three theme modes in canonical order: Dark, Light, System.
    ///
    /// Useful for rendering a complete settings-selector UI without
    /// hard-coding the variant list at the call site.
    ///
    /// # Examples
    ///
    /// ```
    /// use themelion::ThemeMode;
    /// for mode in ThemeMode::all() {
    ///     println!("{}", mode.label());
    /// }
    /// ```
    #[must_use]
    pub const fn all() -> [Self; 3] {
        [Self::Dark, Self::Light, Self::System]
    }

    /// Lowercase storage slug suitable for config files (`"dark"`,
    /// `"light"`, `"system"`).
    ///
    /// Companion to [`from_slug`](Self::from_slug) for round-tripping
    /// the mode through a config / settings layer that wants a
    /// stable, lowercase, label-independent wire format. Distinct
    /// from [`label`](Self::label) (which produces human-facing
    /// `"Dark"` / `"Light"` / `"System"` for UI display).
    ///
    /// # Examples
    ///
    /// ```
    /// use themelion::ThemeMode;
    ///
    /// assert_eq!(ThemeMode::Dark.slug(), "dark");
    /// assert_eq!(ThemeMode::Light.slug(), "light");
    /// assert_eq!(ThemeMode::System.slug(), "system");
    /// ```
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::System => "system",
        }
    }

    /// Parse a lowercase storage slug back into a [`ThemeMode`].
    ///
    /// Returns `None` if the input doesn't match `"dark"`, `"light"`,
    /// or `"system"`. The match is **case-sensitive** and intended
    /// for config-file parsing — consumers persisting display
    /// labels should use [`from_label`](Self::from_label) instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use themelion::ThemeMode;
    ///
    /// assert_eq!(ThemeMode::from_slug("dark"), Some(ThemeMode::Dark));
    /// assert_eq!(ThemeMode::from_slug("Dark"), None); // case-sensitive
    /// assert_eq!(ThemeMode::from_slug("garbage"), None);
    /// ```
    #[must_use]
    pub fn from_slug(s: &str) -> Option<Self> {
        match s {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            "system" => Some(Self::System),
            _ => None,
        }
    }
}

/// Detect OS color preference from environment variables.
///
/// Checks `GTK_THEME` for a `:dark` variant suffix or a `-dark` name
/// suffix (GTK3/GTK4 convention), then `COLORFGBG` for background
/// brightness (same heuristic as the TUI). Falls back to dark.
fn detect_system_preference() -> ResolvedTheme {
    detect_system_preference_from(|name| std::env::var(name).ok())
}

/// Environment-injectable core of [`detect_system_preference`].
///
/// `env` returns the value of an environment variable, or `None` when
/// unset. Tests pass a closure over controlled values — `std::env::set_var`
/// is `unsafe` in edition 2024, so mutation-based env testing is not an
/// option. Mirrors the `Env`-trait seam in `parodos::env`.
fn detect_system_preference_from(env: impl Fn(&str) -> Option<String>) -> ResolvedTheme {
    if let Some(gtk_theme) = env("GTK_THEME") {
        // WHY: GTK selects a theme's dark variant via a `:dark` suffix
        // (`GTK_THEME=Adwaita:dark`); standalone dark themes ship as
        // `<Name>-dark` packages. A bare substring match would
        // misclassify light-variant themes named e.g. `Darkly`.
        let lower = gtk_theme.to_ascii_lowercase();
        let is_dark = lower.ends_with(":dark") || lower.ends_with("-dark") || lower == "dark";
        return if is_dark {
            ResolvedTheme::Dark
        } else {
            ResolvedTheme::Light
        };
    }

    // WHY: COLORFGBG format is "fg;bg" or "fg;X;bg". Background is always
    // the last component. Indices 0-7 are dark, 8+ are light. Matches the
    // TUI detection logic in koilon/src/theme.rs.
    if let Some(val) = env("COLORFGBG")
        && let Some(bg_str) = val.rsplit(';').next()
        && let Ok(bg) = bg_str.parse::<u8>()
    {
        return if bg >= 8 {
            ResolvedTheme::Light
        } else {
            ResolvedTheme::Dark
        };
    }

    ResolvedTheme::Dark
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_dark_returns_dark() {
        assert_eq!(ThemeMode::Dark.resolve(), ResolvedTheme::Dark);
    }

    #[test]
    fn resolve_light_returns_light() {
        assert_eq!(ThemeMode::Light.resolve(), ResolvedTheme::Light);
    }

    #[test]
    fn resolved_as_str_matches_css_selectors() {
        assert_eq!(ResolvedTheme::Dark.as_str(), "dark");
        assert_eq!(ResolvedTheme::Light.as_str(), "light");
    }

    #[test]
    fn labels_are_nonempty() {
        for mode in [ThemeMode::Dark, ThemeMode::Light, ThemeMode::System] {
            assert!(!mode.label().is_empty());
        }
    }

    #[test]
    fn icons_are_nonempty() {
        for mode in [ThemeMode::Dark, ThemeMode::Light, ThemeMode::System] {
            assert!(!mode.icon().is_empty());
        }
    }

    #[test]
    fn system_resolve_returns_valid_theme() {
        // Exercises the real-environment path end-to-end (no panic);
        // branch behavior is covered by the injected-env tests below.
        let resolved = ThemeMode::System.resolve();
        assert!(resolved == ResolvedTheme::Dark || resolved == ResolvedTheme::Light);
    }

    /// Injected env: returns values from `pairs`, `None` otherwise.
    fn env_from(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_string())
        }
    }

    #[test]
    fn gtk_theme_colon_dark_variant_resolves_dark() {
        let env = env_from(&[("GTK_THEME", "Adwaita:dark")]);
        assert_eq!(detect_system_preference_from(env), ResolvedTheme::Dark);
    }

    #[test]
    fn gtk_theme_dash_dark_suffix_resolves_dark() {
        let env = env_from(&[("GTK_THEME", "Arc-Dark")]);
        assert_eq!(detect_system_preference_from(env), ResolvedTheme::Dark);
    }

    #[test]
    fn gtk_theme_bare_dark_resolves_dark() {
        let env = env_from(&[("GTK_THEME", "dark")]);
        assert_eq!(detect_system_preference_from(env), ResolvedTheme::Dark);
    }

    #[test]
    fn gtk_theme_without_dark_suffix_resolves_light() {
        let env = env_from(&[("GTK_THEME", "Adwaita")]);
        assert_eq!(detect_system_preference_from(env), ResolvedTheme::Light);
    }

    #[test]
    fn gtk_theme_dark_named_theme_without_variant_resolves_light() {
        // Substring matching would misclassify these as dark (#127):
        // a dark-sounding theme *name* is not the `:dark` variant.
        for theme in ["Darkly", "Darkroom", "Arc-Darker"] {
            let env = move |name: &str| (name == "GTK_THEME").then(|| theme.to_string());
            assert_eq!(
                detect_system_preference_from(env),
                ResolvedTheme::Light,
                "GTK_THEME={theme} should resolve Light"
            );
        }
    }

    #[test]
    fn gtk_theme_takes_precedence_over_colorfgbg() {
        let env = env_from(&[("GTK_THEME", "Adwaita"), ("COLORFGBG", "15;0")]);
        assert_eq!(detect_system_preference_from(env), ResolvedTheme::Light);
    }

    #[test]
    fn colorfgbg_dark_background_resolves_dark() {
        let env = env_from(&[("COLORFGBG", "15;0")]);
        assert_eq!(detect_system_preference_from(env), ResolvedTheme::Dark);
    }

    #[test]
    fn colorfgbg_light_background_resolves_light() {
        // Three-component form "fg;X;bg" — background is the last field.
        let env = env_from(&[("COLORFGBG", "15;0;12")]);
        assert_eq!(detect_system_preference_from(env), ResolvedTheme::Light);
    }

    #[test]
    fn colorfgbg_threshold_boundary_is_eight() {
        let dark = env_from(&[("COLORFGBG", "15;7")]);
        let light = env_from(&[("COLORFGBG", "15;8")]);
        assert_eq!(detect_system_preference_from(dark), ResolvedTheme::Dark);
        assert_eq!(detect_system_preference_from(light), ResolvedTheme::Light);
    }

    #[test]
    fn colorfgbg_unparseable_background_falls_back_to_dark() {
        let env = env_from(&[("COLORFGBG", "15;default")]);
        assert_eq!(detect_system_preference_from(env), ResolvedTheme::Dark);
    }

    #[test]
    fn no_theme_env_vars_falls_back_to_dark() {
        let env = env_from(&[]);
        assert_eq!(detect_system_preference_from(env), ResolvedTheme::Dark);
    }

    #[test]
    fn next_cycles_dark_light_system_dark() {
        assert_eq!(ThemeMode::Dark.next(), ThemeMode::Light);
        assert_eq!(ThemeMode::Light.next(), ThemeMode::System);
        assert_eq!(ThemeMode::System.next(), ThemeMode::Dark);
    }

    #[test]
    fn next_full_cycle_returns_to_start() {
        let start = ThemeMode::Dark;
        let cycled = start.next().next().next();
        assert_eq!(cycled, start);
    }

    #[test]
    fn label_values_match_user_facing_strings() {
        assert_eq!(ThemeMode::Dark.label(), "Dark");
        assert_eq!(ThemeMode::Light.label(), "Light");
        assert_eq!(ThemeMode::System.label(), "System");
    }

    #[test]
    fn icon_values_are_distinct_unicode_glyphs() {
        let dark = ThemeMode::Dark.icon();
        let light = ThemeMode::Light.icon();
        let system = ThemeMode::System.icon();
        assert_ne!(dark, light);
        assert_ne!(light, system);
        assert_ne!(system, dark);
        // Each is a single Unicode scalar, not an empty string or a multi-char sequence.
        assert_eq!(dark.chars().count(), 1);
        assert_eq!(light.chars().count(), 1);
        assert_eq!(system.chars().count(), 1);
    }

    #[test]
    fn resolved_theme_eq_and_copy_are_derived() {
        // Compile-time check: ResolvedTheme is Copy + Eq (the
        // derive in the source must hold). Used by chalkeion /
        // proskenion view code to copy across closure boundaries.
        fn assert_copy<T: Copy>() {}
        fn assert_eq_trait<T: Eq>() {}
        assert_copy::<ResolvedTheme>();
        assert_copy::<ThemeMode>();
        assert_eq_trait::<ResolvedTheme>();
        assert_eq_trait::<ThemeMode>();

        // Runtime confirmation: a copied value equals its source and
        // the source remains usable after the copy (move would fail).
        let source = ResolvedTheme::Dark;
        let copied = source;
        assert_eq!(source, copied);
        let mode_source = ThemeMode::System;
        let mode_copied = mode_source;
        assert_eq!(mode_source, mode_copied);
    }

    #[test]
    fn resolve_is_pure_for_dark_and_light() {
        // Calling resolve repeatedly on Dark / Light returns the
        // same value (pure; no side effects, no state).
        for _ in 0..3 {
            assert_eq!(ThemeMode::Dark.resolve(), ResolvedTheme::Dark);
            assert_eq!(ThemeMode::Light.resolve(), ResolvedTheme::Light);
        }
    }

    #[test]
    fn theme_mode_debug_includes_variant_name() {
        // Debug derive yields the variant name verbatim. Useful
        // for logging in consumer code.
        assert_eq!(format!("{:?}", ThemeMode::Dark), "Dark");
        assert_eq!(format!("{:?}", ThemeMode::Light), "Light");
        assert_eq!(format!("{:?}", ThemeMode::System), "System");
    }

    #[test]
    fn from_label_round_trips_each_variant() {
        for mode in ThemeMode::all() {
            assert_eq!(ThemeMode::from_label(mode.label()), Some(mode));
        }
    }

    #[test]
    fn from_label_is_case_insensitive() {
        // Unified semantics (#129): from_label is the forgiving
        // human-input channel; from_slug is the strict wire format.
        assert_eq!(ThemeMode::from_label("dark"), Some(ThemeMode::Dark));
        assert_eq!(ThemeMode::from_label("DARK"), Some(ThemeMode::Dark));
        assert_eq!(ThemeMode::from_label("Dark"), Some(ThemeMode::Dark));
        assert_eq!(ThemeMode::from_label("system"), Some(ThemeMode::System));
        assert_eq!(ThemeMode::from_label("LIGHT"), Some(ThemeMode::Light));
    }

    #[test]
    fn forced_maps_fixed_preferences_and_defers_system() {
        assert_eq!(ThemeMode::Dark.forced(), Some(ResolvedTheme::Dark));
        assert_eq!(ThemeMode::Light.forced(), Some(ResolvedTheme::Light));
        assert_eq!(ThemeMode::System.forced(), None);
    }

    #[test]
    fn forced_agrees_with_resolve_for_fixed_preferences() {
        // For Dark/Light the two paths must never diverge; System is
        // exactly the case where forced() defers and resolve() probes.
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            assert_eq!(mode.forced(), Some(mode.resolve()));
        }
    }

    #[test]
    fn from_label_returns_none_for_unknown() {
        assert_eq!(ThemeMode::from_label(""), None);
        assert_eq!(ThemeMode::from_label("Auto"), None);
        assert_eq!(ThemeMode::from_label("garbage"), None);
        assert_eq!(ThemeMode::from_label("dark "), None);
    }

    #[test]
    fn all_returns_three_variants_in_canonical_order() {
        let modes = ThemeMode::all();
        assert_eq!(modes.len(), 3);
        assert_eq!(modes[0], ThemeMode::Dark);
        assert_eq!(modes[1], ThemeMode::Light);
        assert_eq!(modes[2], ThemeMode::System);
    }

    #[test]
    fn all_covers_every_variant_exhaustively() {
        // If a fourth variant is ever added, this loop forces a
        // compile-time consideration of whether it should appear in
        // all() — the match is exhaustive.
        let mut matched = 0;
        for mode in ThemeMode::all() {
            match mode {
                ThemeMode::Dark | ThemeMode::Light | ThemeMode::System => matched += 1,
            }
        }
        assert_eq!(matched, ThemeMode::all().len());
    }

    #[test]
    fn theme_mode_is_dark_true_only_for_dark() {
        assert!(ThemeMode::Dark.is_dark());
        assert!(!ThemeMode::Light.is_dark());
        assert!(!ThemeMode::System.is_dark());
    }

    #[test]
    fn theme_mode_is_light_true_only_for_light() {
        assert!(!ThemeMode::Dark.is_light());
        assert!(ThemeMode::Light.is_light());
        assert!(!ThemeMode::System.is_light());
    }

    #[test]
    fn theme_mode_is_system_true_only_for_system() {
        assert!(!ThemeMode::Dark.is_system());
        assert!(!ThemeMode::Light.is_system());
        assert!(ThemeMode::System.is_system());
    }

    #[test]
    fn theme_mode_predicates_form_an_exhaustive_partition() {
        // Exactly one of is_dark / is_light / is_system is true for
        // any given variant. Mirrors the ColorDepth partition test
        // in parodos and the ResolvedTheme mutual-exclusivity test.
        for mode in ThemeMode::all() {
            let count = u32::from(mode.is_dark())
                + u32::from(mode.is_light())
                + u32::from(mode.is_system());
            assert_eq!(count, 1, "exactly one predicate true for {mode:?}");
        }
    }

    #[test]
    fn theme_mode_slug_returns_lowercase_canonical() {
        assert_eq!(ThemeMode::Dark.slug(), "dark");
        assert_eq!(ThemeMode::Light.slug(), "light");
        assert_eq!(ThemeMode::System.slug(), "system");
    }

    #[test]
    fn theme_mode_from_slug_round_trips_with_slug() {
        for mode in ThemeMode::all() {
            assert_eq!(ThemeMode::from_slug(mode.slug()), Some(mode));
        }
    }

    #[test]
    fn theme_mode_from_slug_is_case_sensitive() {
        // Distinct from from_label (which takes "Dark") — slugs are
        // lowercase storage form.
        assert_eq!(ThemeMode::from_slug("Dark"), None);
        assert_eq!(ThemeMode::from_slug("DARK"), None);
        assert_eq!(ThemeMode::from_slug("Light"), None);
        assert_eq!(ThemeMode::from_slug("System"), None);
    }

    #[test]
    fn theme_mode_from_slug_returns_none_for_unrecognized() {
        assert_eq!(ThemeMode::from_slug(""), None);
        assert_eq!(ThemeMode::from_slug("auto"), None);
        assert_eq!(ThemeMode::from_slug("garbage"), None);
        assert_eq!(ThemeMode::from_slug("dark "), None);
    }

    #[test]
    fn theme_mode_slug_distinct_from_label() {
        // slug and label are intentionally different surfaces:
        // slug for config, label for UI.
        for mode in ThemeMode::all() {
            assert_ne!(mode.slug(), mode.label());
            // slug should be a lowercase variant of label
            assert_eq!(mode.slug(), mode.label().to_lowercase());
        }
    }

    #[test]
    fn resolved_theme_is_dark_returns_true_only_for_dark() {
        assert!(ResolvedTheme::Dark.is_dark());
        assert!(!ResolvedTheme::Light.is_dark());
    }

    #[test]
    fn resolved_theme_is_light_returns_true_only_for_light() {
        assert!(ResolvedTheme::Light.is_light());
        assert!(!ResolvedTheme::Dark.is_light());
    }

    #[test]
    fn resolved_theme_predicates_are_mutually_exclusive() {
        // is_dark and is_light are exhaustive partitions of
        // ResolvedTheme — exactly one is true for any value.
        for theme in [ResolvedTheme::Dark, ResolvedTheme::Light] {
            assert_ne!(theme.is_dark(), theme.is_light());
        }
    }

    #[test]
    fn resolved_theme_parse_data_attr_recognizes_canonical_values() {
        assert_eq!(
            ResolvedTheme::parse_data_attr("dark"),
            Some(ResolvedTheme::Dark)
        );
        assert_eq!(
            ResolvedTheme::parse_data_attr("light"),
            Some(ResolvedTheme::Light)
        );
    }

    #[test]
    fn resolved_theme_parse_data_attr_is_case_sensitive() {
        // Canonical [data-theme="..."] is lowercase; case-sensitive
        // parsing matches what's actually written. Same semantics
        // as ThemeMode::from_label.
        assert_eq!(ResolvedTheme::parse_data_attr("Dark"), None);
        assert_eq!(ResolvedTheme::parse_data_attr("DARK"), None);
        assert_eq!(ResolvedTheme::parse_data_attr("Light"), None);
    }

    #[test]
    fn resolved_theme_parse_data_attr_returns_none_for_unrecognized() {
        assert_eq!(ResolvedTheme::parse_data_attr(""), None);
        assert_eq!(ResolvedTheme::parse_data_attr("system"), None);
        assert_eq!(ResolvedTheme::parse_data_attr("auto"), None);
        assert_eq!(ResolvedTheme::parse_data_attr("dark "), None);
    }

    #[test]
    fn resolved_theme_parse_data_attr_round_trips_with_as_str() {
        for theme in [ResolvedTheme::Dark, ResolvedTheme::Light] {
            assert_eq!(ResolvedTheme::parse_data_attr(theme.as_str()), Some(theme));
        }
    }

    #[test]
    fn resolved_theme_all_returns_every_variant() {
        let variants = ResolvedTheme::all();
        assert_eq!(variants.len(), 2);
        assert!(variants.contains(&ResolvedTheme::Dark));
        assert!(variants.contains(&ResolvedTheme::Light));
    }
}
