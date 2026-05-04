//! Diff state: unified diff parsing and structured representation.

use std::fmt;

/// Maximum tokens per line pair before falling back to whole-line highlighting.
const WORD_DIFF_TOKEN_LIMIT: usize = 500;

/// View mode for the diff viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum DiffViewMode {
    /// Single-column unified diff (default).
    #[default]
    Unified,
    /// Two-column side-by-side display with old on left, new on right.
    SideBySide,
}

impl DiffViewMode {
    /// Toggle between unified and side-by-side.
    #[must_use]
    pub fn toggle(self) -> Self {
        match self {
            Self::Unified => Self::SideBySide,
            Self::SideBySide => Self::Unified,
        }
    }

    /// Whether this is the unified single-column view (`Unified`).
    ///
    /// Convenience predicate matching the pattern from
    /// [`ChangeType::is_add`] and `themelion::ResolvedTheme::is_dark`.
    #[must_use]
    pub const fn is_unified(self) -> bool {
        matches!(self, Self::Unified)
    }

    /// Whether this is the two-column side-by-side view (`SideBySide`).
    #[must_use]
    pub const fn is_side_by_side(self) -> bool {
        matches!(self, Self::SideBySide)
    }
}

impl fmt::Display for DiffViewMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unified => f.write_str("Unified"),
            Self::SideBySide => f.write_str("Side-by-Side"),
        }
    }
}

/// Type of change for a single diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChangeType {
    /// Unchanged context line — present in both old and new.
    Context,
    /// Inserted line — present only in new.
    Add,
    /// Deleted line — present only in old.
    Remove,
}

impl ChangeType {
    /// Whether this line is an addition (`Add`).
    ///
    /// Convenience predicate: `change.is_add()` reads better than
    /// `change == ChangeType::Add` at consumer call sites.
    #[must_use]
    pub const fn is_add(self) -> bool {
        matches!(self, Self::Add)
    }

    /// Whether this line is a deletion (`Remove`).
    #[must_use]
    pub const fn is_remove(self) -> bool {
        matches!(self, Self::Remove)
    }

    /// Whether this line is unchanged context (`Context`).
    #[must_use]
    pub const fn is_context(self) -> bool {
        matches!(self, Self::Context)
    }

    /// Whether this line carries a change (added or removed).
    ///
    /// Useful for filtering out context lines when computing
    /// stats or rendering only the modified rows.
    #[must_use]
    pub const fn is_change(self) -> bool {
        !self.is_context()
    }
}

/// A single line in a diff hunk.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffLine {
    /// Whether this line was added, removed, or unchanged.
    pub change_type: ChangeType,
    /// Line number in the old file, or None if newly added.
    pub old_line_no: Option<u32>,
    /// Line number in the new file, or None if removed.
    pub new_line_no: Option<u32>,
    /// Line content with the leading +/-/space stripped.
    pub content: String,
    /// Word-level change spans within the line content.
    /// Empty if this is a context line or word diff was skipped.
    pub word_spans: Vec<WordSpan>,
}

/// A span within a diff line, marking whether it changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordSpan {
    /// Token text (word or punctuation).
    pub text: String,
    /// True if this token differs between old and new lines.
    pub changed: bool,
}

/// A single hunk in a diff file.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffHunk {
    /// Starting line number in the old file.
    pub old_start: u32,
    /// Number of lines from the old file in this hunk.
    pub old_count: u32,
    /// Starting line number in the new file.
    pub new_start: u32,
    /// Number of lines from the new file in this hunk.
    pub new_count: u32,
    /// Optional context label trailing the `@@` marker (e.g. function name).
    pub context_label: String,
    /// Lines belonging to this hunk, in display order.
    pub lines: Vec<DiffLine>,
}

/// Parsed diff for a single file.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffFile {
    /// File path the diff applies to.
    pub path: String,
    /// Hunks within this file, in source order.
    pub hunks: Vec<DiffHunk>,
    /// Total inserted lines across all hunks.
    pub additions: u32,
    /// Total deleted lines across all hunks.
    pub deletions: u32,
    /// Default view mode for rendering this diff.
    pub mode: DiffViewMode,
}

/// Aligned row for side-by-side display.
#[derive(Debug, Clone, PartialEq)]
pub struct SideBySideRow {
    /// Old-side line, or None when the row is a pure addition.
    pub left: Option<DiffLine>,
    /// New-side line, or None when the row is a pure deletion.
    pub right: Option<DiffLine>,
}

/// Aggregate stats summed across one or more [`DiffFile`]s.
///
/// Common use: a PR list view rendering "N files changed, +X / -Y"
/// without iterating the file list at every render.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DiffStats {
    /// Number of files in the aggregate.
    pub files_changed: usize,
    /// Total inserted lines across all files.
    pub additions: u32,
    /// Total deleted lines across all files.
    pub deletions: u32,
}

impl DiffStats {
    /// Compute aggregate stats over a slice of `DiffFile`s.
    ///
    /// `additions` and `deletions` saturate at `u32::MAX` if the
    /// total exceeds 4 billion lines (impossible for a real PR;
    /// the saturating add prevents overflow panic on adversarial
    /// input).
    #[must_use]
    pub fn from_files(files: &[DiffFile]) -> Self {
        let mut additions: u32 = 0;
        let mut deletions: u32 = 0;
        for file in files {
            additions = additions.saturating_add(file.additions);
            deletions = deletions.saturating_add(file.deletions);
        }
        Self {
            files_changed: files.len(),
            additions,
            deletions,
        }
    }

    /// Total lines changed (additions plus deletions). Saturating;
    /// see [`from_files`](Self::from_files).
    #[must_use]
    pub const fn total_lines_changed(self) -> u32 {
        self.additions.saturating_add(self.deletions)
    }

    /// Whether the aggregate is empty (no files, no line changes).
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.files_changed == 0 && self.additions == 0 && self.deletions == 0
    }
}

/// Parse a unified diff string into a `DiffFile`.
#[must_use]
pub fn parse_unified_diff(path: &str, raw: &str) -> DiffFile {
    let mut hunks = Vec::new();
    let mut additions: u32 = 0;
    let mut deletions: u32 = 0;

    let mut current_hunk: Option<HunkBuilder> = None;

    for line in raw.lines() {
        if let Some(hunk_header) = parse_hunk_header(line) {
            if let Some(builder) = current_hunk.take() {
                hunks.push(builder.build());
            }
            current_hunk = Some(HunkBuilder::new(hunk_header));
            continue;
        }

        // NOTE: Skip file-level headers (---, +++, diff, index).
        if line.starts_with("---")
            || line.starts_with("+++")
            || line.starts_with("diff ")
            || line.starts_with("index ")
        {
            continue;
        }

        if let Some(ref mut builder) = current_hunk {
            if let Some(stripped) = line.strip_prefix('+') {
                additions += 1;
                builder.add_line(ChangeType::Add, stripped);
            } else if let Some(stripped) = line.strip_prefix('-') {
                deletions += 1;
                builder.add_line(ChangeType::Remove, stripped);
            } else if let Some(stripped) = line.strip_prefix(' ') {
                builder.add_line(ChangeType::Context, stripped);
            } else if line == "\\ No newline at end of file" {
                // NOTE: Git marker, not actual content.
            } else {
                // WHY: Some diffs omit the leading space for context lines.
                builder.add_line(ChangeType::Context, line);
            }
        }
    }

    if let Some(builder) = current_hunk {
        hunks.push(builder.build());
    }

    // NOTE: Compute word-level diffs for adjacent remove+add pairs.
    for hunk in &mut hunks {
        compute_word_diffs(&mut hunk.lines);
    }

    DiffFile {
        path: path.to_string(),
        hunks,
        additions,
        deletions,
        mode: DiffViewMode::default(),
    }
}

/// Align hunk lines for side-by-side display.
#[must_use]
pub fn align_side_by_side(lines: &[DiffLine]) -> Vec<SideBySideRow> {
    let mut rows = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let Some(line) = lines.get(i) else { break };
        match line.change_type {
            ChangeType::Context => {
                rows.push(SideBySideRow {
                    left: Some(line.clone()),
                    right: Some(line.clone()),
                });
                i += 1;
            }
            ChangeType::Remove => {
                // WHY: Collect consecutive removes, then pair with consecutive adds.
                let remove_start = i;
                while lines
                    .get(i)
                    .is_some_and(|l| l.change_type == ChangeType::Remove)
                {
                    i += 1;
                }
                let removes = lines.get(remove_start..i).unwrap_or_default();

                let add_start = i;
                while lines
                    .get(i)
                    .is_some_and(|l| l.change_type == ChangeType::Add)
                {
                    i += 1;
                }
                let adds = lines.get(add_start..i).unwrap_or_default();

                let max_len = removes.len().max(adds.len());
                for j in 0..max_len {
                    rows.push(SideBySideRow {
                        left: removes.get(j).cloned(),
                        right: adds.get(j).cloned(),
                    });
                }
            }
            ChangeType::Add => {
                rows.push(SideBySideRow {
                    left: None,
                    right: Some(line.clone()),
                });
                i += 1;
            }
        }
    }

    rows
}

// -- Hunk header parsing ------------------------------------------------------

struct HunkHeader {
    old_start: u32,
    old_count: u32,
    new_start: u32,
    new_count: u32,
    context_label: String,
}

/// Parse `@@ -old,count +new,count @@ context`.
fn parse_hunk_header(line: &str) -> Option<HunkHeader> {
    let line = line.strip_prefix("@@ ")?;
    let rest = line.strip_prefix('-')?;
    let at_idx = rest.find(" +")?;
    let old_part = rest.get(..at_idx).unwrap_or("");
    let rest = rest.get(at_idx + 2..).unwrap_or("");

    let end_idx = rest.find(" @@")?;
    let new_part = rest.get(..end_idx).unwrap_or("");
    let context_label = rest.get(end_idx + 3..).unwrap_or("").trim().to_string();

    let (old_start, old_count) = parse_range(old_part);
    let (new_start, new_count) = parse_range(new_part);

    Some(HunkHeader {
        old_start,
        old_count,
        new_start,
        new_count,
        context_label,
    })
}

/// Parse `start,count` or `start` (count defaults to 1).
fn parse_range(s: &str) -> (u32, u32) {
    if let Some((start, count)) = s.split_once(',') {
        (start.parse().unwrap_or(1), count.parse().unwrap_or(1))
    } else {
        (s.parse().unwrap_or(1), 1)
    }
}

// -- Hunk builder -------------------------------------------------------------

struct HunkBuilder {
    old_start: u32,
    old_count: u32,
    new_start: u32,
    new_count: u32,
    context_label: String,
    lines: Vec<DiffLine>,
    old_line: u32,
    new_line: u32,
}

impl HunkBuilder {
    fn new(header: HunkHeader) -> Self {
        Self {
            old_start: header.old_start,
            old_count: header.old_count,
            new_start: header.new_start,
            new_count: header.new_count,
            context_label: header.context_label,
            lines: Vec::new(),
            old_line: header.old_start,
            new_line: header.new_start,
        }
    }

    fn add_line(&mut self, change_type: ChangeType, content: &str) {
        let (old_line_no, new_line_no) = match change_type {
            ChangeType::Context => {
                let old = self.old_line;
                let new = self.new_line;
                self.old_line += 1;
                self.new_line += 1;
                (Some(old), Some(new))
            }
            ChangeType::Add => {
                let new = self.new_line;
                self.new_line += 1;
                (None, Some(new))
            }
            ChangeType::Remove => {
                let old = self.old_line;
                self.old_line += 1;
                (Some(old), None)
            }
        };

        self.lines.push(DiffLine {
            change_type,
            old_line_no,
            new_line_no,
            content: content.to_string(),
            word_spans: Vec::new(),
        });
    }

    fn build(self) -> DiffHunk {
        DiffHunk {
            old_start: self.old_start,
            old_count: self.old_count,
            new_start: self.new_start,
            new_count: self.new_count,
            context_label: self.context_label,
            lines: self.lines,
        }
    }
}

// -- Word-level diff ----------------------------------------------------------

/// Compute word-level diffs for adjacent remove+add line pairs within a hunk.
#[expect(
    clippy::indexing_slicing,
    reason = "indices are derived from length-checked loop counters and pair counts"
)]
fn compute_word_diffs(lines: &mut [DiffLine]) {
    let mut i = 0;
    while i < lines.len() {
        if lines[i].change_type == ChangeType::Remove {
            // NOTE: Find the run of removes followed by adds.
            let remove_start = i;
            while i < lines.len() && lines[i].change_type == ChangeType::Remove {
                i += 1;
            }
            let add_start = i;
            while i < lines.len() && lines[i].change_type == ChangeType::Add {
                i += 1;
            }
            let add_end = i;

            let remove_count = add_start - remove_start;
            let add_count = add_end - add_start;
            let pairs = remove_count.min(add_count);

            for p in 0..pairs {
                let ri = remove_start + p;
                let ai = add_start + p;
                let old_content = lines[ri].content.clone(); // kanon:ignore RUST/indexing-slicing -- ri bounded by remove_count via min(remove_count, add_count)
                let new_content = lines[ai].content.clone(); // kanon:ignore RUST/indexing-slicing -- ai bounded by add_count via min(remove_count, add_count)

                let old_tokens = tokenize(&old_content);
                let new_tokens = tokenize(&new_content);

                if old_tokens.len() + new_tokens.len() > WORD_DIFF_TOKEN_LIMIT {
                    continue;
                }

                let (old_spans, new_spans) = diff_tokens(&old_tokens, &new_tokens);
                lines[ri].word_spans = old_spans; // kanon:ignore RUST/indexing-slicing -- ri bounded by remove_count
                lines[ai].word_spans = new_spans; // kanon:ignore RUST/indexing-slicing -- ai bounded by add_count
            }
        } else {
            i += 1;
        }
    }
}

/// Split text into tokens on whitespace and punctuation boundaries.
fn tokenize(s: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = 0;
    let bytes = s.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        let is_boundary = b == b' ' || b == b'\t' || b.is_ascii_punctuation();
        if is_boundary {
            if start < i {
                tokens.push(&s[start..i]);
            }
            tokens.push(&s[i..=i]);
            start = i + 1;
        }
    }
    if start < s.len() {
        tokens.push(&s[start..]);
    }
    tokens
}

/// Compute LCS-based word diff, returning spans for old and new lines.
fn diff_tokens(old: &[&str], new: &[&str]) -> (Vec<WordSpan>, Vec<WordSpan>) {
    let lcs = lcs_table(old, new);
    let mut old_spans = Vec::new();
    let mut new_spans = Vec::new();

    let mut i = old.len();
    let mut j = new.len();

    // NOTE: Backtrace from LCS table to build diff spans.
    let mut old_rev = Vec::new();
    let mut new_rev = Vec::new();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old[i - 1] == new[j - 1] {
            old_rev.push(WordSpan {
                text: old[i - 1].to_string(),
                changed: false,
            });
            new_rev.push(WordSpan {
                text: new[j - 1].to_string(),
                changed: false,
            });
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || lcs_val(&lcs, i, j - 1) >= lcs_val(&lcs, i - 1, j)) {
            new_rev.push(WordSpan {
                text: new[j - 1].to_string(),
                changed: true,
            });
            j -= 1;
        } else if i > 0 {
            old_rev.push(WordSpan {
                text: old[i - 1].to_string(),
                changed: true,
            });
            i -= 1;
        }
    }

    old_rev.reverse();
    new_rev.reverse();

    // NOTE: Merge adjacent spans with the same `changed` flag for cleaner output.
    merge_spans(&old_rev, &mut old_spans);
    merge_spans(&new_rev, &mut new_spans);

    (old_spans, new_spans)
}

/// Build LCS length table (m+1 x n+1).
#[expect(
    clippy::indexing_slicing,
    reason = "indices are bounded by loop ranges 1..=m and 1..=n against allocated table size"
)]
fn lcs_table(old: &[&str], new: &[&str]) -> Vec<Vec<u32>> {
    let m = old.len();
    let n = new.len();
    let mut table = vec![vec![0u32; n + 1]; m + 1];

    for i in 1..=m {
        for j in 1..=n {
            if old[i - 1] == new[j - 1] {
                table[i][j] = table[i - 1][j - 1] + 1;
            } else {
                table[i][j] = table[i - 1][j].max(table[i][j - 1]);
            }
        }
    }

    table
}

#[expect(
    clippy::indexing_slicing,
    reason = "caller guarantees i,j are within table dimensions"
)]
fn lcs_val(table: &[Vec<u32>], i: usize, j: usize) -> u32 {
    table[i][j]
}

/// Merge adjacent spans with the same `changed` flag.
fn merge_spans(input: &[WordSpan], output: &mut Vec<WordSpan>) {
    for span in input {
        if let Some(last) = output.last_mut() {
            if last.changed == span.changed {
                last.text.push_str(&span.text);
                continue;
            }
        }
        output.push(span.clone());
    }
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;
