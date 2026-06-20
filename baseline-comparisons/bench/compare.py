"""Transform recorded runs into comparison rows. Pure: no I/O beyond reading
Sensez' report via `loading`, no presentation concerns."""

from .loading import load_sensez
from .model import Comp, Row
from .solutions import SOLUTIONS


def build_rows(latest, order):
    """Build one `Row` per target that has an sensez run, each carrying a `Comp`
    for every solution present for that target."""
    rows = []
    for target in order:
        sensez = latest.get((target, "sensez"))
        if not sensez:
            continue
        findings = load_sensez(sensez.out)
        comps = []
        for comp in SOLUTIONS:
            run = latest.get((target, comp.name))
            if not run:
                continue
            v = comp.judge(findings, run.out)
            comps.append(
                Comp(
                    tool=comp.name,
                    label=comp.label,
                    secs=run.seconds,
                    sensez_n=v.sensez_n,
                    comp_n=v.comp_n,
                    parity=v.parity,
                )
            )
        rows.append(
            Row(
                target=target,
                lang=sensez.lang,
                files=sensez.files,
                lines=sensez.lines,
                path=sensez.path,
                sensez_secs=sensez.seconds,
                comps=comps,
            )
        )
    return rows
