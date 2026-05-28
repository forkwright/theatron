use super::super::*;

#[test]
fn diff_stats_from_empty_slice_is_empty() {
    let stats = DiffStats::from_files(&[]);
    assert!(stats.is_empty());
    assert_eq!(stats.files_changed, 0);
    assert_eq!(stats.additions, 0);
    assert_eq!(stats.deletions, 0);
}

#[test]
fn diff_stats_sums_across_multiple_files() {
    let files = vec![
        DiffFile {
            path: "a.rs".to_string(),
            hunks: Vec::new(),
            additions: 10,
            deletions: 3,
            mode: DiffViewMode::Unified,
        },
        DiffFile {
            path: "b.rs".to_string(),
            hunks: Vec::new(),
            additions: 5,
            deletions: 7,
            mode: DiffViewMode::Unified,
        },
        DiffFile {
            path: "c.rs".to_string(),
            hunks: Vec::new(),
            additions: 0,
            deletions: 12,
            mode: DiffViewMode::Unified,
        },
    ];
    let stats = DiffStats::from_files(&files);
    assert_eq!(stats.files_changed, 3);
    assert_eq!(stats.additions, 15);
    assert_eq!(stats.deletions, 22);
}

#[test]
fn diff_stats_total_lines_changed_sums_additions_and_deletions() {
    let stats = DiffStats {
        files_changed: 1,
        additions: 100,
        deletions: 50,
    };
    assert_eq!(stats.total_lines_changed(), 150);
}

#[test]
fn diff_stats_saturates_on_overflow() {
    // Two u32::MAX additions sum to u32::MAX (saturating), not panic.
    let files = vec![
        DiffFile {
            path: "huge1".to_string(),
            hunks: Vec::new(),
            additions: u32::MAX,
            deletions: 0,
            mode: DiffViewMode::Unified,
        },
        DiffFile {
            path: "huge2".to_string(),
            hunks: Vec::new(),
            additions: u32::MAX,
            deletions: 0,
            mode: DiffViewMode::Unified,
        },
    ];
    let stats = DiffStats::from_files(&files);
    assert_eq!(stats.additions, u32::MAX);
}

#[test]
fn diff_stats_default_is_empty() {
    let stats = DiffStats::default();
    assert!(stats.is_empty());
}

#[test]
fn diff_stats_is_empty_returns_false_when_files_present() {
    let stats = DiffStats {
        files_changed: 1,
        additions: 0,
        deletions: 0,
    };
    assert!(!stats.is_empty());
}

#[test]
fn diff_file_stats_returns_single_file_aggregate() {
    let file = DiffFile {
        path: "src/main.rs".to_string(),
        hunks: vec![],
        additions: 17,
        deletions: 4,
        mode: DiffViewMode::Unified,
    };
    let stats = file.stats();
    assert_eq!(stats.files_changed, 1);
    assert_eq!(stats.additions, 17);
    assert_eq!(stats.deletions, 4);
}

#[test]
fn diff_file_stats_matches_from_files_for_single_element_slice() {
    let file = DiffFile {
        path: "src/main.rs".to_string(),
        hunks: vec![],
        additions: 42,
        deletions: 9,
        mode: DiffViewMode::SideBySide,
    };
    let from_method = file.stats();
    let from_aggregate = DiffStats::from_files(std::slice::from_ref(&file));
    assert_eq!(from_method, from_aggregate);
}

#[test]
fn diff_file_stats_zero_changes_still_counts_as_one_file() {
    // Even an empty file (0 additions, 0 deletions) counts as 1
    // file when stats() is called — the file existed in the diff.
    // is_empty() returns false because files_changed > 0.
    let file = DiffFile {
        path: "renamed.rs".to_string(),
        hunks: vec![],
        additions: 0,
        deletions: 0,
        mode: DiffViewMode::Unified,
    };
    let stats = file.stats();
    assert_eq!(stats.files_changed, 1);
    assert!(!stats.is_empty());
}

#[test]
fn diff_stats_net_change_is_signed_difference() {
    let grew = DiffStats {
        files_changed: 1,
        additions: 100,
        deletions: 30,
    };
    let shrank = DiffStats {
        files_changed: 1,
        additions: 5,
        deletions: 50,
    };
    let balanced = DiffStats {
        files_changed: 1,
        additions: 42,
        deletions: 42,
    };
    assert_eq!(grew.net_change(), 70);
    assert_eq!(shrank.net_change(), -45);
    assert_eq!(balanced.net_change(), 0);
}

#[test]
fn diff_stats_net_change_handles_saturated_bounds_without_overflow() {
    // additions == u32::MAX, deletions == 0 must fit in i64
    // (the whole point of returning i64).
    let huge = DiffStats {
        files_changed: 1,
        additions: u32::MAX,
        deletions: 0,
    };
    assert_eq!(huge.net_change(), i64::from(u32::MAX));

    let huge_neg = DiffStats {
        files_changed: 1,
        additions: 0,
        deletions: u32::MAX,
    };
    assert_eq!(huge_neg.net_change(), -i64::from(u32::MAX));
}

#[test]
fn diff_stats_is_pure_addition_returns_true_when_no_deletions() {
    let pure_add = DiffStats {
        files_changed: 3,
        additions: 100,
        deletions: 0,
    };
    let mixed = DiffStats {
        files_changed: 1,
        additions: 100,
        deletions: 1,
    };
    assert!(pure_add.is_pure_addition());
    assert!(!mixed.is_pure_addition());
}

#[test]
fn diff_stats_is_pure_deletion_returns_true_when_no_additions() {
    let pure_del = DiffStats {
        files_changed: 2,
        additions: 0,
        deletions: 50,
    };
    let mixed = DiffStats {
        files_changed: 1,
        additions: 1,
        deletions: 100,
    };
    assert!(pure_del.is_pure_deletion());
    assert!(!mixed.is_pure_deletion());
}

#[test]
fn diff_stats_empty_is_both_pure_addition_and_pure_deletion() {
    // Vacuous truth: no lines means every line (of which there are
    // none) is an addition AND every line is a deletion.
    let empty = DiffStats::default();
    assert!(empty.is_pure_addition());
    assert!(empty.is_pure_deletion());
    assert_eq!(empty.net_change(), 0);
}

#[test]
fn change_type_is_add_returns_true_only_for_add() {
    assert!(ChangeType::Add.is_add());
    assert!(!ChangeType::Remove.is_add());
    assert!(!ChangeType::Context.is_add());
}

#[test]
fn change_type_is_remove_returns_true_only_for_remove() {
    assert!(ChangeType::Remove.is_remove());
    assert!(!ChangeType::Add.is_remove());
    assert!(!ChangeType::Context.is_remove());
}

#[test]
fn change_type_is_context_returns_true_only_for_context() {
    assert!(ChangeType::Context.is_context());
    assert!(!ChangeType::Add.is_context());
    assert!(!ChangeType::Remove.is_context());
}

#[test]
fn change_type_is_change_inverts_is_context() {
    assert!(ChangeType::Add.is_change());
    assert!(ChangeType::Remove.is_change());
    assert!(!ChangeType::Context.is_change());
}

#[test]
fn change_type_predicates_form_an_exhaustive_partition() {
    // Exactly one of is_add / is_remove / is_context is true for
    // any given variant. If a fourth variant is added, this
    // count would change and surface the ambiguity.
    for change in [ChangeType::Add, ChangeType::Remove, ChangeType::Context] {
        let count = u32::from(change.is_add())
            + u32::from(change.is_remove())
            + u32::from(change.is_context());
        assert_eq!(count, 1, "exactly one predicate true for {change:?}");
    }
}

#[test]
fn change_type_glyph_returns_canonical_unified_diff_prefix() {
    assert_eq!(ChangeType::Add.glyph(), '+');
    assert_eq!(ChangeType::Remove.glyph(), '-');
    assert_eq!(ChangeType::Context.glyph(), ' ');
}

#[test]
fn change_type_glyph_round_trips_through_predicates() {
    // Every variant's glyph round-trips with its predicate: the
    // glyph maps 1:1 to the variant identity.
    for change in [ChangeType::Add, ChangeType::Remove, ChangeType::Context] {
        match change.glyph() {
            '+' => assert!(change.is_add()),
            '-' => assert!(change.is_remove()),
            ' ' => assert!(change.is_context()),
            other => panic!("unexpected glyph {other:?} for {change:?}"),
        }
    }
}

#[test]
fn change_type_glyphs_are_unique() {
    // Every variant must produce a distinct glyph. If a future
    // variant is added that aliases an existing prefix, this
    // catches it.
    let glyphs: Vec<char> = [ChangeType::Add, ChangeType::Remove, ChangeType::Context]
        .iter()
        .map(|c| c.glyph())
        .collect();
    let mut sorted = glyphs.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        glyphs.len(),
        "glyphs must be unique: {glyphs:?}"
    );
}

#[test]
fn diff_view_mode_is_unified_returns_true_only_for_unified() {
    assert!(DiffViewMode::Unified.is_unified());
    assert!(!DiffViewMode::SideBySide.is_unified());
}

#[test]
fn diff_view_mode_is_side_by_side_returns_true_only_for_side_by_side() {
    assert!(DiffViewMode::SideBySide.is_side_by_side());
    assert!(!DiffViewMode::Unified.is_side_by_side());
}

#[test]
fn diff_view_mode_predicates_are_mutually_exclusive() {
    for mode in [DiffViewMode::Unified, DiffViewMode::SideBySide] {
        assert_ne!(mode.is_unified(), mode.is_side_by_side());
    }
}
