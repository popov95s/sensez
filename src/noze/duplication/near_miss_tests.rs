use super::detect;
use crate::noze::duplication::test_support::write_files;

fn renamed_pairs() -> Vec<(String, String)> {
    ["+", "-", "*", "/", "%"]
        .iter()
        .enumerate()
        .flat_map(|(i, op)| {
            let body = |owner: &str, method: &str| {
                format!(
                    "def total(items, extra):\n    value = {owner}.{method}(items)\n    return value {op} extra\n"
                )
            };
            [
                (format!("{i}a.py"), body("store", "load")),
                (format!("{i}b.py"), body("cache", "fetch")),
            ]
        })
        .collect()
}

#[test]
fn equal_weight_classes_come_out_in_the_same_order_every_run() {
    let tmp = tempfile::tempdir().unwrap();
    let sources = renamed_pairs();
    let borrowed: Vec<(&str, &str)> = sources
        .iter()
        .map(|(name, body)| (name.as_str(), body.as_str()))
        .collect();
    let files = write_files(tmp.path(), &borrowed);
    let refs: Vec<_> = files.iter().collect();
    let order = || -> Vec<_> {
        detect(&refs, 5)
            .into_iter()
            .map(|class| class.occurrences[0].file.clone())
            .collect()
    };

    let first = order();

    assert_eq!(first.len(), 5, "{first:?}");
    assert_eq!(first, order());
}
