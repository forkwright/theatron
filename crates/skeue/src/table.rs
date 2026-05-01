//! Markdown table rendering component.

use dioxus::prelude::*;

/// Per-column horizontal alignment for [`MdTable`].
///
/// skeue defines this enum locally rather than re-using
/// `pulldown_cmark::Alignment` so consumers don't have to share a major
/// version of pulldown-cmark with us. Use the `From<pulldown_cmark::Alignment>`
/// impl when you've already parsed markdown with pulldown-cmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum TableAlignment {
    /// Default — renders left-aligned.
    #[default]
    None,
    /// Left-aligned.
    Left,
    /// Center-aligned.
    Center,
    /// Right-aligned.
    Right,
}

impl TableAlignment {
    fn css(self) -> &'static str {
        match self {
            Self::None | Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }
}

impl From<pulldown_cmark::Alignment> for TableAlignment {
    fn from(value: pulldown_cmark::Alignment) -> Self {
        match value {
            pulldown_cmark::Alignment::None => Self::None,
            pulldown_cmark::Alignment::Left => Self::Left,
            pulldown_cmark::Alignment::Center => Self::Center,
            pulldown_cmark::Alignment::Right => Self::Right,
        }
    }
}

/// Render a markdown table with column alignment and alternating row backgrounds.
#[component]
pub fn MdTable(
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    alignments: Vec<TableAlignment>,
) -> Element {
    rsx! {
        div {
            style: "
                overflow-x: auto;
                margin: var(--space-2) 0;
                border: 1px solid var(--border);
                border-radius: var(--radius-lg);
            ",
            table {
                style: "
                    width: 100%;
                    border-collapse: collapse;
                    font-size: var(--text-sm);
                    font-family: var(--font-body);
                ",
                thead {
                    tr {
                        style: "
                            background: var(--bg-surface-dim);
                            border-bottom: 2px solid var(--border);
                        ",
                        for (i , header) in headers.iter().enumerate() {
                            th {
                                key: "{i}",
                                style: "
                                    padding: var(--space-2) var(--space-3);
                                    text-align: {alignment_css(alignments.get(i).copied())};
                                    color: var(--text-primary);
                                    font-weight: var(--weight-semibold);
                                ",
                                "{header}"
                            }
                        }
                    }
                }
                tbody {
                    for (row_idx , row) in rows.iter().enumerate() {
                        tr {
                            key: "{row_idx}",
                            style: "
                                background: {row_bg(row_idx)};
                                border-bottom: 1px solid var(--border-separator);
                            ",
                            for (col_idx , cell) in row.iter().enumerate() {
                                td {
                                    key: "{col_idx}",
                                    style: "
                                        padding: var(--space-2) var(--space-3);
                                        text-align: {alignment_css(alignments.get(col_idx).copied())};
                                        color: var(--text-secondary);
                                    ",
                                    "{cell}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn alignment_css(align: Option<TableAlignment>) -> &'static str {
    align.unwrap_or_default().css()
}

fn row_bg(idx: usize) -> &'static str {
    if idx % 2 == 0 {
        "var(--bg-surface)"
    } else {
        "var(--bg)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_css_values() {
        assert_eq!(alignment_css(None), "left");
        assert_eq!(alignment_css(Some(TableAlignment::Left)), "left");
        assert_eq!(alignment_css(Some(TableAlignment::None)), "left");
        assert_eq!(alignment_css(Some(TableAlignment::Center)), "center");
        assert_eq!(alignment_css(Some(TableAlignment::Right)), "right");
    }

    #[test]
    fn row_bg_alternates() {
        assert_eq!(row_bg(0), "var(--bg-surface)");
        assert_eq!(row_bg(1), "var(--bg)");
        assert_eq!(row_bg(2), "var(--bg-surface)");
        assert_eq!(row_bg(3), "var(--bg)");
    }

    #[test]
    fn from_pulldown_cmark_alignment_maps_correctly() {
        use pulldown_cmark::Alignment as PA;
        assert_eq!(TableAlignment::from(PA::None), TableAlignment::None);
        assert_eq!(TableAlignment::from(PA::Left), TableAlignment::Left);
        assert_eq!(TableAlignment::from(PA::Center), TableAlignment::Center);
        assert_eq!(TableAlignment::from(PA::Right), TableAlignment::Right);
    }
}
