use ratatui::style::Color;

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

impl Colors {
    /// Create a background/accent palette group.
    #[must_use]
    pub const fn new(
        bg: Color,
        surface: Color,
        surface_bright: Color,
        surface_dim: Color,
        accent: Color,
        accent_dim: Color,
    ) -> Self {
        Self {
            bg,
            surface,
            surface_bright,
            surface_dim,
            accent,
            accent_dim,
        }
    }
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

impl TextColors {
    /// Create a text and role-speaker palette group.
    #[must_use]
    pub const fn new(
        fg: Color,
        fg_muted: Color,
        fg_dim: Color,
        user: Color,
        assistant: Color,
        system: Color,
    ) -> Self {
        Self {
            fg,
            fg_muted,
            fg_dim,
            user,
            assistant,
            system,
        }
    }
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

impl Borders {
    /// Create a structural border and selection palette group.
    #[must_use]
    pub const fn new(normal: Color, focused: Color, separator: Color, selected: Color) -> Self {
        Self {
            normal,
            focused,
            separator,
            selected,
        }
    }
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

impl StatusColors {
    /// Create a status and animation-state palette group.
    #[must_use]
    pub const fn new(
        success: Color,
        warning: Color,
        error: Color,
        info: Color,
        spinner: Color,
        idle: Color,
        streaming: Color,
        compacting: Color,
    ) -> Self {
        Self {
            success,
            warning,
            error,
            info,
            spinner,
            idle,
            streaming,
            compacting,
        }
    }
}

/// Code-block colors.
#[derive(Debug, Clone)]
#[expect(missing_docs, reason = "palette field names are self-documenting")]
pub struct CodeColors {
    pub fg: Color,
    pub bg: Color,
    pub lang: Color,
}

impl CodeColors {
    /// Create a code-block palette group.
    #[must_use]
    pub const fn new(fg: Color, bg: Color, lang: Color) -> Self {
        Self { fg, bg, lang }
    }
}

/// Thinking-block colors.
#[derive(Debug, Clone)]
#[expect(missing_docs, reason = "palette field names are self-documenting")]
pub struct ThinkingColors {
    pub fg: Color,
    pub border: Color,
}

impl ThinkingColors {
    /// Create a thinking-block palette group.
    #[must_use]
    pub const fn new(fg: Color, border: Color) -> Self {
        Self { fg, border }
    }
}
