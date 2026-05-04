//! Theme state management for the Dioxus desktop app.
//!
//! Provides `ThemeProvider` (wraps root, applies `data-theme`) and a
//! `Signal<ThemeMode>` context so any descendant can read or switch themes.

use dioxus::prelude::*;

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
    /// CSS selector value applied by [`ThemeProvider`].
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
}

impl ThemeMode {
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
    /// Recognized labels (case-sensitive): `"Dark"`, `"Light"`,
    /// `"System"`. The match is case-sensitive so consumer code
    /// that wants a forgiving round-trip should normalize before
    /// calling.
    ///
    /// # Examples
    ///
    /// ```
    /// use themelion::ThemeMode;
    /// assert_eq!(ThemeMode::from_label("Dark"), Some(ThemeMode::Dark));
    /// assert_eq!(ThemeMode::from_label("dark"), None); // case-sensitive
    /// assert_eq!(ThemeMode::from_label("garbage"), None);
    /// ```
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "Dark" => Some(Self::Dark),
            "Light" => Some(Self::Light),
            "System" => Some(Self::System),
            _ => None,
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
}

/// Detect OS color preference from environment variables.
///
/// Checks `GTK_THEME` for a "dark" suffix and `COLORFGBG` for background
/// brightness (same heuristic as the TUI). Falls back to dark.
fn detect_system_preference() -> ResolvedTheme {
    if let Ok(gtk_theme) = std::env::var("GTK_THEME") {
        return if gtk_theme.to_ascii_lowercase().contains("dark") {
            ResolvedTheme::Dark
        } else {
            ResolvedTheme::Light
        };
    }

    // WHY: COLORFGBG format is "fg;bg" or "fg;X;bg". Background is always
    // the last component. Indices 0-6 are dark, 7+ are light. Matches the
    // TUI detection logic in koilon/src/theme.rs.
    if let Ok(val) = std::env::var("COLORFGBG")
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

/// Root theme provider.
///
/// Wraps the component tree with a `div[data-theme]` so CSS custom properties
/// in `themes.css` activate. Provides `Signal<ThemeMode>` as context for
/// descendant components (including `ThemeToggle`).
#[component]
pub fn ThemeProvider(children: Element, initial_mode: Option<ThemeMode>) -> Element {
    let mode = use_signal(|| initial_mode.unwrap_or(ThemeMode::System));
    use_context_provider(|| mode);
    let resolved = use_memo(move || mode().resolve());

    rsx! {
        div {
            "data-theme": resolved().as_str(),
            style: "display: contents",
            {children}
        }
    }
}

const TOGGLE_STYLE: &str = "\
    display: inline-flex; \
    align-items: center; \
    gap: var(--space-2); \
    padding: var(--space-1) var(--space-3); \
    border: 1px solid var(--border); \
    border-radius: var(--radius-md); \
    background: var(--bg-surface); \
    color: var(--text-secondary); \
    font-family: var(--font-body); \
    font-size: var(--text-sm); \
    cursor: pointer; \
    transition: \
        border-color var(--transition-quick), \
        color var(--transition-quick), \
        background-color var(--transition-quick);\
";

/// A button that cycles through theme modes (Dark → Light → System → Dark).
///
/// Reads `Signal<ThemeMode>` from context (provided by [`ThemeProvider`])
/// and advances to the next mode on click. After the mode is advanced,
/// fires `on_change` with the new mode — consumers wire this to their own
/// persistence layer (proskenion writes to settings.toml; chalkeion to
/// its own state dir).
///
/// The callback is optional — pass `EventHandler::default()` (or omit
/// in shorthand form) for surfaces that don't persist.
#[component]
pub fn ThemeToggle(#[props(default)] on_change: EventHandler<ThemeMode>) -> Element {
    let mut mode = use_context::<Signal<ThemeMode>>();
    let current = mode();
    let icon = current.icon();
    let label = current.label();

    rsx! {
        button {
            r#type: "button",
            onclick: move |_| {
                let next = mode().next();
                mode.set(next);
                on_change.call(next);
            },
            title: "Theme: {label}",
            "aria-label": "Switch theme, current: {label}",
            style: "{TOGGLE_STYLE}",
            span { "{icon}" }
            span { "{label}" }
        }
    }
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
        let resolved = ThemeMode::System.resolve();
        assert!(resolved == ResolvedTheme::Dark || resolved == ResolvedTheme::Light);
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
    fn from_label_is_case_sensitive() {
        assert_eq!(ThemeMode::from_label("dark"), None);
        assert_eq!(ThemeMode::from_label("DARK"), None);
        assert_eq!(ThemeMode::from_label("Dark"), Some(ThemeMode::Dark));
    }

    #[test]
    fn from_label_returns_none_for_unknown() {
        assert_eq!(ThemeMode::from_label(""), None);
        assert_eq!(ThemeMode::from_label("Auto"), None);
        assert_eq!(ThemeMode::from_label("garbage"), None);
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
        for mode in ThemeMode::all() {
            match mode {
                ThemeMode::Dark | ThemeMode::Light | ThemeMode::System => (),
            }
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
}
