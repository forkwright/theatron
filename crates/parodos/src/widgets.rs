//! Small string helpers for terminal widget assembly.

/// Returns a fixed-width percentage meter made from `filled` and `empty` cells.
///
/// `pct` is clamped to 100 before cell counts are calculated. The
/// filled cell count uses integer flooring, matching common terminal
/// gauge behavior. A zero `width` returns an empty string.
#[must_use]
// kanon:ignore RUST/pub-visibility -- external TUI consumers share terminal widget string helpers
pub fn meter_string(pct: u8, width: usize, filled: char, empty: char) -> String {
    let pct = usize::from(pct.min(100));
    let filled_count = pct * width / 100;
    let empty_count = width.saturating_sub(filled_count);

    let mut meter = String::with_capacity(width);
    meter.extend(std::iter::repeat_n(filled, filled_count));
    meter.extend(std::iter::repeat_n(empty, empty_count));
    meter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_string_renders_empty_width_as_empty_string() {
        assert_eq!(meter_string(75, 0, '=', '.'), "");
    }

    #[test]
    fn meter_string_renders_zero_percent_as_all_empty() {
        assert_eq!(meter_string(0, 5, '=', '.'), ".....");
    }

    #[test]
    fn meter_string_renders_full_percent_as_all_filled() {
        assert_eq!(meter_string(100, 5, '=', '.'), "=====");
    }

    #[test]
    fn meter_string_clamps_percent_above_one_hundred() {
        assert_eq!(meter_string(150, 5, '=', '.'), "=====");
    }

    #[test]
    fn meter_string_uses_integer_floor_for_partial_cells() {
        assert_eq!(meter_string(33, 10, '=', '.'), "===.......");
    }

    #[test]
    fn meter_string_preserves_custom_glyphs() {
        assert_eq!(meter_string(50, 4, '█', '░'), "██░░");
    }
}
