//! Syntax-highlighted code block component.
//!
//! Calls [`gramma::highlight_code`] for the per-line styled
//! spans, then renders them as a Dioxus tree with a header (language
//! label + copy-to-clipboard button) and a line-numbered gutter.

use dioxus::prelude::*;
use gramma::{HighlightedSpan, highlight_code};

const BLOCK_STYLE: &str = "\
    position: relative; \
    background: var(--code-bg); \
    border: 1px solid var(--border); \
    border-radius: var(--radius-lg); \
    margin: var(--space-2) 0; \
    overflow: hidden;\
";

const HEADER_STYLE: &str = "\
    display: flex; \
    justify-content: space-between; \
    align-items: center; \
    padding: var(--space-1) var(--space-3); \
    background: var(--bg-surface-dim); \
    border-bottom: 1px solid var(--border); \
    font-family: var(--font-mono); \
    font-size: var(--text-xs); \
    color: var(--code-lang);\
";

const COPY_BUTTON_STYLE: &str = "\
    background: none; \
    border: 1px solid var(--border); \
    border-radius: var(--radius-md); \
    color: var(--text-muted); \
    font-family: var(--font-mono); \
    font-size: var(--text-xs); \
    padding: var(--space-1) var(--space-2); \
    cursor: pointer; \
    transition: \
        background-color var(--transition-quick), \
        color var(--transition-quick), \
        border-color var(--transition-quick);\
";

const CODE_BODY_STYLE: &str = "\
    overflow-x: auto; \
    padding: var(--space-2) 0; \
    font-family: var(--font-mono); \
    font-size: var(--text-sm); \
    line-height: var(--leading-normal);\
";

const LINE_STYLE: &str = "display: flex; min-height: 1.5em;";
const CONTENT_STYLE: &str = "white-space: pre; flex: 1;";

/// Render a syntax-highlighted code block with copy-to-clipboard.
///
/// # Accessibility
///
/// - **Role**: `region` — the code block is a landmark region.
/// - **Name**: `aria-label` is set to `"Code: {language}"`.
/// - **Keyboard navigation**: The copy button is focusable and has an
///   `aria-label` describing its action.
/// - **Consumer responsibility**: None.
#[component]
pub fn CodeBlock(code: String, language: String) -> Element {
    let lang_display = if language.is_empty() {
        "text".to_string()
    } else {
        language.clone()
    };

    let highlighted = highlight_code(&code, &language);
    let line_count = highlighted.len();
    // WHY: digit width for line-number gutter padding.
    let gutter_width = format!("{line_count}").len();

    let aria_label = format!("Code: {lang_display}");
    rsx! {
        div {
            class: "code-block",
            role: "region",
            aria_label: "{aria_label}",
            style: "{BLOCK_STYLE}",
            div {
                style: "{HEADER_STYLE}",
                span { "{lang_display}" }
                button {
                    aria_label: "Copy {lang_display} code to clipboard",
                    onclick: {
                        let code_clone = code.clone();
                        move |_| {
                            let escaped = code_clone
                                .replace('\\', "\\\\")
                                .replace('`', "\\`")
                                .replace('$', "\\$");
                            let js = format!("navigator.clipboard.writeText(`{escaped}`)");
                            document::eval(&js);
                        }
                    },
                    style: "{COPY_BUTTON_STYLE}",
                    "copy"
                }
            }
            div {
                style: "{CODE_BODY_STYLE}",
                for (i , line_spans) in highlighted.iter().enumerate() {
                    div {
                        key: "{i}",
                        style: "{LINE_STYLE}",
                        span {
                            style: "
                                display: inline-block;
                                width: {gutter_width + 2}ch;
                                text-align: right;
                                padding-right: var(--space-3);
                                color: var(--text-muted);
                                user-select: none;
                                flex-shrink: 0;
                            ",
                            "{i + 1}"
                        }
                        span {
                            style: "{CONTENT_STYLE}",
                            for (j , span) in line_spans.iter().enumerate() {
                                {render_span(j, span)}
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_span(key: usize, span: &HighlightedSpan) -> Element {
    let bold = if span.bold {
        " font-weight: var(--weight-bold);"
    } else {
        ""
    };
    let italic = if span.italic {
        " font-style: italic;"
    } else {
        ""
    };
    let style = format!("color: {};{bold}{italic}", span.color);
    let text = span.text.clone();
    rsx! {
        span {
            key: "{key}",
            style: "{style}",
            "{text}"
        }
    }
}

#[cfg(test)]
mod ssr_tests {
    use dioxus_ssr::render_element;

    use super::*;

    #[test]
    fn renders_region_and_copy_aria_label() {
        let html = render_element(rsx! {
            CodeBlock {
                code: "fn main() {}".to_string(),
                language: "rust".to_string(),
            }
        });
        assert!(
            html.contains("role=\"region\""),
            "expected role=region in {html}"
        );
        assert!(
            html.contains("aria-label=\"Code: rust\""),
            "expected region aria-label in {html}"
        );
        assert!(
            html.contains("aria-label=\"Copy rust code to clipboard\""),
            "expected copy button aria-label in {html}"
        );
    }
}
