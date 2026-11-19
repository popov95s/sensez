use super::*;

#[test]
fn every_smell_kind_has_a_distinct_title_and_explanation() {
    let titles: std::collections::BTreeSet<_> = ALL_SMELLS.iter().map(|k| k.title()).collect();
    assert_eq!(titles.len(), ALL_SMELLS.len(), "titles must be unique");
    for kind in ALL_SMELLS {
        assert!(!kind.explanation().is_empty());
        assert!(
            lookup(kind.as_str()).is_some(),
            "{} looks up",
            kind.as_str()
        );
    }
}

#[test]
fn lookup_resolves_pillars_and_unknown_is_none() {
    let cycles = match lookup("cycles") {
        Some(entry) => entry,
        None => panic!("cycles should resolve"),
    };
    assert_eq!(cycles.title, "Import Cycle");
    assert!(lookup("not_a_thing").is_none());
}

#[cfg(feature = "docs")]
#[test]
fn docs_metadata_covers_every_smell_kind() {
    for kind in ALL_SMELLS {
        let doc = docs::all().find(|doc| doc.kind == kind);
        assert!(doc.is_some(), "missing docs metadata for {}", kind.as_str());
        let doc = match doc {
            Some(doc) => doc,
            None => continue,
        };
        assert!(!doc.why_bad.is_empty());
        assert!(doc.fixes.iter().any(|block| block.language == "python"));
        assert!(doc.fixes.iter().any(|block| block.language == "typescript"));
    }
}
