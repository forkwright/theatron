//! Shared ratatui layout helpers.

use ratatui::layout::Rect;

/// Returns a centered rectangle sized as a percentage of `area`.
///
/// Percent values above 100 are clamped to 100. Zero percent on either
/// axis yields a zero-sized rectangle centered in that axis.
#[must_use]
pub fn centered_rect_pct(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let width = size_from_percent(area.width, percent_x);
    let height = size_from_percent(area.height, percent_y);

    centered_rect_size(width, height, area)
}

/// Returns a centered rectangle with the requested size inside `area`.
///
/// Requested dimensions larger than `area` are clamped to `area`. When
/// the remaining space on an axis is odd, the extra cell stays on the
/// trailing side.
#[must_use]
pub fn centered_rect_size(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x_offset = (area.width - width) / 2;
    let y_offset = (area.height - height) / 2;

    Rect {
        x: area.x.saturating_add(x_offset),
        y: area.y.saturating_add(y_offset),
        width,
        height,
    }
}

fn size_from_percent(size: u16, percent: u16) -> u16 {
    let scaled = u32::from(size) * u32::from(percent.min(100)) / 100;

    #[expect(
        clippy::cast_possible_truncation,
        reason = "percentage is clamped to 100, so scaled never exceeds the u16 input size"
    )]
    {
        scaled as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: u16, y: u16, width: u16, height: u16) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn centered_rect_pct_centers_normal_percentages() {
        let area = rect(0, 0, 100, 80);

        assert_eq!(centered_rect_pct(50, 25, area), rect(25, 30, 50, 20));
    }

    #[test]
    fn centered_rect_pct_clamps_percentages_above_one_hundred() {
        let area = rect(4, 6, 30, 20);

        assert_eq!(centered_rect_pct(150, 200, area), area);
    }

    #[test]
    fn centered_rect_pct_allows_zero_sized_axes() {
        let area = rect(0, 0, 10, 8);

        assert_eq!(centered_rect_pct(0, 0, area), rect(5, 4, 0, 0));
    }

    #[test]
    fn centered_rect_size_preserves_exact_size() {
        let area = rect(7, 9, 20, 10);

        assert_eq!(centered_rect_size(20, 10, area), area);
    }

    #[test]
    fn centered_rect_size_clamps_oversized_requests() {
        let area = rect(2, 3, 12, 5);

        assert_eq!(centered_rect_size(40, 10, area), area);
    }

    #[test]
    fn centered_rect_size_keeps_odd_leftover_on_trailing_side() {
        let area = rect(0, 0, 11, 9);

        assert_eq!(centered_rect_size(4, 4, area), rect(3, 2, 4, 4));
    }

    #[test]
    fn centered_rect_size_preserves_nonzero_origin() {
        let area = rect(10, 20, 30, 18);

        assert_eq!(centered_rect_size(10, 6, area), rect(20, 26, 10, 6));
    }

    #[test]
    fn centered_rect_pct_does_not_overflow_large_rectangles() {
        let area = rect(0, 0, u16::MAX, u16::MAX);

        assert_eq!(
            centered_rect_pct(100, 100, area),
            rect(0, 0, u16::MAX, u16::MAX)
        );
    }
}
