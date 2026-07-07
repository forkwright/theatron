//! Virtual scrolling utilities for large lists.
//!
//! Provides a container component and a `visible_range` utility used by chat,
//! sessions, and memory views to render only the items inside the viewport
//! plus a configurable overscan buffer.

use dioxus::prelude::*;

/// Default overscan -- extra items rendered above and below the visible window.
pub const DEFAULT_OVERSCAN: usize = 10;

/// Compute which item indices are visible given scroll position and item height.
///
/// Returns `(range_start, range_end)` -- a half-open slice into the item list.
/// Both ends are clamped to `[0, total_items]`. Non-finite `scroll_top`,
/// `container_height`, or `item_height` inputs (uninitialized layout state)
/// and non-positive `item_height` return the empty range `(0, 0)`.
#[must_use]
pub fn visible_range(
    scroll_top: f64,
    container_height: f64,
    total_items: usize,
    item_height: f64,
    overscan: usize,
) -> (usize, usize) {
    // WHY: `NaN <= 0.0` is false, so a plain sign check lets NaN through and
    // the divisions below produce a garbage non-empty range.
    if total_items == 0
        || !scroll_top.is_finite()
        || !container_height.is_finite()
        || !item_height.is_finite()
        || item_height <= 0.0
    {
        return (0, 0);
    }
    #[expect(
        clippy::as_conversions,
        reason = "scroll position to index, clamped to total_items"
    )]
    let first = ((scroll_top / item_height) as usize).min(total_items);
    #[expect(
        clippy::as_conversions,
        reason = "visible item count from container height, clamped"
    )]
    let count = ((container_height / item_height).ceil() as usize + 1).min(total_items);
    let start = first.saturating_sub(overscan);
    let end = (first + count + overscan).min(total_items);
    (start, end)
}

/// Compute spacer heights for virtual scroll from a visible range.
///
/// Returns `(pad_top_px, pad_bottom_px)`.
#[must_use]
pub fn spacer_heights(
    range_start: usize,
    range_end: usize,
    total_items: usize,
    item_height: f64,
) -> (f64, f64) {
    #[expect(
        clippy::as_conversions,
        reason = "index to pixel offset for virtual scroll spacer"
    )]
    let pad_top = range_start as f64 * item_height;
    #[expect(
        clippy::as_conversions,
        reason = "index to pixel offset for virtual scroll spacer"
    )]
    let pad_bottom = total_items.saturating_sub(range_end) as f64 * item_height;
    (pad_top, pad_bottom)
}

/// A scrollable container that manages virtual scroll state.
///
/// Renders a vertically scrollable div with spacer divs above and below
/// the visible items. The parent is responsible for:
/// - Computing visible range with [`visible_range`]
/// - Rendering only the visible items as `children`
/// - Passing the correct `pad_top` / `pad_bottom` values
///
/// The `on_scroll` callback receives `(scroll_top, container_height)` whenever
/// the user scrolls. Use these values to recompute the visible range.
///
/// # Accessibility
///
/// - **Role**: `region` — the scrollable container is a landmark region.
/// - **Name**: The `scroll_key` value is used as `aria-label`.
/// - **Consumer responsibility**: Choose a descriptive `scroll_key` that
///   identifies the scrolled content (e.g. `"chat-messages"`).
#[component]
pub fn VirtualScrollContainer(
    pad_top: f64,
    pad_bottom: f64,
    on_scroll: EventHandler<(f64, f64)>,
    /// ARIA label naming the scrolled content region (e.g. `"chat-messages"`).
    scroll_key: &'static str,
    children: Element,
) -> Element {
    rsx! {
        div {
            style: "flex: 1; overflow-y: auto; position: relative;",
            role: "region",
            "aria-label": scroll_key,
            onscroll: move |evt: Event<ScrollData>| {
                // WHY: ScrollData reads scrollTop/clientHeight off the
                // element that dispatched THIS event, not a page-global
                // `document.querySelector` lookup -- the prior
                // implementation always resolved the FIRST DOM match for a
                // selector, which broke with multiple
                // VirtualScrollContainer instances mounted at once
                // (issue #184.3).
                let top = evt.scroll_top();
                let height = f64::from(evt.client_height());
                if height > 0.0 {
                    on_scroll.call((top, height));
                }
            },

            // Top spacer -- represents off-screen items above viewport
            div {
                style: "height: {pad_top}px; flex-shrink: 0;",
                "aria-hidden": "true",
            }

            {children}

            // Bottom spacer -- represents off-screen items below viewport
            div {
                style: "height: {pad_bottom}px; flex-shrink: 0;",
                "aria-hidden": "true",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test for issue #184.3: the prior onscroll handler resolved
    /// its target element via a page-global `document.querySelector`, which
    /// always returns the FIRST DOM match regardless of which instance
    /// fired the event. Two instances with distinct `scroll_key`s must
    /// render independent `aria-label`s and carry no shared selector target
    /// for a future regression to collide on.
    #[test]
    fn two_instances_render_independent_labels_without_shared_selector() {
        #[component]
        fn Wrapper() -> Element {
            rsx! {
                div {
                    VirtualScrollContainer {
                        pad_top: 0.0,
                        pad_bottom: 0.0,
                        on_scroll: |_| {},
                        scroll_key: "chat-messages",
                        div { "chat item" }
                    }
                    VirtualScrollContainer {
                        pad_top: 0.0,
                        pad_bottom: 0.0,
                        on_scroll: |_| {},
                        scroll_key: "session-list",
                        div { "session item" }
                    }
                }
            }
        }
        let mut dom = VirtualDom::new(Wrapper);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(
            html.contains(r#"aria-label="chat-messages""#),
            "expected first instance's aria-label in {html}"
        );
        assert!(
            html.contains(r#"aria-label="session-list""#),
            "expected second instance's aria-label in {html}"
        );
        assert!(
            !html.contains("data-vscroll"),
            "no page-global scroll-selector target should remain: {html}"
        );
    }

    #[test]
    fn visible_range_empty_list() {
        let (s, e) = visible_range(0.0, 600.0, 0, 80.0, 10);
        assert_eq!((s, e), (0, 0));
    }

    #[test]
    fn visible_range_at_top_no_scroll() {
        // Container 600px, item 80px → ceil(600/80)+1 = 9 visible; +10 overscan each side
        let (s, e) = visible_range(0.0, 600.0, 100, 80.0, 10);
        assert_eq!(s, 0); // clamped at 0
        // first=0, count=9, end=0+9+10=19
        assert_eq!(e, 19);
    }

    #[test]
    fn visible_range_scrolled_mid_list() {
        // scroll=800px → first=10, count=9, start=0, end=29
        let (s, e) = visible_range(800.0, 600.0, 100, 80.0, 10);
        assert_eq!(s, 0); // 10 - 10 = 0
        assert_eq!(e, 29); // 10 + 9 + 10 = 29
    }

    #[test]
    fn visible_range_clamped_at_end() {
        let (s, e) = visible_range(5000.0, 600.0, 20, 80.0, 10);
        assert_eq!(e, 20); // clamped to total
        assert!(s <= e);
    }

    #[test]
    fn visible_range_overscan_only_on_renderable_items() {
        // Only 5 items total
        let (s, e) = visible_range(0.0, 600.0, 5, 80.0, 10);
        assert_eq!(s, 0);
        assert_eq!(e, 5);
    }

    #[test]
    fn spacer_heights_basic() {
        let (top, bottom) = spacer_heights(5, 15, 30, 80.0);
        assert!((top - 400.0).abs() < f64::EPSILON);
        assert!((bottom - 1200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn spacer_heights_at_boundaries() {
        let (top, bottom) = spacer_heights(0, 10, 10, 80.0);
        assert!((top - 0.0).abs() < f64::EPSILON);
        assert!((bottom - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn visible_range_zero_height_item() {
        let (s, e) = visible_range(0.0, 600.0, 100, 0.0, 10);
        assert_eq!((s, e), (0, 0));
    }

    #[test]
    fn visible_range_returns_empty_when_item_height_is_negative() {
        let (s, e) = visible_range(0.0, 600.0, 100, -10.0, 10);
        assert_eq!((s, e), (0, 0));
    }

    #[test]
    fn visible_range_clamps_start_to_zero_when_scroll_top_is_negative() {
        let (s, e) = visible_range(-100.0, 600.0, 100, 80.0, 10);
        assert_eq!(s, 0);
        assert!(e > s);
    }

    #[test]
    fn visible_range_truncates_first_item_correctly_when_scroll_top_is_fractional() {
        // scroll_top=50, item_height=80 → first=0 (truncated)
        let (s, e) = visible_range(50.0, 80.0, 100, 80.0, 0);
        assert_eq!(s, 0);
        // count = ceil(80/80) + 1 = 2
        assert_eq!(e, 2);
    }

    #[test]
    fn visible_range_computes_count_correctly_when_container_height_not_divisible_by_item_height() {
        let (s, e) = visible_range(0.0, 600.0, 100, 70.0, 0);
        // count = ceil(600/70) + 1 = 9 + 1 = 10
        assert_eq!(e - s, 10);
    }

    #[test]
    fn visible_range_uses_zero_overscan_without_panic() {
        let (s, e) = visible_range(0.0, 600.0, 100, 80.0, 0);
        // count = ceil(600/80)+1 = 8+1 = 9
        assert_eq!(e - s, 9);
    }

    #[test]
    fn visible_range_clamps_end_to_total_when_overscan_exceeds_bounds_at_middle_scroll() {
        let (_s, e) = visible_range(800.0, 600.0, 20, 80.0, 15);
        // first = 10, count = 9, end = 10+9+15 = 34 -> clamped to 20
        assert_eq!(e, 20);
    }

    #[test]
    fn visible_range_renders_last_items_when_scroll_top_exactly_equals_total_height() {
        // 20 items * 80px = 1600px
        let (s, e) = visible_range(1600.0, 600.0, 20, 80.0, 5);
        // first = 20, count = 9, end = min(20+9+5, 20) = 20
        assert_eq!(e, 20);
        assert!(s <= e);
    }

    #[test]
    fn visible_range_returns_empty_when_scroll_top_is_nan() {
        let (s, e) = visible_range(f64::NAN, 600.0, 100, 80.0, 10);
        assert_eq!((s, e), (0, 0));
    }

    #[test]
    fn visible_range_returns_empty_when_item_height_is_nan() {
        let (s, e) = visible_range(0.0, 600.0, 100, f64::NAN, 10);
        assert_eq!((s, e), (0, 0));
    }

    #[test]
    fn visible_range_returns_empty_when_container_height_is_nan() {
        let (s, e) = visible_range(0.0, f64::NAN, 100, 80.0, 10);
        assert_eq!((s, e), (0, 0));
    }

    #[test]
    fn visible_range_returns_empty_when_inputs_are_infinite() {
        assert_eq!(visible_range(f64::INFINITY, 600.0, 100, 80.0, 10), (0, 0));
        assert_eq!(visible_range(0.0, f64::INFINITY, 100, 80.0, 10), (0, 0));
        assert_eq!(visible_range(0.0, 600.0, 100, f64::INFINITY, 10), (0, 0));
    }

    #[test]
    fn spacer_heights_returns_zero_both_when_item_height_is_zero() {
        let (top, bottom) = spacer_heights(5, 15, 30, 0.0);
        assert!((top - 0.0).abs() < f64::EPSILON);
        assert!((bottom - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn spacer_heights_returns_zero_bottom_when_range_end_equals_total_items() {
        let (top, bottom) = spacer_heights(5, 20, 20, 80.0);
        assert!((top - 400.0).abs() < f64::EPSILON);
        assert!((bottom - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn spacer_heights_returns_zero_top_when_range_start_is_zero() {
        let (top, bottom) = spacer_heights(0, 10, 20, 80.0);
        assert!((top - 0.0).abs() < f64::EPSILON);
        assert!((bottom - 800.0).abs() < f64::EPSILON);
    }

    #[test]
    fn spacer_heights_pads_entire_list_when_range_is_empty() {
        let (top, bottom) = spacer_heights(5, 5, 20, 80.0);
        assert!((top - 400.0).abs() < f64::EPSILON);
        assert!((bottom - 1200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn spacer_heights_pads_top_correctly_when_range_end_exceeds_total_items() {
        let (top, bottom) = spacer_heights(5, 25, 20, 80.0);
        assert!((top - 400.0).abs() < f64::EPSILON);
        assert!((bottom - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn spacer_heights_returns_zero_both_when_total_items_is_zero() {
        let (top, bottom) = spacer_heights(0, 0, 0, 80.0);
        assert!((top - 0.0).abs() < f64::EPSILON);
        assert!((bottom - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn spacer_heights_returns_negative_pad_when_item_height_is_negative() {
        let (top, bottom) = spacer_heights(5, 10, 20, -10.0);
        assert!(top < 0.0);
        assert!(bottom < 0.0);
    }
}
