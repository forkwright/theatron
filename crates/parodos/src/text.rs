//! Unicode-safe text truncation helpers for terminal display.

use ratatui::text::Span;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const ELLIPSIS: char = '…';

/// Truncates `text` to at most `max_chars` Unicode scalar values.
///
/// When truncation is needed, the returned string ends with `…` and
/// the ellipsis is included in the `max_chars` budget. A zero budget
/// returns an empty string.
#[must_use]
pub fn truncate_chars_ellipsis(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_owned();
    }

    if max_chars == 0 {
        return String::new();
    }

    let prefix: String = text.chars().take(max_chars - 1).collect();
    format!("{prefix}{ELLIPSIS}")
}

/// Truncates `text` to at most `max_cols` terminal display columns.
///
/// Width is measured with Unicode display width. When truncation is
/// needed, the returned string ends with `…` and the ellipsis is
/// included in the `max_cols` budget. A zero budget returns an empty
/// string.
#[must_use]
pub fn truncate_cols_ellipsis(text: &str, max_cols: usize) -> String {
    if UnicodeWidthStr::width(text) <= max_cols {
        return text.to_owned();
    }

    if max_cols == 0 {
        return String::new();
    }

    let mut out = prefix_cols(text, max_cols - 1).to_owned();
    out.push(ELLIPSIS);
    out
}

/// Truncates styled spans to at most `max_cols` terminal display columns.
///
/// Span styles are preserved for retained content. When truncation is
/// needed, the returned spans end with `…` and the ellipsis is included
/// in the `max_cols` budget. A zero budget returns no spans unless the
/// input already has zero width.
#[must_use]
pub fn truncate_spans_cols<'a>(
    spans: impl IntoIterator<Item = Span<'a>>,
    max_cols: usize,
) -> Vec<Span<'a>> {
    let spans: Vec<_> = spans.into_iter().collect();
    let total_width: usize = spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
        .sum();

    if total_width <= max_cols {
        return spans;
    }

    if max_cols == 0 {
        return Vec::new();
    }

    let mut remaining = max_cols - 1;
    let mut out = Vec::new();
    let mut ellipsis_style = spans
        .first()
        .map_or_else(Default::default, |span| span.style);

    for span in spans {
        if remaining == 0 {
            ellipsis_style = span.style;
            break;
        }

        let span_width = UnicodeWidthStr::width(span.content.as_ref());
        if span_width <= remaining {
            remaining -= span_width;
            ellipsis_style = span.style;
            out.push(span);
            continue;
        }

        let prefix = prefix_cols(span.content.as_ref(), remaining);
        ellipsis_style = span.style;
        if !prefix.is_empty() {
            out.push(Span::styled(prefix.to_owned(), span.style));
        }
        break;
    }

    if let Some(last) = out.last_mut() {
        last.content.to_mut().push(ELLIPSIS);
    } else {
        out.push(Span::styled(ELLIPSIS.to_string(), ellipsis_style));
    }

    out
}

fn prefix_cols(text: &str, max_cols: usize) -> &str {
    if UnicodeWidthStr::width(text) <= max_cols {
        return text;
    }

    let mut width = 0usize;
    let mut end = 0usize;

    for (idx, ch) in text.char_indices() {
        let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + char_width > max_cols {
            break;
        }

        width += char_width;
        end = idx + ch.len_utf8();
    }

    text.get(..end).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use ratatui::style::{Color, Style};

    use super::*;

    fn span_text(spans: &[Span<'_>]) -> String {
        spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    }

    #[test]
    fn truncate_chars_ellipsis_leaves_short_text_unchanged() {
        assert_eq!(truncate_chars_ellipsis("hello", 5), "hello");
        assert_eq!(truncate_chars_ellipsis("hello", 10), "hello");
    }

    #[test]
    fn truncate_chars_ellipsis_keeps_ellipsis_inside_budget() {
        let truncated = truncate_chars_ellipsis("hello", 4);

        assert_eq!(truncated, "hel…");
        assert_eq!(truncated.chars().count(), 4);
    }

    #[test]
    fn truncate_chars_ellipsis_respects_multibyte_boundaries() {
        assert_eq!(truncate_chars_ellipsis("aé日b", 3), "aé…");
    }

    #[test]
    fn truncate_chars_ellipsis_handles_tiny_budgets() {
        assert_eq!(truncate_chars_ellipsis("hello", 0), "");
        assert_eq!(truncate_chars_ellipsis("hello", 1), "…");
    }

    #[test]
    fn truncate_cols_ellipsis_leaves_short_text_unchanged() {
        assert_eq!(truncate_cols_ellipsis("hello", 5), "hello");
        assert_eq!(truncate_cols_ellipsis("hello", 8), "hello");
    }

    #[test]
    fn truncate_cols_ellipsis_keeps_ellipsis_inside_budget() {
        let truncated = truncate_cols_ellipsis("hello", 4);

        assert_eq!(truncated, "hel…");
        assert_eq!(UnicodeWidthStr::width(truncated.as_str()), 4);
    }

    #[test]
    fn truncate_cols_ellipsis_respects_wide_characters() {
        let truncated = truncate_cols_ellipsis("ab日本", 5);

        assert_eq!(truncated, "ab日…");
        assert_eq!(UnicodeWidthStr::width(truncated.as_str()), 5);
    }

    #[test]
    fn truncate_cols_ellipsis_handles_tiny_budgets() {
        assert_eq!(truncate_cols_ellipsis("hello", 0), "");
        assert_eq!(truncate_cols_ellipsis("hello", 1), "…");
    }

    #[test]
    fn truncate_spans_cols_leaves_fitting_spans_unchanged() {
        let spans = vec![Span::raw("hi"), Span::raw(" there")];
        let out = truncate_spans_cols(spans, 8);

        assert_eq!(span_text(&out), "hi there");
    }

    #[test]
    fn truncate_spans_cols_truncates_inside_span() {
        let style = Style::default().fg(Color::Yellow);
        let spans = vec![Span::styled("hello", style), Span::raw(" world")];
        let out = truncate_spans_cols(spans, 6);

        assert_eq!(span_text(&out), "hello…");
        assert_eq!(out[0].style, style);
    }

    #[test]
    fn truncate_spans_cols_respects_wide_characters() {
        let spans = vec![Span::raw("ab"), Span::raw("日本語")];
        let out = truncate_spans_cols(spans, 5);

        assert_eq!(span_text(&out), "ab日…");
        assert_eq!(UnicodeWidthStr::width(span_text(&out).as_str()), 5);
    }

    #[test]
    fn truncate_spans_cols_handles_tiny_budgets() {
        let style = Style::default().fg(Color::Blue);
        let out = truncate_spans_cols(vec![Span::styled("hello", style)], 1);

        assert_eq!(span_text(&out), "…");
        assert_eq!(out[0].style, style);
        assert!(truncate_spans_cols(vec![Span::raw("hello")], 0).is_empty());
    }
}
