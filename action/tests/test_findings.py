import unittest
from pathlib import Path

from sensez_action.findings import flatten_duplication, should_fail


class FindingTests(unittest.TestCase):
    def test_flattens_duplication_occurrences_with_peer_message(self) -> None:
        report = {
            "duplication": [
                {
                    "action": "warning",
                    "token_length": 42,
                    "occurrences": [
                        {"file": "/repo/src/a.py", "start_row": 2, "end_row": 8},
                        {"file": "/repo/src/b.py", "start_row": 10, "end_row": 16},
                    ],
                }
            ]
        }

        findings = flatten_duplication(report, Path("/repo"))

        self.assertEqual(len(findings), 2)
        self.assertEqual(findings[0].file, "src/a.py")
        self.assertIn("src/b.py:10-16", findings[0].message)

    def test_filters_to_changed_occurrences_when_diff_lines_are_available(self) -> None:
        report = {
            "duplication": [
                {
                    "action": "warning",
                    "token_length": 42,
                    "occurrences": [
                        {"file": "/repo/src/a.py", "start_row": 2, "end_row": 8},
                        {"file": "/repo/src/b.py", "start_row": 10, "end_row": 16},
                    ],
                }
            ]
        }

        findings = flatten_duplication(report, Path("/repo"), {"src/a.py": {4}})

        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0].file, "src/a.py")

    def test_failure_level_uses_action_order(self) -> None:
        report = {"duplication": [{"action": "warning"}]}

        self.assertTrue(should_fail(report, "warning"))
        self.assertTrue(should_fail(report, "info"))
        self.assertFalse(should_fail(report, "must_fix"))
