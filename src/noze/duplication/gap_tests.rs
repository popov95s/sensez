use super::detect;
use super::test_support::{pairs, write_files};
use crate::config::model::Duplication;

const HEAD: &str = "export function shipmentCost(shipment: Shipment) {\n  let weight = 0;\n  let surcharge = 0;\n  for (const parcel of shipment.parcels) {\n    const kg = parcel.kg * parcel.count;\n    weight += kg;\n";
const LOG: &str = "    console.debug(\"parcel\", shipment.code, kg);\n";
const LOOP: &str = "    if (parcel.fragile) {\n      surcharge += kg * 0.35;\n    }\n  }\n  const zone = shipment.address?.zone ?? \"EU\";\n";
const ZONE_CHECK: &str = "  if (zone !== \"EU\") {\n    surcharge = 0;\n  }\n";
const ZONE_SWAPPED: &str = "  if (zone === \"EU\") {\n    console.info(\"inland\");\n  } else {\n    surcharge = 0;\n  }\n";
const TAIL: &str = "  const total = Math.round((weight + surcharge) * 100) / 100;\n  if (total > 5000) {\n    throw new Error(`shipment ${shipment.code} needs a freight quote`);\n  }\n  return { weight, surcharge, total, zone };\n}\n";

fn defaults() -> Duplication {
    Duplication {
        exclude: vec![],
        ..Default::default()
    }
}

fn original() -> String {
    [HEAD, LOOP, ZONE_CHECK, TAIL].concat()
}

fn detected_pairs(edited: &[(&str, String)]) -> Vec<crate::report::CloneClass> {
    let tmp = tempfile::tempdir().unwrap();
    let original = original();
    let mut files = vec![("a.ts", original.as_str())];
    files.extend(edited.iter().map(|(name, body)| (*name, body.as_str())));
    detect(&write_files(tmp.path(), &files), &defaults())
}

#[test]
fn log_line_after_a_short_head_is_stitched_at_defaults() {
    let logged = [HEAD, LOG, LOOP, ZONE_CHECK, TAIL].concat();
    let clones = detected_pairs(&[("b.ts", logged)]);
    assert!(pairs(&clones, "a.ts", "b.ts"), "{clones:?}");
}

#[test]
fn swapped_if_else_joins_a_group_of_edited_copies_at_defaults() {
    let logged = [HEAD, LOG, LOOP, ZONE_CHECK, TAIL].concat();
    let swapped = [HEAD, LOOP, ZONE_SWAPPED, TAIL].concat();
    let clones = detected_pairs(&[("b.ts", logged), ("c.ts", original()), ("d.ts", swapped)]);
    assert!(pairs(&clones, "a.ts", "d.ts"), "{clones:?}");
}
