#![expect(clippy::unwrap_used, reason = "test assertions may panic on failure")]

use super::*;

const SAMPLE_DIFF: &str = "\
diff --git a/src/main.rs b/src/main.rs
index abc1234..def5678 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,6 @@ fn main() {
     let x = 1;
-    let y = 2;
+    let y = 3;
+    let z = 4;
     println!(\"{x}\");
 }
";

#[test]
fn parses_hunk_header_with_context() {
    let h = parse_hunk_header("@@ -1,5 +1,6 @@ fn main() {").unwrap();
    assert_eq!(h.old_start, 1);
    assert_eq!(h.old_count, 5);
    assert_eq!(h.new_start, 1);
    assert_eq!(h.new_count, 6);
    assert_eq!(h.context_label, "fn main() {");
}

#[test]
fn parses_hunk_header_no_count() {
    let h = parse_hunk_header("@@ -1 +1 @@").unwrap();
    assert_eq!(h.old_start, 1);
    assert_eq!(h.old_count, 1);
    assert_eq!(h.new_start, 1);
    assert_eq!(h.new_count, 1);
    assert_eq!(h.context_label, "");
}

#[test]
fn parse_unified_diff_basic() {
    let diff = parse_unified_diff("src/main.rs", SAMPLE_DIFF);
    assert_eq!(diff.path, "src/main.rs");
    assert_eq!(diff.additions, 2);
    assert_eq!(diff.deletions, 1);
    assert_eq!(diff.hunks.len(), 1);

    let hunk = &diff.hunks[0];
    assert_eq!(hunk.old_start, 1);
    assert_eq!(hunk.new_start, 1);
    assert_eq!(hunk.lines.len(), 6);
}

#[test]
fn diff_line_numbers_tracked_correctly() {
    let diff = parse_unified_diff("test.rs", SAMPLE_DIFF);
    let lines = &diff.hunks[0].lines;

    // First context line: old=1, new=1
    assert_eq!(lines[0].change_type, ChangeType::Context);
    assert_eq!(lines[0].old_line_no, Some(1));
    assert_eq!(lines[0].new_line_no, Some(1));

    // Removed line: old=2, no new
    assert_eq!(lines[1].change_type, ChangeType::Remove);
    assert_eq!(lines[1].old_line_no, Some(2));
    assert_eq!(lines[1].new_line_no, None);

    // First added line: no old, new=2
    assert_eq!(lines[2].change_type, ChangeType::Add);
    assert_eq!(lines[2].old_line_no, None);
    assert_eq!(lines[2].new_line_no, Some(2));

    // Second added line: no old, new=3
    assert_eq!(lines[3].change_type, ChangeType::Add);
    assert_eq!(lines[3].old_line_no, None);
    assert_eq!(lines[3].new_line_no, Some(3));
}

#[test]
fn word_diff_detects_changed_segments() {
    let old_tokens = tokenize("let y = 2;");
    let new_tokens = tokenize("let y = 3;");
    let (old_spans, new_spans) = diff_tokens(&old_tokens, &new_tokens);

    // The changed segment should be "2" vs "3"
    let old_changed: Vec<_> = old_spans.iter().filter(|s| s.changed).collect();
    let new_changed: Vec<_> = new_spans.iter().filter(|s| s.changed).collect();
    assert!(!old_changed.is_empty(), "old should have changed spans");
    assert!(!new_changed.is_empty(), "new should have changed spans");
    assert!(
        old_changed.iter().any(|s| s.text.contains('2')),
        "old changed should contain '2'"
    );
    assert!(
        new_changed.iter().any(|s| s.text.contains('3')),
        "new changed should contain '3'"
    );
}

#[test]
fn word_diff_all_same_produces_no_changed() {
    let tokens = tokenize("hello world");
    let (old_spans, new_spans) = diff_tokens(&tokens, &tokens);
    assert!(
        old_spans.iter().all(|s| !s.changed),
        "identical lines should have no changed spans"
    );
    assert!(
        new_spans.iter().all(|s| !s.changed),
        "identical lines should have no changed spans"
    );
}

#[test]
fn side_by_side_alignment_with_additions() {
    let lines = vec![
        DiffLine {
            change_type: ChangeType::Context,
            old_line_no: Some(1),
            new_line_no: Some(1),
            content: "context".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Remove,
            old_line_no: Some(2),
            new_line_no: None,
            content: "old".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Add,
            old_line_no: None,
            new_line_no: Some(2),
            content: "new1".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Add,
            old_line_no: None,
            new_line_no: Some(3),
            content: "new2".to_string(),
            word_spans: vec![],
        },
    ];

    let rows = align_side_by_side(&lines);
    assert_eq!(rows.len(), 3, "context + 2 aligned rows");

    // Row 0: context on both sides
    assert!(rows[0].left.is_some());
    assert!(rows[0].right.is_some());

    // Row 1: remove paired with first add
    assert_eq!(rows[1].left.as_ref().unwrap().content, "old");
    assert_eq!(rows[1].right.as_ref().unwrap().content, "new1");

    // Row 2: no left (blank), second add on right
    assert!(rows[2].left.is_none());
    assert_eq!(rows[2].right.as_ref().unwrap().content, "new2");
}

#[test]
fn tokenize_splits_on_punctuation() {
    let tokens = tokenize("fn foo(bar: u32)");
    assert!(tokens.contains(&"fn"));
    assert!(tokens.contains(&"foo"));
    assert!(tokens.contains(&"("));
    assert!(tokens.contains(&"bar"));
    assert!(tokens.contains(&":"));
    assert!(tokens.contains(&"u32"));
    assert!(tokens.contains(&")"));
}

#[test]
fn parse_empty_diff_produces_no_hunks() {
    let diff = parse_unified_diff("empty.rs", "");
    assert!(diff.hunks.is_empty());
    assert_eq!(diff.additions, 0);
    assert_eq!(diff.deletions, 0);
}

#[test]
fn merge_spans_combines_adjacent_same_flag() {
    let input = vec![
        WordSpan {
            text: "a".to_string(),
            changed: true,
        },
        WordSpan {
            text: "b".to_string(),
            changed: true,
        },
        WordSpan {
            text: "c".to_string(),
            changed: false,
        },
    ];
    let mut output = Vec::new();
    merge_spans(&input, &mut output);
    assert_eq!(output.len(), 2);
    assert_eq!(output[0].text, "ab");
    assert!(output[0].changed);
    assert_eq!(output[1].text, "c");
    assert!(!output[1].changed);
}

#[test]
fn parse_unified_diff_produces_two_hunks_when_input_contains_two_headers() {
    let raw = "@@ -1,2 +1,2 @@\n a\n-b\n+c\n@@ -5,2 +5,2 @@\n d\n-e\n+f\n";
    let diff = parse_unified_diff("file.rs", raw);
    assert_eq!(diff.hunks.len(), 2);
    assert_eq!(diff.hunks[0].old_start, 1);
    assert_eq!(diff.hunks[1].old_start, 5);
}

#[test]
fn parse_unified_diff_creates_empty_hunk_when_header_has_no_lines() {
    let raw = "@@ -1,0 +1,0 @@\n@@ -5,1 +5,1 @@\n foo\n";
    let diff = parse_unified_diff("file.rs", raw);
    assert_eq!(diff.hunks.len(), 2);
    assert!(diff.hunks[0].lines.is_empty());
    assert_eq!(diff.hunks[1].lines.len(), 1);
}

#[test]
fn parse_unified_diff_skips_no_newline_marker_inside_hunk() {
    let raw = "@@ -1,2 +1,2 @@\n a\n+b\n\\ No newline at end of file\n";
    let diff = parse_unified_diff("file.rs", raw);
    assert_eq!(diff.additions, 1);
    assert_eq!(diff.deletions, 0);
    let lines = &diff.hunks[0].lines;
    assert_eq!(lines.len(), 2);
    assert!(!lines.iter().any(|l| l.content.contains("No newline")));
}

#[test]
fn parse_unified_diff_treats_binary_stub_as_context_fallback() {
    let raw = "@@ -1,1 +1,1 @@\n a\nBinary files a/x and b/x differ\n";
    let diff = parse_unified_diff("file.rs", raw);
    let lines = &diff.hunks[0].lines;
    assert!(
        lines.iter().any(|l| {
            l.change_type == ChangeType::Context && l.content.contains("Binary files")
        })
    );
}

#[test]
fn parse_unified_diff_handles_context_lines_omitting_leading_space() {
    let raw = "@@ -1,2 +1,2 @@\n a\ncontext_without_space\n";
    let diff = parse_unified_diff("file.rs", raw);
    let lines = &diff.hunks[0].lines;
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1].change_type, ChangeType::Context);
    assert_eq!(lines[1].content, "context_without_space");
}

#[test]
fn parse_unified_diff_returns_empty_diff_file_for_gibberish_input() {
    let diff = parse_unified_diff("file.rs", "this is not a diff\nhello world");
    assert!(diff.hunks.is_empty());
    assert_eq!(diff.additions, 0);
    assert_eq!(diff.deletions, 0);
}

#[test]
fn parse_unified_diff_skips_file_headers_before_first_hunk() {
    let raw = "--- a/file.rs\n+++ b/file.rs\n@@ -1,2 +1,2 @@\n a\n-b\n+c\n";
    let diff = parse_unified_diff("file.rs", raw);
    assert_eq!(diff.hunks.len(), 1);
    assert_eq!(diff.additions, 1);
    assert_eq!(diff.deletions, 1);
    assert_eq!(diff.hunks[0].lines.len(), 3);
}

#[test]
fn parse_unified_diff_keeps_removed_line_starting_with_double_dash() {
    // WHY: A removed line whose content starts with `--` (SQL/Haskell
    // comment) arrives as `---content`; the header-skip must not fire
    // inside a hunk body (#79).
    let raw = "--- a/q.sql\n+++ b/q.sql\n@@ -1,2 +1,1 @@\n SELECT 1;\n--- count rows\n";
    let diff = parse_unified_diff("q.sql", raw);
    assert_eq!(diff.deletions, 1);
    let lines = &diff.hunks[0].lines;
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1].change_type, ChangeType::Remove);
    assert_eq!(lines[1].content, "-- count rows");
}

#[test]
fn parse_unified_diff_keeps_added_line_starting_with_double_plus() {
    // WHY: An added line whose content starts with `++` (Haskell
    // concatenation) arrives as `+++content`; the header-skip must not
    // fire inside a hunk body (#79).
    let raw = "--- a/l.hs\n+++ b/l.hs\n@@ -1,1 +1,2 @@\n xs\n+++ ys ++ zs\n";
    let diff = parse_unified_diff("l.hs", raw);
    assert_eq!(diff.additions, 1);
    let lines = &diff.hunks[0].lines;
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1].change_type, ChangeType::Add);
    assert_eq!(lines[1].content, "++ ys ++ zs");
}

#[test]
fn parse_unified_diff_keeps_removed_markdown_separator_line() {
    let raw = "@@ -1,2 +1,1 @@\n title\n----\n";
    let diff = parse_unified_diff("doc.md", raw);
    assert_eq!(diff.deletions, 1);
    let lines = &diff.hunks[0].lines;
    assert_eq!(lines[1].change_type, ChangeType::Remove);
    assert_eq!(lines[1].content, "---");
}

#[test]
fn hunk_line_numbers_saturate_at_u32_max_without_panicking() {
    // WHY: Line-number counters must saturate, not overflow, on
    // adversarial hunk headers near u32::MAX (#58).
    let raw = "@@ -4294967294,3 +4294967294,3 @@\n a\n b\n c\n";
    let diff = parse_unified_diff("huge.rs", raw);
    let lines = &diff.hunks[0].lines;
    assert_eq!(lines[0].old_line_no, Some(u32::MAX - 1));
    assert_eq!(lines[1].old_line_no, Some(u32::MAX));
    assert_eq!(lines[2].old_line_no, Some(u32::MAX));
    assert_eq!(lines[2].new_line_no, Some(u32::MAX));
}

#[test]
fn parse_unified_diff_rolls_multi_file_headers_into_single_diff_file() {
    let raw = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1,1 +1,1 @@\n a\n-b\n+c\ndiff --git a/b.rs b/b.rs\n--- a/b.rs\n+++ b/b.rs\n@@ -1,1 +1,1 @@\n d\n-e\n+f\n";
    let diff = parse_unified_diff("a.rs", raw);
    assert_eq!(diff.hunks.len(), 2);
    assert_eq!(diff.path, "a.rs");
}

#[test]
fn align_side_by_side_returns_empty_vec_when_given_empty_slice() {
    let rows = align_side_by_side(&[]);
    assert!(rows.is_empty());
}

#[test]
fn align_side_by_side_clones_single_context_line_to_both_sides() {
    let lines = vec![DiffLine {
        change_type: ChangeType::Context,
        old_line_no: Some(1),
        new_line_no: Some(1),
        content: "ctx".to_string(),
        word_spans: vec![],
    }];
    let rows = align_side_by_side(&lines);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].left.is_some());
    assert!(rows[0].right.is_some());
    assert_eq!(rows[0].left.as_ref().unwrap().content, "ctx");
    assert_eq!(rows[0].right.as_ref().unwrap().content, "ctx");
}

#[test]
fn align_side_by_side_emits_orphan_remove_when_no_adds_follow() {
    let lines = vec![DiffLine {
        change_type: ChangeType::Remove,
        old_line_no: Some(1),
        new_line_no: None,
        content: "old".to_string(),
        word_spans: vec![],
    }];
    let rows = align_side_by_side(&lines);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].left.is_some());
    assert!(rows[0].right.is_none());
}

#[test]
fn align_side_by_side_emits_orphan_add_when_no_removes_precede() {
    let lines = vec![DiffLine {
        change_type: ChangeType::Add,
        old_line_no: None,
        new_line_no: Some(1),
        content: "new".to_string(),
        word_spans: vec![],
    }];
    let rows = align_side_by_side(&lines);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].left.is_none());
    assert!(rows[0].right.is_some());
}

#[test]
fn align_side_by_side_pairs_equal_count_removes_and_adds() {
    let lines = vec![
        DiffLine {
            change_type: ChangeType::Remove,
            old_line_no: Some(1),
            new_line_no: None,
            content: "a".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Remove,
            old_line_no: Some(2),
            new_line_no: None,
            content: "b".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Add,
            old_line_no: None,
            new_line_no: Some(1),
            content: "c".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Add,
            old_line_no: None,
            new_line_no: Some(2),
            content: "d".to_string(),
            word_spans: vec![],
        },
    ];
    let rows = align_side_by_side(&lines);
    assert_eq!(rows.len(), 2);
    assert!(rows[0].left.is_some() && rows[0].right.is_some());
    assert!(rows[1].left.is_some() && rows[1].right.is_some());
}

#[test]
fn align_side_by_side_balances_more_removes_than_adds() {
    let lines = vec![
        DiffLine {
            change_type: ChangeType::Remove,
            old_line_no: Some(1),
            new_line_no: None,
            content: "a".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Remove,
            old_line_no: Some(2),
            new_line_no: None,
            content: "b".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Remove,
            old_line_no: Some(3),
            new_line_no: None,
            content: "c".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Add,
            old_line_no: None,
            new_line_no: Some(1),
            content: "d".to_string(),
            word_spans: vec![],
        },
    ];
    let rows = align_side_by_side(&lines);
    assert_eq!(rows.len(), 3);
    assert!(rows[1].right.is_none());
    assert!(rows[2].right.is_none());
}

#[test]
fn align_side_by_side_balances_more_adds_than_removes() {
    let lines = vec![
        DiffLine {
            change_type: ChangeType::Remove,
            old_line_no: Some(1),
            new_line_no: None,
            content: "a".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Add,
            old_line_no: None,
            new_line_no: Some(1),
            content: "b".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Add,
            old_line_no: None,
            new_line_no: Some(2),
            content: "c".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Add,
            old_line_no: None,
            new_line_no: Some(3),
            content: "d".to_string(),
            word_spans: vec![],
        },
    ];
    let rows = align_side_by_side(&lines);
    assert_eq!(rows.len(), 3);
    assert!(rows[1].left.is_none());
    assert!(rows[2].left.is_none());
}

#[test]
fn align_side_by_side_handles_alternating_remove_add_pattern() {
    let lines = vec![
        DiffLine {
            change_type: ChangeType::Remove,
            old_line_no: Some(1),
            new_line_no: None,
            content: "r1".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Add,
            old_line_no: None,
            new_line_no: Some(1),
            content: "a1".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Remove,
            old_line_no: Some(2),
            new_line_no: None,
            content: "r2".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Add,
            old_line_no: None,
            new_line_no: Some(2),
            content: "a2".to_string(),
            word_spans: vec![],
        },
    ];
    let rows = align_side_by_side(&lines);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].left.as_ref().unwrap().content, "r1");
    assert_eq!(rows[0].right.as_ref().unwrap().content, "a1");
    assert_eq!(rows[1].left.as_ref().unwrap().content, "r2");
    assert_eq!(rows[1].right.as_ref().unwrap().content, "a2");
}

#[test]
fn align_side_by_side_preserves_lines_with_empty_content() {
    let lines = vec![
        DiffLine {
            change_type: ChangeType::Context,
            old_line_no: Some(1),
            new_line_no: Some(1),
            content: String::new(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Remove,
            old_line_no: Some(2),
            new_line_no: None,
            content: String::new(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Add,
            old_line_no: None,
            new_line_no: Some(2),
            content: String::new(),
            word_spans: vec![],
        },
    ];
    let rows = align_side_by_side(&lines);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].left.as_ref().unwrap().content, "");
    assert_eq!(rows[1].left.as_ref().unwrap().content, "");
    assert_eq!(rows[1].right.as_ref().unwrap().content, "");
}

#[test]
fn diff_view_mode_toggles_from_unified_to_side_by_side() {
    assert_eq!(DiffViewMode::Unified.toggle(), DiffViewMode::SideBySide);
}

#[test]
fn diff_view_mode_toggles_from_side_by_side_to_unified() {
    assert_eq!(DiffViewMode::SideBySide.toggle(), DiffViewMode::Unified);
}

#[test]
fn diff_view_mode_displays_unified_as_expected() {
    assert_eq!(DiffViewMode::Unified.to_string(), "Unified");
}

#[test]
fn diff_view_mode_displays_side_by_side_as_expected() {
    assert_eq!(DiffViewMode::SideBySide.to_string(), "Side-by-Side");
}

#[test]
fn parse_hunk_header_returns_none_for_missing_at_suffix() {
    assert!(parse_hunk_header("@@ -1 +1").is_none());
}

#[test]
fn parse_hunk_header_returns_none_for_random_line() {
    assert!(parse_hunk_header("hello world").is_none());
}

#[test]
fn parse_range_returns_none_for_non_numeric_start() {
    // WHY: A malformed start must reject the range rather than
    // silently defaulting to line 1 and corrupting line-number
    // attribution (#181).
    assert_eq!(parse_range("abc"), None);
}

#[test]
fn parse_range_returns_none_for_non_numeric_pair() {
    assert_eq!(parse_range("abc,def"), None);
}

#[test]
fn parse_range_returns_none_when_only_count_is_non_numeric() {
    assert_eq!(parse_range("5,def"), None);
}

#[test]
fn parse_range_parses_valid_start_and_count() {
    assert_eq!(parse_range("5,3"), Some((5, 3)));
}

#[test]
fn parse_range_defaults_count_to_one_when_absent() {
    assert_eq!(parse_range("7"), Some((7, 1)));
}

#[test]
fn parse_unified_diff_rejects_hunk_with_non_numeric_start() {
    // WHY: A malformed `@@` header must reject the hunk instead of
    // silently attributing every line to line 1 (#181).
    let raw = "@@ -abc,3 +1,3 @@\n context\n-old\n+new\n";
    let diff = parse_unified_diff("file.rs", raw);
    assert!(
        diff.hunks.is_empty(),
        "malformed hunk header must not produce a hunk"
    );
}

#[test]
fn parse_hunk_header_returns_none_for_non_numeric_old_start() {
    assert!(parse_hunk_header("@@ -abc,3 +1,3 @@").is_none());
}

#[test]
fn diff_line_new_maps_fields_directly() {
    let spans = vec![WordSpan::new("content", true)];
    let line = DiffLine::new(ChangeType::Add, None, Some(5), "content", spans);
    assert_eq!(line.change_type, ChangeType::Add);
    assert_eq!(line.old_line_no, None);
    assert_eq!(line.new_line_no, Some(5));
    assert_eq!(line.content, "content");
    assert_eq!(line.word_spans.len(), 1);
    assert!(line.word_spans[0].changed);
}

#[test]
fn diff_hunk_new_maps_fields_directly() {
    let lines = vec![DiffLine {
        change_type: ChangeType::Context,
        old_line_no: Some(1),
        new_line_no: Some(1),
        content: "ctx".to_string(),
        word_spans: vec![],
    }];
    let hunk = DiffHunk::new(1, 2, 3, 4, "fn main()", lines.clone());
    assert_eq!(hunk.old_start, 1);
    assert_eq!(hunk.old_count, 2);
    assert_eq!(hunk.new_start, 3);
    assert_eq!(hunk.new_count, 4);
    assert_eq!(hunk.context_label, "fn main()");
    assert_eq!(hunk.lines, lines);
}

#[test]
fn compute_word_diffs_populates_spans_when_token_count_is_within_limit() {
    let mut lines = vec![
        DiffLine {
            change_type: ChangeType::Remove,
            old_line_no: Some(1),
            new_line_no: None,
            content: "let y = 2;".to_string(),
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Add,
            old_line_no: None,
            new_line_no: Some(1),
            content: "let y = 3;".to_string(),
            word_spans: vec![],
        },
    ];
    compute_word_diffs(&mut lines);
    assert!(!lines[0].word_spans.is_empty());
    assert!(!lines[1].word_spans.is_empty());
}

#[test]
fn compute_word_diffs_skips_word_level_diff_when_token_count_exceeds_limit() {
    // WORD_DIFF_TOKEN_LIMIT = 500; combined old+new token count here
    // exceeds it, so the guard must fall back to whole-line (no
    // word_spans) rather than paying for an O(n*m) LCS table on an
    // adversarially long line pair (#181).
    let long_old = "word ".repeat(300);
    let long_new = "term ".repeat(300);
    let mut lines = vec![
        DiffLine {
            change_type: ChangeType::Remove,
            old_line_no: Some(1),
            new_line_no: None,
            content: long_old,
            word_spans: vec![],
        },
        DiffLine {
            change_type: ChangeType::Add,
            old_line_no: None,
            new_line_no: Some(1),
            content: long_new,
            word_spans: vec![],
        },
    ];
    compute_word_diffs(&mut lines);
    assert!(
        lines[0].word_spans.is_empty(),
        "over-limit pair must fall back to no word spans"
    );
    assert!(
        lines[1].word_spans.is_empty(),
        "over-limit pair must fall back to no word spans"
    );
}

#[test]
fn merge_spans_passes_through_alternating_flags_without_merging() {
    let input = vec![
        WordSpan {
            text: "a".to_string(),
            changed: true,
        },
        WordSpan {
            text: "b".to_string(),
            changed: false,
        },
        WordSpan {
            text: "c".to_string(),
            changed: true,
        },
    ];
    let mut output = Vec::new();
    merge_spans(&input, &mut output);
    assert_eq!(output.len(), 3);
    assert!(output[0].changed);
    assert!(!output[1].changed);
    assert!(output[2].changed);
}

#[test]
fn tokenize_returns_empty_for_empty_string() {
    assert!(tokenize("").is_empty());
}

#[test]
fn tokenize_treats_whitespace_as_boundary() {
    let tokens = tokenize("a b");
    assert_eq!(tokens, vec!["a", " ", "b"]);
}

#[path = "diff_tests_predicates.rs"]
mod predicates;
