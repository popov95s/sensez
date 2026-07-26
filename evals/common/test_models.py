"""Regression tests for typed evaluation JSON boundaries."""

from __future__ import annotations

import unittest

from models import ScanPayload
from quality_score import score_payload
from summary_models import MetricRow


class ScanPayloadTests(unittest.TestCase):
    def test_malformed_payload_degrades_to_empty_scan(self) -> None:
        payload = ScanPayload.parse({"duplication": None, "smells": "bad"})
        self.assertEqual(payload.duplication, ())
        self.assertEqual(payload.smells, ())

    def test_quality_preserves_new_existing_and_inherent_provenance(self) -> None:
        payload = ScanPayload.parse(
            {
                "dead_code": [
                    {"reason": "added_unreferenced"},
                    {"reason": "touched"},
                ],
                "duplication": [
                    {
                        "hint": "2 copies written in this change",
                        "token_length": 40,
                        "occurrences": [
                            {"file": "a.py", "start_row": 1},
                            {"file": "b.py", "start_row": 1},
                        ],
                    },
                    {
                        "token_length": 50,
                        "occurrences": [
                            {"file": "mirror.py", "start_row": 1},
                            {"file": "mirror.py", "start_row": 250},
                        ],
                    },
                ],
            }
        )
        score = score_payload(payload)
        self.assertEqual(score.by_pillar["dead_code"].new, 1)
        self.assertEqual(score.by_pillar["dead_code"].existing, 1)
        self.assertEqual(score.by_pillar["duplication"].new, 1)
        self.assertEqual(score.by_pillar["duplication"].inherent, 1)
        self.assertEqual(score.severity.clone_new_tokens, 40)
        self.assertEqual(score.severity.clone_inherent_tokens, 50)


class MetricRowTests(unittest.TestCase):
    def test_missing_historical_fields_receive_typed_defaults(self) -> None:
        row = MetricRow.parse({"task_id": "demo", "variant": "control"})
        self.assertEqual(row.task_id, "demo")
        self.assertEqual(row.counts("sensez_diff").total, 0)
        self.assertEqual(row.diff_stats.lines_added, 0)
        self.assertEqual(row.severity.clone_total_tokens, 0)


if __name__ == "__main__":
    unittest.main()
