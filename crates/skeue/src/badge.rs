//! Shared CSS shell for compact text badges.

/// Foreground and background colors for [`badge_style`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BadgeColors {
    /// Badge background CSS color, usually a design token like
    /// `var(--status-success-bg)`.
    pub bg: &'static str,

    /// Badge foreground CSS color, usually a design token like
    /// `var(--status-success)`.
    pub fg: &'static str,
}

impl BadgeColors {
    /// Creates badge colors from background and foreground CSS colors.
    #[must_use]
    pub const fn new(bg: &'static str, fg: &'static str) -> Self {
        Self { bg, fg }
    }

    /// Returns the shared badge style for these colors.
    #[must_use]
    pub fn style(self) -> String {
        badge_style(self)
    }
}

const BADGE_STYLE_BASE: &str = concat!(
    "display: inline-flex; ",
    "align-items: center; ",
    "padding: var(--space-1) var(--space-2); ",
    "border-radius: var(--radius-md); ",
    "font-size: var(--text-xs); ",
    "font-weight: var(--weight-semibold);"
);

/// Returns a compact badge CSS style using the shared skeue badge shell.
#[must_use]
pub fn badge_style(colors: BadgeColors) -> String {
    format!(
        "{BADGE_STYLE_BASE} background: {}; color: {};",
        colors.bg, colors.fg
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUCCESS: BadgeColors =
        BadgeColors::new("var(--status-success-bg)", "var(--status-success)");

    #[test]
    fn badge_colors_are_copy_and_eq() {
        let copied = SUCCESS;

        assert_eq!(copied, SUCCESS);
    }

    #[test]
    fn badge_style_includes_shared_shell() {
        let style = badge_style(SUCCESS);

        assert!(style.contains("display: inline-flex;"));
        assert!(style.contains("align-items: center;"));
        assert!(style.contains("border-radius: var(--radius-md);"));
        assert!(style.contains("font-size: var(--text-xs);"));
        assert!(style.contains("font-weight: var(--weight-semibold);"));
    }

    #[test]
    fn badge_style_uses_supplied_colors() {
        let style = badge_style(SUCCESS);

        assert!(style.contains("background: var(--status-success-bg);"));
        assert!(style.contains("color: var(--status-success);"));
    }

    #[test]
    fn badge_colors_style_delegates_to_badge_style() {
        assert_eq!(SUCCESS.style(), badge_style(SUCCESS));
    }

    #[test]
    fn badge_style_changes_when_colors_change() {
        let warning = BadgeColors::new("var(--status-warning-bg)", "var(--status-warning)");

        assert_ne!(badge_style(SUCCESS), badge_style(warning));
    }
}
