"""Render per-finding Sensez tuning snippets for generated docs."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from finding_types import SmellTerm
from rust_metadata import (
    LANGUAGE_ROWS,
    StrictnessRule,
    default_enabled,
    default_knob_values,
    strictness_rules,
)

ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / "docs/examples/smells"
EXAMPLE_LANGUAGES = (
    ("Python", "python", "py"),
    ("JS / TS", "ts", "ts"),
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


@dataclass(frozen=True)
class TuningTab:
    title: str
    knobs: tuple[tuple[str, str], ...]
    description: str


@dataclass(frozen=True)
class ExampleTab:
    title: str
    description: str
    problem: str
    fix_text: str
    fixed: str


def example_knob_value(smell_term: SmellTerm, key: str) -> str:
    return default_knob_values().get(f"{smell_term}.{key}", "10")


def render_tuning(smell_term: SmellTerm, knobs: list[str]) -> str:
    strictness_rule = strictness_rules().get(smell_term)
    if strictness_rule:
        return render_ranked_tuning(smell_term, strictness_rule)
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


def has_ranked_examples(smell_term: SmellTerm) -> bool:
    return smell_term in strictness_rules()


def render_ranked_examples(smell_term: SmellTerm, fixes) -> str:
    lines = []
    rule = strictness_rules()[smell_term]
    for title, fence, ext in EXAMPLE_LANGUAGES:
        lines.append(f'=== "{title}"\n\n')
        lines.extend(indent_block(render_ranked_example_tabs(smell_term, rule, fixes, fence, ext)))
    return "".join(lines)


def render_ranked_tuning(smell_term: SmellTerm, rule: StrictnessRule) -> str:
    return render_tabbed_tuning(smell_term, ranked_tuning_tabs(rule))


def ranked_tuning_tabs(rule: StrictnessRule) -> list[TuningTab]:
    return [
        TuningTab(
            title=level.title,
            knobs=((rule.knob, f'"{level.value}"'),),
            description=level.description,
        )
        for level in rule.levels
    ]


def read_ranked_example(smell_term: SmellTerm, level: str, ext: str) -> str:
    return (EXAMPLES / smell_term / level / f"example.{ext}").read_text().rstrip()


def render_tabbed_tuning(smell_term: SmellTerm, tabs: list[TuningTab]) -> str:
    heading = [
        "**Tune It**\n\n",
        "Replace `<lang>` with `python`, `javascript`, `typescript`, or `rust`.\n\n",
    ]
    body = [line for tab in tabs for line in render_tuning_tab(smell_term, tab)]
    return "".join([*heading, *body, render_default_states(smell_term)])


def render_tuning_tab(smell_term: SmellTerm, tab: TuningTab) -> list[str]:
    lines = [f'=== "{tab.title}"\n\n']
    lines.extend(indent_block(render_toml_example(smell_term, tab.knobs)))
    lines.append(f"    {tab.description}\n\n")
    return lines


def render_ranked_example_tabs(
    smell_term: SmellTerm,
    rule: StrictnessRule,
    fixes,
    fence: str,
    ext: str,
) -> list[str]:
    tabs = [
        ExampleTab(
            title=level.title,
            description=level.description,
            problem=read_ranked_example(smell_term, level.value, ext),
            fix_text=fixes[language_key(ext)],
            fixed=read_fixed_example(smell_term, ext),
        )
        for level in rule.levels
    ]
    return [line for tab in tabs for line in render_example_tab(fence, tab)]


def render_example_tab(fence: str, tab: ExampleTab) -> list[str]:
    content = [
        f'=== "{tab.title}"\n\n',
        f"    {tab.description}\n\n",
        "    **Problem**\n\n",
        *indent_block(render_code_block(fence, tab.problem)),
        "    <details class=\"sensez-proposed-fix\" markdown=\"1\">\n",
        "    <summary>Proposed fix</summary>\n\n",
        *indent_markdown(tab.fix_text),
        "\n",
        *indent_block(render_code_block(fence, tab.fixed)),
        "    </details>\n\n",
    ]
    return content


def read_fixed_example(smell_term: SmellTerm, ext: str) -> str:
    return (EXAMPLES / smell_term / f"fixed.{ext}").read_text().rstrip()


def language_key(ext: str) -> str:
    return "python" if ext == "py" else "typescript"


def render_toml_example(smell_term: SmellTerm, knobs: tuple[tuple[str, str], ...]) -> list[str]:
    lines = [
        "```toml\n",
        f"[smells.<lang>.rules.{smell_term}]\n",
        "enabled = true\n",
        'action = "warning"\n',
    ]
    lines.extend(f"{key} = {value}\n" for key, value in knobs)
    lines.append("```\n\n")
    return lines


def render_code_block(language: str, code: str) -> list[str]:
    lines = [f"```{language}\n"]
    lines.extend(f"{line}\n" for line in code.rstrip().splitlines())
    lines.append("```\n\n")
    return lines


def indent_block(lines: list[str]) -> list[str]:
    return [f"    {line}" if line.strip() else line for line in lines]


def indent_markdown(markdown: str) -> list[str]:
    return [f"    {line}\n" if line else "\n" for line in markdown.splitlines()]


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
