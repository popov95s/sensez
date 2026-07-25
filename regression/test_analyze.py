import json
import tempfile
import unittest
from pathlib import Path

from regression.analyze import compare_tree


class CompareTreeTests(unittest.TestCase):
    def test_ignores_json_whitespace(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            results = root / "results"
            baselines = root / "baselines"
            results.mkdir()
            baselines.mkdir()
            (results / "report.json").write_text('{"tools": ["scan"]}\n')
            (baselines / "report.json").write_text('{\n  "tools": ["scan"]\n}')

            self.assertEqual(compare_tree(results, baselines), ())

    def test_reports_semantic_json_changes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            results = root / "results"
            baselines = root / "baselines"
            results.mkdir()
            baselines.mkdir()
            (results / "report.json").write_text(json.dumps({"tools": ["scan"]}))
            (baselines / "report.json").write_text(json.dumps({"tools": ["search"]}))

            self.assertEqual(len(compare_tree(results, baselines)), 1)
