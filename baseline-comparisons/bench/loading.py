"""Read benchmark artifacts from the results/ directory: the run log
(runs.jsonl) and Sensez' own JSON reports. The only layer that touches disk."""

import collections
import json
import os
from dataclasses import fields, replace

from .model import SensezFindings, Run

_RUN_FIELDS = {f.name for f in fields(Run)}


def load_runs(runs_path):
    """Latest `Run` per (target, tool), plus target order of first sighting.

    Returns ({(target, tool): Run}, [target, ...]). Unknown keys in a record
    are ignored and missing optional keys fall back to Run's defaults, so the
    log stays forward/backward compatible across schema tweaks.
    """
    if not os.path.exists(runs_path):
        return {}, []
    latest, order = {}, []
    with open(runs_path, encoding="utf-8") as fh:
        for raw in fh:
            line = raw.strip()
            if not line:
                continue
            rec = json.loads(line)
            run = Run(**{k: v for k, v in rec.items() if k in _RUN_FIELDS})
            run = _relocate_output(runs_path, run)
            if run.target not in order:
                order.append(run.target)
            key = (run.target, run.tool)
            if key not in latest or run.ts >= latest[key].ts:
                latest[key] = run
    return latest, order


def _relocate_output(runs_path, run):
    """Make old absolute artifact paths portable across checkout locations."""
    if os.path.exists(run.out):
        return run
    candidate = os.path.join(
        os.path.dirname(runs_path),
        run.target,
        os.path.basename(run.out),
    )
    return replace(run, out=candidate) if os.path.exists(candidate) else run


def load_sensez(out_path):
    """Parse one sensez JSON report into `SensezFindings`."""
    with open(out_path, encoding="utf-8") as fh:
        d = json.load(fh)
    dead = d["dead_code"]
    tiers = collections.Counter(f["confidence"] for f in dead)
    dead_set = {(_base(f["file"]), f["symbol"]) for f in dead}
    high_set = {(_base(f["file"]), f["symbol"]) for f in dead if f["confidence"] == "High"}
    cycles, dead = len(d["cycles"]), len(d["dead_code"])
    dup, smells = len(d["duplication"]), len(d.get("smells", []))
    return SensezFindings(
        cycles=cycles,
        dead=dead,
        dup=dup,
        smells=smells,
        total=cycles + dead + dup + smells,
        tiers={t: tiers.get(t, 0) for t in ("High", "Medium", "Low")},
        dead_set=dead_set,
        high_set=high_set,
    )


def _base(path):
    return os.path.basename(path)
