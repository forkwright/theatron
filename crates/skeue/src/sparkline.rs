//! Sparkline — minimal inline trend chart.
//!
//! Per DESIGN-TOKENS.md component anatomy:
//! - Structure: SVG path (line) or series of bars
//! - Size: fits within `--row-h-standard` height; width flexible
//! - Token use: line/bar fill from `--status-*` or `--accent`;
//!   baseline from `--border`
//! - No axes, no labels — the number lives in the metric tile;
//!   the sparkline only shows shape
//!
//! References (folds in #40):
//! - Grafana sparkline: thin line, no axes, accent fill
//! - Datadog inline trend: bars, status-color fill
//! - GitHub contribution graph: bars, density-coded

use dioxus::prelude::*;

/// Visual register for a [`Sparkline`] — controls the line/bar fill
/// color independently of the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SparklineTone {
    /// `--accent` (default — neutral / brand).
    #[default]
    Accent,
    /// `--status-success`.
    Success,
    /// `--status-warning`.
    Warning,
    /// `--status-error`.
    Error,
    /// `--status-info`.
    Info,
}

impl SparklineTone {
    const fn color_token(self) -> &'static str {
        match self {
            Self::Accent => "var(--accent)",
            Self::Success => "var(--status-success)",
            Self::Warning => "var(--status-warning)",
            Self::Error => "var(--status-error)",
            Self::Info => "var(--status-info)",
        }
    }
}

/// Render shape — thin line vs bar series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SparklineShape {
    /// Connected line through data points (default).
    #[default]
    Line,
    /// Vertical bars rising from the baseline.
    Bars,
}

/// Convert `usize` to `f64` for pixel-coordinate math.
///
/// WHY: `f64` does not implement `TryFrom<usize>`. Sparkline data arrays are
/// bounded by screen-pixel counts (hundreds to thousands of points), well
/// within `f64` integer precision.
#[expect(
    clippy::cast_precision_loss,
    reason = "no TryFrom impl; values are bounded by screen size"
)]
const fn usize_to_f64(n: usize) -> f64 {
    n as f64
}

/// Compute SVG `points` attribute for a polyline through `values`.
///
/// Pure function: produced as `"x1,y1 x2,y2 ..."` strings for given
/// width/height. Empty input yields an empty string.
#[must_use]
pub fn polyline_points(values: &[f64], width: f64, height: f64) -> String {
    if values.is_empty() || width <= 0.0 || height <= 0.0 {
        return String::new();
    }
    // WHY: SVG coordinates must be finite. NaN/+Inf/-Inf samples would
    // otherwise silently produce an invalid `points` attribute that browsers
    // discard entirely, making the sparkline vanish without error.
    let values: Vec<f64> = values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .collect();
    if values.is_empty() {
        return String::new();
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(f64::EPSILON);
    let n = values.len();
    let step = if n > 1 {
        width / usize_to_f64(n - 1)
    } else {
        0.0
    };
    let mut out = String::with_capacity(n * 12);
    for (i, &v) in values.iter().enumerate() {
        let x = step * usize_to_f64(i);
        // Higher value renders nearer the top — invert.
        let y = height - ((v - min) / range) * height;
        if i > 0 {
            out.push(' ');
        }
        // Two decimal places — sparkline geometry doesn't need more.
        // kanon:ignore RUST/no-silent-result-swallow -- write_fmt into String cannot fail (String's fmt::Write impl is infallible)
        let _ = std::fmt::Write::write_fmt(&mut out, format_args!("{x:.2},{y:.2}"));
    }
    out
}

/// Compute (x, height) bar positions for `values` at fixed bar width.
#[must_use]
pub fn bar_positions(values: &[f64], width: f64, height: f64) -> Vec<(f64, f64, f64, f64)> {
    if values.is_empty() || width <= 0.0 || height <= 0.0 {
        return Vec::new();
    }
    // WHY: Bar heights derive from a finite normalization divisor. Non-finite
    // samples would otherwise turn into NaN heights and render as zero.
    let values: Vec<f64> = values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .collect();
    if values.is_empty() {
        return Vec::new();
    }
    let max = values
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        .max(f64::EPSILON);
    let n = values.len();
    let bar_w = (width / usize_to_f64(n)).max(1.0);
    let gap = (bar_w * 0.2).min(2.0);
    let inner = (bar_w - gap).max(0.5);
    values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = bar_w * usize_to_f64(i) + gap / 2.0;
            // Bars extend downward from the top of the available height.
            // Render Y = height - bar_h so the bar starts at the bottom.
            let normalized = (v.max(0.0) / max).clamp(0.0, 1.0);
            let bar_h = normalized * height;
            let y = height - bar_h;
            (x, y, inner, bar_h)
        })
        .collect()
}

/// Inline trend chart. Bars or polyline depending on `shape`.
///
/// # Accessibility
///
/// - **Role**: None — the SVG is marked `aria-hidden="true"` because it
///   is purely decorative.
/// - **Name**: None — no accessible name is generated.
/// - **Consumer responsibility**: The surrounding region or parent
///   component must provide an accessible label that describes what the
///   sparkline represents.
#[component]
pub fn Sparkline(
    /// Data points. Empty array renders an empty SVG.
    values: Vec<f64>,
    /// Color tone — accent default, status colors for emphasis.
    #[props(default)]
    tone: SparklineTone,
    /// Bars or line. Defaults to line.
    #[props(default)]
    shape: SparklineShape,
    /// Optional explicit width in pixels. Default 80.
    #[props(default = 80.0)]
    width: f64,
    /// Optional explicit height in pixels. Default 20.
    #[props(default = 20.0)]
    height: f64,
) -> Element {
    let color = tone.color_token();
    let viewbox = format!("0 0 {width} {height}");
    rsx! {
        svg {
            "aria-hidden": "true",
            view_box: viewbox,
            width: "{width}",
            height: "{height}",
            preserve_aspect_ratio: "none",
            // Baseline rule — subtle border-color line at the bottom.
            line {
                x1: "0",
                y1: "{height}",
                x2: "{width}",
                y2: "{height}",
                stroke: "var(--border)",
                stroke_width: "1",
            }
            match shape {
                SparklineShape::Line => {
                    let pts = polyline_points(&values, width, height);
                    rsx! {
                        polyline {
                            fill: "none",
                            stroke: color,
                            stroke_width: "1.5",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            points: pts,
                        }
                    }
                }
                SparklineShape::Bars => {
                    rsx! {
                        for (i , (x , y , w , h)) in bar_positions(&values, width, height).iter().enumerate() {
                            rect {
                                key: "{i}",
                                x: "{x}",
                                y: "{y}",
                                width: "{w}",
                                height: "{h}",
                                fill: color,
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_aria_hidden() {
        use dioxus::prelude::*;
        use dioxus_ssr::render_element;
        let html = render_element(rsx! {
            Sparkline { values: vec![1.0, 2.0, 3.0] }
        });
        assert!(
            html.contains("aria-hidden=\"true\""),
            "expected aria-hidden on svg in {html}"
        );
    }

    #[test]
    fn tone_color_tokens_use_canonical_namespaces() {
        assert_eq!(SparklineTone::Accent.color_token(), "var(--accent)");
        assert_eq!(
            SparklineTone::Success.color_token(),
            "var(--status-success)"
        );
        assert_eq!(SparklineTone::Error.color_token(), "var(--status-error)");
    }

    #[test]
    fn defaults_are_accent_and_line() {
        assert_eq!(SparklineTone::default(), SparklineTone::Accent);
        assert_eq!(SparklineShape::default(), SparklineShape::Line);
    }

    #[test]
    fn polyline_points_empty_input() {
        assert_eq!(polyline_points(&[], 100.0, 20.0), "");
    }

    #[test]
    fn polyline_points_zero_dimensions() {
        assert_eq!(polyline_points(&[1.0, 2.0], 0.0, 20.0), "");
        assert_eq!(polyline_points(&[1.0, 2.0], 100.0, 0.0), "");
    }

    #[test]
    fn polyline_points_distributes_x_evenly() {
        let s = polyline_points(&[0.0, 10.0, 20.0], 100.0, 20.0);
        // 3 points across width 100 → x at 0, 50, 100.
        assert!(s.contains("0.00,"), "expected x=0.00 in {s:?}");
        assert!(s.contains("50.00,"), "expected x=50.00 in {s:?}");
        assert!(s.contains("100.00,"), "expected x=100.00 in {s:?}");
    }

    #[test]
    fn polyline_points_higher_value_is_higher_on_screen() {
        // y=0 is the top of the SVG. Higher value → smaller y.
        let s = polyline_points(&[0.0, 100.0], 50.0, 20.0);
        // Two points: first (low) at y near 20, second (high) at y near 0.
        assert!(s.contains("0.00,20.00") || s.contains("0,20.00"), "{s:?}");
        assert!(s.contains("50.00,0.00") || s.contains("50,0.00"), "{s:?}");
    }

    #[test]
    fn polyline_points_constant_values_render_at_bottom() {
        // All values equal: range is f64::EPSILON, normalizes to ~0,
        // y = height - 0 = height (bottom).
        let s = polyline_points(&[5.0, 5.0, 5.0], 30.0, 20.0);
        assert!(s.contains("20.00") || s.contains("19.9"), "{s:?}");
    }

    #[test]
    fn bar_positions_empty_input() {
        assert!(bar_positions(&[], 100.0, 20.0).is_empty());
    }

    #[test]
    fn bar_positions_one_bar_per_value() {
        let bars = bar_positions(&[1.0, 2.0, 3.0], 90.0, 20.0);
        assert_eq!(bars.len(), 3);
    }

    #[test]
    fn bar_positions_bar_height_proportional_to_value() {
        let bars = bar_positions(&[0.0, 50.0, 100.0], 60.0, 20.0);
        assert!(bars[0].3 < bars[1].3, "0 < 50 height");
        assert!(bars[1].3 < bars[2].3, "50 < 100 height");
        // Last bar (max value) reaches full height.
        assert!((bars[2].3 - 20.0).abs() < 0.01, "max → full height");
    }

    #[test]
    fn bar_positions_clamps_negative_values_to_zero() {
        let bars = bar_positions(&[-10.0, 5.0, 10.0], 30.0, 20.0);
        assert_eq!(bars[0].3, 0.0, "negative → zero height");
    }

    #[test]
    fn polyline_points_returns_empty_when_width_is_negative() {
        assert_eq!(polyline_points(&[1.0, 2.0], -10.0, 20.0), "");
    }

    #[test]
    fn polyline_points_returns_empty_when_height_is_negative() {
        assert_eq!(polyline_points(&[1.0, 2.0], 100.0, -5.0), "");
    }

    #[test]
    fn polyline_points_centers_single_value_at_x_zero() {
        let s = polyline_points(&[42.0], 100.0, 20.0);
        assert!(
            s.starts_with("0.00,"),
            "single point should start at x=0: {s:?}"
        );
    }

    #[test]
    fn polyline_points_handles_all_negative_values() {
        let s = polyline_points(&[-20.0, -10.0], 50.0, 20.0);
        // Highest (least negative) value maps to top (y≈0), lowest to bottom (y≈20)
        assert!(
            s.contains("50.00,0.00") || s.contains("50,0.00"),
            "highest negative at top: {s:?}"
        );
        assert!(
            s.contains("0.00,20.00") || s.contains("0,20.00"),
            "lowest negative at bottom: {s:?}"
        );
    }

    #[test]
    fn polyline_points_handles_mixed_positive_and_negative_values() {
        let s = polyline_points(&[-10.0, 10.0], 50.0, 20.0);
        // Lowest value at bottom, highest at top
        assert!(
            s.contains("0.00,20.00") || s.contains("0,20.00"),
            "negative value at bottom: {s:?}"
        );
        assert!(
            s.contains("50.00,0.00") || s.contains("50,0.00"),
            "positive value at top: {s:?}"
        );
    }

    #[test]
    fn polyline_points_renders_at_bottom_when_values_are_extremely_close() {
        let s = polyline_points(&[1.0, 1.0 + 1e-20], 30.0, 20.0);
        // range gets clamped to EPSILON, both points map to y≈height
        assert!(
            s.contains("20.00"),
            "extremely close values should render at bottom: {s:?}"
        );
    }

    #[test]
    fn polyline_points_renders_single_negative_value_at_bottom() {
        let s = polyline_points(&[-5.0], 100.0, 20.0);
        assert!(
            s.contains("20.00"),
            "single negative value should render at bottom: {s:?}"
        );
    }

    #[test]
    fn bar_positions_returns_empty_when_width_is_zero() {
        assert!(bar_positions(&[1.0, 2.0], 0.0, 20.0).is_empty());
    }

    #[test]
    fn bar_positions_returns_empty_when_height_is_zero() {
        assert!(bar_positions(&[1.0, 2.0], 100.0, 0.0).is_empty());
    }

    #[test]
    fn bar_positions_returns_empty_when_width_is_negative() {
        assert!(bar_positions(&[1.0, 2.0], -10.0, 20.0).is_empty());
    }

    #[test]
    fn bar_positions_returns_empty_when_height_is_negative() {
        assert!(bar_positions(&[1.0, 2.0], 100.0, -5.0).is_empty());
    }

    #[test]
    fn bar_positions_returns_single_bar_with_full_height_for_one_positive_value() {
        let bars = bar_positions(&[10.0], 60.0, 20.0);
        assert_eq!(bars.len(), 1);
        assert!(
            (bars[0].3 - 20.0).abs() < 0.01,
            "single positive bar should reach full height"
        );
    }

    #[test]
    fn bar_positions_returns_single_bar_with_zero_height_for_one_negative_value() {
        let bars = bar_positions(&[-5.0], 60.0, 20.0);
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].3, 0.0, "single negative value → zero height");
    }

    #[test]
    fn bar_positions_clamps_all_negative_values_to_zero_height() {
        let bars = bar_positions(&[-5.0, -2.0], 30.0, 20.0);
        assert!(
            bars.iter().all(|b| b.3 == 0.0),
            "all-negative inputs → all zero-height bars"
        );
    }

    #[test]
    fn bar_positions_returns_zero_height_for_all_zero_values() {
        let bars = bar_positions(&[0.0, 0.0], 30.0, 20.0);
        assert!(
            bars.iter().all(|b| b.3 == 0.0),
            "all-zero inputs → all zero-height bars"
        );
    }

    #[test]
    fn bar_positions_bar_x_exceeds_width_when_width_smaller_than_bar_count() {
        let bars = bar_positions(&[1.0; 100], 10.0, 20.0);
        let last_x = bars.last().unwrap().0;
        assert!(
            last_x > 10.0,
            "last bar x ({last_x}) should exceed viewport width"
        );
    }

    #[test]
    fn polyline_points_returns_empty_for_all_nan() {
        assert_eq!(polyline_points(&[f64::NAN, f64::NAN], 100.0, 20.0), "");
    }

    #[test]
    fn polyline_points_returns_empty_for_all_inf() {
        assert_eq!(
            polyline_points(&[f64::INFINITY, f64::NEG_INFINITY], 100.0, 20.0),
            ""
        );
    }

    #[test]
    fn polyline_points_filters_nan_and_preserves_finite_points() {
        let s = polyline_points(&[f64::NAN, 0.0, 10.0, f64::NAN], 100.0, 20.0);
        assert!(!s.contains("NaN"), "output must not contain NaN: {s:?}");
        assert!(s.contains("0.00,"), "finite low point preserved: {s:?}");
        assert!(s.contains("100.00,"), "finite high point preserved: {s:?}");
    }

    #[test]
    fn polyline_points_filters_positive_inf() {
        let s = polyline_points(&[f64::INFINITY, 5.0, 15.0], 50.0, 20.0);
        assert!(!s.contains("NaN"), "output must not contain NaN: {s:?}");
        assert!(s.contains("0.00,"), "finite point preserved: {s:?}");
        assert!(s.contains("50.00,"), "finite point preserved: {s:?}");
    }

    #[test]
    fn polyline_points_filters_negative_inf() {
        let s = polyline_points(&[f64::NEG_INFINITY, 5.0, 15.0], 50.0, 20.0);
        assert!(!s.contains("NaN"), "output must not contain NaN: {s:?}");
        assert!(s.contains("0.00,"), "finite point preserved: {s:?}");
        assert!(s.contains("50.00,"), "finite point preserved: {s:?}");
    }

    #[test]
    fn bar_positions_returns_empty_for_all_nan() {
        assert!(bar_positions(&[f64::NAN, f64::NAN], 100.0, 20.0).is_empty());
    }

    #[test]
    fn bar_positions_returns_empty_for_all_inf() {
        assert!(bar_positions(&[f64::INFINITY, f64::NEG_INFINITY], 100.0, 20.0).is_empty());
    }

    #[test]
    fn bar_positions_filters_nan_and_preserves_finite_bars() {
        let bars = bar_positions(&[f64::NAN, 0.0, 10.0, f64::NAN], 100.0, 20.0);
        assert_eq!(bars.len(), 2, "only finite values produce bars");
        assert!(
            bars.iter().all(|b| b.3.is_finite()),
            "all bar heights must be finite: {bars:?}"
        );
    }

    #[test]
    fn bar_positions_filters_positive_inf() {
        let bars = bar_positions(&[f64::INFINITY, 5.0, 15.0], 60.0, 20.0);
        assert_eq!(bars.len(), 2, "only finite values produce bars");
        assert!(
            bars.iter().all(|b| b.3.is_finite()),
            "all bar heights must be finite: {bars:?}"
        );
    }

    #[test]
    fn bar_positions_filters_negative_inf() {
        let bars = bar_positions(&[f64::NEG_INFINITY, 5.0, 15.0], 60.0, 20.0);
        assert_eq!(bars.len(), 2, "only finite values produce bars");
        assert!(
            bars.iter().all(|b| b.3.is_finite()),
            "all bar heights must be finite: {bars:?}"
        );
    }
}
