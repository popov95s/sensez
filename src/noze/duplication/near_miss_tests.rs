use super::detect;
use crate::config::model::Duplication;
use crate::noze::duplication::test_support::write_files;
use crate::report::CloneDetector;

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

#[test]
fn local_renamed_away_from_a_shared_property_name_still_pairs() {
    let tmp = tempfile::tempdir().unwrap();
    let body = |func: &str, owner: &str, local: &str| {
        format!(
            "export function {func}(shipment) {{\n  const {local} = shipment.{owner}?.zone ?? \"EU\";\n  if ({local} !== \"EU\") {{\n    shipment.fee = 0;\n  }}\n  return {{ fee: shipment.fee, {local} }};\n}}\n"
        )
    };
    let files = write_files(
        tmp.path(),
        &[
            ("a.ts", &body("feeFor", "address", "zone")),
            ("b.ts", &body("chargeFor", "target", "region")),
        ],
    );
    let refs: Vec<_> = files.iter().collect();

    assert_eq!(detect(&refs, 10).len(), 1);
}

fn ledger(func: &str, rows: &str, amount: &str, flag: &str) -> String {
    format!(
        "export function {func}(book) {{\n  let open = 0;\n  let closed = 0;\n  for (const row of book.{rows}) {{\n    const value = row.{amount} * row.rate;\n    if (row.{flag}) {{\n      closed += value;\n    }} else {{\n      open += value;\n    }}\n    if (value > book.limit) {{\n      book.alerts.push({{ id: row.id, value }});\n    }}\n  }}\n  const ratio = closed / Math.max(open + closed, 1);\n  const label = ratio > 0.5 ? book.labels.high : book.labels.low;\n  book.audit.record(book.id, {{ open, closed, label }});\n  if (book.alerts.length > book.maxAlerts) {{\n    book.audit.flag(book.id, book.alerts.length);\n  }}\n  if (ratio < book.floor) {{\n    throw new Error(`book ${{book.id}} is under its floor`);\n  }}\n  return {{ open, closed, ratio, alerts: book.alerts.length }};\n}}\n"
    )
}

#[test]
fn long_member_renamed_copy_is_reported_at_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let files = write_files(
        tmp.path(),
        &[
            ("a.ts", &ledger("ledgerTotals", "rows", "amount", "settled")),
            ("b.ts", &ledger("journalTotals", "lines", "sum", "booked")),
        ],
    );
    let config = Duplication {
        exclude: vec![],
        ..Default::default()
    };

    let clones = crate::noze::duplication::detect(&files, &config);

    assert!(
        clones.iter().any(|c| c.detector == CloneDetector::NearMiss
            || c.absorbed
                .iter()
                .any(|a| a.detector == CloneDetector::NearMiss)),
        "{clones:?}"
    );
}
