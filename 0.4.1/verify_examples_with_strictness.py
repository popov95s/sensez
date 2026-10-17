"""Strictness-aware docs example verification helpers."""

from __future__ import annotations

import shutil
import tempfile
from itertools import product
from pathlib import Path

BLOCKS = ("cycles", "dead_code", "boundaries", "duplication")
LANGS = ((".py", "python"), (".ts", "typescript"))


def verify_examples_with_strictness(
    folder,
    config,
    suffixes,
    expected_kind,
    rule,
    run_noze,
    build_scan_root,
):
    failures = variant_failures(folder, config, suffixes, expected_kind, rule, run_noze)
    fixed_reports = tuple(
        fixed_report(folder, suffix, run_noze, build_scan_root) for suffix in suffixes
    )
    for suffix, report in fixed_reports:
        failures.extend(fixed_failures(report, folder.name, suffix))
    return failures


def variant_failures(folder, config, suffixes, expected_kind, rule, run_noze):
    failures = []
    for level, suffix in product(rule.levels, suffixes):
        bad = build_variant_root(folder, config, rule, level, suffix)
        try:
            report = run_noze(bad)
            if not finding_seen(report, folder.name, level.value, suffix, expected_kind):
                failures.append(
                    missing_message(
                        report,
                        folder.name,
                        level.value,
                        suffix,
                        expected_kind,
                    ),
                )
        finally:
            shutil.rmtree(bad, ignore_errors=True)
    return failures


def build_variant_root(folder, config, rule, level, suffix):
    tmp = Path(
        tempfile.mkdtemp(prefix=f"sensez-{folder.name}-{level.value}-{suffix[1:]}-"),
    )
    text = (
        config.read_text()
        + f"\n[smells.{lang_for_suffix(suffix)}.rules.{folder.name}]\n"
        + f'{rule.knob} = "{level.value}"\n'
    )
    (tmp / "sensez.toml").write_text(text)
    src = folder / level.value / f"example{suffix}"
    dst = tmp / folder.name / level.value / f"example{suffix}"
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(src, dst)
    return tmp


def lang_for_suffix(suffix):
    return next(lang for ext, lang in LANGS if ext == suffix)


def fixed_report(folder, suffix, run_noze, build_scan_root):
    fixed = build_scan_root(folder, "fixed", suffix)
    try:
        return suffix, run_noze(fixed)
    finally:
        shutil.rmtree(fixed, ignore_errors=True)


def finding_seen(report, box, variant, suffix, expected_kind):
    wanted = f"/{box}/{variant}/example{suffix}"
    return any(
        norm(finding.get("file", "")).endswith(wanted)
        and finding.get("kind") == expected_kind
        for finding in report.get("smells", [])
    )


def missing_message(report, box, variant, suffix, expected_kind):
    wanted = f"/{box}/{variant}/example{suffix}"
    seen = sorted(
        finding.get("kind")
        for finding in report.get("smells", [])
        if norm(finding.get("file", "")).endswith(wanted)
    )
    return f"{box}/{variant}/example{suffix}: expected {expected_kind}, saw {seen or 'no smells'}"


def fixed_failures(report, box, suffix):
    return fixed_smell_failures(report, box, suffix) + fixed_pillar_failures(report, box, suffix)


def fixed_smell_failures(report, box, suffix):
    wanted = f"/{box}/fixed{suffix}"
    kinds = sorted(
        finding.get("kind")
        for finding in report.get("smells", [])
        if norm(finding.get("file", "")).endswith(wanted)
    )
    return [] if not kinds else [f"{box}/fixed{suffix}: must emit no smell, saw {kinds}"]


def fixed_pillar_failures(report, box, suffix):
    return [
        f"{box}/fixed{suffix}: must emit no {block} pillar, saw {count} finding(s)"
        for block in BLOCKS
        if (count := fixed_pillar_count(report, block, box, suffix))
    ]


def fixed_pillar_count(report, block, box, suffix):
    wanted = f"/{box}/fixed{suffix}"
    return sum(path.endswith(wanted) for path in block_files(report, block))


def block_files(report, block):
    files = []
    for finding in report.get(block, []):
        if block == "cycles":
            files.extend(norm(edge.get("file", "")) for edge in finding.get("edges", []))
        elif block == "duplication":
            files.extend(norm(item.get("file", "")) for item in finding.get("occurrences", []))
        else:
            files.append(norm(finding.get("file", "")))
    return files


def norm(path):
    return path.replace("\\", "/")
