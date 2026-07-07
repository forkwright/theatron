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
    // the last component. Indices 0-6 are dark, 7+ are light — ANSI index 7
    // is "white" (light-grey), the conventional dark/light boundary matched
    // by the TUI detection logic in koilon/src/theme.rs (#180).
    if let Some(val) = env("COLORFGBG")
        && let Some(bg_str) = val.rsplit(';').next()
        && let Ok(bg) = bg_str.parse::<u8>()
    {
        return if bg >= 7 {
            ResolvedTheme::Light
        } else {
            ResolvedTheme::Dark
        };
    }

    ResolvedTheme::Dark
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod tests;
