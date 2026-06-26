import unittest

from sensez_action.diff import added_lines, changed_lines_from_git_diff


class DiffTests(unittest.TestCase):
    def test_parses_added_lines_from_unified_patch(self) -> None:
        patch = "\n".join(
            [
                "@@ -2,2 +2,4 @@",
                " keep",
                "+added",
                "-removed",
                " context",
                "+again",
            ]
        )

        self.assertEqual(added_lines(patch), [3, 5])

    def test_parses_changed_lines_from_git_diff(self) -> None:
        diff = "\n".join(
            [
                "diff --git a/src/a.py b/src/a.py",
                "--- a/src/a.py",
                "+++ b/src/a.py",
                "@@ -1,0 +2,2 @@",
                "+one",
                "+two",
            ]
        )

        self.assertEqual(changed_lines_from_git_diff(diff), {"src/a.py": {2, 3}})
