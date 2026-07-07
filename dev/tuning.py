"""Render per-finding Sensez tuning snippets for generated docs."""

from __future__ import annotations

from finding_types import SmellTerm
from rust_metadata import (
    LANGUAGE_ROWS,
    default_enabled,
    default_knob_values,
)

KNOB_COMMENTS = {
    "include_attributes": "also flag mutation of fields on parameters",
    "max_bool_params": "allowed boolean parameters before flagging",
    "max_chain_depth": "allowed property/call chain depth",
    "max_cognitive": "allowed cognitive complexity score",
    "max_cyclomatic": "allowed independent control-flow paths",
    "max_depth": "allowed message-chain depth",
    "max_lines": "allowed function or nested-function length",
    "max_methods": "allowed methods before a class is large",
    "max_nesting": "allowed nested block depth",
    "max_params": "allowed parameters before flagging",
    "max_returns": "allowed return statements before flagging",
    "max_tuple_return": "allowed positional tuple items in a return",
    "min_assigns": "assignments to one local before flagging",
    "min_blast": "dependent modules before shotgun-surgery risk",
    "min_fan": "combined incoming/outgoing fan before god-module risk",
    "min_fields": "shared fields required for a data clump",
    "min_keys": "string keys on one object before implicit-schema risk",
    "min_occurrences": "functions sharing the clump before flagging",
}


def example_knob_value(smell_term: SmellTerm, key: str) -> str:
    return default_knob_values().get(f"{smell_term}.{key}", "10")


def render_tuning(smell_term: SmellTerm, knobs: list[str]) -> str:
    lines = [
        "**Tune It**\n\n",
        "Replace `<lang>` with `python`, `javascript`, `typescript`, or `rust`.\n\n",
        "```toml\n",
        f"[smells.<lang>.rules.{smell_term}]\n",
        "enabled = true\n",
        'action = "warning"\n',
    ]
    for key in knobs:
        comment = KNOB_COMMENTS.get(key, "detector-specific threshold")
        lines.append(f"{key} = {example_knob_value(smell_term, key)} # {comment}\n")
    if not knobs:
        lines.append("# This detector has no extra threshold knobs.\n")
    lines.extend(["```\n\n", render_default_states(smell_term)])
    return "".join(lines)


def render_default_states(smell_term: SmellTerm) -> str:
    rows = []
    for languages, label in LANGUAGE_ROWS:
        enabled = {default_enabled(smell_term, language) for language in languages}
        state = "Yes" if enabled == {True} else "No"
        rows.append(f"<tr><td>{label}</td><td>{state}</td></tr>\n")
    return (
        '<details class="sensez-proposed-fix" markdown="1">\n'
        "<summary>Default enabled state</summary>\n\n"
        "<table>\n"
        "<thead><tr><th>Language</th><th>Enabled by default</th></tr></thead>\n"
        "<tbody>\n"
        f"{''.join(rows)}"
        "</tbody>\n"
        "</table>\n"
        "</details>\n\n"
    )
