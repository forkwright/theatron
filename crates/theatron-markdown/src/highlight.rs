//! Syntect-backed source-code highlighting.
//!
//! Returns line-by-line styled spans (no Dioxus dependency). The
//! Dioxus component that renders these spans lives in
//! `theatron_components::CodeBlock`.

use std::str::FromStr;
use std::sync::OnceLock;

use syntect::highlighting::{
    Color as SynColor, FontStyle, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSettings,
};
use syntect::parsing::SyntaxSet;

/// Cached syntax set (loaded once, shared across renders).
fn syntax_set() -> &'static SyntaxSet {
    static SS: OnceLock<SyntaxSet> = OnceLock::new();
    SS.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Warm-shifted theme matching the fleet design system.
///
/// Hand-tuned to align with `--syntax-*` CSS tokens from
/// DESIGN-TOKENS.md. Constructed programmatically rather than loading
/// a `.tmTheme` file so we ship no binary asset.
fn warm_theme() -> &'static Theme {
    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME.get_or_init(|| {
        let settings = ThemeSettings {
            foreground: Some(SynColor {
                r: 0xd4,
                g: 0xd0,
                b: 0xca,
                a: 0xff,
            }),
            background: Some(SynColor {
                r: 0x1a,
                g: 0x18,
                b: 0x16,
                a: 0xff,
            }),
            ..ThemeSettings::default()
        };

        let items = vec![
            theme_item("keyword", 0xCC, 0x77, 0x55),
            theme_item("storage.type", 0xCC, 0x77, 0x55),
            theme_item("string", 0x7A, 0x9B, 0x6A),
            theme_item("comment", 0x70, 0x6c, 0x66),
            theme_item("entity.name.function", 0xB0, 0x8E, 0x5C),
            theme_item("entity.name.type", 0x8A, 0x9A, 0xB0),
            theme_item("support.type", 0x8A, 0x9A, 0xB0),
            theme_item("constant.numeric", 0xC4, 0x91, 0x3A),
            theme_item("keyword.operator", 0xa8, 0xa4, 0x9e),
            theme_item("punctuation", 0xa8, 0xa4, 0x9e),
            theme_item("variable", 0xd4, 0xd0, 0xca),
            theme_item("meta.attribute", 0x70, 0x6c, 0x66),
        ];

        Theme {
            name: Some("theatron-warm".to_string()),
            settings,
            scopes: items,
            ..Theme::default()
        }
    })
}

fn theme_item(scope: &str, r: u8, g: u8, b: u8) -> ThemeItem {
    let scope_selector = ScopeSelectors::from_str(scope).unwrap_or_default();
    ThemeItem {
        scope: scope_selector,
        style: StyleModifier {
            foreground: Some(SynColor { r, g, b, a: 0xff }),
            background: None,
            font_style: if scope == "keyword" || scope == "storage.type" {
                Some(FontStyle::BOLD)
            } else {
                None
            },
        },
    }
}

/// One styled span within a highlighted line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightedSpan {
    /// Text content of the span.
    pub text: String,
    /// CSS color string in `#rrggbb` format.
    pub color: String,
    /// Bold weighting.
    pub bold: bool,
    /// Italic style.
    pub italic: bool,
}

/// Highlight `code` for `language` and return one `Vec<HighlightedSpan>` per line.
///
/// `language` is matched as a syntect token (e.g. `"rust"`, `"py"`, `"json"`).
/// An empty string or unknown language falls back to plain-text highlighting.
#[must_use]
pub fn highlight_code(code: &str, language: &str) -> Vec<Vec<HighlightedSpan>> {
    let ss = syntax_set();
    let theme = warm_theme();

    let syntax = if language.is_empty() {
        ss.find_syntax_plain_text()
    } else {
        ss.find_syntax_by_token(language)
            .unwrap_or_else(|| ss.find_syntax_plain_text())
    };

    let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);
    let mut result = Vec::new();

    for line in syntect::util::LinesWithEndings::from(code) {
        let ranges = highlighter.highlight_line(line, ss).unwrap_or_default();

        let spans: Vec<HighlightedSpan> = ranges
            .into_iter()
            .map(|(style, text)| HighlightedSpan {
                text: text.to_string(),
                color: syn_color_to_css(style.foreground),
                bold: style.font_style.contains(FontStyle::BOLD),
                italic: style.font_style.contains(FontStyle::ITALIC),
            })
            .collect();

        result.push(spans);
    }

    result
}

fn syn_color_to_css(c: SynColor) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// Extract the language identifier from a fenced code block info string.
///
/// Takes the first whitespace-delimited token (e.g. `"rust"` from
/// `"rust playground"`). Returns an empty string if the input is empty.
#[must_use]
pub fn detect_language(info_string: &str) -> &str {
    info_string.split_whitespace().next().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_language_basic() {
        assert_eq!(detect_language("rust"), "rust");
        assert_eq!(detect_language("python file.py"), "python");
        assert_eq!(detect_language(""), "");
    }

    #[test]
    fn highlight_code_returns_one_vec_per_line() {
        let code = "fn main() {\n    println!(\"hello\");\n}";
        let lines = highlight_code(code, "rust");
        assert_eq!(lines.len(), 3);
        for line in &lines {
            assert!(!line.is_empty());
        }
    }

    #[test]
    fn highlight_code_unknown_language_falls_back_to_plain() {
        let lines = highlight_code("some random text", "nonexistent-lang-xyz");
        assert!(!lines.is_empty());
    }

    #[test]
    fn highlight_code_empty_input_yields_empty_output() {
        let lines = highlight_code("", "rust");
        assert!(lines.is_empty());
    }

    #[test]
    fn syn_color_to_css_format() {
        let c = SynColor {
            r: 0xCC,
            g: 0x77,
            b: 0x55,
            a: 0xff,
        };
        assert_eq!(syn_color_to_css(c), "#cc7755");
    }

    #[test]
    fn keyword_scope_is_bold() {
        let lines = highlight_code("fn main() {}", "rust");
        // 'fn' should hit the keyword scope and be bold.
        let bold_spans: Vec<_> = lines.iter().flatten().filter(|s| s.bold).collect();
        assert!(
            !bold_spans.is_empty(),
            "expected at least one bold span (keyword), got none"
        );
    }
}
