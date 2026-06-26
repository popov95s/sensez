from __future__ import annotations

import os
import sys

from .annotations import annotate
from .comments import CommentError, post_comments
from .config import Config, ConfigError
from .findings import flatten_duplication, should_fail
from .runner import SensezError, run_sensez
from .summary import write_summary


def main() -> int:
    try:
        config = Config.from_env(os.environ)
        scan = run_sensez(config)
        findings = flatten_duplication(scan.report, config.workspace, scan.changed_lines)
        annotate(findings, config.level)
        if config.with_comments:
            post_comments(findings, config)
        write_summary(findings, config)
        if should_fail(scan.report, config.fail_on_new):
            print("Sensez found duplication at or above the configured failure level.")
            return 1
        return 0
    except (ConfigError, SensezError, CommentError) as error:
        print(f"::error title=Sensez action::{error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
