use super::*;
use crate::brainz::fingerprint::Print;

#[test]
fn partition_by_fingerprint_sinks_matching_items_keeping_order() {
    let print = |hash, noisy| {
        Print::identity(
            hash,
            fingerprint::Pillar::Smells,
            fingerprint::Label::Smell {
                smell: crate::report::SmellKind::LongFunction,
                file: String::new(),
                symbol: String::new(),
            },
            fingerprint::Detector::Smell {
                smell: if noisy {
                    crate::report::SmellKind::GodModule
                } else {
                    crate::report::SmellKind::LongFunction
                },
            },
        )
    };
    let mut items = vec!["a", "b", "c", "d"];
    let prints = vec![
        print(1, false),
        print(2, true),
        print(3, false),
        print(4, true),
    ];
    fingerprint::partition_by_fingerprint(
        &mut items,
        Some(&prints),
        |p| matches!(p.class, fingerprint::Detector::Smell { smell } if smell == crate::report::SmellKind::GodModule),
    );
    assert_eq!(items, vec!["a", "c", "b", "d"]);
}
