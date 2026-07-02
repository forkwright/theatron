//! Simple subsequence fuzzy matcher for command palette and slash completion.
//!
//! This is the first beat of the koilon → parodos extraction wave (kanon
//! Task #82). It is a verbatim move of `aletheia/koilon/src/fuzzy.rs`
//! into the new generic `parodos` substrate, with visibility widened
//! from `pub(crate)` to `pub` so external consumers (aletheia, future
//! theatron-resident TUI apps) can re-export it. No behavioural changes.
//!
//! Scores are calculated based on:
//! - Consecutive matches (bonus)
//! - Word boundary matches (bonus)
//! - Start of string match (bonus)
//! - Shorter candidates with same match quality score higher

/// Result of a fuzzy match, containing the score and match positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    /// Match score (higher is better).
    pub score: i64,
    /// Indices of matched characters in the candidate string.
    pub indices: Vec<usize>,
}

/// Match a pattern against a candidate string.
///
/// Returns `Some(MatchResult)` if the pattern is a subsequence of the candidate,
/// `None` otherwise. The match is case-insensitive.
#[must_use]
pub fn fuzzy_match(candidate: &str, pattern: &str) -> Option<MatchResult> {
    if pattern.is_empty() {
        return Some(MatchResult {
            score: 0,
            indices: Vec::new(),
        });
    }

    let candidate_lower = candidate.to_lowercase();
    let pattern_lower = pattern.to_lowercase();

    let mut indices = Vec::new();
    let mut pattern_chars = pattern_lower.chars().peekable();
    let mut current_pattern_char = pattern_chars.next()?;

    for (idx, candidate_char) in candidate_lower.char_indices() {
        if candidate_char == current_pattern_char {
            indices.push(idx);
            match pattern_chars.next() {
                Some(c) => current_pattern_char = c,
                None => break,
            }
        }
    }

    // Pattern not fully matched
    if indices.len() != pattern.len() {
        return None;
    }

    let score = calculate_score(candidate, &indices);

    Some(MatchResult { score, indices })
}

/// Calculate a score for a match based on heuristics.
fn calculate_score(candidate: &str, indices: &[usize]) -> i64 {
    let mut score: i64 = 100; // Base score
    let chars_by_byte: Vec<(usize, char)> = candidate.char_indices().collect();

    // Bonus for matching at the start
    if let Some(&first) = indices.first()
        && first == 0
    {
        score += 50;
    }

    // Bonus for consecutive matches. `windows(2)` always yields 2-element
    // slices; destructuring rather than indexing keeps the lint clean.
    for window in indices.windows(2) {
        let &[prev, curr] = window else { continue };
        let prev_char = char_at_byte_offset(&chars_by_byte, prev);
        let curr_char = char_at_byte_offset(&chars_by_byte, curr);

        if let (Some(p), Some(c)) = (prev_char, curr_char) {
            // Consecutive character bonus
            if curr == prev + p.len_utf8() {
                score += 30;
            }

            // Word boundary bonus (after space, hyphen, underscore, etc.)
            if is_word_boundary(p) && !is_word_boundary(c) {
                score += 25;
            }
        }
    }

    // Penalty for length (shorter is better). Saturate at i64::MAX for
    // the impossible >2^63 string case; the lint-clean path matters more
    // than the unreachable branch.
    let candidate_len = i64::try_from(chars_by_byte.len()).unwrap_or(i64::MAX);
    score -= candidate_len * 2;

    // Bonus for matching more of the pattern.
    let match_count = i64::try_from(indices.len()).unwrap_or(i64::MAX);
    score += match_count * 10;

    score
}

fn char_at_byte_offset(chars_by_byte: &[(usize, char)], byte_offset: usize) -> Option<char> {
    chars_by_byte
        .binary_search_by_key(&byte_offset, |entry| entry.0)
        .ok()
        .and_then(|position| chars_by_byte.get(position))
        .map(|(_, ch)| *ch)
}

/// Check if a character is a word boundary.
fn is_word_boundary(c: char) -> bool {
    c.is_whitespace() || c == '-' || c == '_' || c == '.' || c == '/' || c == ':'
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "test assertions may panic on failure")]
mod tests {
    use super::*;

    #[test]
    fn empty_pattern_matches_everything() {
        let result = fuzzy_match("hello", "").unwrap();
        assert_eq!(result.score, 0);
        assert!(result.indices.is_empty());
    }

    #[test]
    fn exact_match_scores_high() {
        let result = fuzzy_match("quit", "quit").unwrap();
        assert!(result.score > 100);
    }

    #[test]
    fn partial_match_works() {
        let result = fuzzy_match("sessions", "sess").unwrap();
        assert_eq!(result.indices, vec![0, 1, 2, 3]);
        assert!(result.score > 0);
    }

    #[test]
    fn case_insensitive_match() {
        let result = fuzzy_match("Sessions", "sess").unwrap();
        assert_eq!(result.indices, vec![0, 1, 2, 3]);
    }

    #[test]
    fn non_match_returns_none() {
        assert!(fuzzy_match("quit", "xyz").is_none());
    }

    #[test]
    fn consecutive_bonus_applied() {
        // "sess" in "sessions" is consecutive
        let consecutive = fuzzy_match("sessions", "sess").unwrap();
        // "sns" in "sessions" is not consecutive
        let non_consecutive = fuzzy_match("sessions", "sns").unwrap();
        assert!(consecutive.score > non_consecutive.score);
    }

    #[test]
    fn start_bonus_applied() {
        // "quit" at start
        let at_start = fuzzy_match("quit now", "quit").unwrap();
        // "quit" not at start
        let not_at_start = fuzzy_match("please quit", "quit").unwrap();
        assert!(at_start.score > not_at_start.score);
    }

    #[test]
    fn word_boundary_bonus() {
        // "cmd" matches at word boundaries in "my-cmd-here"
        let boundary_match = fuzzy_match("my-cmd-here", "cmd").unwrap();
        assert!(boundary_match.score > 0);
    }

    #[test]
    fn shorter_candidate_scores_higher() {
        let short = fuzzy_match("quit", "q").unwrap();
        let long = fuzzy_match("quite-long-name-here", "q").unwrap();
        assert!(short.score > long.score);
    }

    #[test]
    fn fuzzy_skips_characters() {
        // "qt" matches "quit" by skipping 'u' and 'i'
        let result = fuzzy_match("quit", "qt").unwrap();
        assert_eq!(result.indices, vec![0, 3]);
    }

    #[test]
    fn unicode_handling() {
        let result = fuzzy_match("héllo world", "hw").unwrap();
        assert_eq!(result.indices, vec![0, 7]);
    }

    #[test]
    fn consecutive_bonus_uses_byte_offsets_for_multibyte_candidates() {
        let unicode = fuzzy_match("éab", "ab").unwrap();
        let ascii = fuzzy_match("xab", "ab").unwrap();

        assert_eq!(unicode.indices, vec![2, 3]);
        assert_eq!(unicode.score, ascii.score);
    }

    #[test]
    fn word_boundary_bonus_uses_byte_offsets_for_multibyte_candidates() {
        let unicode = fuzzy_match("é-c", "-c").unwrap();
        let ascii = fuzzy_match("x-c", "-c").unwrap();

        assert_eq!(unicode.indices, vec![2, 3]);
        assert_eq!(unicode.score, ascii.score);
    }
}
