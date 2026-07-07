#!/usr/bin/env python3
"""Generate docs pages from the source metadata tables."""

from __future__ import annotations

import ast
import re
from dataclasses import dataclass
from pathlib import Path

from finding_groups import grouped_smells
from finding_types import Smell, SmellTerm
from rust_metadata import parse_rule_knobs
from tuning import render_tuning

ROOT = Path(__file__).resolve().parents[1]
GLOSSARY_RS = ROOT / "src/noze/glossary.rs"
FINDINGS_MD = ROOT / "docs/reference/findings.md"
EXAMPLES_ROOT = ROOT / "docs/examples/smells"
TAB_INDENT = "    "


@dataclass(frozen=True)
class RichDocs:
    why_bad: str
    lints: list[tuple[str, str]]
    examples: dict[str, tuple[Path, Path]]
    references: list[tuple[str, str]]
    fixes: dict[str, str]


def decode_rust_string(value: str) -> str:
    return ast.literal_eval(f'"{value}"')


def camel_to_snake(value: str) -> str:
    return re.sub(r"(?<!^)(?=[A-Z])", "_", value).lower()


def parse_smells() -> list[Smell]:
    pattern = re.compile(
        r'SmellDoc \{ kind: (\w+), title: "((?:\\.|[^"\\])*)", '
        r'explanation: "((?:\\.|[^"\\])*)" \},',
    )
    smells = []
    for line in GLOSSARY_RS.read_text().splitlines():
        if match := pattern.search(line):
            kind, title, explanation = match.groups()
            smells.append(
                Smell(
                    kind=kind,
                    term=SmellTerm(camel_to_snake(kind)),
                    title=decode_rust_string(title),
                    explanation=decode_rust_string(explanation),
                ),
            )
    return smells


def discover_examples(smell_kind: str) -> dict[str, tuple[Path, Path]]:
    """Auto-discover ``example.<ext>`` / ``fixed.<ext>`` pairs from disk.

    The src/ metadata no longer hard-codes docs/ paths; the docs script walks
    ``docs/examples/smells/<kind>/`` to find each language's bad+fixed pair.
    Missing pairs fall back to an empty template so a half-staged smell still
    renders without dropping section anchors.
    """
    folder = EXAMPLES_ROOT / smell_kind
    out: dict[str, tuple[Path, Path]] = {}
    for language, ext in (("python", "py"), ("typescript", "ts")):
        bad = folder / f"example.{ext}"
        fixed = folder / f"fixed.{ext}"
        if bad.exists() or fixed.exists():
            out[language] = (bad, fixed)
    return out


def parse_rich_docs() -> dict[str, RichDocs]:
    pattern = re.compile(
        (
            r"FindingDocs\s*\{\s*kind:\s*(\w+),\s*"
            r'why_bad:\s*"((?:\\.|[^"\\])*)",\s*'
            r"external_lints:\s*&\[(.*?)\],\s*"
            r"references:\s*&\[(.*?)\],\s*"
            r"fixes:\s*&\[(.*?)\],\s*\},"
        ),
        re.S,
    )
    rich = {}
    for match in pattern.finditer(GLOSSARY_RS.read_text()):
        kind, why_bad, lints, references, fixes = match.groups()
        rich[kind] = RichDocs(
            why_bad=decode_rust_string(why_bad),
            lints=parse_lints(lints),
            examples=discover_examples(str(SmellTerm(camel_to_snake(kind)))),
            references=parse_references(references),
            fixes=parse_language_blocks(fixes),
        )
    return rich


def parse_lints(source: str) -> list[tuple[str, str]]:
    pattern = re.compile(
        r'ExternalLint \{ tool: "((?:\\.|[^"\\])*)", rule: "((?:\\.|[^"\\])*)" \}',
    )
    return [
        (decode_rust_string(tool), decode_rust_string(rule))
        for tool, rule in pattern.findall(source)
    ]


def parse_language_blocks(source: str) -> dict[str, str]:
    pattern = re.compile(
        r'LanguageBlock \{\s*language: "((?:\\.|[^"\\])*)",\s*'
        r'body: "((?:\\.|[^"\\])*)",\s*\}',
        re.S,
    )
    return {
        decode_rust_string(language): decode_rust_string(body)
        for language, body in pattern.findall(source)
    }


def parse_references(source: str) -> list[tuple[str, str]]:
    named = {
        "RG_DATA_CLUMPS": (
            "Refactoring.Guru: Data Clumps",
            "https://refactoring.guru/smells/data-clumps",
        ),
        "RG_DIVERGENT_CHANGE": (
            "Refactoring.Guru: Divergent Change",
            "https://refactoring.guru/smells/divergent-change",
        ),
        "RG_FEATURE_ENVY": (
            "Refactoring.Guru: Feature Envy",
            "https://refactoring.guru/smells/feature-envy",
        ),
        "RG_INAPPROPRIATE_INTIMACY": (
            "Refactoring.Guru: Inappropriate Intimacy",
            "https://refactoring.guru/smells/inappropriate-intimacy",
        ),
        "RG_LARGE_CLASS": (
            "Refactoring.Guru: Large Class",
            "https://refactoring.guru/smells/large-class",
        ),
        "RG_LONG_METHOD": (
            "Refactoring.Guru: Long Method",
            "https://refactoring.guru/smells/long-method",
        ),
        "RG_LONG_PARAMETER_LIST": (
            "Refactoring.Guru: Long Parameter List",
            "https://refactoring.guru/smells/long-parameter-list",
        ),
        "RG_MESSAGE_CHAINS": (
            "Refactoring.Guru: Message Chains",
            "https://refactoring.guru/smells/message-chains",
        ),
        "RG_REFUSED_BEQUEST": (
            "Refactoring.Guru: Refused Bequest",
            "https://refactoring.guru/smells/refused-bequest",
        ),
        "RG_SHOTGUN_SURGERY": (
            "Refactoring.Guru: Shotgun Surgery",
            "https://refactoring.guru/smells/shotgun-surgery",
        ),
    }
    links = []
    for name in re.findall(r"\b[A-Z][A-Z0-9_]+\b", source):
        if name in named:
            links.append(named[name])
    pattern = re.compile(
        r'ReferenceLink \{ label: "((?:\\.|[^"\\])*)", url: "((?:\\.|[^"\\])*)" \}',
    )
    links.extend(
        (decode_rust_string(label), decode_rust_string(url))
        for label, url in pattern.findall(source)
    )
    return links


def render_lints(lints: list[tuple[str, str]]) -> str:
    lines = []
    for tool, rule in lints:
        label = "Ruff" if tool == "ruff" else "ESLint"
        lines.append(f"- {label}: `{rule}`")
    return "\n".join(lines)


def render_references_sentence(references: list[tuple[str, str]]) -> str:
    if not references:
        return ""
    links = ", ".join(f"[{label}]({url})" for label, url in references)
    return f" Related code-smell reference: {links}."


def indent_tab_line(line: str) -> str:
    if not line:
        return line
    return f"{TAB_INDENT}{line}"


def indent_tab_content(markdown: str) -> str:
    return "\n".join(indent_tab_line(line) for line in markdown.splitlines())


def read_example(path: Path) -> str:
    if not path.exists():
        return str()
    return path.read_text().rstrip()


def render_example_tabs(blocks: dict[str, tuple[Path, Path]]) -> str:
    labels = {"python": "Python", "typescript": "JS / TS"}
    fences = {"python": "python", "typescript": "ts"}
    out = []
    for language in ("python", "typescript"):
        if language not in blocks:
            continue
        paths = blocks[language]
        bad_path, fixed_path = paths
        bad = read_example(bad_path)
        fixed = read_example(fixed_path)
        content = (
            "**Problem**\n\n"
            f"```{fences[language]}\n{bad}\n```\n\n"
            '<details class="sensez-proposed-fix" markdown="1">\n'
            "<summary>Proposed fix</summary>\n\n"
            f"{blocks_to_fix_text(language)}\n\n"
            f"```{fences[language]}\n{fixed}\n```\n"
            "</details>"
        )
        out.append(f'=== "{labels[language]}"\n')
        out.append(f"{indent_tab_content(content)}\n")
    return "\n".join(out)


def blocks_to_fix_text(language: str) -> str:
    return "{{FIX_TEXT:" + language + "}}"


def render_example_and_fix_tabs(doc: RichDocs) -> str:
    rendered = render_example_tabs(doc.examples)
    for language, body in doc.fixes.items():
        rendered = rendered.replace(blocks_to_fix_text(language), body)
    return rendered


def render_optional_lints(lints: list[tuple[str, str]]) -> str:
    if not lints:
        return ""
    return f"**External linter coverage**\n\n{render_lints(lints)}\n\n"


def render_findings() -> str:
    smells = parse_smells()
    rich = parse_rich_docs()
    knobs = parse_rule_knobs()
    missing = [smell.kind for smell in smells if smell.kind not in rich]
    if missing:
        raise SystemExit(f"missing rich docs for: {', '.join(missing)}")

    out = [
        "# Finding Reference\n\n",
        "This page is generated by `docs/generate.py` from `src/noze/glossary.rs`.\n\n",
        '<div id="sensez-findings-tabs" hidden></div>\n\n',
    ]
    for group_title, group in grouped_smells(smells):
        out.append(f"## {group_title}\n\n")
        for smell in group:
            doc = rich[smell.kind]
            out.extend(
                [
                    f"### {smell.title} (`{smell.term}`)\n\n",
                    "**What it is**\n\n",
                    f"{smell.explanation}{render_references_sentence(doc.references)}\n\n",
                    "**Why it's bad**\n\n",
                    f"{doc.why_bad}\n\n",
                    "**Example**\n\n",
                    render_example_and_fix_tabs(doc),
                    "\n",
                    render_tuning(smell.term, knobs.get(smell.term, [])),
                    render_optional_lints(doc.lints),
                ],
            )
    return "".join(out)


def main() -> None:
    FINDINGS_MD.write_text(render_findings())


if __name__ == "__main__":
    main()
