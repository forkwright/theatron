//! Terminal palette + color-depth detection for ratatui apps.
//!
//! Provides the [`Theme`] semantic palette plus per-depth ([`ColorDepth`])
//! and per-mode ([`ThemeMode`]) palette constructors. The detection layer
//! reads terminal environment variables via the [`Env`] trait so tests
//! can supply deterministic environment values.
//!
//! Palette field names ([`Colors::bg`], [`TextColors::fg_muted`],
//! [`Borders::focused`], etc.) are self-documenting; enumerating them
//! in prose adds noise without signal. Each palette struct carries a
//! single `#[expect(missing_docs, …)]` attribute scoped to its fields
//! so the project-level `deny(missing_docs)` still catches new
//! undocumented items elsewhere.

use ratatui::style::{Color, Modifier, Style};

use crate::env::{Env, RealEnv};

/// Terminal color depth, detected at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColorDepth {
    /// 24-bit RGB (COLORTERM=truecolor, iTerm2, Kitty, Alacritty, etc.)
    TrueColor,
    /// 256-color (xterm-256color)
    Color256,
    /// Basic 16 ANSI colors
    Basic,
}

impl ColorDepth {
    /// Whether this is the 24-bit RGB depth (`TrueColor`).
    ///
    /// Convenience predicate matching the pattern from
    /// `gramma::diff::ChangeType::is_add` and
    /// `themelion::ResolvedTheme::is_dark`.
    #[must_use]
    pub const fn is_truecolor(self) -> bool {
        matches!(self, Self::TrueColor)
    }

    /// Whether this is the 256-color palette (`Color256`).
    #[must_use]
    pub const fn is_256(self) -> bool {
        matches!(self, Self::Color256)
    }

    /// Whether this is the basic 16-color depth (`Basic`).
    #[must_use]
    pub const fn is_basic(self) -> bool {
        matches!(self, Self::Basic)
    }

    /// Whether this depth supports at least 256 colors
    /// (`TrueColor` or `Color256`).
    ///
    /// Useful for "use a richer palette if available" branches:
    /// `let palette = if depth.has_256() { rich } else { basic };`.
    #[must_use]
    pub const fn has_256(self) -> bool {
        matches!(self, Self::TrueColor | Self::Color256)
    }
}

/// Background brightness: drives palette selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[expect(
    missing_docs,
    reason = "Dark/Light variant names are self-documenting; from_label and is_* methods carry the prose"
)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    /// Whether this is the dark palette (`Dark`).
    ///
    /// Convenience predicate matching the pattern from
    /// `themelion::ResolvedTheme::is_dark` and the rest of the
    /// v1.1 enum predicates.
    #[must_use]
    pub const fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }

    /// Whether this is the light palette (`Light`).
    #[must_use]
    pub const fn is_light(self) -> bool {
        matches!(self, Self::Light)
    }

    /// Parse a string label into a `ThemeMode`.
    ///
    /// Recognises `"dark"` and `"light"` (case-insensitive). Returns
    /// `None` for any other input, including `"system"` — parodos
    /// runs in a terminal where there is no OS-level light/dark
    /// preference to resolve, so unlike `themelion::ThemeMode` this
    /// enum has no `System` variant.
    ///
    /// Symmetric with `themelion::ThemeMode::from_label` (theatron
    /// PR #57) for crates that round-trip a config string into the
    /// TUI palette.
    #[must_use]
    pub fn from_label(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            _ => None,
        }
    }

    /// Every `ThemeMode` variant, in canonical order.
    ///
    /// Useful for building selection UIs and for exhaustiveness
    /// tests that need to iterate every variant. Returns a fixed-size
    /// array so callers can iterate without allocation.
    ///
    /// Symmetric with `themelion::ThemeMode::all` (theatron PR #57);
    /// parodos's array has two elements (`[Dark, Light]`) because
    /// the terminal-side enum has no `System` variant.
    #[must_use]
    pub const fn all() -> [Self; 2] {
        [Self::Dark, Self::Light]
    }
}

/// Background and accent colors.
#[derive(Debug, Clone)]
#[expect(missing_docs, reason = "palette field names are self-documenting")]
pub struct Colors {
    pub bg: Color,
    pub surface: Color,
    pub surface_bright: Color,
    pub surface_dim: Color,
    pub accent: Color,
    pub accent_dim: Color,
}

/// Foreground text and role-speaker colors.
#[derive(Debug, Clone)]
#[expect(missing_docs, reason = "palette field names are self-documenting")]
pub struct TextColors {
    pub fg: Color,
    pub fg_muted: Color,
    pub fg_dim: Color,
    pub user: Color,
    pub assistant: Color,
    pub system: Color,
}

/// Structural border and selection colors.
#[derive(Debug, Clone)]
#[expect(missing_docs, reason = "palette field names are self-documenting")]
pub struct Borders {
    pub normal: Color,
    pub focused: Color,
    pub separator: Color,
    pub selected: Color,
}

/// Semantic feedback and animation-state colors.
#[derive(Debug, Clone)]
#[expect(missing_docs, reason = "palette field names are self-documenting")]
pub struct StatusColors {
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub spinner: Color,
    pub idle: Color,
    pub streaming: Color,
    pub compacting: Color,
}

/// Code-block colors.
#[derive(Debug, Clone)]
#[expect(missing_docs, reason = "palette field names are self-documenting")]
pub struct CodeColors {
    pub fg: Color,
    pub bg: Color,
    pub lang: Color,
}

/// Thinking-block colors.
#[derive(Debug, Clone)]
#[expect(missing_docs, reason = "palette field names are self-documenting")]
pub struct ThinkingColors {
    pub fg: Color,
    pub border: Color,
}

/// Semantic color palette for the entire TUI.
/// Every color usage flows through this: no ad-hoc `Color::Cyan` calls.
///
/// Structured as nested groups so the active theme can be swapped as a single
/// value without touching individual call sites.
#[derive(Debug, Clone)]
#[expect(
    missing_docs,
    reason = "grouped palette fields named for the type they hold"
)]
pub struct Theme {
    pub colors: Colors,
    pub text: TextColors,
    pub borders: Borders,
    pub status: StatusColors,
    pub code: CodeColors,
    pub thinking: ThinkingColors,
    /// Color depth (for conditional rendering).
    pub depth: ColorDepth,
    /// Light or dark background.
    pub mode: ThemeMode,
}

/// Auto-detected theme from the terminal environment.
/// Used as the default when no config override is set.
#[cfg(test)]
pub static THEME: std::sync::LazyLock<Theme> = std::sync::LazyLock::new(Theme::default);

impl Default for Theme {
    fn default() -> Self {
        Self::detect()
    }
}

impl Theme {
    /// Create theme based on detected terminal capability and background.
    #[must_use]
    pub fn detect() -> Self {
        Self::for_mode(None)
    }

    /// Create theme for a specific mode. `None` means auto-detect from the terminal.
    #[must_use]
    pub fn for_mode(mode: Option<ThemeMode>) -> Self {
        Self::for_mode_with_env(mode, &RealEnv)
    }

    /// Create theme for a specific mode using an injectable environment.
    ///
    /// `None` means auto-detect from the supplied terminal environment.
    #[must_use]
    pub fn for_mode_with_env(mode: Option<ThemeMode>, env: &impl Env) -> Self {
        let resolved = mode.unwrap_or_else(|| detect_background(env));
        let depth = detect_color_depth(env);
        match (resolved, depth) {
            (ThemeMode::Light, ColorDepth::TrueColor) => Self::truecolor_light(),
            (ThemeMode::Light, ColorDepth::Color256) => Self::color256_light(),
            (ThemeMode::Light, ColorDepth::Basic) => Self::basic_light(),
            (ThemeMode::Dark, ColorDepth::TrueColor) => Self::truecolor(),
            (ThemeMode::Dark, ColorDepth::Color256) => Self::color256(),
            (ThemeMode::Dark, ColorDepth::Basic) => Self::basic(),
        }
    }

    /// Full 24-bit RGB palette: the target experience.
    #[must_use]
    pub fn truecolor() -> Self {
        Self {
            colors: Colors {
                bg: Color::Reset,
                surface: Color::Rgb(30, 30, 36),
                surface_bright: Color::Rgb(45, 45, 54),
                surface_dim: Color::Rgb(22, 22, 28),
                accent: Color::Rgb(120, 180, 255),
                accent_dim: Color::Rgb(70, 110, 170),
            },
            text: TextColors {
                fg: Color::Rgb(220, 220, 230),
                fg_muted: Color::Rgb(140, 140, 160),
                fg_dim: Color::Rgb(85, 85, 100),
                user: Color::Rgb(120, 180, 255),
                assistant: Color::Rgb(120, 220, 150),
                system: Color::Rgb(140, 140, 160),
            },
            borders: Borders {
                normal: Color::Rgb(60, 60, 75),
                focused: Color::Rgb(120, 180, 255),
                separator: Color::Rgb(50, 50, 62),
                selected: Color::Rgb(120, 180, 255),
            },
            status: StatusColors {
                success: Color::Rgb(120, 220, 150),
                warning: Color::Rgb(240, 190, 80),
                error: Color::Rgb(240, 100, 100),
                info: Color::Rgb(120, 180, 255),
                spinner: Color::Rgb(240, 190, 80),
                idle: Color::Rgb(85, 85, 100),
                streaming: Color::Rgb(120, 220, 150),
                compacting: Color::Rgb(180, 120, 240),
            },
            code: CodeColors {
                fg: Color::Rgb(200, 200, 215),
                bg: Color::Rgb(35, 35, 42),
                lang: Color::Rgb(100, 100, 120),
            },
            thinking: ThinkingColors {
                fg: Color::Rgb(100, 100, 120),
                border: Color::Rgb(60, 60, 75),
            },
            depth: ColorDepth::TrueColor,
            mode: ThemeMode::Dark,
        }
    }

    /// 24-bit RGB light palette: readable on white/light terminal backgrounds.
    #[must_use]
    pub fn truecolor_light() -> Self {
        Self {
            colors: Colors {
                bg: Color::Reset,
                surface: Color::Rgb(245, 245, 248),
                surface_bright: Color::Rgb(255, 255, 255),
                surface_dim: Color::Rgb(230, 230, 236),
                accent: Color::Rgb(30, 100, 210),
                accent_dim: Color::Rgb(100, 140, 200),
            },
            text: TextColors {
                fg: Color::Rgb(30, 30, 40),
                fg_muted: Color::Rgb(100, 100, 120),
                fg_dim: Color::Rgb(150, 150, 165),
                user: Color::Rgb(30, 100, 210),
                assistant: Color::Rgb(20, 140, 60),
                system: Color::Rgb(100, 100, 120),
            },
            borders: Borders {
                normal: Color::Rgb(200, 200, 212),
                focused: Color::Rgb(30, 100, 210),
                separator: Color::Rgb(215, 215, 225),
                selected: Color::Rgb(30, 100, 210),
            },
            status: StatusColors {
                success: Color::Rgb(20, 140, 60),
                warning: Color::Rgb(180, 130, 0),
                error: Color::Rgb(200, 50, 50),
                info: Color::Rgb(30, 100, 210),
                spinner: Color::Rgb(180, 130, 0),
                idle: Color::Rgb(150, 150, 165),
                streaming: Color::Rgb(20, 140, 60),
                compacting: Color::Rgb(130, 60, 200),
            },
            code: CodeColors {
                fg: Color::Rgb(40, 40, 50),
                bg: Color::Rgb(235, 235, 240),
                lang: Color::Rgb(130, 130, 145),
            },
            thinking: ThinkingColors {
                fg: Color::Rgb(130, 130, 145),
                border: Color::Rgb(200, 200, 212),
            },
            depth: ColorDepth::TrueColor,
            mode: ThemeMode::Light,
        }
    }

    /// 256-color fallback: approximates the true color palette.
    #[must_use]
    pub fn color256() -> Self {
        Self {
            colors: Colors {
                bg: Color::Reset,
                surface: Color::Indexed(235),
                surface_bright: Color::Indexed(237),
                surface_dim: Color::Indexed(233),
                accent: Color::Indexed(111),
                accent_dim: Color::Indexed(67),
            },
            text: TextColors {
                fg: Color::Indexed(253),
                fg_muted: Color::Indexed(245),
                fg_dim: Color::Indexed(240),
                user: Color::Indexed(111),
                assistant: Color::Indexed(114),
                system: Color::Indexed(245),
            },
            borders: Borders {
                normal: Color::Indexed(238),
                focused: Color::Indexed(111),
                separator: Color::Indexed(236),
                selected: Color::Indexed(111),
            },
            status: StatusColors {
                success: Color::Indexed(114),
                warning: Color::Indexed(221),
                error: Color::Indexed(167),
                info: Color::Indexed(111),
                spinner: Color::Indexed(221),
                idle: Color::Indexed(240),
                streaming: Color::Indexed(114),
                compacting: Color::Indexed(177),
            },
            code: CodeColors {
                fg: Color::Indexed(252),
                bg: Color::Indexed(236),
                lang: Color::Indexed(242),
            },
            thinking: ThinkingColors {
                fg: Color::Indexed(242),
                border: Color::Indexed(238),
            },
            depth: ColorDepth::Color256,
            mode: ThemeMode::Dark,
        }
    }

    /// 256-color light palette.
    #[must_use]
    pub fn color256_light() -> Self {
        Self {
            colors: Colors {
                bg: Color::Reset,
                surface: Color::Indexed(255),
                surface_bright: Color::Indexed(231),
                surface_dim: Color::Indexed(254),
                accent: Color::Indexed(25),
                accent_dim: Color::Indexed(67),
            },
            text: TextColors {
                fg: Color::Indexed(234),
                fg_muted: Color::Indexed(243),
                fg_dim: Color::Indexed(249),
                user: Color::Indexed(25),
                assistant: Color::Indexed(28),
                system: Color::Indexed(243),
            },
            borders: Borders {
                normal: Color::Indexed(252),
                focused: Color::Indexed(25),
                separator: Color::Indexed(254),
                selected: Color::Indexed(25),
            },
            status: StatusColors {
                success: Color::Indexed(28),
                warning: Color::Indexed(136),
                error: Color::Indexed(160),
                info: Color::Indexed(25),
                spinner: Color::Indexed(136),
                idle: Color::Indexed(249),
                streaming: Color::Indexed(28),
                compacting: Color::Indexed(128),
            },
            code: CodeColors {
                fg: Color::Indexed(234),
                bg: Color::Indexed(254),
                lang: Color::Indexed(246),
            },
            thinking: ThinkingColors {
                fg: Color::Indexed(246),
                border: Color::Indexed(252),
            },
            depth: ColorDepth::Color256,
            mode: ThemeMode::Light,
        }
    }

    /// Basic 16-color ANSI: widest compatibility.
    #[must_use]
    pub fn basic() -> Self {
        Self {
            colors: Colors {
                bg: Color::Reset,
                surface: Color::Reset,
                surface_bright: Color::DarkGray,
                surface_dim: Color::Reset,
                accent: Color::Cyan,
                accent_dim: Color::DarkGray,
            },
            text: TextColors {
                fg: Color::White,
                fg_muted: Color::Gray,
                fg_dim: Color::DarkGray,
                user: Color::Cyan,
                assistant: Color::Green,
                system: Color::DarkGray,
            },
            borders: Borders {
                normal: Color::DarkGray,
                focused: Color::Cyan,
                separator: Color::DarkGray,
                selected: Color::Cyan,
            },
            status: StatusColors {
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Cyan,
                spinner: Color::Yellow,
                idle: Color::DarkGray,
                streaming: Color::Green,
                compacting: Color::Magenta,
            },
            code: CodeColors {
                fg: Color::White,
                bg: Color::DarkGray,
                lang: Color::DarkGray,
            },
            thinking: ThinkingColors {
                fg: Color::DarkGray,
                border: Color::DarkGray,
            },
            depth: ColorDepth::Basic,
            mode: ThemeMode::Dark,
        }
    }

    /// Basic 16-color light palette.
    #[must_use]
    pub fn basic_light() -> Self {
        Self {
            colors: Colors {
                bg: Color::Reset,
                surface: Color::Reset,
                surface_bright: Color::White,
                surface_dim: Color::Reset,
                accent: Color::Blue,
                accent_dim: Color::DarkGray,
            },
            text: TextColors {
                fg: Color::Black,
                fg_muted: Color::DarkGray,
                fg_dim: Color::Gray,
                user: Color::Blue,
                assistant: Color::Green,
                system: Color::DarkGray,
            },
            borders: Borders {
                normal: Color::Gray,
                focused: Color::Blue,
                separator: Color::Gray,
                selected: Color::Blue,
            },
            status: StatusColors {
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Blue,
                spinner: Color::Yellow,
                idle: Color::Gray,
                streaming: Color::Green,
                compacting: Color::Magenta,
            },
            code: CodeColors {
                fg: Color::Black,
                bg: Color::White,
                lang: Color::DarkGray,
            },
            thinking: ThinkingColors {
                fg: Color::DarkGray,
                border: Color::Gray,
            },
            depth: ColorDepth::Basic,
            mode: ThemeMode::Light,
        }
    }

    /// Default foreground text style.
    #[must_use]
    pub fn style_fg(&self) -> Style {
        Style::default().fg(self.text.fg)
    }

    /// Muted text style (lower contrast than `style_fg`).
    #[must_use]
    pub fn style_muted(&self) -> Style {
        Style::default().fg(self.text.fg_muted)
    }

    /// Dim text style (lowest contrast tier).
    #[must_use]
    pub fn style_dim(&self) -> Style {
        Style::default().fg(self.text.fg_dim)
    }

    /// Accent foreground style.
    #[must_use]
    pub fn style_accent(&self) -> Style {
        Style::default().fg(self.colors.accent)
    }

    /// Accent foreground with bold modifier.
    #[must_use]
    pub fn style_accent_bold(&self) -> Style {
        Style::default()
            .fg(self.colors.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Success-feedback foreground style.
    #[must_use]
    pub fn style_success(&self) -> Style {
        Style::default().fg(self.status.success)
    }

    /// Warning-feedback foreground style.
    #[must_use]
    pub fn style_warning(&self) -> Style {
        Style::default().fg(self.status.warning)
    }

    /// Error-feedback foreground style.
    #[must_use]
    pub fn style_error(&self) -> Style {
        Style::default().fg(self.status.error)
    }

    /// Success-feedback foreground with bold modifier.
    #[must_use]
    pub fn style_success_bold(&self) -> Style {
        Style::default()
            .fg(self.status.success)
            .add_modifier(Modifier::BOLD)
    }

    /// Error-feedback foreground with bold modifier.
    #[must_use]
    pub fn style_error_bold(&self) -> Style {
        Style::default()
            .fg(self.status.error)
            .add_modifier(Modifier::BOLD)
    }

    /// User role-speaker style (bold).
    #[must_use]
    pub fn style_user(&self) -> Style {
        Style::default()
            .fg(self.text.user)
            .add_modifier(Modifier::BOLD)
    }

    /// Assistant role-speaker style (bold).
    #[must_use]
    pub fn style_assistant(&self) -> Style {
        Style::default()
            .fg(self.text.assistant)
            .add_modifier(Modifier::BOLD)
    }

    /// Code-block foreground+background style.
    #[must_use]
    pub fn style_code(&self) -> Style {
        Style::default().fg(self.code.fg).bg(self.code.bg)
    }

    /// Inline-code style (warning fg over code bg).
    #[must_use]
    pub fn style_inline_code(&self) -> Style {
        Style::default().fg(self.status.warning).bg(self.code.bg)
    }

    /// Surface background style.
    #[must_use]
    pub fn style_surface(&self) -> Style {
        Style::default().bg(self.colors.surface)
    }

    /// Normal-border foreground style.
    #[must_use]
    pub fn style_border(&self) -> Style {
        Style::default().fg(self.borders.normal)
    }

    /// Focused-border foreground style.
    #[must_use]
    pub fn style_border_focused(&self) -> Style {
        Style::default().fg(self.borders.focused)
    }
}

/// Detect terminal background brightness from `$COLORFGBG`.
///
/// Format: `fg;bg` or `fg;X;bg` where values are ANSI color indices.
/// Indices 0-6 are dark colors, 7+ are light. Defaults to dark when unset.
fn detect_background(env: &impl Env) -> ThemeMode {
    if let Some(val) = env.var("COLORFGBG") {
        // WHY: Some terminals emit three values (e.g., "15;0;0"). The background
        // is always the last component.
        if let Some(bg_str) = val.rsplit(';').next()
            && let Ok(bg) = bg_str.parse::<u8>()
        {
            return if bg >= 8 {
                ThemeMode::Light
            } else {
                ThemeMode::Dark
            };
        }
    }
    ThemeMode::Dark
}

/// Detect terminal color capability from environment variables.
fn detect_color_depth(env: &impl Env) -> ColorDepth {
    // WHY: COLORTERM is the most reliable indicator: check it before TERM.
    if let Some(ct) = env.var("COLORTERM") {
        match ct.as_str() {
            "truecolor" | "24bit" => return ColorDepth::TrueColor,
            // NOTE: unrecognized COLORTERM value, check other env vars
            _ => {}
        }
    }

    if let Some(tp) = env.var("TERM_PROGRAM") {
        match tp.as_str() {
            "iTerm.app" | "WezTerm" | "Alacritty" | "kitty" => return ColorDepth::TrueColor,
            // NOTE: unrecognized terminal program, continue probing
            _ => {}
        }
    }

    // NOTE: VTE encodes 0.36.0 as 3600; older versions are not truecolor-capable.
    if let Some(version) = env
        .var("VTE_VERSION")
        .and_then(|version| version.parse::<u32>().ok())
        && version >= 3600
    {
        return ColorDepth::TrueColor;
    }

    if let Some(term) = env.var("TERM")
        && term.contains("256color")
    {
        return ColorDepth::Color256;
    }

    if env.var("TMUX").is_some() {
        return ColorDepth::Color256;
    }

    ColorDepth::Basic
}

/// Braille spinner frames for smooth animation.
pub const BRAILLE_SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Get the current braille spinner frame based on tick count.
#[expect(
    clippy::indexing_slicing,
    reason = "index is computed as expr % BRAILLE_SPINNER.len(), which is always a valid index"
)]
#[must_use]
pub fn spinner_frame(tick: u64) -> char {
    // WHY: mod by BRAILLE_SPINNER.len() in u64 space first, then try_from;
    // the result is < 10, so usize conversion cannot fail on any platform.
    let len = u64::try_from(BRAILLE_SPINNER.len()).unwrap_or(1).max(1);
    let idx = usize::try_from((tick / 3) % len).unwrap_or(0);
    BRAILLE_SPINNER[idx]
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod tests;
