#!/usr/bin/env python3
"""Create tiny local repos for the Sensez TinyForms A/B eval."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import textwrap
from pathlib import Path


TASK_TESTS = {
    "tinyforms-import-cleanup": '''
        import unittest
        from pathlib import Path
        import sys

        sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

        from tinyforms.transforms import prepare_import_rows


        class PrepareImportRowsTests(unittest.TestCase):
            def test_remaps_trims_and_skips_empty_rows(self):
                rows = [
                    {"Full Name": " Ada ", "Email": " ada@example.com ", "Unused": "x"},
                    {"Full Name": "   ", "Email": "  "},
                    {"Full Name": "Grace", "Email": ""},
                ]
                cleaned, skipped = prepare_import_rows(
                    rows,
                    {"Full Name": "name", "Email": "email"},
                    trim=True,
                )
                self.assertEqual(cleaned, [{"name": "Ada", "email": "ada@example.com"}, {"name": "Grace"}])
                self.assertEqual(skipped, 1)


        if __name__ == "__main__":
            unittest.main()
    ''',
    "tinyforms-request-cleanup": '''
        import unittest
        from pathlib import Path
        import sys

        sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

        from tinyforms.sanitize import sanitize_request_data


        class SanitizeRequestDataTests(unittest.TestCase):
            def test_filters_trims_and_drops_empty_values(self):
                data = {"name": " Ada ", "email": " ", "role": "admin", "extra": "x"}
                cleaned = sanitize_request_data(data, {"name", "email", "role"}, strip_strings=True)
                self.assertEqual(cleaned, {"name": "Ada", "role": "admin"})


        if __name__ == "__main__":
            unittest.main()
    ''',
    "tinyforms-report-summary": '''
        import unittest
        from pathlib import Path
        import sys

        sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

        from tinyforms.reporting import summarize_validation_report


        class ValidationReportTests(unittest.TestCase):
            def test_formats_summary_with_defaults(self):
                report = {
                    "form_name": "Signup",
                    "total_fields": 4,
                    "valid_fields": 3,
                    "invalid_fields": 1,
                    "non_field_errors": 2,
                    "elapsed_ms": 18.4,
                }
                self.assertEqual(
                    summarize_validation_report(report),
                    "Signup: 3/4 valid, 1 invalid, 2 form errors in 18.4ms",
                )
                self.assertEqual(
                    summarize_validation_report({"form_name": "Blank"}),
                    "Blank: 0/0 valid, 0 invalid, 0 form errors in 0.0ms",
                )


        if __name__ == "__main__":
            unittest.main()
    ''',
}


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(textwrap.dedent(text).lstrip())


def create_repo(task_id: str, workspace: Path) -> None:
    if task_id not in TASK_TESTS:
        raise SystemExit(f"unknown task id: {task_id}")
    if workspace.exists():
        shutil.rmtree(workspace)
    workspace.mkdir(parents=True)

    write(
        workspace / "README.md",
        """
        # TinyForms

        A deliberately small form helper package used by Sensez synthetic evals.
        """,
    )
    write(workspace / "src/tinyforms/__init__.py", "")
    write(
        workspace / "src/tinyforms/transforms.py",
        """
        def prepare_import_rows(rows, field_map=None, trim=True):
            raise NotImplementedError
        """,
    )
    write(
        workspace / "src/tinyforms/sanitize.py",
        """
        def sanitize_request_data(data, allowed_fields, strip_strings=True):
            raise NotImplementedError
        """,
    )
    write(
        workspace / "src/tinyforms/reporting.py",
        """
        def summarize_validation_report(report):
            raise NotImplementedError
        """,
    )
    write(workspace / "tests/test_task.py", TASK_TESTS[task_id])
    write(
        workspace / "sensez.toml",
        """
        [smells]
        split_variable = true
        split_variable_min_assigns = 2
        """,
    )

    subprocess.run(["git", "init"], cwd=workspace, check=True, capture_output=True)
    subprocess.run(["git", "add", "."], cwd=workspace, check=True, capture_output=True)
    subprocess.run(
        [
            "git",
            "-c",
            "user.name=Sensez Eval",
            "-c",
            "user.email=sensez-eval@example.invalid",
            "commit",
            "-m",
            "initial tinyforms fixture",
        ],
        cwd=workspace,
        check=True,
        capture_output=True,
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("task_id")
    parser.add_argument("workspace", type=Path)
    args = parser.parse_args()
    create_repo(args.task_id, args.workspace)


if __name__ == "__main__":
    main()
