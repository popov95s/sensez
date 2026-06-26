import unittest

from sensez_action.comments import _comment_line, _markers_in
from sensez_action.findings import Finding


class CommentTests(unittest.TestCase):
    def test_selects_first_changed_line_inside_finding(self) -> None:
        finding = Finding(
            "src/a.py", 10, 20, "message", 40, "warning", "<!-- sensez:x -->"
        )

        self.assertEqual(_comment_line(finding, {12, 14}), 12)

    def test_skips_comment_when_finding_does_not_touch_changed_lines(self) -> None:
        finding = Finding(
            "src/a.py", 10, 20, "message", 40, "warning", "<!-- sensez:x -->"
        )

        self.assertIsNone(_comment_line(finding, {25}))

    def test_extracts_existing_sensez_markers(self) -> None:
        body = "hello\n<!-- sensez:duplication:abc -->\nbye"

        self.assertEqual(_markers_in(body), ["<!-- sensez:duplication:abc -->"])
