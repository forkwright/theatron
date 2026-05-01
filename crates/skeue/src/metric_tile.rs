//! Metric tile — a single KPI value with a label.
//!
//! Per DESIGN-TOKENS.md component anatomy:
//! - Structure: large value + optional unit + label + optional delta indicator
//! - Size: grid-cell sized, 3–4 per row
//! - Token use: value in `--text-2xl` / `--weight-bold`,
//!   label in `--text-sm` / `--text-muted`
//! - Background: `--bg-surface`, border `--border`
//!
//! References (folds in #40):
//! - Fly.io region tiles: large number + unit + delta indicator
//! - Grafana stat panels: bold value + small label + colored sparkline
//! - Datadog summary cards: 3-row layout (value, label, delta line)

use dioxus::prelude::*;

/// Direction of a metric delta — controls the indicator color/glyph.
///
/// "Up" / "Down" describe the *trend*, not whether it's good or bad.
/// Consumers choose by attaching tone explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeltaDirection {
    /// Value is rising over the prior interval.
    Up,
    /// Value is falling over the prior interval.
    Down,
    /// No change relative to prior interval.
    Flat,
}

impl DeltaDirection {
    const fn glyph(self) -> &'static str {
        match self {
            // Triangle markers — accessible and font-stack-portable.
            Self::Up => "\u{25B2}",
            Self::Down => "\u{25BC}",
            Self::Flat => "\u{2014}",
        }
    }
}

/// Semantic interpretation of a delta — controls color independently of
/// direction (a falling latency is "good"; a falling success rate is
/// "bad"; both are mechanically `Down`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum DeltaTone {
    /// Neutral — `--text-muted`. Default when no judgment applies.
    #[default]
    Neutral,
    /// Improvement — `--status-success`.
    Good,
    /// Regression — `--status-error`.
    Bad,
}

impl DeltaTone {
    const fn color_token(self) -> &'static str {
        match self {
            Self::Neutral => "var(--text-muted)",
            Self::Good => "var(--status-success)",
            Self::Bad => "var(--status-error)",
        }
    }
}

/// Optional delta annotation for a [`MetricTile`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricDelta {
    /// Trend direction.
    pub direction: DeltaDirection,
    /// Display label, e.g. "+12.4%" or "-3ms" or "no change".
    pub label: String,
    /// Tone — controls color independently of direction.
    pub tone: DeltaTone,
}

const TILE_STYLE: &str = "\
    display: flex; \
    flex-direction: column; \
    gap: var(--space-1); \
    padding: var(--space-3) var(--space-4); \
    background: var(--bg-surface); \
    border: 1px solid var(--border); \
    border-radius: var(--radius-md);\
";

const VALUE_ROW_STYLE: &str = "\
    display: flex; \
    align-items: baseline; \
    gap: var(--space-2);\
";

const VALUE_STYLE: &str = "\
    font-size: var(--text-2xl); \
    font-weight: var(--weight-bold); \
    color: var(--text-primary); \
    line-height: var(--leading-tight);\
";

const UNIT_STYLE: &str = "\
    font-size: var(--text-sm); \
    color: var(--text-secondary);\
";

const LABEL_STYLE: &str = "\
    font-size: var(--text-sm); \
    color: var(--text-muted); \
    line-height: var(--leading-tight);\
";

const DELTA_ROW_STYLE: &str = "\
    display: flex; \
    align-items: center; \
    gap: var(--space-1); \
    font-size: var(--text-sm); \
    line-height: var(--leading-tight);\
";

/// A single KPI tile — large value + label, optional unit and delta.
///
/// Per DESIGN-TOKENS.md component anatomy. See module docs for the
/// reference inventory.
#[component]
pub fn MetricTile(
    /// Large primary value (e.g. "12.4", "98%", "1.2k").
    value: String,
    /// Short label below the value (e.g. "Active sessions", "P95 latency").
    label: String,
    /// Optional unit displayed inline after the value (e.g. "ms", "/s").
    #[props(default)]
    unit: Option<String>,
    /// Optional trend annotation rendered below the label.
    #[props(default)]
    delta: Option<MetricDelta>,
) -> Element {
    rsx! {
        div {
            role: "group",
            "aria-label": "{label}",
            style: "{TILE_STYLE}",
            div {
                style: "{VALUE_ROW_STYLE}",
                span { style: "{VALUE_STYLE}", "{value}" }
                if let Some(ref u) = unit {
                    span { style: "{UNIT_STYLE}", "{u}" }
                }
            }
            div { style: "{LABEL_STYLE}", "{label}" }
            if let Some(ref d) = delta {
                div {
                    style: "{DELTA_ROW_STYLE} color: {d.tone.color_token()};",
                    span { aria_hidden: "true", "{d.direction.glyph()}" }
                    "{d.label}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_direction_glyphs_are_nonempty_unicode() {
        for d in [
            DeltaDirection::Up,
            DeltaDirection::Down,
            DeltaDirection::Flat,
        ] {
            let g = d.glyph();
            assert!(!g.is_empty(), "glyph empty for {d:?}");
            // All three glyphs are single Unicode code points.
            assert_eq!(g.chars().count(), 1, "{d:?} glyph: {g}");
        }
    }

    #[test]
    fn delta_tone_color_tokens_are_canonical() {
        assert_eq!(DeltaTone::Neutral.color_token(), "var(--text-muted)");
        assert_eq!(DeltaTone::Good.color_token(), "var(--status-success)");
        assert_eq!(DeltaTone::Bad.color_token(), "var(--status-error)");
    }

    #[test]
    fn delta_tone_default_is_neutral() {
        assert_eq!(DeltaTone::default(), DeltaTone::Neutral);
    }

    #[test]
    fn delta_construction_via_struct_literal() {
        let d = MetricDelta {
            direction: DeltaDirection::Down,
            label: "-3ms".to_string(),
            tone: DeltaTone::Good,
        };
        assert_eq!(d.direction, DeltaDirection::Down);
        assert_eq!(d.tone, DeltaTone::Good);
    }
}
