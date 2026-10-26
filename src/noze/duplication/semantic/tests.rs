use super::*;

#[test]
fn repeated_method_name_uses_nearest_documentation() {
    let comments = FxHashMap::from_iter([
        (
            "m::First.run".to_string(),
            (
                2,
                DocKind::Comment,
                "Run the first scheduled job safely".to_string(),
            ),
        ),
        (
            "m::Second.run".to_string(),
            (
                21,
                DocKind::Comment,
                "Run the second scheduled job safely".to_string(),
            ),
        ),
    ]);
    let func = FunctionUnit {
        name: "run".into(),
        start_line: 22,
        end_line: 29,
        ..Default::default()
    };
    assert_eq!(comment_for(&comments, &func).unwrap().0, "m::Second.run");
}

#[test]
fn implementation_comment_is_not_function_documentation() {
    let comments = FxHashMap::from_iter([(
        "m::run".to_string(),
        (
            23,
            DocKind::Comment,
            "This comment describes an internal branch".to_string(),
        ),
    )]);
    let func = FunctionUnit {
        name: "run".into(),
        start_line: 20,
        end_line: 29,
        ..Default::default()
    };
    assert!(comment_for(&comments, &func).is_none());
}

#[test]
fn regression_documented_functions_form_a_candidate_without_internal_comment_noise() {
    use crate::spine::parser::parse_file;

    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("regression/fixtures/eyez");
    let files: Vec<_> = [
        "documented_a.js",
        "documented_b.js",
        "internal_comment_a.js",
        "internal_comment_b.js",
    ]
    .iter()
    .enumerate()
    .map(|(index, name)| parse_file(&fixture_root.join(name), index as u32).unwrap())
    .collect();
    let refs: Vec<_> = files.iter().collect();
    let units = collect_units(&refs, &SemanticDuplication::default(), 20);
    assert_eq!(units.len(), 2, "only leading function docs should qualify");
    assert_eq!(
        candidate_pairs(&units, &SemanticDuplication::default()).len(),
        1
    );
    assert!(units.iter().all(|unit| unit
        .doc
        .as_ref()
        .is_some_and(|doc| doc.text.contains("transport"))));
}

const TOTALS: &str = "function totals(orders) {
  const byDay = {};
  for (const order of orders) {
    const day = order.createdAt.slice(0, 10);
    byDay[day] = (byDay[day] ?? 0) + order.amount;
  }
  return Object.entries(byDay).map(([day, amount]) => ({ day, amount }));
}
";

const TOTALS_RENAMED: &str = "function sumByDay(rows) {
  const byDay = {};
  for (const row of rows) {
    const day = row.createdAt.slice(0, 7);
    byDay[day] = (byDay[day] ?? 0) + row.amount;
  }
  return Object.entries(byDay).map(([day, amount]) => ({ day, amount }));
}
";

const TOTALS_EDITED: &str = "function sumByDay(rows) {
  const byDay = {};
  for (const row of rows) {
    const day = row.createdAt.slice(0, 10);
    byDay[day] = (byDay[day] ?? 0) + row.amount;
  }
  delete byDay.unknown;
  return Object.entries(byDay).map(([day, amount]) => ({ day, amount }));
}
";

const TOTALS_GUARDED: &str = "function sumByDay(rows) {
  const byDay = {};
  for (const row of rows) {
    if (row.cancelled) continue;
    const day = row.createdAt.slice(0, 10);
    byDay[day] = (byDay[day] ?? 0) + row.amount;
  }
  return Object.entries(byDay).map(([day, amount]) => ({ day, amount }));
}
";

const CARD: &str = "function renderCard(user) {
  const title = document.createElement('h2');
  title.textContent = user.name;
  const card = document.createElement('div');
  card.className = 'card';
  card.appendChild(title);
  return card;
}
";

fn units_for(
    sources: &[(&str, &str)],
    config: &SemanticDuplication,
    threshold: usize,
) -> Vec<Unit> {
    let tmp = tempfile::tempdir().unwrap();
    let files = crate::noze::duplication::test_support::write_files(tmp.path(), sources);
    let refs: Vec<_> = files.iter().collect();
    collect_units(&refs, config, threshold)
}

fn undocumented() -> SemanticDuplication {
    SemanticDuplication {
        enabled: true,
        comment_required: false,
        ..SemanticDuplication::default()
    }
}

#[test]
fn edited_copy_scores_high_and_unrelated_code_low() {
    let units = units_for(
        &[
            ("a.js", TOTALS),
            ("b.js", TOTALS_EDITED),
            ("c.js", TOTALS_GUARDED),
            ("d.js", CARD),
        ],
        &undocumented(),
        20,
    );
    assert_eq!(units.len(), 4);
    let late_edit = jaccard(&units[0].shingles, &units[1].shingles);
    let early_edit = jaccard(&units[0].shingles, &units[2].shingles);
    let unrelated = jaccard(&units[0].shingles, &units[3].shingles);
    assert!(late_edit >= 0.7, "late edit scored {late_edit}");
    assert!(early_edit >= 0.7, "early edit scored {early_edit}");
    assert!(unrelated <= 0.3, "unrelated code scored {unrelated}");
}

#[test]
fn undocumented_units_get_no_fallback_doc() {
    let units = units_for(&[("a.js", TOTALS), ("b.js", CARD)], &undocumented(), 20);
    assert_eq!(units.len(), 2);
    assert!(units.iter().all(|unit| unit.doc.is_none()));
}

#[test]
fn undocumented_near_copy_is_labelled_without_docs() {
    let units = units_for(
        &[("a.js", TOTALS), ("b.js", TOTALS_RENAMED), ("c.js", CARD)],
        &undocumented(),
        20,
    );
    let found = findings(
        &units,
        candidate_pairs(&units, &undocumented()),
        &[None, None, None],
        &undocumented(),
    );
    assert_eq!(found.len(), 1);
    let hint = found[0].hint.as_deref().unwrap();
    assert!(
        hint.starts_with("semantic: edited copy (structure "),
        "{hint}"
    );
    assert!(hint.ends_with("docs n/a)"), "{hint}");
    assert!(!hint.contains("comment"), "{hint}");
}

#[test]
fn units_below_the_duplication_threshold_are_skipped() {
    let sources = [("a.js", TOTALS), ("b.js", TOTALS_EDITED)];
    assert_eq!(units_for(&sources, &undocumented(), 20).len(), 2);
    assert!(units_for(&sources, &undocumented(), 500).is_empty());
}

#[test]
fn only_overlapping_structure_or_doc_words_become_candidates() {
    let same_doc = "/** Sum the amount of every order grouped by calendar day. */\n";
    let other_doc = "/** Render a profile card element for the given user. */\n";
    let units = units_for(
        &[
            ("a.js", &format!("{same_doc}{TOTALS}")),
            ("b.js", &format!("{same_doc}{CARD}")),
            ("c.js", &format!("{other_doc}{TOTALS_EDITED}")),
        ],
        &SemanticDuplication::default(),
        20,
    );
    assert_eq!(units.len(), 3);
    let pairs = candidate_pairs(&units, &SemanticDuplication::default());
    assert_eq!(pairs, vec![(0, 1), (0, 2)]);
}

#[test]
fn documented_unit_paired_only_with_undocumented_code_is_not_embedded() {
    let units = units_for(
        &[
            (
                "a.js",
                &format!("/** Sum the amounts per day for a report. */\n{TOTALS}"),
            ),
            ("b.js", TOTALS_EDITED),
        ],
        &undocumented(),
        20,
    );
    let vectors = vectors_for(None, &units, &[(0, 1)]).unwrap();
    assert!(vectors.iter().all(Option::is_none));
}
