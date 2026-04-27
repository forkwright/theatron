//! Status pill — a compact badge indicating a discrete state.
//!
//! Per DESIGN-TOKENS.md component anatomy:
//! - Structure: optional icon + label
//! - Size: fits in a table cell; max-width ~100px
//! - Token use: `--status-*` foreground on `--status-*-bg`, or dye palette
//! - Radius: `--radius-full` (pill) or `--radius-md` (rectangular badge)
//! - Text: `--text-xs`, `--weight-medium`
//! - States: success | warning | error | info | aima | aporia | thanatochromia | natural
//!
//! References (folds in kanon discussion docket #40):
//! - Linear status pills: small dot+label, tight padding, semibold
//! - Sourcehut PR states: rectangular badge, monospaced
//! - Fly.io region status: pill with leading icon
//! - GitHub label: pill with optional icon, single-line truncate

use dioxus::prelude::*;

/// Visual register for a [`StatusPill`]. Maps to a foreground/background
/// token pair from DESIGN-TOKENS.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusPillKind {
    /// Functional success — `--status-success` on `--status-success-bg`.
    Success,
    /// Non-critical warning — `--status-warning` on `--status-warning-bg`.
    Warning,
    /// Functional failure — `--status-error` on `--status-error-bg`.
    Error,
    /// Informational — `--status-info` on `--status-info-bg`.
    Info,
    /// Dye palette: blood — vital state demanding immediate response.
    Aima,
    /// Dye palette: aporia — pending / contemplative state.
    Aporia,
    /// Dye palette: color of death — archived / terminal state.
    Thanatochromia,
    /// Dye palette: undyed leather — neutral / muted state.
    Natural,
}

impl StatusPillKind {
    /// Foreground color token for this kind.
    #[must_use]
    pub const fn fg_token(self) -> &'static str {
        match self {
            Self::Success => "var(--status-success)",
            Self::Warning => "var(--status-warning)",
            Self::Error => "var(--status-error)",
            Self::Info => "var(--status-info)",
            Self::Aima => "var(--aima)",
            Self::Aporia => "var(--aporia)",
            Self::Thanatochromia => "var(--thanatochromia)",
            Self::Natural => "var(--natural)",
        }
    }

    /// Background color token for this kind.
    #[must_use]
    pub const fn bg_token(self) -> &'static str {
        match self {
            Self::Success => "var(--status-success-bg)",
            Self::Warning => "var(--status-warning-bg)",
            Self::Error => "var(--status-error-bg)",
            Self::Info => "var(--status-info-bg)",
            Self::Aima => "var(--aima-bg)",
            Self::Aporia => "var(--aporia-bg)",
            Self::Thanatochromia => "var(--thanatochromia-bg)",
            Self::Natural => "var(--natural-bg)",
        }
    }
}

/// Visual shape variant for a [`StatusPill`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusPillShape {
    /// Pill shape (full radius). Default — used for state indicators.
    #[default]
    Pill,
    /// Rectangular badge (medium radius). Used for tags / labels in tables.
    Badge,
}

impl StatusPillShape {
    const fn radius_token(self) -> &'static str {
        match self {
            Self::Pill => "var(--radius-full)",
            Self::Badge => "var(--radius-md)",
        }
    }
}

const PILL_STYLE_FMT: &str = "\
    display: inline-flex; \
    align-items: center; \
    gap: var(--space-1); \
    padding: var(--space-1) var(--space-2); \
    font-size: var(--text-xs); \
    font-weight: var(--weight-medium); \
    line-height: var(--leading-tight); \
    max-width: 100px; \
    overflow: hidden; \
    text-overflow: ellipsis; \
    white-space: nowrap;\
";

/// A compact badge indicating a discrete state.
///
/// Per DESIGN-TOKENS.md component anatomy. See module docs for the
/// reference inventory.
#[component]
pub fn StatusPill(
    /// Visual register — maps to a token pair.
    kind: StatusPillKind,
    /// Label text. Truncates with ellipsis at ~100px.
    label: String,
    /// Optional leading icon (Unicode glyph, single short string).
    #[props(default)]
    icon: Option<String>,
    /// Pill or badge shape. Defaults to pill.
    #[props(default)]
    shape: StatusPillShape,
) -> Element {
    let fg = kind.fg_token();
    let bg = kind.bg_token();
    let radius = shape.radius_token();
    rsx! {
        span {
            role: "status",
            style: "{PILL_STYLE_FMT} background: {bg}; color: {fg}; border-radius: {radius};",
            if let Some(ref glyph) = icon {
                span { aria_hidden: "true", "{glyph}" }
            }
            "{label}"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fg_token_uses_status_namespace_for_status_kinds() {
        assert_eq!(StatusPillKind::Success.fg_token(), "var(--status-success)");
        assert_eq!(StatusPillKind::Error.fg_token(), "var(--status-error)");
    }

    #[test]
    fn fg_token_uses_dye_namespace_for_dye_kinds() {
        assert_eq!(StatusPillKind::Aima.fg_token(), "var(--aima)");
        assert_eq!(
            StatusPillKind::Thanatochromia.fg_token(),
            "var(--thanatochromia)"
        );
    }

    #[test]
    fn bg_token_appends_bg_suffix() {
        for kind in [
            StatusPillKind::Success,
            StatusPillKind::Warning,
            StatusPillKind::Error,
            StatusPillKind::Info,
            StatusPillKind::Aima,
            StatusPillKind::Aporia,
            StatusPillKind::Thanatochromia,
            StatusPillKind::Natural,
        ] {
            assert!(
                kind.bg_token().ends_with("-bg)"),
                "bg_token must end with -bg) for {kind:?}, got {}",
                kind.bg_token()
            );
        }
    }

    #[test]
    fn shape_radius_token_matches_design_tokens() {
        assert_eq!(StatusPillShape::Pill.radius_token(), "var(--radius-full)");
        assert_eq!(StatusPillShape::Badge.radius_token(), "var(--radius-md)");
    }

    #[test]
    fn shape_default_is_pill() {
        assert_eq!(StatusPillShape::default(), StatusPillShape::Pill);
    }

    #[test]
    fn fg_and_bg_token_namespaces_match() {
        // Foreground "var(--X)" should pair with background "var(--X-bg)"
        // for every kind. Catches a copy-paste mistake at the table.
        for kind in [
            StatusPillKind::Success,
            StatusPillKind::Warning,
            StatusPillKind::Error,
            StatusPillKind::Info,
            StatusPillKind::Aima,
            StatusPillKind::Aporia,
            StatusPillKind::Thanatochromia,
            StatusPillKind::Natural,
        ] {
            let fg = kind.fg_token();
            let bg = kind.bg_token();
            // Strip "var(--" prefix and ")" suffix from fg; bg should be
            // "var(--<that>-bg)".
            let fg_inner = fg
                .strip_prefix("var(--")
                .and_then(|s| s.strip_suffix(')'))
                .expect("fg token shape");
            let expected_bg = format!("var(--{fg_inner}-bg)");
            assert_eq!(bg, expected_bg, "kind={kind:?}");
        }
    }
}
