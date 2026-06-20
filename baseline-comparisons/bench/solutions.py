"""The solution registry and per-tool strategies.

Each tool is one `Solution` pairing a label with a `judge`: a function that
reads the tool's saved output and returns a `Verdict` (its count + how it
relates to sensez). Adding a tool means appending one `Solution` here — no
other module changes. Output parsing (one function per format) is separated
from the judging strategy so parsers can be reused and tested independently.
"""

import json
import os
import re
from collections.abc import Callable
from dataclasses import dataclass

from .model import SensezFindings, Verdict

# --------------------------------------------------------------------------- #
# Output parsers — one per tool output format. Each returns None when absent.
# --------------------------------------------------------------------------- #


def vulture_funcs(path):
    """vulture text → (set of (basename, symbol) for unused func/class, total lines)."""
    if not os.path.exists(path):
        return set(), 0
    funcs, total = set(), 0
    rx = re.compile(r"^(.*?):\d+: unused (?:function|class) '([^']+)'")
    with open(path, encoding="utf-8", errors="ignore") as fh:
        for raw in fh:
            ln = raw.strip()
            total += ": unused " in ln
            m = rx.match(ln)
            if m:
                funcs.add((os.path.basename(m.group(1)), m.group(2)))
    return funcs, total


def pycycle_cycle_count(path):
    """pycycle text → number of 'Cycle Found' banners."""
    if not os.path.exists(path):
        return 0
    return _read(path).count("Cycle Found")


def symilar_count(path):
    """symilar text → duplicate count (explicit field, else '==' marker count)."""
    if not os.path.exists(path):
        return None
    txt = _read(path)
    m = re.search(r"duplicates=(\d+)", txt)
    return int(m.group(1)) if m else (txt.count("==") or None)


def json_finding_count(path):
    """Finding count from a tool's JSON output, tolerant of shape and of a
    leading non-JSON header line (repowise prints one)."""
    if not os.path.exists(path):
        return None
    txt = _read(path)
    start = min((i for i in (txt.find("["), txt.find("{")) if i != -1), default=-1)
    if start == -1:
        return None
    try:
        data = json.loads(txt[start:])
    except json.JSONDecodeError:
        return None
    if isinstance(data, list):  # smellcheck, repowise → array of findings
        return len(data)
    if isinstance(data, dict):  # fallow → {total_issues: N, ...}
        for k in ("total_issues", "findings", "issues", "results"):
            v = data.get(k)
            if isinstance(v, int):
                return v
            if isinstance(v, list):
                return len(v)
    return None


def _read(path):
    with open(path, encoding="utf-8", errors="ignore") as fh:
        return fh.read()


def _pct(n, d):
    return f"{100 * n // d}%" if d else "n/a"


# --------------------------------------------------------------------------- #
# Judges — (sensez findings, output path) → Verdict. One strategy per parity kind.
# --------------------------------------------------------------------------- #


def _judge_vulture(f: SensezFindings, out):
    funcs, total = vulture_funcs(out)
    recall = len(f.dead_set & funcs)
    confirmed = len(f.high_set & funcs)
    return Verdict(
        f.dead,
        total,
        f"sensez catches {recall}/{len(funcs)} ({_pct(recall, len(funcs))}) of vulture "
        f"funcs/classes; High tier {f.tiers['High']} ({confirmed} vulture-confirmed)",
    )


def _judge_pycycle(f: SensezFindings, out):
    pyc = pycycle_cycle_count(out)
    if (f.cycles > 0) == (pyc > 0):
        note = f"both {'find' if f.cycles else 'find no'} cycles — agree"
    else:
        note = "differ — likely pycycle bailing on a large tree (it under-reports at scale)"
    return Verdict(f.cycles, pyc, note)


def _judge_symilar(f: SensezFindings, out):
    sym = symilar_count(out)
    return Verdict(
        f.dup,
        sym,
        "token-structural (rename-invariant) vs line-based — different granularity",
    )


def _cmp_judge(sensez_attr, note):
    """A judge for a JSON-emitting, same-pillar tool: compare its finding count
    to one sensez field, with a fixed caveat (counts are same-pillar, not 1:1)."""

    def judge(f: SensezFindings, out):
        count = json_finding_count(out)
        return Verdict(getattr(f, sensez_attr), count, note)

    return judge


@dataclass(frozen=True)
class Solution:
    """A benchmarked tool: its run-log key, display label, and judging strategy."""

    name: str
    label: str
    judge: Callable[[SensezFindings, str], Verdict]


# The registry. Order here is the order tools appear under each target.
SOLUTIONS = [
    Solution("vulture", "Dead code", _judge_vulture),
    Solution("pycycle", "Import cycles", _judge_pycycle),
    Solution("symilar", "Duplication", _judge_symilar),
    Solution(
        "smellcheck",
        "Design smells",
        _cmp_judge(
            "smells",
            "both detect code smells; different catalogs (SC-codes vs sensez families) — not 1:1",
        ),
    ),
    Solution(
        "repowise",
        "Dead code",
        _cmp_judge(
            "dead",
            "both detect dead code; repowise is file/export-level, sensez symbol-level across modules",
        ),
    ),
    Solution(
        "fallow",
        "Dead / structural",
        _cmp_judge(
            "dead",
            "fallow bundles unused code + cycles + deps; sensez_n shown is sensez dead-code only",
        ),
    ),
]
